use spiritos_hal::memory::Memory;

static mut ALLOCATOR: Option<&'static dyn Memory> = None;

pub fn init(mem: &'static dyn Memory) {
    unsafe { ALLOCATOR = Some(mem); }
}

pub fn alloc_bytes(bytes: usize) -> Option<usize> {
    let a = unsafe { ALLOCATOR? };
    let frame_size = a.frame_size();
    let frames = (bytes + frame_size - 1) / frame_size;
    a.alloc_frames(frames)
}

pub fn free_bytes(addr: usize, bytes: usize) {
    if let Some(a) = unsafe { ALLOCATOR } {
        let frame_size = a.frame_size();
        let frames = (bytes + frame_size - 1) / frame_size;
        a.free_frames(addr, frames);
    }
}
