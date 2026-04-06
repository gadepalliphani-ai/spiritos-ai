# SpiritOS.ai

[![CI](https://github.com/gadepalliphani-ai/spiritos-ai/actions/workflows/ci.yml/badge.svg)](https://github.com/gadepalliphani-ai/spiritos-ai/actions/workflows/ci.yml)

**SpiritOS.ai** is a real-time operating system kernel written in Rust, targeting x86_64 and aarch64 bare-metal platforms (QEMU and physical hardware including Raspberry Pi 4).

---

## Features

- ✅ **Pure Rust, `no_std`** — no libc, no allocator required at kernel level
- ✅ **Hardware Abstraction Layer** — 5 clean traits decouple kernel from hardware
- ✅ **Dual-architecture** — x86_64 (Intel/AMD) and aarch64 (ARM, RPi4) from day one
- ✅ **Driver registry** — register drivers before boot; automatic `init()` sequencing
- ✅ **Syscall ABI** — defined calling conventions for x86_64 and aarch64
- ✅ **IPC primitives** — message header types for inter-task communication
- ✅ **CI/CD** — GitHub Actions builds + QEMU boot tests on every push
- 🔜 **Virtual memory** — page tables and kernel heap (v0.3)
- 🔜 **Real scheduler** — round-robin task switching with context save/restore (v0.2)
- 🔜 **RISC-V port** — riscv64gc-unknown-none-elf (v0.6)

---

## Supported Targets

| Architecture | Target Triple                      | Board / Emulator          | Status       |
|--------------|-------------------------------------|---------------------------|--------------|
| x86_64       | `x86_64-unknown-none`               | QEMU, bare metal (PC)     | ✅ Builds     |
| aarch64      | `aarch64-unknown-none-softfloat`    | QEMU virt, Raspberry Pi 4 | ✅ Builds     |
| RISC-V 64    | `riscv64gc-unknown-none-elf`        | QEMU virt                 | 🔜 Planned   |

---

## Quick Start

### 1. Install Rust (nightly)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly
source "$HOME/.cargo/env"
rustup target add x86_64-unknown-none aarch64-unknown-none-softfloat
rustup component add rust-src llvm-tools-preview
```

### 2. Install QEMU

```bash
# macOS
brew install qemu

# Ubuntu/Debian
sudo apt-get install -y qemu-system-x86 qemu-system-arm
```

### 3. Clone and Build

```bash
git clone https://github.com/gadepalliphani-ai/spiritos-ai.git
cd spiritos-ai
```

**Build x86_64:**
```bash
cd crates/spiritos-x86_64
cargo build \
  -Z build-std=core,compiler_builtins \
  -Z build-std-features=compiler-builtins-mem \
  --target x86_64-unknown-none
```

**Build aarch64:**
```bash
cd crates/spiritos-aarch64
cargo build \
  -Z build-std=core,compiler_builtins \
  -Z build-std-features=compiler-builtins-mem \
  --target aarch64-unknown-none-softfloat
```

### 4. Run in QEMU

**x86_64:**
```bash
qemu-system-x86_64 \
  -kernel target/x86_64-unknown-none/debug/spiritos-x86_64 \
  -serial stdio -display none -no-reboot -m 128M
```

**aarch64:**
```bash
qemu-system-aarch64 \
  -M virt -cpu cortex-a53 -m 128M \
  -kernel target/aarch64-unknown-none-softfloat/debug/spiritos-aarch64 \
  -serial stdio -display none -no-reboot
```

Expected output:
```
[spiritos] Kernel starting...
[spiritos] Memory OK
[spiritos] Scheduler OK
[spiritos] Drivers OK
[spiritos] SpiritOS.ai is alive!
```

---

## Project Structure

```
spiritos-ai/
├── Cargo.toml              # Workspace manifest
├── rust-toolchain.toml     # Nightly toolchain pin
├── ARCHITECTURE.md         # System architecture docs
├── docs/
│   ├── HAL.md              # Hardware Abstraction Layer guide
│   ├── ABI.md              # Syscall ABI reference
│   ├── DRIVERS.md          # Driver development guide
│   └── PORTING.md          # Porting to new hardware
└── crates/
    ├── spiritos-abi/       # Syscall numbers, error codes, IPC types
    ├── spiritos-hal/       # HAL traits + Platform registry
    ├── spiritos-kernel/    # Kernel core (mm, scheduler, drivers, syscalls)
    ├── spiritos-x86_64/    # x86_64 boot + HAL implementation
    └── spiritos-aarch64/   # aarch64 boot + HAL implementation
```

---

## Architecture Overview

SpiritOS uses a layered design:

```
┌─────────────────────────────────────────┐
│        Architecture Binary              │
│   (spiritos-x86_64 / spiritos-aarch64)  │
│   - _start entry point                  │
│   - HAL trait implementations           │
│   - platform::register()                │
├─────────────────────────────────────────┤
│           spiritos-kernel               │
│   - kernel_main()                       │
│   - Memory manager (mm)                 │
│   - Scheduler                           │
│   - Driver registry                     │
│   - Syscall dispatch                    │
├─────────────────────────────────────────┤
│           spiritos-hal                  │
│   - Console / Timer / Interrupts        │
│   - Memory / Cpu traits                 │
│   - Platform singleton                  │
├─────────────────────────────────────────┤
│           spiritos-abi                  │
│   - Syscall numbers                     │
│   - Error codes                         │
│   - IPC message types                   │
└─────────────────────────────────────────┘
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for deep-dive documentation.

---

## Documentation

| Document | Description |
|----------|-------------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | System architecture, crate graph, boot sequence, roadmap |
| [docs/HAL.md](docs/HAL.md) | Hardware Abstraction Layer — traits, contracts, examples |
| [docs/ABI.md](docs/ABI.md) | Syscall ABI — calling conventions, syscall table, IPC |
| [docs/DRIVERS.md](docs/DRIVERS.md) | Driver development guide with examples |
| [docs/PORTING.md](docs/PORTING.md) | Porting to new architectures and boards |

---

## Contributing

Contributions welcome! Areas that need work:

- **Scheduler** — real round-robin with context switching
- **Virtual memory** — x86_64 page tables, aarch64 MMU
- **RISC-V port** — `riscv64gc-unknown-none-elf`
- **Drivers** — keyboard, block device, network stub
- **Testing** — mock HAL, integration test harness

Please open an issue before starting large features. Keep all code `no_std`.

---

## License

MIT License — see [LICENSE](LICENSE) for details.

---

*Built with ❤️ in Rust*
