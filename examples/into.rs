extern crate poolite;
use poolite::Pool;
use std::io;

/// `cargo run --example into`
fn main() {
    into_inner();
    println!(); // newline
    println!("{:?}", into_error());
}
fn into_inner() {
    let pool = match Pool::new().run() {
        Ok(p) => p,
        Err(e) => e.into_inner(),
    };
    for i in 0..38 {
        pool.push(move || test(i));
    }

    pool.join(); //wait for the pool
}

fn into_error() -> io::Result<()> {
    let pool = Pool::new().run().map_err(|e| e.into_error())?;
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
