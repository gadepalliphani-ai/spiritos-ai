//! x86_64 context switching.
//!
//! ## Saved context layout (on stack, top-down after switch_to pushes):
//! ```text
//! [rsp+0x38] rflags
//! [rsp+0x30] r15
//! [rsp+0x28] r14
//! [rsp+0x20] r13
//! [rsp+0x18] r12
//! [rsp+0x10] rbp
//! [rsp+0x08] rbx
//! [rsp+0x00] rip  (return address / task entry)
//! ```
//! `switch_to` saves this frame on the current stack, updates `*current_sp`,
//! loads `next_sp`, restores the frame, and `ret`s into the next task.

use spiritos_kernel::context::ContextSwitch;

pub struct X86Context;
unsafe impl Send for X86Context {}
unsafe impl Sync for X86Context {}

pub static X86_CONTEXT: X86Context = X86Context;

impl ContextSwitch for X86Context {
    unsafe fn init_stack(&self, stack_top: usize, entry: fn() -> !) -> usize {
        // x86-64 ABI: rsp must be 16-byte aligned *before* a `call` instruction,
        // which means 8-byte aligned at the function prologue (call pushes 8 bytes).
        // We align to 16 then subtract 8 to simulate "just entered via call".
        let mut sp = (stack_top & !0xF) - 8;

        // Fake return address — the entry function is `fn() -> !` and must never
        // return, but having a sentinel here makes stack traces cleaner.
        sp -= 8;
        *(sp as *mut usize) = 0usize;

        // Entry point: this will be `ret`-ed to by switch_to.
        sp -= 8;
        *(sp as *mut usize) = entry as usize;

        // Callee-saved registers saved by switch_to (pop order: rbx, rbp, r12..r15, rflags).
        // Lay them out so pop order matches push order in switch_to.
        sp -= 8; *(sp as *mut usize) = 0;      // rbx
        sp -= 8; *(sp as *mut usize) = 0;      // rbp
        sp -= 8; *(sp as *mut usize) = 0;      // r12
        sp -= 8; *(sp as *mut usize) = 0;      // r13
        sp -= 8; *(sp as *mut usize) = 0;      // r14
        sp -= 8; *(sp as *mut usize) = 0;      // r15
        sp -= 8; *(sp as *mut usize) = 0x0202; // rflags (IF=1, reserved bit 1)

        sp
    }

    unsafe fn switch_to(&self, current_sp: *mut usize, next_sp: usize) {
        // NOTE: `options(nostack)` is intentionally absent — we manipulate rsp.
        core::arch::asm!(
            // Save callee-saved registers + rflags.
            "pushfq",
            "push r15",
            "push r14",
            "push r13",
            "push r12",
            "push rbp",
            "push rbx",
            // Persist current stack pointer.
            "mov [{current_sp}], rsp",
            // Load next task's stack pointer.
            "mov rsp, {next_sp}",
            // Restore callee-saved registers.
            "pop rbx",
            "pop rbp",
            "pop r12",
            "pop r13",
            "pop r14",
            "pop r15",
            "popfq",
            // Jump into the next task (pops saved rip).
            "ret",
            current_sp = in(reg) current_sp,
            next_sp    = in(reg) next_sp,
            // No `options(nostack)` here — we are modifying rsp.
        );
    }
}
