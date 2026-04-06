# SpiritOS Architecture

## Project Overview

SpiritOS.ai is a real-time operating system kernel written in Rust, designed for embedded and bare-metal targets. The design prioritizes:

- **Portability** — HAL traits decouple kernel logic from hardware
- **Safety** — `no_std` Rust with explicit `unsafe` boundaries
- **Simplicity** — minimal kernel, extension via driver registry
- **Real-time** — deterministic interrupt handling and memory allocation

---

## Crate Dependency Graph

```
                    ┌─────────────────────┐
                    │   spiritos-x86_64   │
                    │  (binary, no_std)   │
                    └──────────┬──────────┘
                               │ depends on
              ┌────────────────┼──────────────────┐
              │                │                  │
              ▼                ▼                  │
  ┌──────────────────┐  ┌────────────────┐        │
  │  spiritos-kernel │  │  spiritos-hal  │        │
  │  (lib, no_std)   │  │  (lib, no_std) │        │
  └──────┬───────────┘  └───────┬────────┘        │
         │ depends on           │ depends on       │
         │              ┌───────┘                  │
         ▼              ▼                          │
  ┌─────────────────────────────┐                  │
  │       spiritos-abi          │                  │
  │      (lib, no_std)          │◄─────────────────┘
  └─────────────────────────────┘

                    ┌─────────────────────┐
                    │  spiritos-aarch64   │
                    │  (binary, no_std)   │
                    └──────────┬──────────┘
                               │
              ┌────────────────┼──────────────────┐
              │                │                  │
              ▼                ▼                  │
  ┌──────────────────┐  ┌────────────────┐  ┌────┴───────────────┐
  │  spiritos-kernel │  │  spiritos-hal  │  │   spiritos-abi     │
  └──────────────────┘  └────────────────┘  └────────────────────┘
```

### Crate Roles

| Crate              | Type    | Role                                                   |
|--------------------|---------|--------------------------------------------------------|
| `spiritos-abi`     | library | Syscall numbers, error codes, IPC types — shared ABI  |
| `spiritos-hal`     | library | Hardware abstraction traits + Platform registry        |
| `spiritos-kernel`  | library | Core kernel: mm, scheduler, driver registry, syscalls  |
| `spiritos-x86_64`  | binary  | x86_64 boot + HAL implementations (serial, PIT, PIC)  |
| `spiritos-aarch64` | binary  | aarch64 boot + HAL implementations (PL011, ARM timer) |

---

## Boot Sequence

### x86_64

```
BIOS/Bootloader
    │
    └─ Loads ELF at 1 MiB, jumps to _start
           │
           ├─ Initialize static HAL objects (SerialConsole, PitTimer, ...)
           ├─ platform::register(Platform { ... })    ← HAL ready
           │
           └─ kernel_main()
                  │
                  ├─ console.writeln("[spiritos] Kernel starting...")
                  ├─ mm::init(platform.memory)         ← frame allocator ready
                  ├─ scheduler::init()                 ← task 0 initialized
                  ├─ driver::init_all()                ← all registered drivers init
                  ├─ interrupts::enable()              ← IRQs live
                  ├─ console.writeln("SpiritOS.ai is alive!")
                  └─ scheduler::run()                  ← idle loop (spin_loop)
```

### aarch64

```
Firmware (U-Boot / QEMU stub)
    │
    └─ Loads ELF at 0x80000, jumps to _start (EL1)
           │
           ├─ Initialize static HAL objects (Pl011Uart, GenericTimer, GicStub, ...)
           ├─ platform::register(Platform { ... })
           │
           └─ kernel_main()
                  │  (same sequence as x86_64 above)
                  └─ ...
```

---

## Memory Layout

### x86_64

