use std::sync::{Arc, Mutex, RwLock, Condvar};
use std::sync::atomic::{Ordering, AtomicUsize};
use std::collections::VecDeque;
use std::time::Duration;
use std::error::Error;
use std::thread;

// use std::panic;

use super::*;

pub struct ArcWater {
    water: Arc<Water>,
}

pub struct Water {
    tasks: Mutex<VecDeque<Box<FnBox() + Send + 'static>>>,
    condvar: Condvar,
    threads: AtomicUsize,
    threads_waited: AtomicUsize,
    min: RwLock<usize>,
    max: RwLock<usize>,
    time_out: RwLock<Duration>,
    name: RwLock<Option<String>>,
    stack_size: RwLock<Option<usize>>,
    load_limit: RwLock<usize>,
    daemon: RwLock<Option<Duration>>,
}

impl Clone for ArcWater {
    fn clone(&self) -> Self {
        ArcWater { water: self.water.clone() }
    }
}
impl ArcWater {
    #[inline]
    pub fn num_cpus() -> usize {
        unsafe { NUM_CPUS }
    }
    pub fn new() -> Self {
        let num_cpus = num_cpus::get();
        if Self::num_cpus() != num_cpus {
            unsafe {
                NUM_CPUS = num_cpus;
            }
        }
        ArcWater {
            water: Arc::new(Water {
                tasks: Mutex::new(VecDeque::new()),
                condvar: Condvar::new(),
                threads: AtomicUsize::new(0),
                threads_waited: AtomicUsize::new(0),

                min: RwLock::new(Self::num_cpus() + 1),
                max: RwLock::new(std::usize::MAX),
                time_out: RwLock::new(Duration::from_millis(TIME_OUT_MS)),
                name: RwLock::new(None),
                stack_size: RwLock::new(None),
                load_limit: RwLock::new(Self::num_cpus() * Self::num_cpus()),
                daemon: RwLock::new(Some(Duration::from_millis(TIME_OUT_MS))),
            }),
        }
    }

    pub fn daemon(&self, daemon: Option<u64>) {
        self.water.daemon.rwlock(daemon.map(Duration::from_millis));
    }

    #[inline]
    pub fn get_daemon(self: &Self) -> Option<Duration> {
        self.water.daemon.rolock()
    }
    pub fn min(&self, min: usize) {
        self.water.min.rwlock(min);
    }
    #[inline]
    pub fn get_min(self: &Self) -> usize {
        self.water.min.rolock()
    }
    pub fn max(&self, max: usize) {
        self.water.max.rwlock(max);
    }
    #[inline]
    pub fn get_max(self: &Self) -> usize {
        self.water.max.rolock()
    }
    pub fn time_out(&self, time_out: u64) {
        self.water.time_out.rwlock(Duration::from_millis(time_out));
    }
    #[inline]
    pub fn get_time_out(self: &Self) -> Duration {
        self.water.time_out.rolock()
    }
    pub fn name<T: AsRef<str>>(&self, name: T)
        where T: std::fmt::Debug
    {
        self.water.name.rwlock(Some(name.as_ref().to_string()));
    }
    #[inline]
    pub fn get_name(&self) -> Option<String> {
        let ro_ = match self.water.name.read() {
            Ok(ok) => ok,
            Err(e) => e.into_inner(),
        };
        ro_.as_ref().cloned()
    }
    pub fn stack_size(&self, size: usize) {
        self.water.stack_size.rwlock(Some(size));

    }
    #[inline]
    pub fn get_stack_size(self: &Self) -> Option<usize> {
        self.water.stack_size.rolock()
    }
    pub fn load_limit(&self, load_limit: usize) {
        self.water.load_limit.rwlock(load_limit);
    }
    #[inline]
    pub fn get_load_limit(self: &Self) -> usize {
        self.water.load_limit.rolock()
    }
    pub fn run(&self) -> std::io::Result<()> {
        for _ in 0..self.get_min() {
            self.add_thread();
        }
        if self.get_daemon().is_some() {
            let arc_water = self.clone();
            thread::Builder::new().spawn(move || while let Some(s) = arc_water.get_daemon() {
                    thread::sleep(s);
                    dbstln!("Poolite_waits/threads/strong_count-1[2](before_daemon): {}/{}/{} \
                             ---tasks_queue: {} /daemon({:?})",
                            arc_water.wait_len(),
                            arc_water.len(),
                            arc_water.strong_count(),
                            arc_water.tasks_len(),
                            arc_water.get_daemon());
                    let min = arc_water.get_min();
                    let strong_count = arc_water.strong_count();
                    //'attempt to subtract with overflow'
                    let add_num = if min > strong_count {
                        min - strong_count
                    } else {
                        0
                    };
                    for _ in 0..add_num {
                        arc_water.add_thread();
                    }
                    if arc_water.strong_count() == 0 && arc_water.tasks_len() > 0 {
                        arc_water.add_thread();
                    }
                })?;
        }
        Ok(())
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        // All threads are waiting and tasks_queue'length is 0.
        self.wait_len() == self.len() && self.tasks_len() == 0
    }

