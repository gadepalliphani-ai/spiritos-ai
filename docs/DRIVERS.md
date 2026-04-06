# Driver Development Guide

## Overview

SpiritOS uses a simple driver registry in `spiritos-kernel`. Drivers implement the `Driver` trait and register themselves before `kernel_main()` is called. The kernel then initializes all registered drivers in order during boot.

Drivers interact with hardware through the HAL or directly (for arch-specific drivers). They may expose typed APIs via static handles for use by other kernel subsystems.

---

## The `Driver` Trait

```rust
pub trait Driver: Send + Sync {
    /// Short human-readable name (e.g., "uart0", "gpio-led").
    fn name(&self) -> &'static str;

    /// Called once during kernel boot, in registration order.
    fn init(&self);

    /// Optional: called on graceful shutdown.
    fn shutdown(&self) {}
}
```

All drivers must be `Send + Sync` because they live as `&'static dyn Driver` and may be accessed from multiple contexts.

---

## Driver Lifecycle

```
Architecture boot
    │
    ├─ platform::register(...)      ← HAL ready
    │
    ├─ driver::register(&MY_DRIVER) ← register before kernel_main
    │
    └─ kernel_main()
           │
           ├─ mm::init()
           ├─ scheduler::init()
           ├─ driver::init_all()    ← calls init() on every registered driver
           └─ interrupts::enable()
```

1. **Registration** — The architecture crate registers drivers before calling `kernel_main()`.
2. **Init** — `init_all()` calls each driver's `init()` in registration order.
3. **Runtime** — Drivers respond to interrupts or are called by kernel subsystems.
4. **Shutdown** — `shutdown()` is called on graceful halt/reboot (future).

---

## Creating a Driver: Step by Step

### 1. Create the driver struct

```rust
pub struct MyDriver {
    base_addr: usize,
}
unsafe impl Send for MyDriver {}
unsafe impl Sync for MyDriver {}
```

### 2. Implement `Driver`

```rust
use spiritos_kernel::driver::Driver;

impl Driver for MyDriver {
    fn name(&self) -> &'static str { "my-driver" }

    fn init(&self) {
        // Initialize hardware registers
        let reg = self.base_addr as *mut u32;
        unsafe { reg.write_volatile(0x1); } // enable
    }

    fn shutdown(&self) {
        let reg = self.base_addr as *mut u32;
        unsafe { reg.write_volatile(0x0); } // disable
    }
}
```

### 3. Create a static instance

```rust
static MY_DRIVER: MyDriver = MyDriver { base_addr: 0x1000_0000 };
```

### 4. Register before `kernel_main()`

In your architecture crate's `_start`:

```rust
#[no_mangle]
pub extern "C" fn _start() -> ! {
    unsafe {
        platform::register(Platform { /* ... */ });
        spiritos_kernel::driver::register(&MY_DRIVER);
    }
    spiritos_kernel::kernel_main()
}
```

---

## Exposing Typed APIs via Static Handles

Drivers often need to expose more than just `init/shutdown`. Use a separate typed static that other code can access:

```rust
pub struct UartDriver {
    base: usize,
}

impl UartDriver {
    pub const fn new(base: usize) -> Self { Self { base } }

    /// Write a byte directly (bypass kernel console).
    pub fn send_byte(&self, byte: u8) {
        unsafe {
            let dr = self.base as *mut u32;
            dr.write_volatile(byte as u32);
        }
    }
}

impl Driver for UartDriver {
    fn name(&self) -> &'static str { "uart0" }
    fn init(&self) { /* configure baud rate, etc. */ }
}

// Exposed typed handle
pub static UART0: UartDriver = UartDriver::new(0x0900_0000);
```

Other modules can import and use `UART0.send_byte(b'!')` directly.

---

## Interacting with the HAL from a Driver

Drivers have full access to the platform via `spiritos_hal::platform::get()`:

```rust
fn init(&self) {
    let p = spiritos_hal::platform::get();
    p.console.writeln("[my-driver] Initializing...");

    // Register an IRQ handler
    p.interrupts.register_handler(self.irq, my_irq_handler);
    p.interrupts.unmask(self.irq);

    p.console.writeln("[my-driver] Ready");
}

fn my_irq_handler(irq: u32) {
    // Handle interrupt — read from hardware, update state
}
```

---

## Example: GPIO LED Blinker Driver

