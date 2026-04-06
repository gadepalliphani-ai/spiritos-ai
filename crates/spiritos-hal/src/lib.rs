//! SpiritOS HAL — hardware abstraction traits.
//!
//! Each architecture crate provides concrete implementations.
//! The kernel depends only on these traits.
//!
//! | Trait          | Purpose                         |
//! |----------------|---------------------------------|
//! | Console        | Debug output (UART / VGA)       |
//! | Timer          | Monotonic clock + tick          |
//! | Interrupts     | IRQ enable/disable/register     |
//! | Memory         | Physical frame allocator        |
//! | Cpu            | Halt, reset, SMP                |
#![no_std]
#![allow(dead_code, unused_variables)]

pub mod console;
pub mod timer;
pub mod interrupts;
pub mod memory;
pub mod cpu;
pub mod platform;
