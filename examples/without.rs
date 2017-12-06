extern crate poolite;
use poolite::Pool;

/// `cargo run --example without`
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
