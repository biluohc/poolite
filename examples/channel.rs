extern crate poolite;
use poolite::Pool;

use std::sync::mpsc::{channel, Sender};

/// `cargo run --example channel`
fn main() {
    let pool = Pool::new().unwrap();
    let (mp, sc) = channel();
    for i in 0..38 {
        let mp = mp.clone();
        pool.push(move || test(i, mp));
    }

    pool.join(); // wait for the pool
    println!("{:?}", pool);

    while let Ok((k, v)) = sc.try_recv() {
        println!("key: {}\tvalue: {}", k, v);
    }
}

fn test(msg: i32, mp: Sender<(i32, i32)>) {
    let res = fib(msg);
    mp.send((msg, res)).unwrap();
}

fn fib(msg: i32) -> i32 {
    match msg {
        0...2 => 1,
        x => fib(x - 1) + fib(x - 2),
    }
}
