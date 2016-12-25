use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{Ordering, AtomicUsize};
use std::collections::VecDeque;
use std::io::{self, Write};
use std::time::Duration;
use std::error::Error;
use std::thread;

// 最低线程数
const MIN_THREADS: usize = 4;
// 线程栈大小 4M
const THREAD_STACK_SIZE: usize = 4 * 1024 * 1024;
// 线程销毁时间 ms
const THREAD_TIME_OUT_MS: u64 = 5000;

pub struct Pool {
    water: Arc<Water>,
}

struct Water {
    tasks: Mutex<VecDeque<Box<FnMut() + Send>>>,
    condvar: Condvar,
    threads: AtomicUsize,
    threads_waited: AtomicUsize,
}

impl Pool {
    pub fn new() -> Pool {
        let pool = Pool {
            water: Arc::new(Water {
                tasks: Mutex::new(VecDeque::new()),
                condvar: Condvar::new(),
                threads: AtomicUsize::new(0),
                threads_waited: AtomicUsize::new(0),
            }),
        };

        for _ in 0..MIN_THREADS {
            pool.add_thread();
        }

        pool
    }

    pub fn spawn(&self, task: Box<FnMut() + Send>) {
        let mut tasks_queue = self.water.tasks.lock().unwrap();
        // {
        //     println!("\nPool_waits/threads: {}/{} ---tasks_queue:  {}",
        //              (&self.water.threads_waited).load(Ordering::Acquire),
        //              (&self.water.threads).load(Ordering::Acquire),
        //              tasks_queue.len());
        // }
        tasks_queue.push_back(task);
        if (&self.water.threads_waited).load(Ordering::Acquire) == 0 {
            self.add_thread();
        } else {
            self.water.condvar.notify_one();
        }
    }

    fn add_thread(&self) {
        let water = self.water.clone();
        // spawn 有延迟,必须等父线程阻塞才运行.
        let spawn_res = thread::Builder::new()
            .stack_size(THREAD_STACK_SIZE)
            .spawn(move || {
                let water = water;
                // 对线程计数.
                let _threads_counter = Counter::new(&water.threads);

                loop {
                    let mut task; //声明任务。
                    loop {
                        let mut tasks_queue = water.tasks.lock().unwrap();// 取得锁                        
                        if let Some(poped_task) = tasks_queue.pop_front() {
                            task = poped_task;// pop成功就break ,执行pop出的任务.
                            break;
                        }
                        // 对在等候的线程计数.
                        let _threads_waited_counter = Counter::new(&water.threads_waited);

                        match (&water.threads).load(Ordering::Acquire) {
                            0...MIN_THREADS => {let _ = water.condvar.wait(tasks_queue).unwrap();} //线程总数<最小限制,不销毁线程.
                            _ => {
                                let (new_tasks_queue, waitres) = water.condvar
                                    .wait_timeout(tasks_queue, Duration::from_millis(THREAD_TIME_OUT_MS))
                                    .unwrap();
                               tasks_queue = new_tasks_queue;
                               // timed_out()为true时(等待超时是收不到通知就知道超时), 且队列空时销毁线程。
                                if waitres.timed_out() &&tasks_queue.is_empty(){
                                return;//销毁线程。
                            }
                        }
                    }; // match 线程数结束。
                    } // loop 取得任务结束。
                    task();//执行任务。
                } // loop 结束。
            }); //spawn 线程结束。

        match spawn_res {
            Ok(_) => {}
            Err(e) => {
                std_err(&format!("Poolite_Warnig:create new thread failed because of {} !",
                                 e.description()));
            }
        };
    }
}
impl Drop for Pool {
    fn drop(&mut self) {
        // 如果线程总数>线程最小限制且waited_out,然后线程销毁.
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
// 格式化标准错误输出
fn std_err(msg: &str) {
    match writeln!(io::stderr(), "{}", msg) {    
        Ok(..) => {}
        Err(_) => {}  //写入标准错误失败了不panic或继续尝试输出。
    };
}
