/// Physical frame allocator.
pub trait Memory: Send + Sync {
    fn alloc_frames(&self, count: usize) -> Option<usize>;
    fn free_frames(&self, base: usize, count: usize);
    fn frame_size(&self) -> usize;
    fn total_bytes(&self) -> usize;
    fn free_bytes(&self) -> usize;
}
