# A lite thread pool library written for Rust. 

## Usage
Cargo.toml

```toml
    [dependencies]
    poolite = "0.3.0"
```
or
```toml
    [dependencies]  
    poolite = { git = "https://github.com/biluohc/poolite",branch = "master", version = "0.3.0" }
```

## Explain
### Create a thread Pool: 
* use `poolite::Pool::new()` create a thread_pool. 

#### The following are optional: 
* `min()` receive `usize` as minimum number of threads in pool,default is cpu's number+1.
* `time_out()` receive `u64` as thread's idle time(ms) except minimum number of threads,default is 5000(ms).
* `name()` receive `AsRef<str>` as thread's name,default is None.
* `stack_size()` receive `usize` as thread's stack_size,default depends on OS.
* `load_limit()` receive `usize` as the load_limit, pool will create new thread while `tasks_queue_len()/threads` bigger than it，default is cpu's number.  
Ps:Pool will always block when `min()` is 0 and `load_limit()` is'not 0,until `tasks_queue_len()/threads` bigger than load_limit.

### Let Pool to start run:
* `run()` let pool to start run.   

### Add a task to the Pool: 
* `spawn()` receive `Box<Fn() + Send + 'static>`，`Box<FnMut() + Send + 'static>` and `Box<FnOnce() + Send + 'static>`(`Box<FnBox() + Send + 'static>`). 

### Get Pool's status  
* `len()` return a usize of the thread'number in pool. 
* `wait_len()` return a usize of the thread'number that is waiting  in pool  
* `tasks_len()` return a usize of the length of the tasks_queue.  
* `is_empty()` return a bool, all threads are waiting and tasks_queue'length is 0.  

### Drop
* while leave scope,pool will drop automatically.   

## Example  
```Rust
extern crate poolite;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;

fn main() {
    let pool = poolite::Pool::new().run();
    let map = Arc::new(Mutex::new(BTreeMap::<i32, i32>::new()));
    for i in 0..28 {
        let map = map.clone();
        pool.spawn(Box::new(move || test(i, map)));
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
    thread::sleep(Duration::from_millis(2000)); //wait for pool 2000ms
    for (k, v) in map.lock().unwrap().iter() {
        println!("key: {}\tvalue: {}", k, v);
    }
}
```
## ChangLog
* 2017-0112 0.3.0 remove all `unwrap()` and add `load_limit(),is_empty(), tasks_len(), len(), wait_len(), strong_count()` methods.
* 2016-0102 0.2.1 use unstable `FnBox()` to support `FnOnce()`(Only support Nightly now,Stable or Beta should use 0.2.0).
* 2016-0101 0.2.0 add `min(),time_out(),name(),stack_size(),run()` methods.