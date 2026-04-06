//! AArch64 exception vector table and IRQ dispatch.
//!
//! ## Design
//! An AArch64 exception vector table (VBAR_EL1) is installed that routes
//! IRQ exceptions to a stub which:
//!   1. Saves the *full* interrupted CPU state (x0–x30, sp, elr_el1, spsr_el1)
//!      onto the current EL1 stack.
//!   2. Calls `irq_dispatch()` to invoke registered handlers.
//!   3. Restores all saved registers and executes `eret` to resume the
//!      interrupted task.
//!
//! This ensures that preemptive context switches triggered from IRQ context
//! preserve the complete CPU state of the interrupted task.
//!
//! The vector table must be aligned to 2 KiB (11-bit aligned) per the
//! AArch64 specification.

use core::arch::global_asm;

// ---- Vector table ----------------------------------------------------------
//
// AArch64 vector table layout (each slot is 0x80 bytes / 32 instructions):
//   0x000  EL1t synchronous
//   0x080  EL1t IRQ
//   0x100  EL1t FIQ
//   0x180  EL1t SError
//   0x200  EL1h synchronous
//   0x280  EL1h IRQ          ← we handle this one
//   0x300  EL1h FIQ
//   0x380  EL1h SError
//   0x400  EL0 64-bit synchronous
//   ... (not used in this minimal implementation)

global_asm!(
    // 2 KiB alignment required by the architecture.
    ".balign 2048",
    ".global __exception_vector_table",
    "__exception_vector_table:",

    // --- EL1t (SP_EL0) slots ---
    ".balign 0x80", "b .",   // EL1t sync  — halt
    ".balign 0x80", "b .",   // EL1t IRQ   — halt
    ".balign 0x80", "b .",   // EL1t FIQ   — halt
    ".balign 0x80", "b .",   // EL1t SError — halt

    // --- EL1h (SP_EL1) slots ---
    ".balign 0x80", "b .",                     // EL1h sync   — halt
    ".balign 0x80", "b __irq_entry_el1h",      // EL1h IRQ    ← handled
    ".balign 0x80", "b .",                     // EL1h FIQ    — halt
    ".balign 0x80", "b .",                     // EL1h SError — halt

    // --- EL0 AArch64 ---
    ".balign 0x80", "b .",   // EL0 sync   — halt
    ".balign 0x80", "b .",   // EL0 IRQ    — halt
    ".balign 0x80", "b .",   // EL0 FIQ    — halt
    ".balign 0x80", "b .",   // EL0 SError — halt

    // --- EL0 AArch32 ---
    ".balign 0x80", "b .",
    ".balign 0x80", "b .",
    ".balign 0x80", "b .",
    ".balign 0x80", "b .",

    // ---- IRQ entry stub ---------------------------------------------------
    // Saves full CPU state, calls irq_dispatch(), restores state, eret.
    ".balign 16",
    "__irq_entry_el1h:",

    // Allocate 272 bytes on the stack:
    //   x0-x29 (30 regs × 8 = 240), x30 (8), sp_el1 would require mrs, but
    //   we save the logical sp before the frame: elr_el1 (8), spsr_el1 (8).
    //   Total saved: x0-x30 (248) + elr (8) + spsr (8) = 264 → round to 272.
    "sub sp, sp, #272",
    "stp x0,  x1,  [sp, #0]",
    "stp x2,  x3,  [sp, #16]",
    "stp x4,  x5,  [sp, #32]",
    "stp x6,  x7,  [sp, #48]",
    "stp x8,  x9,  [sp, #64]",
    "stp x10, x11, [sp, #80]",
    "stp x12, x13, [sp, #96]",
    "stp x14, x15, [sp, #112]",
    "stp x16, x17, [sp, #128]",
    "stp x18, x19, [sp, #144]",
    "stp x20, x21, [sp, #160]",
    "stp x22, x23, [sp, #176]",
    "stp x24, x25, [sp, #192]",
    "stp x26, x27, [sp, #208]",
    "stp x28, x29, [sp, #224]",
    "str x30,       [sp, #240]",
    // Save ELR_EL1 (return address) and SPSR_EL1.
    "mrs x0, elr_el1",
    "mrs x1, spsr_el1",
    "stp x0, x1, [sp, #248]",

    // Call irq_dispatch() — no argument needed (GIC IAR read inside dispatch).
    "mov x0, #0",          // placeholder irq arg (dispatch reads GIC or uses 0)
    "bl {dispatch}",

    // Restore ELR_EL1 and SPSR_EL1.
    "ldp x0, x1, [sp, #248]",
    "msr elr_el1, x0",
    "msr spsr_el1, x1",
    // Restore GPRs.
    "ldr x30,       [sp, #240]",
    "ldp x28, x29, [sp, #224]",
    "ldp x26, x27, [sp, #208]",
    "ldp x24, x25, [sp, #192]",
    "ldp x22, x23, [sp, #176]",
    "ldp x20, x21, [sp, #160]",
    "ldp x18, x19, [sp, #144]",
    "ldp x16, x17, [sp, #128]",
    "ldp x14, x15, [sp, #112]",
    "ldp x12, x13, [sp, #96]",
    "ldp x10, x11, [sp, #80]",
    "ldp x8,  x9,  [sp, #64]",
    "ldp x6,  x7,  [sp, #48]",
    "ldp x4,  x5,  [sp, #32]",
    "ldp x2,  x3,  [sp, #16]",
    "ldp x0,  x1,  [sp, #0]",
    "add sp, sp, #272",

    // Return from exception to the interrupted context.
    "eret",

    dispatch = sym irq_dispatch,
);

// ---- Rust-side dispatch ----------------------------------------------------

/// Called from the IRQ entry stub.  Invokes any registered handler.
extern "C" fn irq_dispatch(_irq: u32) {
    // On a real GIC we would read IAR, extract the IRQ number, call the
    // handler, and write EOIR.  With the current GicStub there is no hardware
    // register to read, so we call handler 30 (Generic Timer PPI) if it is
    // registered, as that is the only IRQ this stub currently handles.
    crate::interrupts::dispatch(30);
}

// ---- Public init -----------------------------------------------------------

extern "C" {
    static __exception_vector_table: u8;
}

/// Install the exception vector table by writing VBAR_EL1.
pub fn init() {
    unsafe {
        let vbar = &__exception_vector_table as *const u8 as u64;
        core::arch::asm!(
            "msr vbar_el1, {v}",
            "isb",
            v = in(reg) vbar,
            options(nomem, nostack)
        );
    }
}
