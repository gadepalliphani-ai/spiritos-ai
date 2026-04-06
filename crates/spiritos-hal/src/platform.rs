use crate::{console::Console, timer::Timer, interrupts::Interrupts, memory::Memory, cpu::Cpu};

pub struct Platform {
    pub console:    &'static dyn Console,
    pub timer:      &'static dyn Timer,
    pub interrupts: &'static dyn Interrupts,
    pub memory:     &'static dyn Memory,
    pub cpu:        &'static dyn Cpu,
}

static mut PLATFORM: Option<Platform> = None;

/// # Safety
/// Must be called exactly once before any kernel code accesses the platform.
pub unsafe fn register(p: Platform) {
    PLATFORM = Some(p);
}

pub fn get() -> &'static Platform {
    #[allow(static_mut_refs)]
    unsafe { PLATFORM.as_ref().expect("Platform not registered") }
}

pub fn get_opt() -> Option<&'static Platform> {
    #[allow(static_mut_refs)]
    unsafe { PLATFORM.as_ref() }
}
