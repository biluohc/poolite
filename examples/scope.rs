extern crate poolite;
use poolite::Pool;

/// `cargo run --example without`
fn main() {
    let pool = Pool::new().unwrap();
    let mut array = (0..38).into_iter().map(|i| (i, 0)).collect::<Vec<_>>();

    pool.scoped(|scope| for i in array.iter_mut() {
        scope.push(move|| i.1 = fib(i.0));
    });
    for (i, j) in array {
        println!("key: {}\tvalue: {}", i, j);
    }
}

fn fib(msg: i32) -> i32 {
    match msg {
        0...2 => 1,
        x => fib(x - 1) + fib(x - 2),
    }
}
