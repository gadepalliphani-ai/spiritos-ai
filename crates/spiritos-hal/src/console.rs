/// Debug console (UART, VGA, semihosting, etc.).
pub trait Console: Send + Sync {
    fn write_byte(&self, byte: u8);
    fn write_str(&self, s: &str) {
        for b in s.bytes() { self.write_byte(b); }
    }
    fn writeln(&self, s: &str) {
        self.write_str(s);
        self.write_byte(b'\n');
    }
}