```rust
use spiritos_kernel::driver::Driver;
use spiritos_hal::platform;

const GPIO_BASE: usize = 0x0200_0000; // board-specific
const GPIO_DIR:  usize = GPIO_BASE + 0x00; // direction register
const GPIO_OUT:  usize = GPIO_BASE + 0x04; // output register

pub struct LedDriver {
    pin: u32,
}
unsafe impl Send for LedDriver {}
unsafe impl Sync for LedDriver {}

impl LedDriver {
    pub const fn new(pin: u32) -> Self { Self { pin } }

    pub fn on(&self) {
        unsafe {
            let out = GPIO_OUT as *mut u32;
            let val = out.read_volatile();
            out.write_volatile(val | (1 << self.pin));
        }
    }

    pub fn off(&self) {
        unsafe {
            let out = GPIO_OUT as *mut u32;
            let val = out.read_volatile();
            out.write_volatile(val & !(1 << self.pin));
        }
    }
}

impl Driver for LedDriver {
    fn name(&self) -> &'static str { "gpio-led" }

    fn init(&self) {
        // Set pin as output
        unsafe {
            let dir = GPIO_DIR as *mut u32;
            let val = dir.read_volatile();
            dir.write_volatile(val | (1 << self.pin));
        }
        let p = platform::get();
        p.console.writeln("[gpio-led] Initialized");
        // Blink once to signal life
        self.on();
        p.timer.spin_wait_ns(100_000_000); // 100ms
        self.off();
    }
}

pub static STATUS_LED: LedDriver = LedDriver::new(25); // GPIO 25 (RPi style)
```

---

## Example: UART Driver (Full)

```rust
use spiritos_kernel::driver::Driver;
use spiritos_hal::{console::Console, platform};
use core::sync::atomic::{AtomicBool, Ordering};

pub struct Pl011 {
    base: usize,
}
unsafe impl Send for Pl011 {}
unsafe impl Sync for Pl011 {}

// Pl011 register offsets
const DR:   usize = 0x000; // Data Register
const FR:   usize = 0x018; // Flag Register
const IBRD: usize = 0x024; // Integer Baud Rate
const FBRD: usize = 0x028; // Fractional Baud Rate
const LCR:  usize = 0x02C; // Line Control
const CR:   usize = 0x030; // Control Register

impl Pl011 {
    pub const fn new(base: usize) -> Self { Self { base } }

    fn reg(&self, offset: usize) -> *mut u32 {
        (self.base + offset) as *mut u32
    }

    fn tx_full(&self) -> bool {
        unsafe { self.reg(FR).read_volatile() & (1 << 5) != 0 }
    }
}

impl Console for Pl011 {
    fn write_byte(&self, byte: u8) {
        while self.tx_full() { core::hint::spin_loop(); }
        unsafe { self.reg(DR).write_volatile(byte as u32); }
    }
}

impl Driver for Pl011 {
    fn name(&self) -> &'static str { "pl011-uart" }

    fn init(&self) {
        unsafe {
            // Disable UART
            self.reg(CR).write_volatile(0);
            // Set 115200 baud (assuming 48MHz UART clock)
            // IBRD = 26, FBRD = 3
            self.reg(IBRD).write_volatile(26);
            self.reg(FBRD).write_volatile(3);
            // 8N1, FIFO enabled
            self.reg(LCR).write_volatile((0b11 << 5) | (1 << 4));
            // Enable UART, TX, RX
            self.reg(CR).write_volatile((1 << 0) | (1 << 8) | (1 << 9));
        }
    }
}

pub static UART0: Pl011 = Pl011::new(0x0900_0000);
```

---

## Testing Drivers

### Unit Tests (Hosted)

Use a mock HAL and test driver logic without hardware:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct MockConsole(core::cell::RefCell<Vec<u8>>);
    // impl Console for MockConsole { ... }

    #[test]
    fn led_init_sets_direction_bit() {
        let led = LedDriver::new(5);
        // Use mock GPIO memory buffer
        // led.init() with mock platform
        // assert direction register bit 5 is set
    }
}
```

### QEMU Integration

Register your driver, boot in QEMU, and observe serial output:

```bash
qemu-system-aarch64 -M virt -cpu cortex-a53 -m 128M \
  -kernel target/aarch64-unknown-none-softfloat/debug/spiritos-aarch64 \
  -serial stdio -display none
```

Expected output:
```
[spiritos] Kernel starting...
[spiritos] Memory OK
[spiritos] Scheduler OK
[gpio-led] Initialized
[spiritos] Drivers OK
[spiritos] SpiritOS.ai is alive!
```

---

## Driver Registry Limits

Currently, the driver registry supports up to 64 drivers (`MAX_DRIVERS = 64`). This can be increased in `spiritos-kernel/src/driver.rs` if needed.

Registration order matters: drivers are initialized in the order they are registered. Ensure dependencies are registered first (e.g., UART before anything that logs).
