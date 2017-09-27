extern crate poolite;
use poolite::Pool;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// `cargo run --example arc_mutex`
fn main() {
    let pool = Pool::new().run().unwrap();
    // You also can use RwLock instead of Mutex if you read more than write.
    let map = Arc::new(Mutex::new(BTreeMap::<i32, i32>::new()));
    for i in 0..38 {
        let map = Arc::clone(&map);
        pool.push(move || test(i, map));
    }

    pool.join(); //wait for the pool

    for (k, v) in map.lock().unwrap().iter() {
        println!("key: {}\tvalue: {}", k, v);
    }
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
