# Porting Guide

## What is a "Port"?

SpiritOS supports two levels of porting:

| Level          | What changes                          | Example                              |
|----------------|---------------------------------------|--------------------------------------|
| **New arch**   | ISA, calling convention, HAL impl     | Adding RISC-V support                |
| **New board**  | UART base, memory map, IRQ numbers    | RPi4 vs QEMU virt (both aarch64)    |

A **board port** (BSP) reuses an existing architecture crate and adjusts hardware-specific constants. An **architecture port** creates a new crate from scratch with full HAL implementation.

---

## Adding a New Architecture

### 1. Add the target to `rust-toolchain.toml`

```toml
[toolchain]
channel = "nightly"
targets = [
    "x86_64-unknown-none",
    "aarch64-unknown-none-softfloat",
    "riscv64gc-unknown-none-elf",   # add here
]
```

### 2. Create the crate

```bash
mkdir -p crates/spiritos-riscv64/src
mkdir -p crates/spiritos-riscv64/.cargo
```

### 3. `Cargo.toml`

```toml
[package]
name = "spiritos-riscv64"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "spiritos-riscv64"
path = "src/main.rs"

[dependencies]
spiritos-hal    = { path = "../spiritos-hal" }
spiritos-kernel = { path = "../spiritos-kernel" }

[profile.dev]
panic = "abort"
opt-level = 1
```

### 4. `.cargo/config.toml`

```toml
[build]
target = "riscv64gc-unknown-none-elf"

[target.riscv64gc-unknown-none-elf]
rustflags = [
    "-C", "link-arg=-Tsrc/../linker.ld",
]
```

### 5. Linker Script (`linker.ld`)

```ld
OUTPUT_ARCH(riscv)
ENTRY(_start)
SECTIONS {
    . = 0x80000000;   /* QEMU virt load address */
    .text   : { *(.text.init) *(.text .text.*) }
    .rodata : { *(.rodata .rodata.*) }
    .data   : { *(.data .data.*) }
    .bss    : {
        _bss_start = .;
        *(.bss .bss.*) *(COMMON)
        _bss_end = .;
    }
    /DISCARD/ : { *(.eh_frame) }
}
```

### 6. Entry Point (`src/main.rs`)

```rust
#![no_std]
#![no_main]

use spiritos_hal::platform::{self, Platform};

mod console;
mod timer;
mod interrupts;
mod memory;
mod cpu;

static CONSOLE:    console::SbiConsole       = console::SbiConsole;
static TIMER:      timer::ClintTimer         = timer::ClintTimer;
static INTERRUPTS: interrupts::PlicInterrupts = interrupts::PlicInterrupts;
static MEMORY:     memory::BumpAllocator     = memory::BumpAllocator::new();
static CPU_HAL:    cpu::RiscvCpu             = cpu::RiscvCpu;

/// _start is called by the firmware (OpenSBI) with:
/// a0 = hart ID, a1 = DTB pointer
#[no_mangle]
#[link_section = ".text.init"]
pub extern "C" fn _start() -> ! {
    // Zero BSS
    unsafe {
        extern "C" { static mut _bss_start: u8; static mut _bss_end: u8; }
        let start = &mut _bss_start as *mut u8;
        let end   = &mut _bss_end   as *mut u8;
        let len   = end as usize - start as usize;
        core::ptr::write_bytes(start, 0, len);
    }
    unsafe {
        platform::register(Platform {
            console:    &CONSOLE,
            timer:      &TIMER,
            interrupts: &INTERRUPTS,
            memory:     &MEMORY,
            cpu:        &CPU_HAL,
        });
    }
    spiritos_kernel::kernel_main()
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }
```

---

## Boot Sequence Requirements

Every port must satisfy these boot requirements before calling `kernel_main()`:

### Required
1. **BSS zeroed** — `static mut` variables initialized to zero.
2. **Stack established** — `sp` register points to valid stack memory (at least 8 KiB).
3. **HAL registered** — `platform::register()` called exactly once.
4. **Single core** — On SMP systems, only core 0 (hart 0, CPU 0) calls `kernel_main()`; others must spin or park.

### Optional (but recommended)
5. **MMU disabled or identity-mapped** — Simpler for early boot; the kernel sets up paging later.
6. **Interrupts disabled** — The kernel enables them after initialization.
7. **Firmware handoff** — On RISC-V, OpenSBI handles M-mode; we start in S-mode. On aarch64, firmware may hand off in EL1 or EL2.

---

## Linker Script Requirements

Every architecture linker script must:

1. Define `ENTRY(_start)` — the kernel entry point symbol.
2. Place `.text` at the correct load address for the target (1 MiB for x86_64, 0x80000 for aarch64, 0x80000000 for RISC-V QEMU).
3. Discard `.eh_frame` and `.note*` — unused in `no_std` kernels.
4. Optionally export `_bss_start` / `_bss_end` symbols if BSS zeroing is done in Rust.

