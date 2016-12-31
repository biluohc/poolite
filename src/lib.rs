#[macro_use]
extern crate stderr;
extern crate num_cpus;

use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{Ordering, AtomicUsize};
use std::collections::VecDeque;
use std::time::Duration;
use std::error::Error;
use std::thread;

// 默认初始化线程数由num_cpus决定。
// 默认线程销毁超时时间 ms 。
const TIME_OUT_MS: u64 = 5_000;

pub struct Pool {
    water: Arc<Water>,
    name: Option<String>,
    stack_size: Option<usize>,
    min: usize,
    time_out: u64,
}

struct Water {
    tasks: Mutex<VecDeque<Box<FnMut() + Send>>>,
    condvar: Condvar,
    threads: AtomicUsize,
    threads_waited: AtomicUsize,
}

impl Default for Pool {
    fn default() -> Self {
        Self::new()
    }
}

impl Pool {
    pub fn new() -> Pool {
        Pool {
            water: Arc::new(Water {
                tasks: Mutex::new(VecDeque::new()),
                condvar: Condvar::new(),
                threads: AtomicUsize::new(0),
                threads_waited: AtomicUsize::new(0),
            }),
            name: None,
            stack_size: None,
            min: num_cpus::get(),
            time_out: TIME_OUT_MS,
        }
    }
    pub fn min(mut self, min: usize) -> Pool {
        self.min = min;
        self
    }
    pub fn time_out(mut self, time_out: u64) -> Pool {
        self.time_out = time_out;
        self
    }

    pub fn name(mut self, name: &str) -> Pool {
        self.name = Some(name.to_string());
        self
    }
    pub fn stack_size(mut self, size: usize) -> Pool {
        self.stack_size = Some(size);
        self
    }
    // 按理来说spawn够用了。对，不调用run也可以，只是开始反应会迟钝，因为线程还未创建。
    pub fn run(self) -> Pool {
        for _ in 0..self.min {
            self.add_thread();
        }
        self
    }
    pub fn spawn(&self, task: Box<FnMut() + Send>) {
        {
            // 减小锁的作用域。
            let mut tasks_queue = self.water.tasks.lock().unwrap();
            // {
            //     println!("\nPool_waits/threads: {}/{} ---tasks_queue:  {}",
            //              (&self.water.threads_waited).load(Ordering::Acquire),
            //              (&self.water.threads).load(Ordering::Acquire),
            //              tasks_queue.len());
            // }
            tasks_queue.push_back(task);
        }
        if (&self.water.threads_waited).load(Ordering::Acquire) == 0 {
            self.add_thread();
        } else {
            self.water.condvar.notify_one();
        }
    }

    fn add_thread(&self) {
        let water = self.water.clone();
        let time_out = self.time_out;
        let min = self.min;
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
                let _threads_counter = Counter::new(&water.threads);

                loop {
                    let mut task; //声明任务。
                    {
                        let mut tasks_queue = water.tasks.lock().unwrap();
                        // 移入内层loop=>解决全局锁问题；移出内层loop到单独的{}=>解决重复look()问题。
                    loop { 
                        // let mut tasks_queue = water.tasks.lock().unwrap();// 取得锁                        
                        if let Some(poped_task) = tasks_queue.pop_front() {
                            task = poped_task;// pop成功就break ,执行pop出的任务.
                            break;
                        }
                        // 对在等候的线程计数.
                        let _threads_waited_counter = Counter::new(&water.threads_waited);
                       
                       if (&water.threads).load(Ordering::Acquire) <= min  {
                            tasks_queue = water.condvar.wait(tasks_queue).unwrap();
                       } else {
                         let (new_tasks_queue, waitres) = water.condvar
                                    .wait_timeout(tasks_queue, Duration::from_millis(time_out))
                                    .unwrap();
                        tasks_queue=new_tasks_queue;
                        // timed_out()为true时(等待超时是收不到通知就知道超时), 且队列空时销毁线程。
                        if waitres.timed_out()&&tasks_queue.is_empty() { return; }
                            }
                    } // loop 取得任务结束。
                    }
                    task();//执行任务。
                } // loop 结束。
            }); //spawn 线程结束。

        match spawn_res {
            Ok(_) => {}
            Err(e) => {
                errstln!("Poolite_Warnig: create new thread failed because of '{}' !",
                         e.description());
            }
        };
    }
}
impl Drop for Pool {
    fn drop(&mut self) {
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
    fn new(count: &'a AtomicUsize) -> Counter<'a> {
        count.fetch_add(1, Ordering::Release);
        Counter { count: count }
    }
}

impl<'a> Drop for Counter<'a> {
    fn drop(&mut self) {
        self.count.fetch_sub(1, Ordering::Release);
    }
}
