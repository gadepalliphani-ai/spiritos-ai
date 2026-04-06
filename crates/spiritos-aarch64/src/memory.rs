use spiritos_hal::memory::Memory;
use core::sync::atomic::{AtomicUsize, Ordering};

pub struct BumpAllocator { next: AtomicUsize }
unsafe impl Send for BumpAllocator {}
unsafe impl Sync for BumpAllocator {}

const BASE:  usize = 0x4000_0000;
const END:   usize = 0x4040_0000;
const FRAME: usize = 4096;

impl BumpAllocator {
    pub const fn new() -> Self { Self { next: AtomicUsize::new(BASE) } }
}

impl Memory for BumpAllocator {
    fn alloc_frames(&self, count: usize) -> Option<usize> {
        let size = count * FRAME;
        let base = self.next.fetch_add(size, Ordering::Relaxed);
        if base + size <= END { Some(base) } else { None }
    }
    fn free_frames(&self, _: usize, _: usize) {}
    fn frame_size(&self) -> usize { FRAME }
    fn total_bytes(&self) -> usize { END - BASE }
    fn free_bytes(&self) -> usize {
        let n = self.next.load(Ordering::Relaxed);
        if n < END { END - n } else { 0 }
    }
}
