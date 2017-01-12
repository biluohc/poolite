#![feature(fnbox)]
use std::boxed::FnBox;

#[macro_use]
extern crate stderr;
extern crate num_cpus;

use std::sync::{Arc, Mutex, RwLock, Condvar};
use std::sync::atomic::{Ordering, AtomicUsize};
use std::collections::VecDeque;
use std::time::Duration;
use std::error::Error;
use std::thread;
// use std::panic;

// 默认初始化线程数由num_cpus决定。
// 默认线程销毁超时时间 ms 。
const TIME_OUT_MS: u64 = 5_000;

pub struct Pool {
    water: Arc<Water>,
    name: Option<String>,
    stack_size: Option<usize>,
    load_limit: usize,
}

struct Water {
    tasks: Mutex<VecDeque<Box<FnBox() + Send + 'static>>>,
    condvar: Condvar,
    threads: AtomicUsize,
    threads_waited: AtomicUsize,
    min_timeout: RwLock<(usize, u64)>,
}

impl Pool {
    pub fn new() -> Pool {
        let cpus_num = num_cpus::get();
        Pool {
            water: Arc::new(Water {
                tasks: Mutex::new(VecDeque::new()),
                condvar: Condvar::new(),
                threads: AtomicUsize::new(0),
                threads_waited: AtomicUsize::new(0),
                min_timeout: RwLock::new((cpus_num + 1, TIME_OUT_MS)),
            }),
            name: None,
            stack_size: None,
            load_limit: cpus_num,
        }
    }
    pub fn min(self, min: usize) -> Pool {
        {
            let mut rw_min_timeout = match self.water.min_timeout.write() {
                Ok(ok) => ok,
                Err(e) => e.into_inner(),
            };
            // err!("min({}): {:?} -> ", min, *rw_min_timeout);
            let (.., timeout) = *rw_min_timeout;
            *rw_min_timeout = (min, timeout);
        }
        // errln!("{:?}", *self.water.min_timeout.read().unwrap());
        self
    }
    pub fn time_out(self, time_out: u64) -> Pool {
        {
            let mut rw_min_timeout = match self.water.min_timeout.write() {
                Ok(ok) => ok,
                Err(e) => e.into_inner(),
            };
            // err!("time_out({}): {:?} -> ", time_out, *rw_min_timeout);
            let (min, ..) = *rw_min_timeout;
            *rw_min_timeout = (min, time_out);
        }
        // errln!("{:?}", *self.water.min_timeout.read().unwrap());
        self
    }

