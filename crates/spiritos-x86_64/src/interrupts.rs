use spiritos_hal::interrupts::{Interrupts, IrqHandler};

/// Called by IDT stubs (via `idt::irq_dispatch`) to invoke the registered handler.
pub fn dispatch(irq: u32) {
    if (irq as usize) < 16 {
        let handler = unsafe {
            #[allow(static_mut_refs)]
            HANDLERS[irq as usize]
        };
        if let Some(h) = handler {
            h(irq);
        }
    }
}

pub struct PicInterrupts;
unsafe impl Send for PicInterrupts {}
unsafe impl Sync for PicInterrupts {}

static mut HANDLERS: [Option<IrqHandler>; 16] = [None; 16];

impl Interrupts for PicInterrupts {
    fn disable(&self) { unsafe { core::arch::asm!("cli", options(nomem, nostack)); } }
    fn enable(&self)  { unsafe { core::arch::asm!("sti", options(nomem, nostack)); } }
    fn register_handler(&self, irq: u32, h: IrqHandler) {
        if (irq as usize) < 16 { unsafe { HANDLERS[irq as usize] = Some(h); } }
    }
    fn unregister_handler(&self, irq: u32) {
        if (irq as usize) < 16 { unsafe { HANDLERS[irq as usize] = None; } }
    }
    fn mask(&self, irq: u32) {
        if irq < 8 {
            unsafe {
                let cur: u8;
                core::arch::asm!("in al, dx", out("al") cur, in("dx") 0x21u16, options(nomem, nostack));
                core::arch::asm!("out dx, al", in("dx") 0x21u16, in("al") cur | (1u8 << irq), options(nomem, nostack));
            }
        }
    }
    fn unmask(&self, irq: u32) {
        if irq < 8 {
            unsafe {
                let cur: u8;
                core::arch::asm!("in al, dx", out("al") cur, in("dx") 0x21u16, options(nomem, nostack));
                core::arch::asm!("out dx, al", in("dx") 0x21u16, in("al") cur & !(1u8 << irq), options(nomem, nostack));
            }
        }
    }
}
