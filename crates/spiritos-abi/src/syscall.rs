//! Syscall numbers and calling convention.
//!
//! ## Calling Convention
//!
//! | Purpose    | x86_64 | aarch64 |
//! |------------|--------|---------|
//! | syscall nr | rax    | x8      |
//! | arg0       | rdi    | x0      |
//! | arg1       | rsi    | x1      |
//! | arg2       | rdx    | x2      |
//! | arg3       | r10    | x3      |
//! | return     | rax    | x0      |
//!
//! Negative return values indicate an error (see `error::Error`).

#[repr(usize)]
#[non_exhaustive]
#[allow(dead_code)]
pub enum Syscall {
    /// Terminate the current task. arg0 = exit code.
    Exit      = 0,
    /// Write bytes to fd. arg0=fd, arg1=buf_ptr, arg2=len.
    Write     = 1,
    /// Read bytes from fd. arg0=fd, arg1=buf_ptr, arg2=len.
    Read      = 2,
    /// Allocate pages. arg0=size_bytes. Returns base address.
    AllocPage = 3,
    /// Free pages. arg0=ptr, arg1=size_bytes.
    FreePage  = 4,
    /// Send IPC. arg0=dest_pid, arg1=msg_ptr, arg2=msg_len.
    IpcSend   = 5,
    /// Receive IPC (blocking). arg0=buf_ptr, arg1=buf_len.
    IpcRecv   = 6,
    /// Get current task ID.
    GetPid    = 7,
    /// Yield CPU to scheduler.
    Yield     = 8,
    /// Sleep arg0 milliseconds.
    Sleep     = 9,
    /// Register IRQ handler (privileged). arg0=irq_nr, arg1=handler_fn_ptr.
    IrqBind   = 10,
}
