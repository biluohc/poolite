//! # [poolite](https://github.com/biluohc/poolite)
//!  A lite threadpool library written for Rust.
//!

//! ## Usage
//!
//! On Cargo.toml:
//!
//! ```toml
//!  [dependencies]
//!  poolite = "0.6.2"
//! ```
//!
//! ## [Examples](https://github.com/biluohc/poolite/blob/master/examples/)
//! * [without return values](https://github.com/biluohc/poolite/blob/master/examples/without.rs)
//!
//! * [return values by `Arc<Mutex<T>>`](https://github.com/biluohc/poolite/blob/master/examples/arc_mutex.rs)
//!
//! * [return values by `channel`](https://github.com/biluohc/poolite/blob/master/examples/channel.rs)

#![allow(stable_features)]
#![feature(fnbox,integer_atomics,arc_counts)]
//#[stable(feature = "arc_counts", since = "1.15.0")]

use std::fmt::{self, Debug, Display};
use std::time::Duration;
use std::error::Error;
use std::boxed::FnBox;
use std::thread;

#[macro_use]
extern crate stderr;
use stderr::Loger;
extern crate num_cpus;

mod inner;
#[allow(unused_imports)] //TIME_OUT_MS
use inner::{ArcWater, TIME_OUT_MS};
pub use inner::{Task, IntoTask};

/// The Pool struct.
pub struct Pool {
    arc_water: ArcWater,
}
/// # Creating and Settings
/// The memory loading depends your task's costing,
///
/// `get_min()`(idle time) and `get_max()`(when received large number tasks).
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
    #[inline]
    pub fn num_cpus() -> usize {
        ArcWater::num_cpus()
    }

    /// Sets whether to open the daemon for the Pool, the default is `Some(5000)`(thread's default idle time(ms)).
    ///
    /// You can use `None` to close.
    ///
    /// **Warning**: If you tasks maybe `panic`,please don't close it.
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
    pub fn name<T: Into<String>>(self, name: T) -> Self
        where T: Debug
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
    /// fn main() {
    /// let pool = Pool::new()
    ///     .daemon(Some(5000))
    ///     .min(Pool::num_cpus() + 1)
    ///     .max(std::usize::MAX)
    ///     .time_out(5000) //5000ms
    ///     .name("name")
    ///     .stack_size(2 * 1024 * 1024) //2MiB
    ///     .load_limit(Pool::num_cpus() * Pool::num_cpus())
    ///     .run()
    ///     .unwrap();
    /// }
    /// ```
    ///
    #[inline]
    pub fn get_load_limit(&self) -> usize {
        self.arc_water.get_load_limit()
    }
}

/// # Running and adding tasks.
impl Pool {
    /// Lets the Pool to start running:
    ///
    /// * Add the number of min threads to the pool.
    ///
    /// * Add the daemon thread for the pool(if dont't close it).
    ///
    /// returns `Err<PoolError>` if the pool spawning the daemon thread fails.
    ///
    /// So if you close the daemon,`unwrap()` is safe.
    ///
    /// You maybe need `IntoPool` or `IntoIOResult`(trait).
    ///
    /// ```Rust
    /// extern crate poolite;
    /// use poolite::{Pool, IntoPool, IntoIOResult};
    ///
    /// fn fun() -> std::io::Result<()> {
    ///     let pool = Pool::new().run().into_pool();
    ///     let pool_io_rst = Pool::new().run().into_iorst()?;
    ///     let pool_nodaemon = Pool::new().daemon(None).unwrap();
    ///     Ok(())
    /// }
    ///```
    #[inline]
    pub fn run(self) -> Result<Self, PoolError> {
        if let Err(s) = self.arc_water.run() {
            return Err(PoolError::new(self, s));
        }
        Ok(self)
    }