    pub fn name<T: AsRef<str>>(mut self, name: T) -> Pool {
        self.name = Some(name.as_ref().to_string());
        self
    }
    pub fn stack_size(mut self, size: usize) -> Pool {
        self.stack_size = Some(size);
        self
    }
    pub fn load_limit(mut self, load_limit: usize) -> Pool {
        self.load_limit = load_limit;
        self
    }
    // 按理来说spawn够用了。对，不调用run也可以，只是开始反应会迟钝，因为线程还未创建。
    pub fn run(self) -> Pool {
        let ro_min = match self.water.min_timeout.read() {
            Ok(ok) => ok.0,
            Err(e) => e.into_inner().0,
        };
        for _ in 0..ro_min {
            self.add_thread();
        }
        self
    }
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
        Arc::strong_count(&self.water)
    }
    #[inline]
    pub fn len(&self) -> usize {
        (&self.water.threads).load(Ordering::Acquire)
    }
    #[inline]
    pub fn wait_len(&self) -> usize {
        (&self.water.threads_waited).load(Ordering::Acquire)
    }
    // task'panic look like could'not to let Mutex be PoisonError,and counter will work nomally.
    // pub fn once_panic(&self) -> bool {
    //     // task once panic
    //     self.water.tasks.is_poisoned()
    // }
    pub fn spawn(&self, task: Box<FnBox() + Send + 'static>) {
        let tasks_queue_len = {
            // 减小锁的作用域。
            let mut tasks_queue = match self.water.tasks.lock() {
                Ok(ok) => ok,
                Err(e) => e.into_inner(),            
            };
            // {
            //     errstln!("\nPool_waits/threads/strong_count-1(spawn): {}/{}/{} ---tasks_queue:  \
            //               {}",
            //              self.wait_len(),
            //              self.len(),
            //              self.strong_count() - 1,
            //              tasks_queue.len());
            // }
            tasks_queue.push_back(task);
            tasks_queue.len()
        };
        // 因为创建的线程有延迟，所有用 strong_count()-1 (pool本身持有一个引用)更合适，否则会创建一堆线程(白白浪费内存，性能还差！)。
        // (&self.water.threads_waited).load(Ordering::Acquire) 在前性能好一些。
        // 注意min==0 且 load_limit>0 时,线程池里无线程则前 load_limit 个请求会一直阻塞。
        if self.wait_len() == 0 && tasks_queue_len / self.strong_count() > self.load_limit + 1 {
            self.add_thread();
        } else {
            self.water.condvar.notify_one();
        }
    }

    fn add_thread(&self) {
        let water = self.water.clone();
        // 线程命名。
        let mut thread = match self.name.clone() {
            Some(name) => thread::Builder::new().name(name),
            None => thread::Builder::new(),
        };
        // 线程栈大小设置。
        thread = match self.stack_size {
            Some(size) => thread.stack_size(size),
            None => thread,
        };
        // spawn 有延迟,必须等父线程阻塞才运行.
        let spawn_res = thread
            .spawn(move || {
                let water = water;
                // 对线程计数.
                let _threads_counter = Counter::add(&water.threads);

                loop {
                    let task; //声明任务。
                    {
                    let mut tasks_queue = match water.tasks.lock() {
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
                        let _threads_waited_counter = Counter::add(&water.threads_waited);

                        let (ro_min,ro_time_out) = match water.min_timeout.read() {
                            Ok(ok) => *ok,
                            Err(e) => *e.into_inner(),
                        };
                        let (new_tasks_queue, waitres) =match  water.condvar
                                    .wait_timeout(tasks_queue, Duration::from_millis(ro_time_out)) {
                                        Ok(ok)=>ok,
                                        Err(e)=>e.into_inner(),
                                    };
                        tasks_queue=new_tasks_queue;
                        // timed_out()为true时(等待超时是收不到通知就知道超时), 且队列空时销毁线程。
                        if waitres.timed_out() && tasks_queue.is_empty() && (&water.threads).load(Ordering::Acquire) >ro_min { 
                            // {
                            //     errstln!("\nPool_waits/threads/strong_count-1(return): {}/{}/{} ---tasks_queue:  {}",
                            //     (&water.threads_waited).load(Ordering::Acquire),
                            //     (&water.threads).load(Ordering::Acquire),Arc::strong_count(&water)-1,
                            //     tasks_queue.len()); 
                            // }
                            return; 
                            }
                            // }
                    } // loop 取得任务结束。
                    }
                    // the trait `std::panic::UnwindSafe` is not implemented for `std::boxed::FnBox<(), Output=()> + std::marker::Send
                    // let run_res = panic::catch_unwind(|| {
                            task();/*执行任务。*/
                    // });
                } // loop 结束。
            }); //spawn 线程结束。

        match spawn_res {
            Ok(..) => {}
            Err(e) => {
                errstln!("Poolite_Warnig: create new thread failed because of '{}' !",
                         e.description())
            }

        };
    }
}
impl Drop for Pool {
    fn drop(&mut self) {
        // {
        //     errstln!("\nPool_waits/threads/strong_count-1(drop): {}/{}/{} ---tasks_queue:  {}",
        //              self.wait_len(),
        //              self.len(),
        //              self.strong_count() - 1,
        //              self.tasks_len());
        // }
        // 如果线程总数>线程最小限制且waited_out且任务栈空,则线程销毁.
        self.water.threads.store(usize::max_value(), Ordering::Release);
        self.water.condvar.notify_all();
    }
}

// 通过作用域对线程数目计数。
struct Counter<'a> {
    count: &'a AtomicUsize,
}

impl<'a> Counter<'a> {
    fn add(count: &'a AtomicUsize) -> Counter<'a> {
        count.fetch_add(1, Ordering::Release);
        Counter { count: count }
    }
}

impl<'a> Drop for Counter<'a> {
    fn drop(&mut self) {
        self.count.fetch_sub(1, Ordering::Release);
    }
}
