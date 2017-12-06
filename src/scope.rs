impl Pool {
    pub fn scoped<'pool, 'scope, Scheduler>(&'pool self, scheduler: Scheduler)
    where
        Scheduler: FnOnce(&Scoped<'pool, 'scope>),
    {
        let scoped = Scoped::new(&self);
        scheduler(&scoped);
    }
}

/// `Scoped` impl
#[derive(Debug)]
pub struct Scoped<'pool, 'scope> {
    pool: &'pool Pool,
    tasks: Arc<()>,
    maker: PhantomData<std::cell::Cell<&'scope mut ()>>,
}

impl<'pool, 'scope> Scoped<'pool, 'scope> {
    fn new(pool: &'pool Pool) -> Self {
        Self {
            pool: pool,
            maker: PhantomData,
            tasks: Arc::default(),
        }
    }
    pub fn push<T>(&self, task: T)
    where
        T: Runable + Send + 'scope,
    {
        let task = unsafe { transmute::<Box<Runable + Send + 'scope>, Box<Runable + Send + 'static>>(Box::new(task)) };
        let arc = self.tasks.clone();

        let task = move || {
            let _arc = arc; // maintain the number of tasks
            task.call();
        };
        self.pool.push(task);
    }
}
impl<'pool, 'scope> Drop for Scoped<'pool, 'scope> {
    fn drop(&mut self) {
        while Arc::strong_count(&self.tasks) > 1 {
            thread::sleep(Duration::from_millis(10));
        }
    }
}