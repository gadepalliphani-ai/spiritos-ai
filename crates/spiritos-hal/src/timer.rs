/// Monotonic timer + periodic tick source.
pub trait Timer: Send + Sync {
    /// Nanoseconds since boot.
    fn uptime_ns(&self) -> u64;
    /// Set recurring interrupt period in ns. 0 = disable.
    fn set_period_ns(&self, ns: u64);
    fn spin_wait_ns(&self, ns: u64) {
        let end = self.uptime_ns().saturating_add(ns);
        while self.uptime_ns() < end { core::hint::spin_loop(); }
    }
}