```
Physical Address       Contents
0x0000_0000           Real-mode IVT / BIOS data (avoid)
0x0000_7C00           Bootloader (MBR stage)
0x0010_0000 (1 MiB)   ─── Kernel load address ───
0x0010_0000           .text (kernel code)
0x????_????           .rodata
0x????_????           .data
0x????_????           .bss
                       ─── Bump allocator pool ───
0x0010_0000           BumpAllocator BASE (= kernel load)
0x0040_0000 (4 MiB)   BumpAllocator END
```

### aarch64 (QEMU virt)

```
Physical Address       Contents
0x0000_0000           Device tree blob (DTB)
0x0008_0000 (512 KiB) ─── Kernel load address ───
0x0008_0000           .text (kernel code)
0x????_????           .rodata, .data, .bss
                       ─── Bump allocator pool ───
0x4000_0000 (1 GiB)   BumpAllocator BASE
0x4040_0000           BumpAllocator END (~4 MiB pool)
0x4000_0000+          DRAM (up to machine memory limit)
```

---

## Kernel Object Model

```
Platform (singleton)
  ├── Console  &'static dyn Console
  ├── Timer    &'static dyn Timer
  ├── Interrupts &'static dyn Interrupts
  ├── Memory   &'static dyn Memory
  └── Cpu      &'static dyn Cpu

Driver Registry (up to 64 drivers)
  └── [&'static dyn Driver; 64]

Memory Manager (mm)
  └── ALLOCATOR: &'static dyn Memory  ← borrows from Platform

Scheduler
  └── TASK_COUNT: AtomicUsize  ← stub; future: task table
```

---

## Extension Points

### Adding Hardware Support
Implement the 5 HAL traits in a new `crates/spiritos-<arch>` crate.
See [docs/HAL.md](docs/HAL.md) and [docs/PORTING.md](docs/PORTING.md).

### Adding Drivers
Implement `Driver` trait, create static instance, register before `kernel_main()`.
See [docs/DRIVERS.md](docs/DRIVERS.md).

### Adding Syscalls
1. Add entry to `Syscall` enum in `spiritos-abi/src/syscall.rs`.
2. Handle in `spiritos-kernel/src/syscall_dispatch.rs`.
3. Document in `docs/ABI.md`.

### Adding Kernel Subsystems
Add a new module in `spiritos-kernel/src/`. The module can depend on `spiritos-hal` traits via `platform::get()`.

---

## Design Decisions

### Why trait objects (`dyn Trait`) instead of generics?
Generic HAL types would require the kernel to be parameterized over all hardware types, leading to monomorphization of the entire kernel per-target. Trait objects allow a single kernel binary linked against any HAL implementation. The vtable overhead is negligible for the few HAL calls per second.

### Why `static mut` for Platform/Driver registry?
SpiritOS boots single-threaded. `static mut` with `unsafe` is explicit about the contract: "call register() once before concurrent access." Future SMP support will add appropriate synchronization (spinlocks or once-cells).

### Why bump allocator?
A bump allocator is the simplest correct implementation — O(1) alloc, no external dependencies. It's intentionally temporary: a proper buddy allocator or slab allocator will replace it once the kernel is further along.

### Why `no_std` throughout?
The kernel runs on bare metal with no OS beneath it. `no_std` is a hard requirement, not a preference. Even `spiritos-abi` is `no_std` so it can be linked into future userspace without a host libc.

---

## Future Roadmap

| Milestone        | Description                                                      |
|------------------|------------------------------------------------------------------|
| **v0.2**         | Real scheduler (round-robin, task table, context switch)        |
| **v0.3**         | Virtual memory (page tables, MMU enable, kernel heap)           |
| **v0.4**         | Userspace tasks (ELF loader, syscall entry points)              |
| **v0.5**         | IPC implementation (message queues, shared memory)              |
| **v0.6**         | RISC-V port                                                     |
| **v0.7**         | SMP support (multi-core scheduler, per-CPU data)                |
| **v1.0**         | Stable ABI, production-quality docs, POSIX subset               |
