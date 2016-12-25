mod lib;
use lib as poolite;

use std::time::Duration;
use std::thread;

// To observe the change of RAM occupy.
fn main() {
    println!("Tset poolite !");
    let pool = poolite::Pool::new();

    let mut count = 0;
    loop {
        if count == 100 {
            break;
        }
        for i in 0..36 {
            print!("main_loop0: ");
            pool.spawn(Box::new(move || test(count, i)));
        }
        count += 1;
    }
    count = 0;
    thread::sleep(Duration::from_millis(6000));
    println!("main_loop0 finished: ");
    loop {
        if count == 100 {
            break;
        }
        for i in 0..32 {
            print!("main_loop1: ");
            pool.spawn(Box::new(move || test(count, i)));
        }
        thread::sleep(Duration::from_millis(100));
        count += 1;
    }
    println!("loop1 finished !\nmain finished after sleep 6000 ms ! ");
    thread::sleep(Duration::from_millis(6000));
    fn test(count: i32, msg: i32) {
        println!("count({})_fib({})={}", count, msg, fib(msg));
    }
    fn fib(msg: i32) -> i32 {
        match msg {
            0...2 => return 1,
            x @ _ => return fib(x - 1) + fib(x - 2),
        };
    }
}