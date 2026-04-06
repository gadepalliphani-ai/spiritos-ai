use spiritos_hal::timer::Timer;

pub struct GenericTimer;
unsafe impl Send for GenericTimer {}
unsafe impl Sync for GenericTimer {}

impl Timer for GenericTimer {
    fn uptime_ns(&self) -> u64 {
        let count: u64;
        let freq: u64;
        unsafe {
            core::arch::asm!("mrs {}, cntpct_el0", out(reg) count, options(nomem, nostack));
            core::arch::asm!("mrs {}, cntfrq_el0", out(reg) freq, options(nomem, nostack));
        }
        if freq > 0 { count.saturating_mul(1_000_000_000) / freq } else { 0 }
    }
    fn set_period_ns(&self, _ns: u64) {}
}
