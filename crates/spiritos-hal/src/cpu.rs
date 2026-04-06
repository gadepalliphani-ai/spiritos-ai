/// CPU-level operations.
pub trait Cpu: Send + Sync {
    fn wait_for_interrupt(&self);
    fn reboot(&self) -> !;
    fn shutdown(&self) -> !;
    fn relax(&self) { core::hint::spin_loop(); }
}
