use spiritos_hal::console::Console;

/// COM1 serial port console.
pub struct SerialConsole;
unsafe impl Send for SerialConsole {}
unsafe impl Sync for SerialConsole {}

impl Console for SerialConsole {
    fn write_byte(&self, byte: u8) {
        unsafe {
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x3F8u16,
                in("al") byte,
                options(nomem, nostack)
            );
        }
    }
}
