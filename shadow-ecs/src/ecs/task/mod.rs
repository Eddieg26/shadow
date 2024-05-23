use std::{
    sync::{mpsc::Sender, Arc, Condvar, Mutex, MutexGuard},
    thread::{sleep, JoinHandle},
};

struct Worker {
    id: usize,
    thread: Option<JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, thread: JoinHandle<()>) -> Self {
        Self {
            id,
            thread: Some(thread),
        }
    }
}

impl std::fmt::Display for Worker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[Worker {}]", self.id)
    }
}

pub struct TaskPool {
    workers: Vec<Worker>,
    sender: Sender<Job>,
}

type Job = Option<Box<dyn FnOnce() + Send + 'static>>;

impl TaskPool {
    pub fn new(size: usize) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        let receiver = std::sync::Arc::new(std::sync::Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            let receiver = receiver.clone();
            let thread = std::thread::spawn(move || loop {
                let job: Job = receiver.lock().unwrap().recv().unwrap();

                match job {
                    Some(job) => job(),
                    None => break,
                }
            });

            workers.push(Worker::new(id, thread));
        }

        Self { workers, sender }
    }

    pub fn execute(&self, f: impl FnOnce() + Send + 'static) {
        self.sender.send(Some(Box::new(f))).unwrap();
    }

    pub fn join(&mut self) {
        for worker in &mut self.workers {
            self.sender.send(None).unwrap();
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

impl Drop for TaskPool {
    fn drop(&mut self) {
        self.join();
    }
}

pub struct ScopedSender<'a> {
    sender: Sender<ScopedJob<'a>>,
    thread_count: usize,
}

impl<'a> ScopedSender<'a> {
    pub fn new(sender: Sender<ScopedJob<'a>>, thread_count: usize) -> Self {
        Self {
            sender,
            thread_count,
        }
    }

    pub fn send(&self, f: impl FnOnce() + Send + Sync + 'a) {
        let _ = self.sender.send(Some(Box::new(f)));
    }

    pub fn join(&self) {
        for _ in 0..self.thread_count {
            let _ = self.sender.send(None);
        }
    }
}

impl<'a> Drop for ScopedSender<'a> {
    fn drop(&mut self) {
        self.join();
    }
}

type ScopedJob<'a> = Option<Box<dyn FnOnce() + Send + 'a>>;

pub struct ScopedTaskPool<'a> {
    sender: Sender<ScopedJob<'a>>,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> ScopedTaskPool<'a> {
    pub fn new(size: usize, executor: impl Fn(ScopedSender<'a>)) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        let receiver = std::sync::Arc::new(std::sync::Mutex::new(receiver));

        std::thread::scope(|scope| {
            for _ in 0..size {
                let receiver = receiver.clone();
                scope.spawn(move || loop {
                    let receiver = match receiver.lock() {
                        Ok(receiver) => receiver,
                        Err(_) => break,
                    };

                    let job: ScopedJob = match receiver.recv() {
                        Ok(job) => job,
                        Err(_) => break,
                    };

                    match job {
                        Some(job) => {
                            job();
                            sleep(std::time::Duration::from_nanos(1));
                        }
                        None => break,
                    }
                });
            }

            executor(ScopedSender::new(sender.clone(), size));
        });

        Self {
            sender,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn execute(&self, f: impl FnOnce() + Send + 'a) {
        self.sender.send(Some(Box::new(f))).unwrap();
    }

    pub fn join(&mut self) {
        self.sender.send(None).unwrap();
    }
}

pub struct JobBarrier {
    count: usize,
    total: usize,
    condvar: Arc<Condvar>,
}

impl JobBarrier {
    pub fn new<'a>(total: usize) -> (Self, BarrierLock) {
        let condvar = Arc::new(Condvar::new());
        let barrier = Self {
            count: 0,
            total,
            condvar: condvar.clone(),
        };

        let lock = BarrierLock::new(condvar);

        (barrier, lock)
    }

    pub fn notify(&mut self) {
        self.count += 1;

        if self.count >= self.total {
            self.condvar.notify_all();
        }
    }
}

pub struct BarrierLock {
    condvar: Arc<Condvar>,
    guard: Arc<Mutex<()>>,
}

impl BarrierLock {
    fn new(condvar: Arc<Condvar>) -> Self {
        let guard = Arc::new(Mutex::new(()));
        Self { condvar, guard }
    }

    pub fn wait(&self, barrier: MutexGuard<JobBarrier>) {
        let count = barrier.count;
        let total = barrier.total;

        if count < total {
            std::mem::drop(barrier);
            let guard = self.guard.lock().unwrap();
            std::mem::drop(self.condvar.wait(guard));
        }
    }
}
