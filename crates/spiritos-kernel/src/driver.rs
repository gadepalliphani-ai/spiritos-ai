//! Driver registry.
//!
//! # Adding a driver
//! 1. Implement the `Driver` trait.
//! 2. Call `register()` from your arch crate before `kernel_main()`.
//! 3. `init_all()` will call each driver's `init()`.

pub trait Driver: Send + Sync {
    fn name(&self) -> &'static str;
    fn init(&self);
    fn shutdown(&self) {}
}

const MAX_DRIVERS: usize = 64;
static mut DRIVERS: [Option<&'static dyn Driver>; MAX_DRIVERS] = [None; MAX_DRIVERS];
static mut DRIVER_COUNT: usize = 0;

/// # Safety: call before concurrent access.
pub unsafe fn register(d: &'static dyn Driver) {
    let n = DRIVER_COUNT;
    if n < MAX_DRIVERS {
        DRIVERS[n] = Some(d);
        DRIVER_COUNT += 1;
    }
}

pub fn init_all() {
    unsafe {
        for i in 0..DRIVER_COUNT {
            if let Some(d) = DRIVERS[i] { d.init(); }
        }
    }
}
