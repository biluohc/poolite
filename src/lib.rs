//! # [poolite](https://github.com/biluohc/poolite)
//!  A lite thread pool library written for Rust.
//!

//! ## Usage
//!
//! On Cargo.toml:
//!
//! ```toml
//!  [dependencies]
//!  poolite = "0.4.4"
//! ```
//! or
//!
//! ```toml
//!  [dependencies]
//!  poolite = { git = "https://github.com/biluohc/poolite",branch = "master", version = "0.4.4" }
//! ```

//! ## Example
//!
//! On code:
//!
//! ```
//! extern crate poolite;
//! use poolite::Pool;
//!
//! use std::collections::BTreeMap;
//! use std::sync::{Arc, Mutex};
//! use std::time::Duration;
//! use std::thread;
//!
//! fn main() {
//!     let pool = Pool::new().run();
//!     let map = Arc::new(Mutex::new(BTreeMap::<i32, i32>::new()));
//!     for i in 0..28 {
//!         let map = map.clone();
//!         pool.spawn(Box::new(move || test(i, map)));
//!     }
//!     loop {
//!         thread::sleep(Duration::from_millis(100)); //wait for the pool 100ms.
//!         if pool.is_empty() {
//!             break;
//!         }
//!     }
//!     for (k, v) in map.lock().unwrap().iter() {
//!         println!("key: {}\tvalue: {}", k, v);
//!     }
//! }
//!
//! fn test(msg: i32, map: Arc<Mutex<BTreeMap<i32, i32>>>) {
//!     let res = fib(msg);
//!     {
//!         let mut maplock = map.lock().unwrap();
//!         maplock.insert(msg, res);
//!     }
//! }
//!
//! fn fib(msg: i32) -> i32 {
//!     match msg {
//!         0...2 => 1,
//!         x => fib(x - 1) + fib(x - 2),
//!     }
//! }
//! ```

#![feature(fnbox)]
use std::boxed::FnBox;
use std::time::Duration;

#[macro_use]
extern crate stderr;
use stderr::Loger;
extern crate num_cpus;

// 默认线程销毁超时时间 ms 。
// 默认开启 deamon 。
// 默认初始化线程数由num_cpus决定。
/// Defaults thread's idle time(ms).
const TIME_OUT_MS: u64 = 5_000;
/// Defaults open daemon.
// const DAEMON: Option<Duration> = Some(Duration::from_millis(TIME_OUT_MS));
static mut NUM_CPUS: usize = 1;

mod inner;
use inner::ArcWater;

/// Pool struct.
pub struct Pool {
    arc_water: ArcWater,
}
/// # Creating and Settings
impl Pool {
    /// Creates and returns a Pool.
    ///
    #[inline]
    pub fn new() -> Self {
        init!();
        Pool { arc_water: ArcWater::new() }
    }

    /// Returns the number of CPUs of the current machine.
    ///
    /// You can use it on `min()` ,`max()` or `load_limit()`.
    ///
    /// Maybe you also need `std::usize::MIN` or `std::usize::MAX`.
    ///
    /// **Warning**: It  be initialized by use `new()` the first time on the current process,
    ///
    /// Don't use it before `new()`(Otherwise it will return 1).
    ///
    #[inline]
    pub fn num_cpus() -> usize {
        ArcWater::num_cpus()
    }

    /// Sets whether to open the daemon for the Pool, the default is `Some(5000)`(thread's default idle time(ms)).
    ///
    /// You can use `None` to close.
    ///
    #[inline]
    pub fn daemon(self, daemon: Option<u64>) -> Self {
        self.arc_water.daemon(daemon);
        self
    }
    /// Returns the value of `daemon(）`(`Option<Duration>`).
    ///
    #[inline]
    pub fn get_daemon(&self) -> Option<Duration> {
        self.arc_water.get_daemon()
    }

    /// Sets the minimum number of threads in the Pool，default is `num_cpus()+1`.
    ///
    #[inline]
    pub fn min(self, min: usize) -> Self {
        self.arc_water.min(min);
        self
    }
    /// Returns the value of the minimum number of threads in the Pool.
    ///
    #[inline]
    pub fn get_min(&self) -> usize {
        self.arc_water.get_min()
    }

