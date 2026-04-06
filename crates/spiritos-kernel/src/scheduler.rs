//! Round-robin preemptive scheduler.
//!
//! Supports up to MAX_TASKS tasks. Timer interrupt calls `schedule()`.

use crate::task::{Task, TaskState, KERNEL_STACK_SIZE};
use crate::context;
use core::sync::atomic::{AtomicUsize, AtomicBool, Ordering};

const MAX_TASKS: usize = 16;

// Option<Task> is not Copy; use a const sentinel to initialise the array.
const NONE_TASK: Option<Task> = None;

#[allow(static_mut_refs)]
static mut TASKS: [Option<Task>; MAX_TASKS] = [NONE_TASK; MAX_TASKS];
#[allow(static_mut_refs)]
static mut TASK_COUNT: usize = 0;

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initialise the scheduler subsystem.
pub fn init() {
    INITIALIZED.store(true, Ordering::Relaxed);
}

/// Spawn a new task. Returns its task ID.
///
/// # Safety
/// Must not be called concurrently.
pub unsafe fn spawn(name: &'static str, priority: u8, entry: fn() -> !) -> usize {
    #[allow(static_mut_refs)]
    let n = TASK_COUNT;
    assert!(n < MAX_TASKS, "too many tasks");

    // Allocate a kernel stack via the memory manager.
    let stack_base = crate::mm::alloc_bytes(KERNEL_STACK_SIZE)
        .expect("OOM while spawning task");

    let mut task = Task::new(name, priority, stack_base, entry);

    // Ask the arch backend to lay out the initial register frame so that
    // switching to this task will jump to `entry`.
    let ctx = context::get_backend();
    let new_sp = ctx.init_stack(stack_base + KERNEL_STACK_SIZE, entry);
    task.sp = new_sp;

    let id = task.id;
    #[allow(static_mut_refs)]
    {
        TASKS[n] = Some(task);
        TASK_COUNT += 1;
    }
    id
}

/// Called by the timer interrupt. Picks the next Ready task and switches to it.
pub fn schedule() {
    unsafe {
        #[allow(static_mut_refs)]
        let count = TASK_COUNT;
        if count == 0 {
            return;
        }

        let current_idx = CURRENT.load(Ordering::Relaxed);

        // Round-robin: find next runnable task.
        let mut next_idx = (current_idx + 1) % count;
        let mut found = false;
        for _ in 0..count {
            #[allow(static_mut_refs)]
            if let Some(ref t) = TASKS[next_idx] {
                if t.state == TaskState::Ready || t.state == TaskState::Running {
                    found = true;
                    break;
                }
            }
            next_idx = (next_idx + 1) % count;
        }
        if !found {
            return;
        }
        // Nothing to do if the only runnable task is the current one.
        if next_idx == current_idx {
            return;
        }

        // Transition states.
        #[allow(static_mut_refs)]
        if let Some(ref mut cur) = TASKS[current_idx] {
            if cur.state == TaskState::Running {
                cur.state = TaskState::Ready;
            }
        }
        #[allow(static_mut_refs)]
        if let Some(ref mut nxt) = TASKS[next_idx] {
            nxt.state = TaskState::Running;
        }

        CURRENT.store(next_idx, Ordering::Relaxed);

        let current_sp_ptr = {
            #[allow(static_mut_refs)]
            let cur = TASKS[current_idx].as_mut().unwrap();
            &mut cur.sp as *mut usize
        };
        #[allow(static_mut_refs)]
        let next_sp = TASKS[next_idx].as_ref().unwrap().sp;

        let ctx = context::get_backend();
        ctx.switch_to(current_sp_ptr, next_sp);
    }
}

/// Cooperative yield — immediately reschedule.
pub fn yield_now() {
    schedule();
}

/// The idle task — runs when no other task is ready.
pub fn idle_task() -> ! {
    loop {
        spiritos_hal::platform::get().cpu.wait_for_interrupt();
    }
}

/// Start the scheduler: spawn the idle task and switch into it.
pub fn run() -> ! {
    unsafe {
        // Task 0 is always the idle task.
        spawn("idle", 255, idle_task);
        #[allow(static_mut_refs)]
        if let Some(ref mut t) = TASKS[0] {
            t.state = TaskState::Running;
        }
        CURRENT.store(0, Ordering::Relaxed);

        // Bootstrap: switch from a throw-away dummy context into idle.
        let mut dummy_sp: usize = 0;
        #[allow(static_mut_refs)]
        let idle_sp = TASKS[0].as_ref().unwrap().sp;
        let ctx = context::get_backend();
        ctx.switch_to(&mut dummy_sp as *mut usize, idle_sp);
    }
    unreachable!()
}
