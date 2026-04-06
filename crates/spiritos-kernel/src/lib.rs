//! SpiritOS Kernel core.
//! Architecture crates call `kernel_main()` after registering the platform.
#![no_std]
#![allow(dead_code, unused_variables, unused_imports)]

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
    scheduler::init();
    p.console.writeln("[spiritos] Scheduler OK");
    driver::init_all();
    p.console.writeln("[spiritos] Drivers OK");
    p.interrupts.enable();
    p.console.writeln("[spiritos] SpiritOS.ai is alive!");
    scheduler::run()
}