    /// Appends a task to the Pool,
    ///
    /// it receives `Fn() + Send + 'static，FnMut() + Send + 'static` and
    ///
    ///  `FnOnce() + Send + 'static>`.
    #[inline]
    pub fn push<Task: IntoTask>(&self, task: Task) {
        self.arc_water.spawn(task.into_task());
    }
    /// it receives `Box<Fn() + Send + 'static>，Box<FnMut() + Send + 'static>` and
    ///
    ///  `Box<FnOnce() + Send + 'static>(Box<FnBox() + Send + 'static>)`.
    ///
    #[inline]
    #[deprecated(since = "0.6.0",note = "You should use `push` instead")]
    pub fn spawn(&self, task: Box<FnBox() + Send + 'static>) {
        self.arc_water.spawn(Task::new(task));
    }
    ///Manually add threads(Do not need to use it generally).
    #[inline]
    pub fn add_threads(&self, num: usize) {
        for _ in 0..num {
            self.arc_water.add_thread();
        }
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

    ///Use it to wait for the Pool,you can also group `is_empty()` by yourself.
    ///
    ///```fuckrs
    ///pub fn join(&self) {
    ///         self.join_ms(10); //wait for the pool 10ms
    ///    }
    ///```
    #[inline]
    pub fn join(&self) {
        self.join_ms(10);
    }
    ///```fuckrs
    ///pub fn join_ms(&self, time: u64) {
    ///       while !self.is_empty() {
    ///           thread::sleep(Duration::from_millis(time)); //wait for the pool time(ms).
    ///       }
    ///    }
    ///```
    #[inline]
    pub fn join_ms(&self, time: u64) {
        while !self.is_empty() {
            thread::sleep(Duration::from_millis(time)); //wait for the pool time(ms).
        }
    }
    /// Returns the length of the tasks_queue.
    ///
    /// **Warning**: `tasks_len()` will get the lock, please do not abuse it(Affecting performance).
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

impl Drop for Pool {
    #[inline]
    fn drop(&mut self) {
        self.arc_water.drop_pool();
    }
}

impl Default for Pool {
    fn default() -> Self {
        Self::new()
    }
}

/// The error type for the pool's `run()` if the pool spawning the daemon thread fails.
pub struct PoolError {
    pool: Pool,
    error: std::io::Error,
}
impl PoolError {
    #[inline]
    fn new(pool: Pool, error: std::io::Error) -> Self {
        PoolError {
            pool: pool,
            error: error,
        }
    }
    ///  Into `Pool`
    #[inline]
    pub fn into_inner(self) -> Pool {
        self.pool
    }
    /// Into `std::io::Error`
    #[inline]
    pub fn into_error(self) -> std::io::Error {
        self.error
    }
}

impl Error for PoolError {
    fn description(&self) -> &str {
        self.error.description()
    }
}

impl Debug for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::error::Error;
        write!(f,
               "PoolError {{ pool : Pool, err : {:?} }}",
               self.error.description())
    }
}
impl Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::error::Error;
        write!(f,
               "PoolError {{ pool : Pool, err : {} }}",
               self.error.description())
    }
}

/// Into `Pool` for `Result<Pool, PoolError>`
pub trait IntoPool {
    fn into_pool(self) -> Pool;
}
impl IntoPool for Result<Pool, PoolError> {
    #[inline]
    fn into_pool(self) -> Pool {
        match self {
            Ok(o) => o,
            Err(e) => e.into_inner(),
        }
    }
}

/// Into `std::io::Result<Pool>` for `Result<Pool, PoolError>`
pub trait IntoIOResult {
    fn into_iorst(self) -> std::io::Result<Pool>;
}
impl IntoIOResult for Result<Pool, PoolError> {
    #[inline]
    fn into_iorst(self) -> std::io::Result<Pool> {
        match self {
            Ok(o) => Ok(o),
            Err(e) => Err(e.into_error()),
        }
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
        assert!(Pool::num_cpus() > 0);
        errln!("Pool::num_cpus(): {}", Pool::num_cpus());

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
        .run()
        .unwrap();
        let map = Arc::new(Mutex::new(BTreeMap::<i32, i32>::new()));
        for i in 0..33 {
            let map = map.clone();
            pool.push(move || test(i, map));
        }

        while !pool.is_empty() {
            thread::sleep(Duration::from_millis(10)); //wait for the pool 10ms.
            errln!("len()/strong_count()/min()/max(): {}/{}/{}/{}",
                   pool.len(),
                   pool.strong_count(),
                   pool.get_min(),
                   pool.get_max());
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
        let pool = Pool::new().max(0).run().unwrap();
        thread::sleep(Duration::from_millis(100)); //wait for the pool 100ms.
        errln!("len()/strong_count()/min()/max(0): {}/{}/{}/{}",
               pool.len(),
               pool.strong_count(),
               pool.get_min(),
               pool.get_max());
    }

    fn test(msg: i32, map: Arc<Mutex<BTreeMap<i32, i32>>>) {
        let res = fib(msg);
        let mut maplock = map.lock().unwrap();
        maplock.insert(msg, res);
    }

    fn fib(msg: i32) -> i32 {
        match msg {
            0...2 => 1,
            x => fib(x - 1) + fib(x - 2),
        }
    }
}
