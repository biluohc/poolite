# A lite thread pool library written for Rust. 

## Usage
Cargo.toml

```toml
    [dependencies]
    poolite = "0.2.0"
```
or
```toml
    [dependencies]  
    poolite = { git = "https://github.com/biluohc/poolite",branch = "master", version = "0.2.0" }
```

## Explain
### Create a thread pool: 
* use `poolite::pool::new()` create a thread_pool. 

#### The following are optional: 
* `min()` receive `usize` as minimum number of threads in pool,default is cpu's number.
* `time_out()` receive `u64` as thread's idle time(ms) except minimum number of threads,default is 5000(ms).
* `name()` receive `&str` as thread's name,default is None.
* `stack_size()` receive `usize` as thread's stack_size,default depends on OS.

### Let thread pool to start run:
* `run()` let pool to start run.   

### Add a task to the thread pool: 
* `spawn()` receive `Box<FnMut() + Send>`.  
* while leave scope,pool will drop automatically.  

## Example  
```Rust
extern crate poolite;

use std::time::Duration;
use std::thread;

fn main() {
    let pool = poolite::Pool::new().run();
    pool.spawn(Box::new(move || test(32)));

    fn test(msg: i32) {
        println!("fib({})={}", msg, fib(msg));
    }
    fn fib(msg: i32) -> i32 {
        match msg {
            0...2 => 1,
            x => fib(x - 1) + fib(x - 2),
        }
    }
    thread::sleep(Duration::from_millis(2000)); //wait for pool 2000ms
}
```
