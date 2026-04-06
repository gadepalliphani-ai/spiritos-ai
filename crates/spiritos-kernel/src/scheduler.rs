use core::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum TaskState { Ready, Running, Blocked, Dead }

#[allow(dead_code)]
pub struct Task {
    pub id:    usize,
    pub state: TaskState,
    pub name:  &'static str,
}

static TASK_COUNT: AtomicUsize = AtomicUsize::new(0);

pub fn init() {
    TASK_COUNT.store(1, Ordering::Relaxed);
}

pub fn run() -> ! {
    loop { core::hint::spin_loop(); }
}
