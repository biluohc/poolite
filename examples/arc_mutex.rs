extern crate poolite;
use poolite::Pool;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;

/// `cargo run --example arc_mutex`
fn main() {
    let pool = Pool::new().run().unwrap();
    let map = Arc::new(Mutex::new(BTreeMap::<i32, i32>::new()));
    for i in 0..38 {
        let map = map.clone();
        pool.spawn(Box::new(move || test(i, map)));
    }

    while !pool.is_empty() {
        thread::sleep(Duration::from_millis(10)); //wait for the pool 10ms.
    }

    for (k, v) in map.lock().unwrap().iter() {
        println!("key: {}\tvalue: {}", k, v);
    }
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
