# Hardware Abstraction Layer (HAL) Guide

## Purpose

The SpiritOS HAL (`spiritos-hal`) provides a set of Rust traits that decouple the kernel from specific hardware. Every piece of hardware interaction flows through these traits; the kernel never touches hardware registers directly.

This means:
- The kernel compiles and runs identically on x86_64, aarch64, RISC-V, or any future architecture.
- Porting SpiritOS to new hardware means implementing ~5 traits — nothing more.
- Testing kernel logic can be done with mock HAL implementations on hosted targets.

---

## Trait Overview

| Trait        | File               | Purpose                                      |
|--------------|--------------------|----------------------------------------------|
| `Console`    | `src/console.rs`   | Debug output (UART, VGA, semihosting)        |
| `Timer`      | `src/timer.rs`     | Monotonic clock and periodic tick source     |
| `Interrupts` | `src/interrupts.rs`| IRQ enable/disable and handler registration  |
| `Memory`     | `src/memory.rs`    | Physical frame allocator                     |
| `Cpu`        | `src/cpu.rs`       | CPU-level: halt, reboot, shutdown, SMP hints |

All traits require `Send + Sync` because the kernel may access the platform from multiple cores (SMP future support).

---

## Trait Reference

### `Console`

```rust
pub trait Console: Send + Sync {
    fn write_byte(&self, byte: u8);
    fn write_str(&self, s: &str);   // default: loops over bytes
    fn writeln(&self, s: &str);     // default: write_str + newline
}
```

**Contract:**
- `write_byte` must be safe to call in interrupt context (no locks that can deadlock).
- Implementations may spin-wait if the hardware FIFO is full; this is acceptable in early boot.
- Must not allocate memory.

**Example — UART implementation:**

```rust
pub struct SerialConsole;
unsafe impl Send for SerialConsole {}
unsafe impl Sync for SerialConsole {}

impl Console for SerialConsole {
    fn write_byte(&self, byte: u8) {
        // Wait until TX FIFO not full (bit 5 of LSR)
        unsafe {
            loop {
                let lsr: u8;
                core::arch::asm!("in al, dx", out("al") lsr, in("dx") 0x3FDu16,
                    options(nomem, nostack));
                if lsr & 0x20 != 0 { break; }
            }
            core::arch::asm!("out dx, al", in("dx") 0x3F8u16, in("al") byte,
                options(nomem, nostack));
        }
    }
}
```

---

### `Timer`

```rust
pub trait Timer: Send + Sync {
    fn uptime_ns(&self) -> u64;          // nanoseconds since boot
    fn set_period_ns(&self, ns: u64);    // periodic tick interval; 0 = disable
    fn spin_wait_ns(&self, ns: u64);     // default: busy-wait using uptime_ns
}
```

**Contract:**
- `uptime_ns` must be monotonically non-decreasing.
- `set_period_ns` programs the hardware timer to fire an IRQ at the given interval.
- Both methods must be safe to call from interrupt handlers.

**Example — ARM Generic Timer:**

```rust
pub struct GenericTimer;
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
    fn set_period_ns(&self, _ns: u64) { /* program CNTP_TVAL_EL0 */ }
}
```

---

### `Interrupts`

```rust
pub type IrqHandler = fn(irq: u32);

pub trait Interrupts: Send + Sync {
    fn disable(&self);
    fn enable(&self);
    fn register_handler(&self, irq: u32, handler: IrqHandler);
    fn unregister_handler(&self, irq: u32);
    fn mask(&self, irq: u32);
    fn unmask(&self, irq: u32);
}
```

**Contract:**
- `disable`/`enable` affect the current CPU only (local IRQ masking).
- `register_handler` stores the function pointer in a static table; called from the interrupt vector.
- `mask`/`unmask` control individual IRQ lines at the interrupt controller.
- Handler functions must not sleep or allocate.

---

### `Memory`

```rust
pub trait Memory: Send + Sync {
    fn alloc_frames(&self, count: usize) -> Option<usize>;  // returns physical base address
    fn free_frames(&self, base: usize, count: usize);
    fn frame_size(&self) -> usize;    // typically 4096
    fn total_bytes(&self) -> usize;
    fn free_bytes(&self) -> usize;
}
```

