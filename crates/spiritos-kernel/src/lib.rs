//! SpiritOS Kernel core.
//! Architecture crates call `kernel_main()` after registering the platform.
#![no_std]
#![allow(dead_code, unused_variables, unused_imports, static_mut_refs)]

pub mod task;
pub mod context;
pub mod scheduler;
pub mod syscall_dispatch;
pub mod driver;
pub mod mm;

use spiritos_hal::platform;

pub fn kernel_main() -> ! {
    let p = platform::get();
    p.console.writeln("[spiritos] Kernel starting...");

    mm::init(p.memory);
    p.console.writeln("[spiritos] Memory OK");

    // context backend must be registered by arch crate before kernel_main
    // (via context::set_backend)

    scheduler::init();
    p.console.writeln("[spiritos] Scheduler OK");

    driver::init_all();
    p.console.writeln("[spiritos] Drivers OK");

    p.interrupts.enable();
    p.console.writeln("[spiritos] Interrupts enabled");

    p.console.writeln("[spiritos] SpiritOS.ai v0.2 - context switching active");
    p.console.writeln("[rt] scheduler: round-robin preemptive, tick=100Hz");
    p.console.writeln("[rt] latency: QEMU smoke-test only; hard RT requires hardware measurement");
    scheduler::run()
}
