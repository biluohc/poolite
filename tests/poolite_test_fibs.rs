#[macro_use]
extern crate stderr;
extern crate poolite;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use std::time::Duration;
use std::thread;

// cargo test -- --nocapture ,不阻止终端输出。
// To observe the change of CPU/RAM occupy.
const FOR: usize = 1;
#[test]
fn main() {
    errln!("Tset poolite !");
    let st = SystemTime::now();
    for _ in 0..FOR {
        fibm();
    }
    let ed = SystemTime::now();
    errln!("{:?}", (ed.duration_since(st).unwrap()));

}
fn fibm() {
    let pool = poolite::Pool::new().run();
    let map = Arc::new(Mutex::new(BTreeMap::<i32, i32>::new()));

    let mut count = 0;
    loop {
        if count == 100 {
            break;
        }
        for i in 0..36 {
            let map = map.clone();
            pool.spawn(Box::new(move || test(i, map)));
        }
        count += 1;
    }
    count = 0;
    errln!("loop0 finished ! and sleep 5000 ms ! ");
    thread::sleep(Duration::from_millis(5000));
    loop {
        if count == 100 {
            break;
        }
        for i in 0..32 {
            let map = map.clone();
            pool.spawn(Box::new(move || test(i, map)));
        }
        thread::sleep(Duration::from_millis(100));
        count += 1;
    }
    errln!("loop1 finished ! main finished after sleep 6000 ms ! ");
    thread::sleep(Duration::from_millis(6000));

    for (k, v) in map.lock().unwrap().iter() {
        println!("key: {}\tvalue: {}", k, v);
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
}
