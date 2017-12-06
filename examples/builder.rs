extern crate poolite;
use poolite::Builder;

/// `cargo run --example without`
fn main() {
    let pool = Builder::new()
    .min(1)
    .max(9)
    .daemon(None) // Close
    .timeout(None) //Close
    .name("Worker")
    .stack_size(1024*1024*2) //2Mib
    .build()
    .unwrap();

    for i in 0..38 {
        pool.push(move || test(i));
    }

    pool.join(); //wait for the pool
    println!("{:?}", pool);
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
