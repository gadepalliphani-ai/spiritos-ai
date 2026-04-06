//! x86_64 context switching.
//!
//! ## Saved context layout (on stack, ascending from `rsp` after switch_to pushes):
//!
//! `switch_to` saves with: `pushfq`, `push r15`, `push r14`, `push r13`,
//! `push r12`, `push rbp`, `push rbx`.  The last push leaves `rbx` at `[rsp]`.
//!
//! ```text
//! [rsp+0x00] rbx   ← rsp points here (last push / first pop)
//! [rsp+0x08] rbp
//! [rsp+0x10] r12
//! [rsp+0x18] r13
//! [rsp+0x20] r14
//! [rsp+0x28] r15
//! [rsp+0x30] rflags
//! [rsp+0x38] rip   ← ret pops this to resume the task
//! ```
//!
//! `init_stack` must lay out the initial frame in exactly this order so that
//! the first `switch_to` into a new task restores the correct values.

use spiritos_kernel::context::ContextSwitch;

pub struct X86Context;
unsafe impl Send for X86Context {}
unsafe impl Sync for X86Context {}

pub static X86_CONTEXT: X86Context = X86Context;

impl ContextSwitch for X86Context {
    unsafe fn init_stack(&self, stack_top: usize, entry: fn() -> !) -> usize {
        // The restored frame must match switch_to's pop sequence exactly:
        //   pop rbx, pop rbp, pop r12, pop r13, pop r14, pop r15, popfq, ret
        //
        // We build it at fixed offsets from `sp` so the order is unambiguous.
        // Total frame = 7 registers (8 bytes each) + 1 rip (8 bytes) = 64 bytes.
        // Align sp to 16 bytes and subtract the full frame size.
        let sp = (stack_top & !0xF) - 64;

        let frame = sp as *mut usize;
        // Offsets match switch_to's pop order (ascending from rsp):
        *frame.add(0) = 0;            // rbx
        *frame.add(1) = 0;            // rbp
        *frame.add(2) = 0;            // r12
        *frame.add(3) = 0;            // r13
        *frame.add(4) = 0;            // r14
        *frame.add(5) = 0;            // r15
        *frame.add(6) = 0x0202;       // rflags: IF=1 (bit 9), reserved bit 1 set
        *frame.add(7) = entry as usize; // rip: `ret` jumps here

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
