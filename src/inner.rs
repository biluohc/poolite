/// Defaults thread's idle time(ms).
const TIME_OUT_MS: u64 = 5_000;
/// Defaults open daemon.
// const DAEMON: Option<Duration> = Some(Duration::from_millis(TIME_OUT_MS));
static mut NUM_CPUS: usize = 0;
static INIT: Once = ONCE_INIT;

/// The Task Box
pub type Task = Box<Runable + Send + 'static>;

/// The `Runable` trait for `FnOnce()`
pub trait Runable {
    fn call(self: Box<Self>);
}
impl<F: FnOnce()> Runable for F {
    #[inline]
    fn call(self: Box<Self>) {
        (*self)()
    }
}

#[derive(Debug)]
struct Inner {
    workers: Arc<Builder>,
}

/** `Pool`'s Settings

```
extern crate poolite;
use poolite::Builder;

/// `cargo run --example without`
fn main() {
    let pool = Builder::new()
    .min(2)
    .max(9)
    .daemon(None)  // Close daemon
    .timeout(None) // Close timeout
    .name("Worker")
    .stack_size(1024*1024*2) //2Mib
    .run()
    .unwrap();

    for i in 0..33 {
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
```
*/
pub struct Builder {
    name: Option<String>,
    stack_size: Option<usize>,
    min: usize,
    max: usize,
    timeout: Option<Duration>,
    load_limit: usize,
    daemon: Option<Duration>,

    mp: Sender<Task>,
    mc: Receiver<Task>,
    threads_future: AtomicUsize, // contains ready to create, consider create failed
    threads_alive: AtomicUsize, // alive, contains busy with task and wait for Task arrive
    threads_waiting: AtomicUsize, // wait for Task arrive
    daemon_alive: AtomicBool,
    dropped: AtomicBool,
}
impl Debug for Builder {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Builder")
            .field("name", &self.name)
            .field("stack_size", &self.stack_size)
            .field("min", &self.min)
            .field("max", &self.max)
            .field("timeout", &self.timeout)
            .field("load_limit", &self.load_limit)
            .field("daemon", &self.daemon)
            .field("mp", &"Sender<Task>")
            .field("mc", &"Receiver<Task>")
            .field("threads_future", &self.threads_future)
            .field("threads_alive", &self.threads_alive)
            .field("threads_waiting", &self.threads_waiting)
            .field("daemon_alive", &self.daemon_alive)
            .field("dropped", &self.dropped)
            .finish()
    }
}


impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}
impl Builder {
    #[inline]
    pub fn num_cpus() -> usize {
        unsafe {
            INIT.call_once(|| { NUM_CPUS = num_cpus::get(); });
            NUM_CPUS
        }
    }
    pub fn new() -> Self {
        let (mp, mc) = unbounded();
        Self {
            mp: mp,
            mc: mc,
            threads_future: AtomicUsize::default(),
            threads_alive: AtomicUsize::default(),
            threads_waiting: AtomicUsize::default(),

            min: Self::min_default(),
            max: Self::max_default(),
            timeout: Some(Duration::from_millis(TIME_OUT_MS)),
            name: None,
            stack_size: None,
            load_limit: Self::num_cpus() * Self::num_cpus(),
            daemon: Some(Duration::from_millis(TIME_OUT_MS)),
            daemon_alive: AtomicBool::default(),
            dropped: AtomicBool::default(),
        }
    }
    fn min_default() -> usize {
        Self::num_cpus() + 1
    }
    fn max_default() -> usize {
        (Self::num_cpus() + 1) * Self::num_cpus()
    }
    /// Sets thread's name where them in the Pool,default is None(`'<unnamed>'`).
    pub fn name<S>(mut self, name: S) -> Self
    where
        S: Debug + Into<String>,
    {
        self.name = Some(name.into());
        self
    }
    #[inline]
    pub fn name_get(&self) -> Option<&String> {
        self.name.as_ref()
    }
    /// Sets thread's stack_size where them in the Pool,default depends on OS.
    pub fn stack_size(mut self, size: usize) -> Self {
        self.stack_size = Some(size);
        self
    }
    #[inline]
    pub fn stack_size_get(&self) -> Option<&usize> {
        self.stack_size.as_ref()
    }
    /// Sets the minimum number of threads in the Pool，default is `num_cpus()+1`.
    pub fn min(mut self, min: usize) -> Self {
        if self.max < min && self.max == Self::max_default() {
            self.max = min;
        }
        self.min = min;
        self
    }
    #[inline]
    pub fn min_get(self: &Self) -> &usize {
        &self.min
    }
    /// Sets the maximum number of threads in the Pool，default is `(num_cpus()+1)*num_cpus()`.
    pub fn max(mut self, max: usize) -> Self {
        if self.min > max && self.min == Self::min_default() {
            self.min = max;
        }
        self.max = max;
        self
    }
    #[inline]
    pub fn max_get(&self) -> &usize {
        &self.max
    }
    /// Sets thread's idle time(ms) except minimum number of threads,default is 5000(ms).
    pub fn timeout_ms(self, timeout: Option<u64>) -> Self {
        self.timeout(timeout.map(Duration::from_millis))
    }
    pub fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }
    #[inline]
    pub fn timeout_get(&self) -> Option<&Duration> {
        self.timeout.as_ref()
    }
    /// Sets whether to open the daemon for the Pool, the default is `Some(5000)`(thread's default idle time(ms)).
    ///
    /// You can use `None` to close.
    pub fn daemon_ms(mut self, daemon: Option<u64>) -> Self {
        self.daemon = daemon.map(Duration::from_millis);
        self
    }
    pub fn daemon(mut self, daemon: Option<Duration>) -> Self {
        self.daemon = daemon;
        self
    }
    #[inline]
    pub fn daemon_get(&self) -> Option<&Duration> {
        self.daemon.as_ref()
    }
    /// Sets the value of load_limit for the Pool.
    ///
    /// default is `num_cpus() * num_cpus()`.
    pub fn load_limit(mut self, load_limit: usize) -> Self {
        self.load_limit = load_limit;
        self
    }
    #[inline]
    pub fn load_limit_get(&self) -> &usize {
        &self.load_limit
    }
    pub fn run(self) -> Result<Pool, PoolError> {
        Pool::with_builder(self).run()
    }
}

impl Clone for Inner {
    fn clone(&self) -> Self {
        Self { workers: self.workers.clone() }
    }
}

