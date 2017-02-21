extern crate poolite;
use poolite::Pool;

use std::time::Duration;
use std::thread;

/// cargo run --example without
fn main() {
    let pool = Pool::new().run().unwrap();
    for i in 0..38 {
        pool.spawn(Box::new(move || test(i)));
    }

    while !pool.is_empty() {
        thread::sleep(Duration::from_millis(10)); //wait for the pool 10ms.
    }
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
