//! PIT (8253) timer — generates IRQ0 at ~100 Hz to drive preemption.

use spiritos_hal::timer::Timer;
use core::sync::atomic::{AtomicU64, Ordering};

pub struct PitTimer;
unsafe impl Send for PitTimer {}
unsafe impl Sync for PitTimer {}

/// Ticks since boot (incremented by the IRQ0 handler).
static TICK_COUNT: AtomicU64 = AtomicU64::new(0);

/// Nanoseconds per tick at 100 Hz.
const NS_PER_TICK: u64 = 10_000_000;

/// PIT divisor for ~100 Hz: 1_193_182 / 100 ≈ 11932.
const PIT_DIVISOR: u16 = 11932;

impl PitTimer {
    /// Program the PIT channel 0 to fire at ~100 Hz (rate generator, mode 2).
    pub fn init_pit() {
        unsafe {
            // Command: channel 0, lobyte/hibyte access, rate generator (mode 2), binary.
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x43u16,
                in("al") 0x34u8,   // 0b00_11_010_0
                options(nomem, nostack)
            );
            // Low byte of divisor.
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x40u16,
                in("al") (PIT_DIVISOR & 0xFF) as u8,
                options(nomem, nostack)
            );
            // High byte of divisor.
            core::arch::asm!(
                "out dx, al",
                in("dx") 0x40u16,
                in("al") (PIT_DIVISOR >> 8) as u8,
                options(nomem, nostack)
            );
        }
    }
}

/// IRQ0 handler — advance the tick counter, acknowledge the PIC, and preempt.
pub fn pit_irq_handler(_irq: u32) {
    let tick = TICK_COUNT.fetch_add(1, Ordering::Relaxed) + 1;

    // Send End-Of-Interrupt to the master PIC.
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") 0x20u16,
            in("al") 0x20u8,
            options(nomem, nostack)
        );
    }

    // Emit a log line every 10 ticks (~100 ms at 100 Hz) so that CI and
    // human observers can confirm the timer is firing at the expected cadence.
    if tick % 10 == 0 {
        if let Some(p) = spiritos_hal::platform::get_opt() {
            p.console.writeln("[rt] timer tick (100 Hz, PIT)");
        }
    }

    // Trigger round-robin preemption.
    spiritos_kernel::scheduler::schedule();
}

impl Timer for PitTimer {
    fn uptime_ns(&self) -> u64 {
        TICK_COUNT.load(Ordering::Relaxed).saturating_mul(NS_PER_TICK)
    }

    fn set_period_ns(&self, _ns: u64) {
        // Re-initialise the PIT; period is fixed at ~100 Hz for now.
        Self::init_pit();
    }
}
