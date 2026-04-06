#![no_std]
#![no_main]
#![allow(dead_code, unused_variables)]

use core::panic::PanicInfo;
use spiritos_hal::platform::{self, Platform};

mod console;
mod timer;
mod interrupts;
mod memory;
mod cpu;

static CONSOLE:    console::SerialConsole      = console::SerialConsole;
static TIMER:      timer::PitTimer             = timer::PitTimer;
static INTERRUPTS: interrupts::PicInterrupts   = interrupts::PicInterrupts;
static MEMORY:     memory::BumpAllocator       = memory::BumpAllocator::new();
static CPU_HAL:    cpu::X86Cpu                 = cpu::X86Cpu;

#[no_mangle]
pub extern "C" fn _start() -> ! {
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
fn panic(_info: &PanicInfo) -> ! {
    unsafe {
        if let Some(p) = spiritos_hal::platform::get_opt() {
            p.console.writeln("!!! KERNEL PANIC !!!");
        }
    }
    loop { core::hint::spin_loop(); }
}
