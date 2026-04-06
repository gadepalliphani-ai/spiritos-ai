use spiritos_hal::console::Console;

// QEMU virt UART base; RPi4 use 0xFE201000
const UART0_DR: *mut u32 = 0x0900_0000 as *mut u32;

pub struct Pl011Uart;
unsafe impl Send for Pl011Uart {}
unsafe impl Sync for Pl011Uart {}

impl Console for Pl011Uart {
    fn write_byte(&self, byte: u8) {
        unsafe { UART0_DR.write_volatile(byte as u32); }
    }
}
