//! SpiritOS ABI — syscall numbers, argument structures, and return codes.
//! This crate is `no_std` and may be linked into both kernel and userspace.
#![no_std]
#![allow(dead_code)]

pub mod syscall;
pub mod error;
pub mod ipc;