---

## HAL Implementation Checklist

Use this checklist when implementing all 5 HAL traits for a new architecture:

### `Console`
- [ ] Identify the debug output device (UART, semihosting, etc.)
- [ ] Implement `write_byte` — blocking write to device
- [ ] Ensure it works before MMU/cache setup (direct MMIO)

### `Timer`
- [ ] Read the architecture's cycle/time counter
- [ ] Return nanoseconds (compute from frequency if needed)
- [ ] Implement `set_period_ns` if you want timer interrupts

### `Interrupts`
- [ ] Identify the interrupt controller (PLIC, GIC, APIC, CLINT...)
- [ ] Implement `disable`/`enable` (global IRQ masking for this CPU)
- [ ] Implement `register_handler` / `unregister_handler`
- [ ] Implement `mask`/`unmask` per-IRQ control

### `Memory`
- [ ] Determine available physical memory range (from firmware, DTB, or hardcoded)
- [ ] Implement `alloc_frames` — return contiguous frame base addresses
- [ ] Implement `frame_size` — 4096 recommended
- [ ] `free_frames` can be a no-op for early bump allocator

### `Cpu`
- [ ] Implement `wait_for_interrupt` (`WFI`/`HLT`/`wfi`)
- [ ] Implement `reboot` (may use firmware call like PSCI, SBI)
- [ ] Implement `shutdown` (may use QEMU debug exit or firmware)

---

## Board Support Package (BSP) Concept

A BSP adapts an existing arch port for a specific board. For example, `spiritos-aarch64` can target multiple boards by changing constants:

```rust
// In your BSP config module:
#[cfg(feature = "rpi4")]
pub const UART_BASE: usize = 0xFE201000;
#[cfg(feature = "qemu-virt")]
pub const UART_BASE: usize = 0x09000000;
#[cfg(feature = "rpi4")]
pub const MEM_BASE: usize = 0x0000_0000;
#[cfg(feature = "qemu-virt")]
pub const MEM_BASE: usize = 0x4000_0000;
```

Then in `Cargo.toml`:
```toml
[features]
default = ["qemu-virt"]
qemu-virt = []
rpi4 = []
```

Build for RPi4:
```bash
cargo build --features rpi4 --target aarch64-unknown-none-softfloat \
  -Z build-std=core,compiler_builtins
```

---

## Example: RISC-V Port Skeleton

### `src/console.rs` — SBI Console

```rust
use spiritos_hal::console::Console;

pub struct SbiConsole;
unsafe impl Send for SbiConsole {}
unsafe impl Sync for SbiConsole {}

impl Console for SbiConsole {
    fn write_byte(&self, byte: u8) {
        // SBI extension: Console Putchar (EID 0x01)
        unsafe {
            core::arch::asm!(
                "ecall",
                in("a7") 0x01usize,  // SBI_EXT_0_1_CONSOLE_PUTCHAR
                in("a0") byte as usize,
            );
        }
    }
}
```

### `src/cpu.rs` — RISC-V CPU

```rust
use spiritos_hal::cpu::Cpu;

pub struct RiscvCpu;
unsafe impl Send for RiscvCpu {}
unsafe impl Sync for RiscvCpu {}

impl Cpu for RiscvCpu {
    fn wait_for_interrupt(&self) {
        unsafe { core::arch::asm!("wfi", options(nomem, nostack)); }
    }
    fn reboot(&self) -> ! {
        // SBI System Reset extension (EID 0x53525354)
        unsafe {
            core::arch::asm!(
                "ecall",
                in("a7") 0x53525354usize, // SBI_EXT_SRST
                in("a6") 0usize,           // FID: sbi_system_reset
                in("a0") 0usize,           // reset_type: SHUTDOWN
                in("a1") 0usize,           // reset_reason: NO_REASON
            );
        }
        loop {}
    }
    fn shutdown(&self) -> ! { self.reboot() }
}
```

---

## Adding the New Crate to the Workspace

Edit the root `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = [
    "crates/spiritos-hal",
    "crates/spiritos-abi",
    "crates/spiritos-kernel",
    "crates/spiritos-x86_64",
    "crates/spiritos-aarch64",
    "crates/spiritos-riscv64",   # add here
]
```

And add the CI job in `.github/workflows/ci.yml` for the new target.

---

## Common Pitfalls

| Issue | Fix |
|-------|-----|
| BSS not zeroed → random values in statics | Add BSS zeroing in `_start` before platform::register |
| Stack overflow in early boot | Allocate stack in linker script; set `sp` in first asm instruction |
| `hlt`/`wfi` not working without interrupts enabled | Ensure `interrupts::enable()` is called before the idle loop |
| Linking errors: undefined `memcpy`/`memset` | Add `-Z build-std-features=compiler-builtins-mem` |
| Load address wrong → UART writes to wrong address | Double-check linker script load address vs QEMU `-kernel` flag |
