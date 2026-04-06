//! Task Control Block (TCB).
#![allow(dead_code)]

use core::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

/// Kernel stack size per task (8 KiB).
pub const KERNEL_STACK_SIZE: usize = 8192;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Dead,
}

/// Task Control Block.
pub struct Task {
    pub id: usize,
    pub name: &'static str,
    pub state: TaskState,
    pub priority: u8,
    /// Stack pointer — points to top of saved context on the stack.
    pub sp: usize,
    /// Base of the kernel stack allocation.
    pub stack_base: usize,
}

impl Task {
    /// Create a new task. `stack_base` must be a valid allocation of at least
    /// `KERNEL_STACK_SIZE` bytes. `entry` is the task's entry function.
    ///
    /// # Safety
    /// `stack_base` must point to valid, exclusive memory of at least `KERNEL_STACK_SIZE` bytes.
    pub unsafe fn new(
        name: &'static str,
        priority: u8,
        stack_base: usize,
        _entry: fn() -> !,
    ) -> Self {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        // Stack grows downward; start at top.
        // Architecture-specific context setup is done by the arch crate.
        // We store sp = stack_top initially; arch init_stack() will fill it in.
        let stack_top = stack_base + KERNEL_STACK_SIZE;
        Task {
            id,
            name,
            state: TaskState::Ready,
            priority,
            sp: stack_top,
            stack_base,
        }
    }
}