    #[inline]
    pub fn tasks_len(&self) -> usize {
        match self.water.tasks.lock() {
            Ok(ok) => ok.len(),
            Err(e) => e.into_inner().len(),
        }
    }
    #[inline]
    pub fn strong_count(&self) -> usize {
        let one_two = if self.get_daemon().is_some() { 2 } else { 1 };
        Arc::strong_count(&self.water) - one_two
    }
    #[inline]
    pub fn len(&self) -> usize {
        (&self.water.threads).load(Ordering::Acquire)
    }
    #[inline]
    pub fn wait_len(&self) -> usize {
        (&self.water.threads_waited).load(Ordering::Acquire)
    }
    pub fn spawn(&self, task: Box<FnBox() + Send + 'static>) {
        let tasks_queue_len = {
            // 减小锁的作用域。
            let mut tasks_queue = match self.water.tasks.lock() {
                Ok(ok) => ok,
                Err(e) => e.into_inner(),            
            };
            dbstln!("Poolite_waits/threads/strong_count-1[2](before_spawn): {}/{}/{} \
                     ---tasks_queue:  {}",
                    self.wait_len(),
                    self.len(),
                    self.strong_count(),
                    tasks_queue.len());
            tasks_queue.push_back(task);
            tasks_queue.len()
        };
        // 因为创建的线程有延迟，所有用 strong_count()-1[2] (ArcWater本身和daemon各持有一个引用)更合适，否则会创建一堆线程(白白浪费内存，性能还差！)。
        // (&self.water.threads_waited).load(Ordering::Acquire) 在前性能好一些。
        // 注意min==0 且 load_limit>0 时,线程池里无线程则前 load_limit 个请求会一直阻塞。
        let count = self.strong_count();
        if count == 0 ||
           count <= self.get_max() && self.wait_len() == 0 &&
           tasks_queue_len / count > self.get_load_limit() {
            self.add_thread();
        } else {
            self.water.condvar.notify_one();
        }
    }

    pub fn add_thread(&self) {
        let arc_water = self.clone();
        // 线程命名。
        let mut thread = match self.get_name() {
            Some(name) => thread::Builder::new().name(name),
            None => thread::Builder::new(),
        };
        // 线程栈大小设置。
        thread = match self.get_stack_size() {
            Some(size) => thread.stack_size(size),
            None => thread,
        };
        // spawn 有延迟,必须等父线程阻塞才运行.
        let spawn_res = thread
            .spawn(move || {
                let arc_water= arc_water;
                // 对线程计数.
                let _threads_counter = Counter::add(&arc_water.water.threads);

                loop {
                    let task; //声明任务。
                    {
                    let mut tasks_queue = match arc_water.water.tasks.lock() {
                        Ok(ok) => ok,
                        Err(e) => e.into_inner(),            
                    };
                        // 移入内层loop=>解决全局锁问题；移出内层loop到单独的{}=>解决重复look()问题。
                    loop { 
                        if let Some(poped_task) = tasks_queue.pop_front() {
                            task = poped_task;// pop成功就break ,执行pop出的任务.
                            break;
                        }
                        // 对在等候的线程计数.
                        let _threads_waited_counter = Counter::add(&arc_water.water.threads_waited);

                        let (new_tasks_queue, waitres) =match arc_water.water.condvar
                                    .wait_timeout(tasks_queue, arc_water.get_time_out()) {
                                        Ok(ok)=>ok,
                                        Err(e)=>e.into_inner(),
                                    };
                        tasks_queue=new_tasks_queue;
                        // timed_out()为true时(等待超时是收不到通知就知道超时), 且队列空时销毁线程。
                        if waitres.timed_out() && tasks_queue.is_empty() && arc_water.len() >arc_water.get_min() { 
                            dbstln!("Poolite_waits/threads/strong_count-1[2](before_return): {}/{}/{} ---tasks_queue:  {}",
                                arc_water.wait_len(),
                                arc_water.len(),
                                arc_water.strong_count(),
                                 tasks_queue.len());
                            return; 
                            }
                    } // loop 取得任务结束。
                    }
                    // the trait `std::panic::UnwindSafe` is not implemented for `std::boxed::FnBox<(), Output=()> + std::marker::Send
                    // let run_res = panic::catch_unwind(|| {
                            task();/*执行任务。*/
                    // });
                } // loop 结束。
            }); //spawn 线程结束。

        if let Err(e) = spawn_res {
            errstln!("Poolite_Warnig: add thread failed because of '{}' !",
                     e.description());
        }
    }
    pub fn drop_pool(&mut self) {
        dbstln!("Pool_waits/threads/strong_count-1[2](before_drop): {}/{}/{} ---tasks_queue:  {}",
                self.wait_len(),
                self.len(),
                self.strong_count(),
                self.tasks_len());
        self.daemon(None);
        self.water.threads.store(usize::max_value(), Ordering::Release);
        self.water.condvar.notify_all();
    }
}

// 通过作用域对线程数目计数。
struct Counter<'a> {
    count: &'a AtomicUsize,
}

impl<'a> Counter<'a> {
    #[inline]
    fn add(count: &'a AtomicUsize) -> Counter<'a> {
        count.fetch_add(1, Ordering::Release);
        Counter { count: count }
    }
}

impl<'a> Drop for Counter<'a> {
    #[inline]
    fn drop(&mut self) {
        self.count.fetch_sub(1, Ordering::Release);
    }
}

trait RwLockRWlock<T> {
    fn rolock(self: &Self) -> T where T: Copy;
    fn rwlock(&self, content: T);
}
impl<T> RwLockRWlock<T> for RwLock<T> {
    #[inline]
    fn rolock(self: &Self) -> T
        where T: Copy //deref() lifetime does not enough in super::get*,so do not use &T.
    {
        let ro_ = match self.read() {
            Ok(ok) => ok,
            Err(e) => e.into_inner(),
        };
        *ro_
    }
    #[inline]
    fn rwlock(&self, content: T) {
        let mut rw_ = match self.write() {
            Ok(ok) => ok,
            Err(e) => e.into_inner(),
        };
        *rw_ = content;
    }
}
