//! IPC message header.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MessageHeader {
    pub src_pid:     u32,
    pub dst_pid:     u32,
    pub msg_type:    u32,
    pub payload_len: u32,
}
