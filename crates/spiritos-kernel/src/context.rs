//! Architecture-agnostic context switching trait.
//!
//! Each architecture crate implements this trait and registers it via
//! `context::set_backend()`.

/// Context switching backend — implemented per architecture.
pub trait ContextSwitch: Send + Sync {
    /// Set up the initial stack frame for a new task so that switching to it
    /// will jump to `entry`.
    ///
    /// Returns the new stack pointer (top of saved context frame).
    ///
    /// # Safety
    /// `stack_top` must be the top of a valid, exclusive stack allocation.
    unsafe fn init_stack(&self, stack_top: usize, entry: fn() -> !) -> usize;

    /// Switch CPU context from `current_sp` to `next_sp`.
    ///
    /// Saves callee-saved registers + stack pointer into `*current_sp`,
    /// restores from `next_sp`, and returns into the next task.
    ///
    /// # Safety
    /// Both pointers must point to valid task stacks set up by `init_stack`.
    unsafe fn switch_to(&self, current_sp: *mut usize, next_sp: usize);
}

#[allow(static_mut_refs)]
static mut BACKEND: Option<&'static dyn ContextSwitch> = None;

/// Register the arch context-switch backend. Call once during boot.
///
/// # Safety
/// Must be called exactly once before `get_backend()` is used.
pub unsafe fn set_backend(b: &'static dyn ContextSwitch) {
    #[allow(static_mut_refs)]
    {
        BACKEND = Some(b);
    }
}

/// Retrieve the registered context-switch backend.
///
/// # Panics
/// Panics if `set_backend` has not been called.
pub fn get_backend() -> &'static dyn ContextSwitch {
    #[allow(static_mut_refs)]
    unsafe {
        BACKEND.expect("context switch backend not registered")
    }
}
