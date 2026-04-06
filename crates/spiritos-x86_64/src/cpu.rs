use spiritos_hal::cpu::Cpu;

pub struct X86Cpu;
unsafe impl Send for X86Cpu {}
unsafe impl Sync for X86Cpu {}

impl Cpu for X86Cpu {
    fn wait_for_interrupt(&self) {
        unsafe { core::arch::asm!("hlt", options(nomem, nostack)); }
    }
    fn reboot(&self) -> ! {
        unsafe { core::arch::asm!("out dx, al", in("dx") 0x64u16, in("al") 0xFEu8, options(nomem, nostack)); }
        loop { core::hint::spin_loop(); }
    }
    fn shutdown(&self) -> ! {
        // QEMU isa-debug-exit device
        unsafe { core::arch::asm!("out dx, al", in("dx") 0x501u16, in("al") 0x31u8, options(nomem, nostack)); }
        loop { core::hint::spin_loop(); }
    }
}