**Contract:**
- Returned addresses are physical (not virtual; paging is the kernel's job).
- `alloc_frames` returns `None` on OOM, never panics.
- `free_frames` with an invalid address is implementation-defined (may silently ignore).
- `frame_size` must return the same value across the lifetime of the system (power of 2, ≥ 4096).

**Memory Safety:** The kernel must ensure physical frames are not aliased at the logical level.

---

### `Cpu`

```rust
pub trait Cpu: Send + Sync {
    fn wait_for_interrupt(&self);  // e.g. HLT, WFI
    fn reboot(&self) -> !;
    fn shutdown(&self) -> !;
    fn relax(&self);               // default: spin_loop hint
}
```

**Contract:**
- `reboot` and `shutdown` must never return.
- `wait_for_interrupt` is used in idle loops; must wake on any pending IRQ.

---

## Platform Registration

The `Platform` struct bundles all trait objects and is registered once at boot:

```rust
pub struct Platform {
    pub console:    &'static dyn Console,
    pub timer:      &'static dyn Timer,
    pub interrupts: &'static dyn Interrupts,
    pub memory:     &'static dyn Memory,
    pub cpu:        &'static dyn Cpu,
}
```

### Lifecycle

1. **Architecture boot code** (e.g., `_start` in `spiritos-x86_64`) initializes static HAL implementations.
2. It calls `unsafe { platform::register(Platform { ... }) }` — exactly once.
3. It then calls `spiritos_kernel::kernel_main()`.
4. The kernel calls `platform::get()` to access the platform at any time.
5. `platform::get_opt()` is available for contexts where the platform may not yet be registered (e.g., panic handler).

### Example Boot Sequence

```rust
#[no_mangle]
pub extern "C" fn _start() -> ! {
    // 1. Register platform (HAL impls are static)
    unsafe {
        platform::register(Platform {
            console:    &CONSOLE,
            timer:      &TIMER,
            interrupts: &INTERRUPTS,
            memory:     &MEMORY,
            cpu:        &CPU_HAL,
        });
    }
    // 2. Hand off to kernel
    spiritos_kernel::kernel_main()
}
```

---

## Implementing HAL Traits for New Hardware

### Checklist

- [ ] Implement `Console` — pick your UART or semihosting output
- [ ] Implement `Timer` — read the architecture's monotonic counter
- [ ] Implement `Interrupts` — wrap your interrupt controller (GIC, APIC, PLIC...)
- [ ] Implement `Memory` — implement a bump or bitmap frame allocator
- [ ] Implement `Cpu` — `WFI`/`HLT` for idle, board-specific reset

All types implementing these traits must be `Send + Sync`. If your hardware registers are memory-mapped and non-re-entrant, protect with a spinlock or ensure single-threaded access during boot.

---

## Testing Strategies

### Mock HAL (Hosted Target)

Create a `spiritos-hal-mock` crate that implements all traits with in-memory buffers:

```rust
pub struct MockConsole(RefCell<Vec<u8>>);
impl Console for MockConsole {
    fn write_byte(&self, byte: u8) { self.0.borrow_mut().push(byte); }
}
```

Then run `spiritos-kernel` integration tests on your development machine (x86_64-unknown-linux-gnu) with:

```bash
cargo test -p spiritos-kernel --features mock-hal
```

### QEMU

Both architectures support QEMU boot tests as part of CI:
- x86_64: `qemu-system-x86_64 -kernel <elf> -serial stdio`
- aarch64: `qemu-system-aarch64 -M virt -cpu cortex-a53 -kernel <elf> -serial stdio`

QEMU serial output maps to stdout, making boot log verification easy in CI.

---

## Memory Safety Contracts

1. `register()` must be called exactly once. Calling it twice is UB.
2. All `&'static dyn Trait` references passed to `Platform` must remain valid for the lifetime of the system.
3. Trait method implementations must not call `platform::get()` recursively (no re-entrancy via HAL).
4. Frame allocator must never return overlapping ranges.
