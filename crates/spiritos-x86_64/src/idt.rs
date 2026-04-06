//! x86_64 Interrupt Descriptor Table (IDT) and interrupt dispatch.
//!
//! ## Design
//! - A minimal IDT is installed to handle hardware IRQs 0–15 (PIC lines).
//! - Each IRQ has a naked entry stub (generated via macro) that:
//!   1. Saves the full GPR set (caller-saved + callee-saved) + RFLAGS.
//!   2. Calls `irq_dispatch(irq_number)`.
//!   3. Restores all registers and executes `iretq`.
//! - The saved frame satisfies the correctness requirement for preemptive
//!   context switching: any task can be interrupted and its full state
//!   preserved / restored.
//!
//! Note: for true preemptive switching from an IRQ, the timer handler (via
//! `irq_dispatch`) calls `scheduler::schedule()` which uses the cooperative
//! `switch_to()` (saves/restores callee-saved regs).  This is correct because
//! `irq_dispatch` is a Rust function called from a C-ABI-compatible context:
//! the compiler already saves/restores caller-saved registers around the call,
//! and our stub saves them explicitly.  The net result is that the IRQ stub
//! saves and restores *all* GPRs around the call to `schedule()`.

use core::arch::asm;

// ---- IDT entry type --------------------------------------------------------

/// An IDT gate descriptor (64-bit mode, interrupt gate).
#[derive(Clone, Copy)]
#[repr(C, packed)]
struct IdtEntry {
    offset_lo:  u16,
    selector:   u16,
    ist_and_zero: u8,
    type_attr:  u8,
    offset_mid: u16,
    offset_hi:  u32,
    _reserved:  u32,
}

impl IdtEntry {
    const fn missing() -> Self {
        IdtEntry {
            offset_lo: 0, selector: 0, ist_and_zero: 0,
            type_attr: 0, offset_mid: 0, offset_hi: 0, _reserved: 0,
        }
    }

    fn set(&mut self, handler: u64) {
        // Code segment selector: 0x08 (first GDT entry after null).
        // Type: 64-bit interrupt gate = 0x8E (present, DPL=0, type=0xE).
        self.offset_lo  = handler as u16;
        self.selector   = 0x08;
        self.ist_and_zero = 0;
        self.type_attr  = 0x8E;
        self.offset_mid = (handler >> 16) as u16;
        self.offset_hi  = (handler >> 32) as u32;
        self._reserved  = 0;
    }
}

/// The IDT: 256 entries.  We only populate the PIC IRQ vectors (0x20–0x2F).
static mut IDT: [IdtEntry; 256] = [IdtEntry::missing(); 256];

#[repr(C, packed)]
struct Idtr {
    limit: u16,
    base:  u64,
}

// ---- Naked ISR stubs -------------------------------------------------------
//
// Each ISR stub:
//   - pushes all GPRs (rax, rcx, rdx, rsi, rdi, r8-r11 = caller-saved;
//     rbx, rbp, r12-r15 = callee-saved) plus rflags (already pushed by CPU
//     at interrupt: ss, rsp, rflags, cs, rip).
//   - passes the IRQ number in rdi (System V AMD64 calling convention arg 1).
//   - calls `irq_dispatch`.
//   - restores GPRs and executes iretq.
//
// We use a macro to generate one stub per IRQ line (0–15, mapped to vectors
// 0x20–0x2F after PIC remapping).

macro_rules! irq_stub {
    ($name:ident, $irq:expr) => {
        #[unsafe(naked)]
        unsafe extern "C" fn $name() {
            core::arch::naked_asm!(
                // Push all GPRs
                "push rax",
                "push rcx",
                "push rdx",
                "push rsi",
                "push rdi",
                "push r8",
                "push r9",
                "push r10",
                "push r11",
                "push rbx",
                "push rbp",
                "push r12",
                "push r13",
                "push r14",
                "push r15",
                // Pass IRQ number as first argument
                concat!("mov rdi, ", $irq),
                // Call the Rust dispatch function
                "call {dispatch}",
                // Restore all GPRs
                "pop r15",
                "pop r14",
                "pop r13",
                "pop r12",
                "pop rbp",
                "pop rbx",
                "pop r11",
                "pop r10",
                "pop r9",
                "pop r8",
                "pop rdi",
                "pop rsi",
                "pop rdx",
                "pop rcx",
                "pop rax",
                // Return from interrupt
                "iretq",
                dispatch = sym irq_dispatch,
            );
        }
    };
}

