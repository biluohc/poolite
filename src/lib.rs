/*!
# [poolite](https://github.com/biluohc/poolite)

A lite threadpool library written for Rust.

## Usage

On Cargo.toml:

```toml
 [dependencies]
 poolite = "0.7.0"
```

## Documentation  
* Visit [Docs.rs](https://docs.rs/poolite/)  

or 

* Run `cargo doc --open` after modified the toml file.

## [Examples](https://github.com/biluohc/poolite/blob/master/examples/)
* [without return values](https://github.com/biluohc/poolite/blob/master/examples/without.rs)

* [return values by `Arc<Mutex<T>>`](https://github.com/biluohc/poolite/blob/master/examples/arc_mutex.rs)

* [return values by `channel`](https://github.com/biluohc/poolite/blob/master/examples/channel.rs)

* [about `PoolError`](https://github.com/biluohc/poolite/blob/master/examples/into.rs)

* [about `Builder`](https://github.com/biluohc/poolite/blob/master/examples/builder.rs)
*/
#[macro_use]
extern crate log;
extern crate mxo_env_logger;
extern crate crossbeam_channel;
extern crate num_cpus;

use crossbeam_channel::{unbounded, Sender, Receiver, RecvTimeoutError};
use mxo_env_logger::{init, LogErr};

use std::sync::atomic::{Ordering, AtomicUsize, AtomicBool};
use std::sync::{Arc, Once, ONCE_INIT};
use std::fmt::{self, Debug, Display};
use std::time::Duration;
use std::error::Error;
use std::thread;
use std::io;
unsafe impl Send for Pool {}
unsafe impl Sync for Pool {}

/// Defaults thread's idle time(ms).
const TIME_OUT_MS: u64 = 5_000;
/// Defaults open daemon.
// const DAEMON: Option<Duration> = Some(Duration::from_millis(TIME_OUT_MS));
static mut NUM_CPUS: usize = 0;
static INIT: Once = ONCE_INIT;

/// The Task Box
pub type Task = Box<Runable + Send + 'static>;

/// The `Runable` trait for `FnOnce()`
pub trait Runable {
    fn call(self: Box<Self>);
}
impl<F: FnOnce()> Runable for F {
    #[inline]
    fn call(self: Box<Self>) {
        (*self)()
    }
}

/// The ThreadPool struct
#[derive(Debug)]
pub struct Pool {
    inner: Inner,
}

#[derive(Debug)]
struct Inner {
    workers: Arc<Builder>,
}

/// `Pool`'s Settings
pub struct Builder {
    name: Option<String>,
    stack_size: Option<usize>,
    min: usize,
    max: usize,
    timeout: Option<Duration>,
    load_limit: usize,
    daemon: Option<Duration>,

    mp: Sender<Task>,
    mc: Receiver<Task>,
    threads_future: AtomicUsize, // contains ready to create, consider create failed 
    threads_alive: AtomicUsize,    // alive, contains busy with task and wait for Task arrive
    threads_waiting: AtomicUsize,// wait for Task arrive
    daemon_alive: AtomicBool,
    dropped: AtomicBool,
}
impl Debug for Builder {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Builder")
            .field("name", &self.name)
            .field("stack_size", &self.stack_size)
            .field("min", &self.min)
            .field("max", &self.max)
            .field("timeout", &self.timeout)
            .field("load_limit", &self.load_limit)
            .field("daemon", &self.daemon)
            .field("mp", &"Sender<Task>")
            .field("mc", &"Receiver<Task>")
            .field("threads_future", &self.threads_future)
            .field("threads_alive", &self.threads_alive)
            .field("threads_waiting", &self.threads_waiting)
            .field("daemon_alive", &self.daemon_alive)
            .field("dropped", &self.dropped)
            .finish()
    }
}


impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}
impl Builder {
    #[inline]
    pub fn num_cpus() -> usize {
        unsafe {
            INIT.call_once(|| { NUM_CPUS = num_cpus::get(); });
            NUM_CPUS
        }
    }
    pub fn new() -> Self {
        let (mp, mc) = unbounded();
        Self {
            mp: mp,
            mc: mc,
            threads_future: AtomicUsize::default(),
            threads_alive: AtomicUsize::default(),
            threads_waiting: AtomicUsize::default(),

            min: Self::min_default(),
            max: Self::max_default(),
            timeout: Some(Duration::from_millis(TIME_OUT_MS)),
            name: None,
            stack_size: None,
            load_limit: Self::num_cpus() * Self::num_cpus(),
            daemon: Some(Duration::from_millis(TIME_OUT_MS)),
            daemon_alive: AtomicBool::default(),
            dropped: AtomicBool::default(),
        }
    }
    fn min_default() -> usize {
        Self::num_cpus() + 1
    }
    fn max_default() -> usize {
        (Self::num_cpus() + 1) * Self::num_cpus()
    }
    /// Sets thread's name where them in the Pool,default is None(`'<unnamed>'`).
    pub fn name<S>(mut self, name: S) -> Self
    where
        S: Debug + Into<String>,
    {
        self.name = Some(name.into());
        self
    }
    #[inline]
    pub fn name_get(&self) -> Option<&String> {
        self.name.as_ref()
    }
    /// Sets thread's stack_size where them in the Pool,default depends on OS.
    pub fn stack_size(mut self, size: usize) -> Self {
        self.stack_size = Some(size);
        self
    }
    #[inline]
    pub fn stack_size_get(&self) -> Option<&usize> {
        self.stack_size.as_ref()
    }
    /// Sets the minimum number of threads in the Pool，default is `num_cpus()+1`.
    pub fn min(mut self, min: usize) -> Self {
        if self.max < min && self.max == Self::max_default() {
            self.max = min;
        }
        self.min = min;
        self
    }
    #[inline]
    pub fn min_get(self: &Self) -> &usize {
        &self.min
    }
    /// Sets the maximum number of threads in the Pool，default is `(num_cpus()+1)*num_cpus()`.
    pub fn max(mut self, max: usize) -> Self {
        if self.min > max && self.min == Self::min_default() {
            self.min = max;
        }
        self.max = max;
        self
    }
    #[inline]
    pub fn max_get(&self) -> &usize {
        &self.max
    }
    /// Sets thread's idle time(ms) except minimum number of threads,default is 5000(ms).
    pub fn timeout_ms(self, timeout: Option<u64>) -> Self {
        self.timeout(timeout.map(Duration::from_millis))
    }
    pub fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }
    #[inline]
    pub fn timeout_get(&self) -> Option<&Duration> {
        self.timeout.as_ref()
    }
    /// Sets whether to open the daemon for the Pool, the default is `Some(5000)`(thread's default idle time(ms)).
    ///
    /// You can use `None` to close.
    pub fn daemon_ms(mut self, daemon: Option<u64>) -> Self {
        self.daemon = daemon.map(Duration::from_millis);
        self
    }
    pub fn daemon(mut self, daemon: Option<Duration>) -> Self {
        self.daemon = daemon;
        self
    }
    #[inline]
    pub fn daemon_get(&self) -> Option<&Duration> {
        self.daemon.as_ref()
    }
    /// Sets the value of load_limit for the Pool.
    ///
    /// default is `num_cpus() * num_cpus()`.
    pub fn load_limit(mut self, load_limit: usize) -> Self {
        self.load_limit = load_limit;
        self
    }
    #[inline]
    pub fn load_limit_get(&self) -> &usize {
        &self.load_limit
    }
    pub fn run(self) -> Result<Pool, PoolError> {
        Pool::with_builder(self).run()
    }
}
impl Pool {
    pub fn new() -> Self {
        Self::with_builder(Builder::default())
    }
    pub fn with_builder(b: Builder) -> Self {
        assert!(b.max >= b.min, "min > max");
        assert!(b.max != 0, "max == 0");

        Self { inner: Inner::with_builder(b) }
    }
    /// Get `Pool`'s settings
    pub fn as_builder(&self) -> &Builder {
        self.inner.as_builder()
    }
    pub fn run(mut self) -> Result<Self, PoolError> {
        let _ = init();
        match (&mut self).inner.run() {
            Ok(_) => Ok(self),
            Err(e) => Err(PoolError::new(self, e)),
        }
    }
    /// All threads are waiting and tasks_queue'length is 0.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    /// Returns the length of the tasks_queue.
    pub fn tasks_len(&self) -> usize {
        self.inner.tasks_len()
    }
    // #[doc(hidden)]
    /// Contains the number of ready to create
    pub fn threads_future(&self) -> usize
    {
        self.inner.threads_future()
    }
    /// Returns the number of threads in the Pool.
    pub fn threads_alive(&self) -> usize {
        self.inner.threads_alive()
    }
    /// Returns the number of threads that is waiting for Task in the Pool
    pub fn threads_waiting(&self) -> usize {
        self.inner.threads_waiting()
    }
    /// The daemon thread's status
    pub fn daemon_alive(&self) -> bool {
        self.inner.daemon_alive()
    }
    #[doc(hidden)]
    pub fn dropped(&self) -> bool {
        self.inner.dropped()
    }
    /// Appends a task to the Pool,
    ///
    /// it receives `Fn() + Send + 'static，FnMut() + Send + 'static` and `FnOnce() + Send + 'static>`.
    pub fn push<T>(&self, task: T)
    where
        T: Runable + Send + 'static,
    {
        self.inner.push(
            Box::new(task) as Box<Runable + Send + 'static>,
        )
    }
    /// Manually add the number of threads to `Pool`
    pub fn add_threads(&self, add_num: usize) -> Result<(), (usize, io::Error)> {
        self.inner.add_threads(add_num)
    }
    ///wait for the pool(10ms).
    pub fn join(&self) {
        self.join_ms(10);
    }
    pub fn join_ms(&self, ms: u64) {
        while !self.is_empty() {
            thread::sleep(Duration::from_millis(ms)); //wait for the pool time(ms).
        }
    }
}

