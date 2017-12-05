extern crate poolite;
use poolite::Builder;

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
    eprintln!("\nTset poolite !");
    let st = SystemTime::now();
    for _ in 0..FOR {
        fibm();
    }
    let ed = SystemTime::now();
    eprintln!("{:?}\n", (ed.duration_since(st).unwrap()));
}
fn fibm() {
    let pool = Builder::new()
        .stack_size(3 * 1024 * 1024)
        .min(9)
        .max(10)
        .daemon(None)
        .timeout(None)
        .load_limit(Builder::num_cpus() * Builder::num_cpus())
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
    eprint!(
        "\nis_empty()/tasks_len(): {}/{} -> waiting()/alive()/future(): {}/{}/{}\n",
        pool.is_empty(),
        pool.tasks_len(),
        pool.threads_waiting(),
        pool.threads_alive(),
        pool.threads_future(),
    );
    thread::sleep(Duration::from_millis(5000));
    eprint!("loop0 finished ! main slept 5000 ms !\n");

    eprint!(
        "\nis_empty()/tasks_len(): {}/{} -> waiting()/alive()/future(): {}/{}/{}\n",
        pool.is_empty(),
        pool.tasks_len(),
        pool.threads_waiting(),
        pool.threads_alive(),
        pool.threads_future(),
    );

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
    eprint!("loop1 finished ! main slept 6000 ms !\n");
    eprint!(
        "\nis_empty()/tasks_len(): {}/{} -> waiting()/alive()/future(): {}/{}/{}\n",
        pool.is_empty(),
        pool.tasks_len(),
        pool.threads_waiting(),
        pool.threads_alive(),
        pool.threads_future(),
    );

    for (k, v) in map.lock().unwrap().iter() {
        println!("key: {}\tvalue: {}", k, v);
    }
    eprint!(
        "\nis_empty()/tasks_len(): {}/{} -> waiting()/alive()/future(): {}/{}/{}\n",
        pool.is_empty(),
        pool.tasks_len(),
        pool.threads_waiting(),
        pool.threads_alive(),
        pool.threads_future(),
    );
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
