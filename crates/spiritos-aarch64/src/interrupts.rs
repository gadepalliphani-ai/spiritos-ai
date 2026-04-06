use spiritos_hal::interrupts::{Interrupts, IrqHandler};

/// Called by the IRQ vector stub to invoke the registered handler for `irq`.
pub fn dispatch(irq: u32) {
    if (irq as usize) < 256 {
        let handler = unsafe {
            #[allow(static_mut_refs)]
            HANDLERS[irq as usize]
        };
        if let Some(h) = handler {
            h(irq);
        }
    }
}

pub struct GicStub;
unsafe impl Send for GicStub {}
unsafe impl Sync for GicStub {}

static mut HANDLERS: [Option<IrqHandler>; 256] = [None; 256];

impl Interrupts for GicStub {
    fn disable(&self) { unsafe { core::arch::asm!("msr daifset, #0xf", options(nomem, nostack)); } }
    fn enable(&self)  { unsafe { core::arch::asm!("msr daifclr, #0xf", options(nomem, nostack)); } }
    fn register_handler(&self, irq: u32, h: IrqHandler) {
        if (irq as usize) < 256 { unsafe { HANDLERS[irq as usize] = Some(h); } }
    }
    fn unregister_handler(&self, irq: u32) {
        if (irq as usize) < 256 { unsafe { HANDLERS[irq as usize] = None; } }
    }
    fn mask(&self, _irq: u32) {}
    fn unmask(&self, _irq: u32) {}
}
