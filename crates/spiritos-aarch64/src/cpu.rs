use spiritos_hal::cpu::Cpu;

pub struct Aarch64Cpu;
unsafe impl Send for Aarch64Cpu {}
unsafe impl Sync for Aarch64Cpu {}

impl Cpu for Aarch64Cpu {
    fn wait_for_interrupt(&self) {
        unsafe { core::arch::asm!("wfi", options(nomem, nostack)); }
    }
    fn reboot(&self) -> ! { loop { core::hint::spin_loop(); } }
    fn shutdown(&self) -> ! {
        // QEMU exit via HLT (aarch64 uses HLT #0 for PSCI-like exits in some setups)
        // Fall back to infinite loop for safety
        loop { core::hint::spin_loop(); }
    }
}
