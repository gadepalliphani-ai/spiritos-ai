//! aarch64 context switching.
//!
//! ## Saved context layout (on stack, ascending from new `sp`):
//! Per the AArch64 PCS, callee-saved general-purpose registers are x19–x28,
//! plus x29 (frame pointer) and x30 (link register / return address).
//!
//! ```text
//! [sp+ 0] x19
//! [sp+ 8] x20
//! [sp+16] x21
//! [sp+24] x22
//! [sp+32] x23
//! [sp+40] x24
//! [sp+48] x25
//! [sp+56] x26
//! [sp+64] x27
//! [sp+72] x28
//! [sp+80] x29  (frame pointer)
//! [sp+88] x30  (link register — entry point on first switch)
//! ```

use spiritos_kernel::context::ContextSwitch;

pub struct Aarch64Context;
unsafe impl Send for Aarch64Context {}
unsafe impl Sync for Aarch64Context {}

pub static AARCH64_CONTEXT: Aarch64Context = Aarch64Context;

impl ContextSwitch for Aarch64Context {
    unsafe fn init_stack(&self, stack_top: usize, entry: fn() -> !) -> usize {
        // Stack must be 16-byte aligned on AArch64.
        let sp = (stack_top & !0xF) - 96; // 12 registers × 8 bytes = 96 bytes

        let frame = sp as *mut usize;
        // x19–x28 initialised to zero.
        for i in 0..10 {
            *frame.add(i) = 0;
        }
        // x29 (frame pointer) = 0 — no previous frame.
        *frame.add(10) = 0;
        // x30 (link register) = entry — `ret` branches here on first switch.
        *frame.add(11) = entry as usize;

        sp
    }

    unsafe fn switch_to(&self, current_sp: *mut usize, next_sp: usize) {
        // NOTE: `options(nostack)` is intentionally absent — we manipulate sp.
        core::arch::asm!(
            // --- Save callee-saved registers onto the current stack ---
            // Allocate 96 bytes and store x19:x20 at [sp].
            "stp x19, x20, [sp, #-96]!",
            "stp x21, x22, [sp, #16]",
            "stp x23, x24, [sp, #32]",
            "stp x25, x26, [sp, #48]",
            "stp x27, x28, [sp, #64]",
            "stp x29, x30, [sp, #80]",

            // --- Persist current sp ---
            // sp cannot be used as a source in ordinary instructions directly,
            // so move it to a temporary general-purpose register first.
            "mov x9, sp",
            "str x9, [{current_sp}]",

            // --- Switch to next task's stack ---
            "mov sp, {next_sp}",

            // --- Restore callee-saved registers ---
            "ldp x29, x30, [sp, #80]",
            "ldp x27, x28, [sp, #64]",
            "ldp x25, x26, [sp, #48]",
            "ldp x23, x24, [sp, #32]",
            "ldp x21, x22, [sp, #16]",
            "ldp x19, x20, [sp], #96",

            // Return into the next task (branches to x30 = saved lr or entry).
            "ret",

            current_sp = in(reg) current_sp,
            next_sp    = in(reg) next_sp,
            // No `options(nostack)` — we are modifying sp.
        );
    }
}
