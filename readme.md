# A lite thread pool library written for Rust. 

## Usage
Cargo.toml

```toml
    [dependencies]
    poolite = "0.2.1"
```
or
```toml
    [dependencies]  
    poolite = { git = "https://github.com/biluohc/poolite",branch = "master", version = "0.2.1" }
```

## Explain
### Create a thread pool: 
* use `poolite::Pool::new()` create a thread_pool. 

#### The following are optional: 
* `min()` receive `usize` as minimum number of threads in pool,default is cpu's number.
* `time_out()` receive `u64` as thread's idle time(ms) except minimum number of threads,default is 5000(ms).
* `name()` receive `&str` as thread's name,default is None.
* `stack_size()` receive `usize` as thread's stack_size,default depends on OS.

### Let thread pool to start run:
* `run()` let pool to start run.   

### Add a task to the thread pool: 
* `spawn()` receive `Box<Fn() + Send>`，`Box<FnMut() + Send>` and `Box<FnOnce() + Send>`(`Box<FnBox()+Send>`).  
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
* 2016-0102 0.2.1 use unstable `FnBox()` to support `FnOnce()`(Only support Nightly now,Stable or Beta should use 0.2.0).
* 2016-0101 0.2.0 added `min(),time_out(),name(),stack_size(),run()` methods.