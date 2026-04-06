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

static CONSOLE:    console::Pl011Uart       = console::Pl011Uart;
static TIMER:      timer::GenericTimer      = timer::GenericTimer;
static INTERRUPTS: interrupts::GicStub      = interrupts::GicStub;
static MEMORY:     memory::BumpAllocator    = memory::BumpAllocator::new();
static CPU_HAL:    cpu::Aarch64Cpu          = cpu::Aarch64Cpu;

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
    loop { core::hint::spin_loop(); }
}
