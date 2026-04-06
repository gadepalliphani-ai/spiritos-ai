use spiritos_hal::timer::Timer;

pub struct PitTimer;
unsafe impl Send for PitTimer {}
unsafe impl Sync for PitTimer {}

static TICK_NS: core::sync::atomic::AtomicU64 =
    core::sync::atomic::AtomicU64::new(0);

impl Timer for PitTimer {
    fn uptime_ns(&self) -> u64 {
        TICK_NS.load(core::sync::atomic::Ordering::Relaxed)
    }
    fn set_period_ns(&self, _ns: u64) {}
}