irq_stub!(irq0_stub,  0);
irq_stub!(irq1_stub,  1);
irq_stub!(irq2_stub,  2);
irq_stub!(irq3_stub,  3);
irq_stub!(irq4_stub,  4);
irq_stub!(irq5_stub,  5);
irq_stub!(irq6_stub,  6);
irq_stub!(irq7_stub,  7);
irq_stub!(irq8_stub,  8);
irq_stub!(irq9_stub,  9);
irq_stub!(irq10_stub, 10);
irq_stub!(irq11_stub, 11);
irq_stub!(irq12_stub, 12);
irq_stub!(irq13_stub, 13);
irq_stub!(irq14_stub, 14);
irq_stub!(irq15_stub, 15);

/// Dispatch table indexed by IRQ number.
type StubFn = unsafe extern "C" fn();
static IRQ_STUBS: [StubFn; 16] = [
    irq0_stub,  irq1_stub,  irq2_stub,  irq3_stub,
    irq4_stub,  irq5_stub,  irq6_stub,  irq7_stub,
    irq8_stub,  irq9_stub,  irq10_stub, irq11_stub,
    irq12_stub, irq13_stub, irq14_stub, irq15_stub,
];

// ---- Central IRQ dispatcher ------------------------------------------------

/// Called from each ISR stub with the IRQ number.
///
/// Looks up the registered handler in the global HANDLERS table and calls it.
extern "C" fn irq_dispatch(irq: u32) {
    crate::interrupts::dispatch(irq);
}

// ---- PIC remapping ---------------------------------------------------------

/// Remap the 8259 PIC so that IRQ 0–7 → vectors 0x20–0x27 and
/// IRQ 8–15 → vectors 0x28–0x2F (avoiding collision with CPU exceptions).
unsafe fn remap_pic() {
    // ICW1: begin init
    asm!("out dx, al", in("dx") 0x20u16, in("al") 0x11u8, options(nomem, nostack));
    asm!("out dx, al", in("dx") 0xA0u16, in("al") 0x11u8, options(nomem, nostack));
    // ICW2: vector offsets
    asm!("out dx, al", in("dx") 0x21u16, in("al") 0x20u8, options(nomem, nostack)); // master → 0x20
    asm!("out dx, al", in("dx") 0xA1u16, in("al") 0x28u8, options(nomem, nostack)); // slave  → 0x28
    // ICW3: cascade identity
    asm!("out dx, al", in("dx") 0x21u16, in("al") 0x04u8, options(nomem, nostack));
    asm!("out dx, al", in("dx") 0xA1u16, in("al") 0x02u8, options(nomem, nostack));
    // ICW4: 8086 mode
    asm!("out dx, al", in("dx") 0x21u16, in("al") 0x01u8, options(nomem, nostack));
    asm!("out dx, al", in("dx") 0xA1u16, in("al") 0x01u8, options(nomem, nostack));
    // Mask all IRQs initially (handlers will unmask what they need).
    asm!("out dx, al", in("dx") 0x21u16, in("al") 0xFFu8, options(nomem, nostack));
    asm!("out dx, al", in("dx") 0xA1u16, in("al") 0xFFu8, options(nomem, nostack));
}

// ---- Public init -----------------------------------------------------------

/// Initialise the IDT: remap the PIC, populate vector entries, and load IDTR.
pub fn init() {
    unsafe {
        remap_pic();

        // Populate IDT entries for IRQ 0–15 (vectors 0x20–0x2F).
        for (i, &stub) in IRQ_STUBS.iter().enumerate() {
            #[allow(static_mut_refs)]
            IDT[0x20 + i].set(stub as u64);
        }

        let idtr = Idtr {
            limit: (core::mem::size_of::<[IdtEntry; 256]>() - 1) as u16,
            #[allow(static_mut_refs)]
            base:  IDT.as_ptr() as u64,
        };
        asm!("lidt [{0}]", in(reg) &idtr, options(readonly, nostack, preserves_flags));
    }
}
