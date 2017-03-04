#[macro_use]
extern crate stderr;
extern crate poolite;
use poolite::Pool;

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
    errln!("\nTset poolite !");
    let st = SystemTime::now();
    for _ in 0..FOR {
        fibm();
    }
    let ed = SystemTime::now();
    errln!("{:?}\n", (ed.duration_since(st).unwrap()));
}
fn fibm() {
    let pool = Pool::new()
        .stack_size(3 * 1024 * 1024)
        .min(9)
        .daemon(None)
        .time_out(5120)
        .load_limit(Pool::num_cpus() * Pool::num_cpus())
        .run()
        .unwrap();
    let map = Arc::new(Mutex::new(BTreeMap::<i32, i32>::new()));

    let mut count = 0;
    while count != 100 {
        for i in 0..36 {
            let map = map.clone();
            pool.push(move || test(i, map));
        }
        count += 1;
    }
    errln!("\nis_empty(): {}\ttasks_len(): {}",
           pool.is_empty(),
           pool.tasks_len());
    errln!("wait_len()/len()/strong_count()-1[2]: {}/{}/{}\n",
           pool.wait_len(),
           pool.len(),
           pool.strong_count());
    thread::sleep(Duration::from_millis(5000));
    errln!("loop0 finished ! main slept 5000 ms ! ");
    errln!("is_empty(): {}\ttasks_len(): {}",
           pool.is_empty(),
           pool.tasks_len());
    errln!("wait_len()/len()/strong_count()-1[2]: {}/{}/{}\n",
           pool.wait_len(),
           pool.len(),
           pool.strong_count());
    count = 0;
    while count != 100 {
        for i in 0..32 {
            let map = map.clone();
            pool.push(move || test(i, map));
        }
        thread::sleep(Duration::from_millis(100));
        count += 1;
    }
    thread::sleep(Duration::from_millis(6000));
    errln!("loop1 finished ! main slept 6000 ms ! ");
    errln!("is_empty(): {}\ttasks_len(): {}",
           pool.is_empty(),
           pool.tasks_len());
    errln!("wait_len()/len()/strong_count()-1[2]: {}/{}/{}\n",
           pool.wait_len(),
           pool.len(),
           pool.strong_count());

    for (k, v) in map.lock().unwrap().iter() {
        println!("key: {}\tvalue: {}", k, v);
    }
    errln!("is_empty(): {}\ttasks_len(): {}",
           pool.is_empty(),
           pool.tasks_len());

    errln!("wait_len()/len()/strong_count()-1[2]: {}/{}/{}\n",
           pool.wait_len(),
           pool.len(),
           pool.strong_count());
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
}

#[allow(deprecated)]
#[test]
fn old_and_new() {
    use poolite::Pool;
    use poolite::Task;

    let mut msg = "call FnMut() spawn".to_owned();
    println!("Pool::num_cpus(): {}", Pool::num_cpus());

    let pool = Pool::new().run().unwrap();
    pool.spawn(Box::new(fnn_old)); //old: spawn(Fn)
    pool.spawn(Box::new(move || fnm(&mut msg))); //old: spawn(FnMut)
    let msg = "call FnOnce() spwan".to_owned();
    pool.spawn(Box::new(|| fno(msg))); //old: spawn(FnOnce)

    let mut msg = "call FnMut() push".to_owned();
    pool.push(fnn); //push(Fn())
    pool.push(move || fnm(&mut msg)); //push(FnMut())
    let msg = "call FnOnce() push".to_owned();
    pool.push(|| fno(msg)); //push(FnOnce())
    pool.push(Task::new(Box::new(fnn_task))); // push(Task)
    pool.join();
}

fn fnn() {
    println!("call Fn() push");
}

fn fnn_task() {
    println!("call Fn() push Task");
}

fn fnn_old() {
    println!("call Fn() spawn");
}

fn fnm(msg: &mut String) {
    println!("{}", msg);
    *msg = "call FnMut() return".to_owned()
}

fn fno(msg: String) {
    println!("{}", msg);
}

// #[allow(deprecated)]
// #[test]
// fn push_2_panic() {
//     let pool = Pool::new().run().unwrap();
//     pool.push(move || panic!("push a panic!()"));
//     pool.spawn(Box::new(||panic!("spawn a panic!()" )));
//     pool.join();
// }
