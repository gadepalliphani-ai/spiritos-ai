#![no_std]
#![no_main]
#![allow(dead_code, unused_variables, static_mut_refs)]

use core::panic::PanicInfo;
use spiritos_hal::platform::{self, Platform};
use spiritos_hal::interrupts::Interrupts;

mod console;
mod timer;
mod interrupts;
mod memory;
mod cpu;
mod context;

use console::Pl011Uart;
use timer::GenericTimer;
use interrupts::GicStub;
use memory::BumpAllocator;
use cpu::Aarch64Cpu;
use context::AARCH64_CONTEXT;

static CONSOLE:    Pl011Uart    = Pl011Uart;
static TIMER:      GenericTimer = GenericTimer;
static INTERRUPTS: GicStub      = GicStub;
static MEMORY:     BumpAllocator = BumpAllocator::new();
static CPU_HAL:    Aarch64Cpu   = Aarch64Cpu;

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

        // Register the aarch64 context-switch backend before kernel_main.
        spiritos_kernel::context::set_backend(&AARCH64_CONTEXT);
    }

    // Arm the Generic Timer for ~100 Hz preemption ticks.
    let ticks = GenericTimer::freq() / 100;
    GenericTimer::set_tval(ticks);

    // Wire the physical timer IRQ (GIC PPI 30) to our handler and unmask it.
    INTERRUPTS.register_handler(30, timer::timer_irq_handler);
    INTERRUPTS.unmask(30);

    // Spawn a demonstration task.
    unsafe {
        spiritos_kernel::scheduler::spawn("hello", 1, demo_task);
    }

    spiritos_kernel::kernel_main()
}

/// A simple demo task: print a message, spin briefly, then yield cooperatively.
fn demo_task() -> ! {
    let p = platform::get();
    loop {
        p.console.writeln("[task:hello] Hello from context-switched task!");
        for _ in 0..5_000_000u64 {
            core::hint::spin_loop();
        }
        spiritos_kernel::scheduler::yield_now();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