    /// Sets the maximum number of threads in the Pool，default is `std::usize::MAX`.
    ///
    /// **Warning**: even if `get_min()>get_max()`,the `run()` method still working well.
    ///
    #[inline]
    pub fn max(self, max: usize) -> Self {
        self.arc_water.max(max);
        self
    }
    /// Returns the value of the maximum number of threads in the Pool.
    ///
    #[inline]
    pub fn get_max(&self) -> usize {
        self.arc_water.get_max()
    }

    ///  Sets thread's idle time(ms) except minimum number of threads,default is 5000(ms).
    ///
    #[inline]
    pub fn time_out(self, time_out: u64) -> Self {
        self.arc_water.time_out(time_out);
        self
    }
    /// Returns the value of the thread's idle time(`Duration`).
    ///
    #[inline]
    pub fn get_time_out(&self) -> Duration {
        self.arc_water.get_time_out()
    }

    /// Sets thread's name where them in the Pool,default is None(`'<unnamed>'`).
    ///
    #[inline]
    pub fn name<T: AsRef<str>>(self, name: T) -> Self
        where T: std::fmt::Debug
    {
        self.arc_water.name(name);
        self
    }

    /// Returns thread's name.
    ///
    #[inline]
    pub fn get_name(&self) -> Option<String> {
        self.arc_water.get_name()
    }

    /// Sets thread's stack_size where them in the Pool,default depends on OS.
    ///
    #[inline]
    pub fn stack_size(self, size: usize) -> Self {
        self.arc_water.stack_size(size);
        self
    }

    ///  Returns thread's stack_size.
    ///
    #[inline]
    pub fn get_stack_size(&self) -> Option<usize> {
        self.arc_water.get_stack_size()
    }

    /// Sets the value of load_limit for the Pool,
    ///
    /// The pool will create new thread while `strong_count() == 0` or `tasks_queue_len()/strong_count()` bigger than it，
    ///
    /// default is `num_cpus() * num_cpus()`.
    ///
    #[inline]
    pub fn load_limit(self, load_limit: usize) -> Self {
        self.arc_water.load_limit(load_limit);
        self
    }

    /// Returns the value of load_limit.
    ///
    /// ### Complete Example for Creating and Settings:
    ///
    /// ```Rust
    /// extern crate poolite;
    /// use poolite::Pool;
    ///
    /// let pool = Pool::new()
    ///     .daemon(Some(5000))
    ///     .min(Pool::num_cpus() + 1)
    ///     .max(std::usize::MAX)
    ///     .time_out(5000) //5000ms
    ///     .name("name")
    ///     .stack_size(2 * 1024 * 1024) //2MiB
    ///     .load_limit(Pool::num_cpus() * Pool::num_cpus())
    ///     .run();
    /// ```
    ///
    #[inline]
    pub fn get_load_limit(&self) -> usize {
        self.arc_water.get_load_limit()
    }
}

/// # Running and adding tasks
impl Pool {
    // 按理来说spawn够用了。对，不调用run也可以，只是开始反应会迟钝，因为线程还未创建。
    /// Lets the Pool to start running(Add the number of min threads to the pool).
    ///
    #[inline]
    pub fn run(self) -> Self {
        self.arc_water.run();
        self
    }

    /// Adds a task to the Pool,
    ///
    /// it receives `Box<Fn() + Send + 'static>，Box<FnMut() + Send + 'static>` and
    ///
    ///  `Box<FnOnce() + Send + 'static>(Box<FnBox() + Send + 'static>)`.
    ///
    #[inline]
    pub fn spawn(&self, task: Box<FnBox() + Send + 'static>) {
        self.arc_water.spawn(task);
    }
}