impl Inner {
    // pub fn new() -> Self {
    //     Self::with_builder(Builder::default())
    // }
    pub fn with_builder(builder: Builder) -> Self {
        Self { workers: Arc::new(builder) }
    }
    #[inline]
    pub fn as_builder(&self) -> &Builder {
        &self.workers
    }
    pub fn run(&mut self) -> io::Result<()> {
        let mut result = Ok(());
        if self.as_builder().daemon_get().is_some() {
            let daemon = self.clone();
            let mut b = thread::Builder::new();
            if let Some(name) = daemon.as_builder().name_get() {
                b = b.name(name.to_string());
            }

            result = b.spawn(move || {
                let daemon = daemon;
                let min = daemon.as_builder().min_get();
                let _alive = Alive::add(&daemon.as_builder().daemon_alive);
                let time = daemon.as_builder().daemon_get().unwrap();
                loop {
                    thread::sleep(*time);
                    if daemon.dropped() {
                        return;
                    }
                    let future = daemon.threads_future();
                    let add_num = if future < *min { min - future } else { 0 };
                    let _result = daemon.add_threads(add_num);
                }
            }).map(|_| ());
        }
        self.add_threads(*self.as_builder().min_get()).map_err(
            |e| e.1,
        )?;
        result
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        // All threads are waiting and tasks_queue'length is 0.
        self.threads_waiting() == self.threads_alive() && self.tasks_len() == 0
    }
    #[inline]
    pub fn tasks_len(&self) -> usize {
        self.as_builder().mc.len()
    }
    #[inline]
    pub fn threads_future(&self) -> usize {
        self.as_builder().threads_future.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn threads_alive(&self) -> usize {
        self.as_builder().threads_alive.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn threads_waiting(&self) -> usize {
        self.as_builder().threads_waiting.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn daemon_alive(&self) -> bool {
        self.as_builder().daemon_alive.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn dropped(&self) -> bool {
        self.as_builder().dropped.load(Ordering::Relaxed)
    }
    #[inline]
    pub fn push(&self, task: Task) {
        // 注意min==0 且 load_limit>0 时,线程池里无线程则前 load_limit 个请求会一直阻塞。
        self.as_builder()
            .mp
            .send(task)
            .log_err(|e| error!("Send Task failed: {}", e.description()))
            .unwrap();
        let len = self.threads_future();
        if len == 0 || len < *self.as_builder().max_get() && self.threads_waiting() == 0 && self.tasks_len() / len > *self.as_builder().load_limit_get() {
            let _ = self.add_threads(1);
        }
    }
    pub fn add_threads(&self, add_num: usize) -> Result<(), (usize, io::Error)> {
        self.as_builder().threads_future.fetch_add(
            add_num,
            Ordering::SeqCst,
        );
        for idx in 0..add_num {
            if let Err(e) = self.add_thread() {
                self.as_builder().threads_future.fetch_sub(
                    add_num - idx - 1,
                    Ordering::SeqCst,
                );
                return Err((idx + 1, e));
            }
        }
        Ok(())
    }
    ///use the above to maintain future
    fn add_thread(&self) -> io::Result<()> {
        let worker = self.clone();
        // 线程命名。
        let mut thread = match self.as_builder().name_get() {
            Some(name) => thread::Builder::new().name(name.to_string()),
            None => thread::Builder::new(),
        };
        // 线程栈大小设置。
        thread = match self.as_builder().stack_size_get() {
            Some(size) => thread.stack_size(*size),
            None => thread,
        };
        // spawn 有延迟, 不能直接检测len大小就新建.
        let spawn_result = thread.spawn(move || {
            // 对线程计数.
            let _threads_counter = Counter::add(&worker.as_builder().threads_alive);

            let min = worker.as_builder().min_get();
            let mut task: Task;
            if let Some(timeout) = worker.as_builder().timeout_get() {
                loop {
                    loop {
                        if worker.dropped() {
                            return;
                        }
                        // 对在等候的线程计数.
                        let _threads_waited_counter = Counter::add(&worker.as_builder().threads_waiting);
                        match worker.as_builder().mc.recv_timeout(*timeout) {
                            Ok(t) => {
                                task = t;
                                break;
                            }
                            Err(RecvTimeoutError::Timeout) => {
                                if !worker.as_builder().mc.is_empty() && worker.threads_future() > *min {
                                    return;
                                }
                            }
                            _ => {
                                return;
                            }
                        }
                    }
                    task.call();
                }
            } else {
                loop {
                    loop {
                        if worker.dropped() {
                            return;
                        }
                        let _threads_waited_counter = Counter::add(&worker.as_builder().threads_waiting);
                        if let Ok(t) = worker.as_builder().mc.recv() {
                            task = t;
                            break;
                        } else {
                            return;
                        }
                    }
                    task.call();
                }
            }
        }); //spawn 线程结束。

        spawn_result.map(|_| ()).log_err(|e| {
            error!("add thread failed: '{}' !", e.description())
        })
    }
}


// 通过作用域对线程数目计数。
struct Counter<'a> {
    count: &'a AtomicUsize,
}

impl<'a> Counter<'a> {
    #[inline]
    fn add(count: &'a AtomicUsize) -> Counter<'a> {
        count.fetch_add(1, Ordering::Release);
        Counter { count: count }
    }
}

impl<'a> Drop for Counter<'a> {
    #[inline]
    fn drop(&mut self) {
        self.count.fetch_sub(1, Ordering::Release);
    }
}
// 通过作用域对Daemon状态管理。
struct Alive<'a> {
    state: &'a AtomicBool,
}

impl<'a> Alive<'a> {
    #[inline]
    fn add(alive: &'a AtomicBool) -> Alive<'a> {
        alive.store(true, Ordering::SeqCst);
        Self { state: alive }
    }
}

impl<'a> Drop for Alive<'a> {
    #[inline]
    fn drop(&mut self) {
        self.state.store(false, Ordering::SeqCst);
    }
}