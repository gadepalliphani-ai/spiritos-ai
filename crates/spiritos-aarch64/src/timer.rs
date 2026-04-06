//! ARM Generic Timer — drives an IRQ at ~100 Hz for preemptive scheduling.
//!
//! Uses the EL1 physical timer (`CNTP_TVAL_EL0` / `CNTP_CTL_EL0`).

use spiritos_hal::timer::Timer;
use core::sync::atomic::{AtomicU64, Ordering};

pub struct GenericTimer;
unsafe impl Send for GenericTimer {}
unsafe impl Sync for GenericTimer {}

/// Counter ticks since boot (incremented by the timer IRQ handler).
static TICK_COUNT: AtomicU64 = AtomicU64::new(0);

impl GenericTimer {
    /// Read the timer frequency from `CNTFRQ_EL0`.
    pub fn freq() -> u64 {
        let f: u64;
        unsafe {
            core::arch::asm!(
                "mrs {f}, cntfrq_el0",
                f = out(reg) f,
                options(nomem, nostack)
            );
        }
        // Guard against uninitialised hardware (e.g., early QEMU).
        if f == 0 { 1_000_000 } else { f }
    }

    /// Programme `CNTP_TVAL_EL0` to fire in `ticks` counter cycles and enable
    /// the EL1 physical timer.
    pub fn set_tval(ticks: u64) {
        unsafe {
            core::arch::asm!(
                "msr cntp_tval_el0, {v}",
                v = in(reg) ticks,
                options(nomem, nostack)
            );
            // Bit 0 = ENABLE, bit 1 = IMASK (0 = not masked) → value 1.
            core::arch::asm!(
                "msr cntp_ctl_el0, {v}",
                v = in(reg) 1u64,
                options(nomem, nostack)
            );
        }
    }
}

/// Timer IRQ handler (~100 Hz): rearm, tick, and trigger preemption.
pub fn timer_irq_handler(_irq: u32) {
    TICK_COUNT.fetch_add(1, Ordering::Relaxed);

    // Rearm the timer for the next tick.
    let ticks = GenericTimer::freq() / 100;
    GenericTimer::set_tval(ticks);

    // Trigger round-robin preemption.
    spiritos_kernel::scheduler::schedule();
}

impl Timer for GenericTimer {
    fn uptime_ns(&self) -> u64 {
        let count: u64;
        let freq = Self::freq();
        unsafe {
            core::arch::asm!(
                "mrs {c}, cntpct_el0",
                c = out(reg) count,
                options(nomem, nostack)
            );
        }
        count.saturating_mul(1_000_000_000) / freq
    }

    fn set_period_ns(&self, _ns: u64) {
        // Fixed at ~100 Hz for now.
        let ticks = Self::freq() / 100;
        Self::set_tval(ticks);
    }
}
