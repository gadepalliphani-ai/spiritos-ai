pub type IrqHandler = fn(irq: u32);

/// Interrupt controller abstraction.
pub trait Interrupts: Send + Sync {
    fn disable(&self);
    fn enable(&self);
    fn register_handler(&self, irq: u32, handler: IrqHandler);
    fn unregister_handler(&self, irq: u32);
    fn mask(&self, irq: u32);
    fn unmask(&self, irq: u32);
}