/// # Status
impl Pool {
    /// All threads are waiting and tasks_queue'length is 0.
    ///
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.arc_water.is_empty()
    }

    /// Returns the length of the tasks_queue.
    ///
    /// **Warning**: `is_empty()` and `tasks_len()` will all get the lock, please do not abuse them(Affecting performance).
    ///
    #[inline]
    pub fn tasks_len(&self) -> usize {
        self.arc_water.tasks_len()
    }

    /// Approximately equal to `len()`.
    ///
    #[inline]
    pub fn strong_count(&self) -> usize {
        self.arc_water.strong_count()
    }

    /// Returns the thread'number in the Pool.
    ///
    #[inline]
    pub fn len(&self) -> usize {
        self.arc_water.len()
    }

    /// Returns the thread'number that is waiting in the Pool
    ///
    #[inline]
    pub fn wait_len(&self) -> usize {
        self.arc_water.wait_len()
    }
}
// task'panic look like could'not to let Mutex be PoisonError,and counter will work nomally.
// pub fn once_panic(&self) -> bool {
//     // task once panic
//     self.water.tasks.is_poisoned()
// }


impl Drop for Pool {
    #[inline]
    fn drop(&mut self) {
        // 如果线程总数>线程最小限制且waited_out且任务栈空,则线程销毁.
        self.arc_water.set_daemon(None);
        self.arc_water.drop_pool();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use std::thread;
    #[test]
    fn main() {
        assert!(Pool::num_cpus() == 1);
        let pool = Pool::new();

        assert!(Pool::num_cpus() >= 1);
        assert_eq!(pool.get_max(), std::usize::MAX);
        assert_eq!(Some(Duration::from_millis(TIME_OUT_MS)), pool.get_daemon());
        assert_eq!(Pool::num_cpus() + 1, pool.get_min());
        assert_eq!(Duration::from_millis(TIME_OUT_MS), pool.get_time_out());
        assert_eq!(None, pool.get_name());
        assert_eq!(None, pool.get_stack_size());
        assert_eq!(Pool::num_cpus() * Pool::num_cpus(), pool.get_load_limit());

        let pool = pool.daemon(None)
        .min(0)
        .max(10)
        .time_out(0) //5000ms
        .name("name")
        .stack_size(0) //2MiB
        .load_limit(0)
        .run();
        let map = Arc::new(Mutex::new(BTreeMap::<i32, i32>::new()));
        for i in 0..33 {
            let map = map.clone();
            pool.spawn(Box::new(move || test(i, map)));
        }
        loop {
            thread::sleep(Duration::from_millis(10)); //wait for the pool 100ms.
            errln!("len()/strong_count()/min()/max(): {}/{}/{}/{}",
                   pool.len(),
                   pool.strong_count(),
                   pool.get_min(),
                   pool.get_max());
            if pool.is_empty() {
                break;
            }
        }

        for (k, v) in map.lock().unwrap().iter() {
            println!("key: {}\tvalue: {}", k, v);
        }
        assert_eq!(None, pool.get_daemon());
        assert_eq!(0, pool.get_min());
        assert_eq!(10, pool.get_max());
        assert_eq!(Duration::from_millis(0), pool.get_time_out());
        assert_eq!(Some("name".into()), pool.get_name());
        assert_eq!(Some(0), pool.get_stack_size());
        assert_eq!(0, pool.get_load_limit());

        println!("name: {:?}", pool.get_name());
        println!("daemon: {:?}", pool.get_daemon());
        println!("min: {:?}", pool.get_min());
        println!("load_limit: {:?}", pool.get_load_limit());
        println!("stack_size: {:?}", pool.get_stack_size());
        println!("time_out: {:?}", pool.get_time_out());
        let pool = Pool::new().max(0).run();
        thread::sleep(Duration::from_millis(100)); //wait for the pool 100ms.
        errln!("len()/strong_count()/min()/max(0): {}/{}/{}/{}",
               pool.len(),
               pool.strong_count(),
               pool.get_min(),
               pool.get_max());
    }

    fn test(msg: i32, map: Arc<Mutex<BTreeMap<i32, i32>>>) {
        let res = fib(msg);
        {
            let mut maplock = map.lock().unwrap();
            maplock.insert(msg, res);
        }
    }

    fn fib(msg: i32) -> i32 {
        match msg {
            0...2 => 1,
            x => fib(x - 1) + fib(x - 2),
        }
    }
}