impl Default for Pool {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        self.inner.as_builder().dropped.store(
            true,
            Ordering::SeqCst,
        )
    }
}

impl Clone for Inner {
    fn clone(&self) -> Self {
        Self { workers: self.workers.clone() }
    }
}

impl Inner {
    // pub fn new() -> Self {
    //     Self::with_builder(Builder::default())
    // }
    pub fn with_builder(builder: Builder) -> Self {
        Self { workers: Arc::new(builder) }
    }
    #[inline]
    pub fn as_builder(&self) -> &Builder {
        &self.workers
    }
    pub fn run(&mut self) -> io::Result<()> {
        let mut result = Ok(());
        if self.as_builder().daemon_get().is_some() {
            let daemon = self.clone();
            let mut b = thread::Builder::new();
            if let Some(name) = daemon.as_builder().name_get() {
                b = b.name(name.to_string());
            }

            result = b.spawn(move || {
                let daemon = daemon;
                let min = daemon.as_builder().min_get();
                let _alive = Alive::add(&daemon.as_builder().daemon_alive);
                let time = daemon.as_builder().daemon_get().unwrap();
                loop {
                    thread::sleep(*time);
                    if daemon.dropped() {
                        return;
                    }
                    let future = daemon.threads_future();
                    let add_num = if future < *min { min - future } else { 0 };
                    let _result = daemon.add_threads(add_num);
                }
            }).map(|_| ());
        }
        self.add_threads(*self.as_builder().min_get()).map_err(
            |e| e.1,
        )?;
        result
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        // All threads are waiting and tasks_queue'length is 0.
        self.threads_waiting() == self.threads_alive() && self.tasks_len() == 0
    }
    #[inline]
    pub fn tasks_len(&self) -> usize {
        self.as_builder().mc.len()
    }
    #[inline]
    pub fn threads_future(&self) -> usize {
        self.as_builder().threads_future.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn threads_alive(&self) -> usize {
        self.as_builder().threads_alive.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn threads_waiting(&self) -> usize {
        self.as_builder().threads_waiting.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn daemon_alive(&self) -> bool {
        self.as_builder().daemon_alive.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn dropped(&self) -> bool {
        self.as_builder().dropped.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn push(&self, task: Task) {
        // 注意min==0 且 load_limit>0 时,线程池里无线程则前 load_limit 个请求会一直阻塞。
        self.as_builder()
            .mp
            .send(task)
            .log_err(|e| error!("Send Task failed: {}", e.description()))
            .unwrap();
        let len = self.threads_future();
        if len == 0 || len < *self.as_builder().max_get() && self.threads_waiting() == 0 && self.tasks_len() / len > *self.as_builder().load_limit_get() {
            let _ = self.add_threads(1);
        }
    }
    pub fn add_threads(&self, add_num: usize) -> Result<(), (usize, io::Error)> {
        self.as_builder().threads_future.fetch_add(add_num,Ordering::SeqCst);
        for idx in 0..add_num {
            if let Err(e) = self.add_thread() {
                self.as_builder().threads_future.fetch_sub(add_num-idx-1,Ordering::SeqCst);
                return Err((idx+1,e));
            }
        }
        Ok(())
    }
    ///use the above to maintain future
    fn add_thread(&self) -> io::Result<()> {
        let worker = self.clone();
        // 线程命名。
        let mut thread = match self.as_builder().name_get() {
            Some(name) => thread::Builder::new().name(name.to_string()),
            None => thread::Builder::new(),
        };
        // 线程栈大小设置。
        thread = match self.as_builder().stack_size_get() {
            Some(size) => thread.stack_size(*size),
            None => thread,
        };
        // spawn 有延迟, 不能直接检测len大小就新建.
        let spawn_result = thread.spawn(move || {
            // 对线程计数.
            let _threads_counter = Counter::add(&worker.as_builder().threads_alive);

            let min = worker.as_builder().min_get();
            let mut task: Task;
            if let Some(timeout) = worker.as_builder().timeout_get() {
                loop {
                    loop {
                        if worker.dropped() {
                            return;
                        }
                        // 对在等候的线程计数.
                        let _threads_waited_counter = Counter::add(&worker.as_builder().threads_waiting);
                        match worker.as_builder().mc.recv_timeout(*timeout) {
                            Ok(t) => {
                                task = t;
                                break;
                            }
                            Err(RecvTimeoutError::Timeout) => {
                                if !worker.as_builder().mc.is_empty() && worker.threads_future() > *min {
                                    return;
                                }
                            }
                            _ => {
                                return;
                            }
                        }
                    }
                    task.call();
                }
            } else {
                loop {
                    loop {
                        if worker.dropped() {
                            return;
                        }
                        let _threads_waited_counter = Counter::add(&worker.as_builder().threads_waiting);
                        if let Ok(t) = worker.as_builder().mc.recv() {
                            task = t;
                            break;
                        } else {
                            return;
                        }
                    }
                    task.call();
                }
            }
        }); //spawn 线程结束。

        spawn_result.map(|_| ()).log_err(|e| {
            error!("add thread failed: '{}' !", e.description())
        })
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
// 通过作用域对Daemon状态管理。
struct Alive<'a> {
    state: &'a AtomicBool,
}

impl<'a> Alive<'a> {
    #[inline]
    fn add(alive: &'a AtomicBool) -> Alive<'a> {
        alive.store(true, Ordering::SeqCst);
        Self { state: alive }
    }
}

impl<'a> Drop for Alive<'a> {
    #[inline]
    fn drop(&mut self) {
        self.state.store(false, Ordering::SeqCst);
    }
}

/// The error type for the pool's `run()` if the pool spawning the daemon thread fails.
#[derive(Debug)]
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

impl Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::error::Error;
        write!(
            f,
            "PoolError {{ pool : Pool, err : {} }}",
            self.error.description()
        )
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use std::thread;
    #[should_panic]
    #[test]
    fn min_bq_max() {
        let _pool = Builder::new().min(100).max(99).run();
    }
    #[should_panic]
    #[test]
    fn max_zero() {
        let _pool = Builder::new().max(0).run();
    }
    #[test]
    fn min_eq_max() {
        let _pool = Builder::new().min(100).max(100).run();
    }
    #[test]
    fn min_max() {
        let min0 = Builder::min_default();
        let max0 = Builder::max_default();

        let p0 = Builder::new().max(min0 - 1);
        assert_eq!(min0 - 1, p0.max);
        assert_eq!(min0 - 1, p0.min);

        let p1 = Builder::new().min(max0 + 1);
        assert_eq!(p1.min, max0 + 1);
        assert_eq!(p1.max, max0 + 1);

        let p2 = Builder::new().min(max0).max(min0);
        assert_eq!(p2.min, max0);
        assert_eq!(p2.max, min0);
    }
    #[test]
    fn fn_fnmut_fnonce_closure() {
        fn fnn() {
            println!("call Fn() push");
        }

        fn fnm(msg: &mut String) {
            println!("{}", msg);
            *msg = "call FnMut() return".to_owned()
        }

        fn fno(msg: String) {
            println!("{}", msg);
        }
        let mut str = std::env::args().nth(0).unwrap();
        let str1 = std::env::args().nth(0).unwrap();
        let str2 = std::env::args().nth(0).unwrap();

        let pool = Pool::new().run().unwrap();
        pool.push(fnn);
        pool.push(move || fnm(&mut str));
        pool.push(move || fno(str1));

        let closure = move || for _ in 0..str2.len() {
            if std::env::args().count() > 100_0000 {
                println!("Fake");
            }
        };
        pool.push(closure);
        pool.join()
    }

    #[test]
    fn pool() {
        let pool = Pool::new();
        assert!(Builder::num_cpus() >= 1);
        assert_eq!(pool.threads_alive(), 0);
        assert_eq!(pool.threads_waiting(), 0);
        assert_eq!(*pool.as_builder().min_get(), Builder::num_cpus() + 1);
        assert_eq!(
            *pool.as_builder().max_get(),
            (Builder::num_cpus() + 1) * Builder::num_cpus()
        );
        assert_eq!(
            pool.as_builder().timeout_get(),
            Some(&Duration::from_millis(TIME_OUT_MS))
        );
        assert!(pool.as_builder().name_get().is_none());
        assert!(pool.as_builder().stack_size_get().is_none());
        assert_eq!(
            *pool.as_builder().load_limit_get(),
            Builder::num_cpus() * Builder::num_cpus()
        );

        assert_eq!(
            pool.as_builder().daemon_get(),
            Some(&Duration::from_millis(TIME_OUT_MS))
        );
        assert!(!pool.daemon_alive());// not run, so
        assert!(!pool.dropped());

        let pool = pool.run().unwrap();
        let map = Arc::new(Mutex::new(BTreeMap::<i32, i32>::new()));
        for i in 0..33 {
            let map = map.clone();
            pool.push(move || test(i, map));
        }

        while !pool.is_empty() {
            thread::sleep(Duration::from_millis(10)); //wait for the pool 10ms.
            eprint!(
                "len()/min()/max(): {}/{}/{}",
                pool.threads_alive(),
                pool.as_builder().min_get(),
                pool.as_builder().max_get()
            );
        }

        for (k, v) in map.lock().unwrap().iter() {
            println!("key: {}\tvalue: {}", k, v);
        }

        assert!(pool.threads_alive() > 0);
        assert!(pool.threads_waiting() > 0);
        assert_eq!(*pool.as_builder().min_get(), Builder::num_cpus() + 1);
        assert_eq!(
            *pool.as_builder().max_get(),
            (Builder::num_cpus() + 1) * Builder::num_cpus()
        );
        assert_eq!(
            pool.as_builder().timeout_get(),
            Some(&Duration::from_millis(TIME_OUT_MS))
        );
        assert!(pool.as_builder().name_get().is_none());
        assert!(pool.as_builder().stack_size_get().is_none());
        assert_eq!(
            *pool.as_builder().load_limit_get(),
            Builder::num_cpus() * Builder::num_cpus()
        );

        assert_eq!(
            pool.as_builder().daemon_get(),
            Some(&Duration::from_millis(TIME_OUT_MS))
        );
        assert!(pool.daemon_alive());
        assert!(!pool.dropped());
        println!("{:?}", pool);
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
