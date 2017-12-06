/*!
# [poolite](https://github.com/biluohc/poolite)

A lite threadpool library written for Rust.

## Usage

On Cargo.toml:

```toml
 [dependencies]
 poolite = "0.7.1"
```

## Documentation  
* Visit [Docs.rs](https://docs.rs/poolite/)  

or 

* Run `cargo doc --open` after modified the toml file.

## Base usage
```
extern crate poolite;
use poolite::Pool;

fn main() {
    let pool = Pool::new().unwrap();
    for i in 0..38 {
        pool.push(move || test(i));
    }

    pool.join(); //wait for the pool
}

fn test(msg: i32) {
    println!("key: {}\tvalue: {}", msg, fib(msg));
}

fn fib(msg: i32) -> i32 {
    match msg {
        0...2 => 1,
        x => fib(x - 1) + fib(x - 2),
    }
}
```

## `Scoped` `Task`
```
extern crate poolite;
use poolite::Pool;

fn main() {
    let pool = Pool::new().unwrap();
    let mut array = (0..100usize).into_iter().map(|i| (i, 0)).collect::<Vec<_>>();

    // scoped method will waiting scoped's task running finish.
    pool.scoped(|scope| for i in array.iter_mut() {
        // have to move
        scope.push(move|| i.1 = i.0*i.0);
    });

    for (i, j) in array {
        println!("key: {}\tvalue: {}", i, j);
    }
}
```

## [More Examples..](https://github.com/biluohc/poolite/blob/master/examples/)
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
use std::mem::transmute;
use std::marker::PhantomData;
use std::error::Error;
use std::thread;
use std::io;
unsafe impl Send for Pool {}
unsafe impl Sync for Pool {}

/// The `Pool` struct
#[derive(Debug)]
pub struct Pool {
    inner: Inner,
}

impl Pool {
    pub fn new() -> Result<Self, PoolError> {
       Self::with_builder(Builder::default())
    }
    pub fn with_builder(b: Builder) -> Result<Self, PoolError> {
        assert!(b.max >= b.min, "min > max");
        assert!(b.max != 0, "max == 0");
        let _ = init();        

        let mut new =  Pool { inner: Inner::with_builder(b) };

        match (&mut new).inner.run() {
            Ok(_) => Ok(new),
            Err(e) => Err(PoolError::new(new, e)),
        }
    }
    /// Get `Pool`'s settings
    pub fn as_builder(&self) -> &Builder {
        self.inner.as_builder()
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
    pub fn threads_future(&self) -> usize {
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
    ///wait for the pool
    pub fn join(&self) {
        self.join_ms(10);
    }
    #[doc(hidden)]
    pub fn join_ms(&self, ms: u64) {
        while !self.is_empty() {
            thread::sleep(Duration::from_millis(ms)); //wait for the pool time(ms).
        }
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

include!("inner.rs");
include!("scope.rs");

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

    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use std::thread;
    #[should_panic]
    #[test]
    fn min_bq_max() {
        let _pool = Builder::new().min(100).max(99).build();
    }
    #[should_panic]
    #[test]
    fn max_zero() {
        let _pool = Builder::new().max(0).build();
    }
    #[test]
    fn min_eq_max() {
        let _pool = Builder::new().min(100).max(100).build();
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

        let pool = Pool::new().unwrap();
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
        let pool = Pool::new().unwrap();
        assert!(Builder::num_cpus() >= 1);
        assert_eq!(*pool.as_builder().min_get(), Builder::min_default());
        assert_eq!(
            *pool.as_builder().max_get(),
            Builder::max_default()
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

        let array = (0..33usize).into_iter().map(|i| (i, 0)).collect::<Vec<_>>();

        let map = Arc::new(Mutex::new(array));
        for i in 0..33 {
            let mutex = map.clone();
            pool.push(move || test(i, mutex));
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

        for &(k, v) in map.lock().unwrap().iter() {
            println!("key: {}\tvalue: {}", k, v);
        }

        assert!(pool.threads_alive() > 0);
        assert!(pool.threads_waiting() > 0);
        assert_eq!(*pool.as_builder().min_get(), Builder::min_default());
        assert_eq!(
            *pool.as_builder().max_get(),
            Builder::max_default()
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

    fn test(msg: usize, map: Arc<Mutex<Vec<(usize,usize)>>>) {
        let res = fib(msg);
        let mut maplock = map.lock().unwrap();
        maplock[msg as usize].1 = res;
    }
    fn fib(msg: usize) -> usize {
        match msg {
            0...2 => 1,
            x => fib(x - 1) + fib(x - 2),
        }
    }
    #[test]
    fn scope_fib() {
        let pool = Pool::new().unwrap();
        let mut array = (0..33usize).into_iter().map(|i| (i, 0)).collect::<Vec<_>>();

        let mutex = Arc::new(Mutex::new(array.clone()));
        for i in 0..33usize {
            let mutex = mutex.clone();
            pool.push(move || test(i, mutex));
        }

        pool.scoped(|scope| for i in array.iter_mut() {
            scope.push(move|| i.1 = fib(i.0));
        });

        pool.join();
        let array_true = mutex.lock().unwrap();
        assert_eq!(*array_true,array);
        for (i, j) in array {
            println!("key: {}\tvalue: {}", i, j);
        }
    }
    #[test]
    fn scope_x2() {
        let pool = Pool::new().unwrap();
        let mut array = (0..100usize).into_iter().map(|i| (i, 0)).collect::<Vec<_>>();

        let mutex = Arc::new(Mutex::new(array.clone()));
        for i in 0..100 {
            let mutex = mutex.clone();
            pool.push(move || x2(i, mutex));
        }

        pool.scoped(|scope| for i in array.iter_mut() {
            scope.push(move|| i.1 = i.0*i.0);
        });

        pool.join();
        let array_true = mutex.lock().unwrap();
        assert_eq!(*array_true,array);
        for (i, j) in array {
            println!("key: {}\tvalue: {}", i, j);
        }
    }
    fn x2(msg: usize, map: Arc<Mutex<Vec<(usize,usize)>>>) {
        let res = msg*msg;
        let mut maplock = map.lock().unwrap();
        maplock[msg].1 = res;
    }
}
