//! Canonical error codes returned by syscalls.
#[repr(i64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Error {
    Perm        = -1,
    NotFound    = -2,
    InvalidArg  = -3,
    OutOfMemory = -4,
    WouldBlock  = -5,
    Io          = -6,
    NotImpl     = -7,
    Busy        = -8,
}
