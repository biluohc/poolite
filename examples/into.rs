extern crate poolite;
use poolite::{Pool,IntoPool, IntoIOResult};
use std::io;

/// `cargo run --example into`
fn main() {
    pool();
    println!(); // newline
    println!("{:?}", io());
}
fn pool() {
    let pool = Pool::new().run().into_pool();
    for i in 0..38 {
        pool.push(move || test(i));
    }

    pool.join(); //wait for the pool
}

fn io()->io::Result<()> {
    let pool = Pool::new().run().into_iorst()?;
    for i in 0..38 {
        pool.push(move || test(i));
    }

    pool.join(); //wait for the pool
    
    Ok(())
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

