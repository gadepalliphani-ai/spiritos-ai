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
mod idt;

use console::SerialConsole;
use timer::PitTimer;
use interrupts::PicInterrupts;
use memory::BumpAllocator;
use cpu::X86Cpu;
use context::X86_CONTEXT;

static CONSOLE:    SerialConsole = SerialConsole;
static TIMER:      PitTimer      = PitTimer;
static INTERRUPTS: PicInterrupts = PicInterrupts;
static MEMORY:     BumpAllocator = BumpAllocator::new();
static CPU_HAL:    X86Cpu        = X86Cpu;

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

        // Register the x86_64 context-switch backend before kernel_main.
        spiritos_kernel::context::set_backend(&X86_CONTEXT);
    }

    // Set up the Interrupt Descriptor Table so that hardware IRQs are
    // dispatched to our registered handlers.
    idt::init();

    // Programme the PIT for ~100 Hz preemption ticks.
    PitTimer::init_pit();

    // Wire IRQ0 (timer) to our handler and unmask it.
    INTERRUPTS.register_handler(0, timer::pit_irq_handler);
    INTERRUPTS.unmask(0);

    // Register the demo task to be spawned after mm::init() inside kernel_main.
    unsafe {
        spiritos_kernel::scheduler::register_startup_task("hello", 1, demo_task);
    }

    spiritos_kernel::kernel_main()
}

/// A simple demo task: print a message, spin briefly, then yield cooperatively.
fn demo_task() -> ! {
    let p = platform::get();
    loop {
        p.console.writeln("[task:hello] Hello from context-switched task!");
        for _ in 0..1_000_000u64 {
            core::hint::spin_loop();
        }
        spiritos_kernel::scheduler::yield_now();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    if let Some(p) = spiritos_hal::platform::get_opt() {
        p.console.writeln("!!! KERNEL PANIC !!!");
    }
    loop {
        core::hint::spin_loop();
    }
}
