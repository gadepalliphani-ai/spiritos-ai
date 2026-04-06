# SpiritOS Application Binary Interface (ABI)

## Overview

The SpiritOS ABI defines the interface between userspace tasks and the kernel. It specifies:
- How syscalls are invoked (register convention, instruction)
- The syscall number table
- Argument layouts
- Return value and error code conventions
- IPC message format
- ABI stability guarantees

All definitions live in `crates/spiritos-abi` — a `no_std` crate usable in both kernel and userspace.

---

## Calling Convention

### x86_64

Syscalls are invoked with the `syscall` instruction.

| Register | Role             |
|----------|------------------|
| `rax`    | Syscall number   |
| `rdi`    | Argument 0       |
| `rsi`    | Argument 1       |
| `rdx`    | Argument 2       |
| `r10`    | Argument 3       |
| `rax`    | Return value     |

Registers `rcx` and `r11` are clobbered by the `syscall` instruction. All other registers are preserved.

**Example (x86_64 assembly):**
```asm
; write(fd=1, buf, len)
mov rax, 1          ; SYS_WRITE
mov rdi, 1          ; fd = stdout
lea rsi, [buf]      ; buf ptr
mov rdx, len        ; length
syscall
; rax = bytes written, or negative error
```

**Example (Rust inline asm):**
```rust
unsafe fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let ret: isize;
    core::arch::asm!(
        "syscall",
        in("rax") 1usize,   // SYS_WRITE
        in("rdi") fd,
        in("rsi") buf,
        in("rdx") len,
        lateout("rax") ret,
        out("rcx") _,
        out("r11") _,
    );
    ret
}
```

---

### aarch64

Syscalls are invoked with the `svc #0` instruction.

| Register | Role             |
|----------|------------------|
| `x8`     | Syscall number   |
| `x0`     | Argument 0       |
| `x1`     | Argument 1       |
| `x2`     | Argument 2       |
| `x3`     | Argument 3       |
| `x0`     | Return value     |

All other general-purpose registers are preserved across syscalls.

**Example (aarch64 assembly):**
```asm
// write(fd=1, buf, len)
mov x8, #1          // SYS_WRITE
mov x0, #1          // fd
adr x1, buf         // buf ptr
mov x2, len         // length
svc #0
// x0 = bytes written, or negative error
```

**Example (Rust inline asm):**
```rust
unsafe fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let ret: isize;
    core::arch::asm!(
        "svc #0",
        in("x8") 1usize,    // SYS_WRITE
        in("x0") fd,
        in("x1") buf,
        in("x2") len,
        lateout("x0") ret,
    );
    ret
}
```

---

## Syscall Table

| Number | Name        | arg0       | arg1       | arg2       | arg3     | Returns               |
|--------|-------------|------------|------------|------------|----------|-----------------------|
| 0      | `Exit`      | exit_code  | —          | —          | —        | (never returns)       |
| 1      | `Write`     | fd         | buf_ptr    | len        | —        | bytes written / error |
| 2      | `Read`      | fd         | buf_ptr    | len        | —        | bytes read / error    |
| 3      | `AllocPage` | size_bytes | —          | —          | —        | base_addr / error     |
| 4      | `FreePage`  | ptr        | size_bytes | —          | —        | 0 / error             |
| 5      | `IpcSend`   | dest_pid   | msg_ptr    | msg_len    | —        | 0 / error             |
| 6      | `IpcRecv`   | buf_ptr    | buf_len    | —          | —        | msg_len / error       |
| 7      | `GetPid`    | —          | —          | —          | —        | pid (u32)             |
| 8      | `Yield`     | —          | —          | —          | —        | 0                     |
| 9      | `Sleep`     | ms         | —          | —          | —        | 0                     |
| 10     | `IrqBind`   | irq_nr     | handler_fn | —          | —        | 0 / error (priv)      |

### Syscall Descriptions

#### `Exit` (0)
Terminates the calling task immediately. The kernel reclaims all resources. Does not return.

```
arg0: exit_code (u32) — task exit status
```

#### `Write` (1)
Write bytes to a file descriptor.

```
arg0: fd      — file descriptor (0=stdin, 1=stdout, 2=stderr)
arg1: buf_ptr — userspace pointer to byte buffer
arg2: len     — number of bytes to write
returns: bytes actually written, or negative error code
```

#### `Read` (2)
Read bytes from a file descriptor. Blocks until data is available.

```
arg0: fd      — file descriptor
arg1: buf_ptr — userspace buffer to read into
arg2: len     — maximum bytes to read
returns: bytes read (0 = EOF), or negative error code
```

#### `AllocPage` (3)
Allocate physically contiguous memory pages.

```
arg0: size_bytes — minimum allocation size (rounded up to frame boundary)
returns: base physical/virtual address on success, or negative error
```

#### `FreePage` (4)
Release previously allocated pages.

```
arg0: ptr        — base address returned by AllocPage
arg1: size_bytes — original size passed to AllocPage
returns: 0 on success, negative error
```

#### `IpcSend` (5)
Send an IPC message to another task.

```
arg0: dest_pid — destination task PID
arg1: msg_ptr  — pointer to message (MessageHeader + payload)
arg2: msg_len  — total message size in bytes
returns: 0 on success, negative error
```

#### `IpcRecv` (6)
Receive an IPC message. Blocks until a message arrives.

```
arg0: buf_ptr — buffer to receive into (must fit MessageHeader + payload)
arg1: buf_len — buffer length
returns: message length on success, negative error
```

#### `GetPid` (7)
Get the current task's process ID.

```
returns: pid as u32 (zero-extended to isize)
```

#### `Yield` (8)
Voluntarily yield the CPU to the scheduler.

```
returns: 0
```

#### `Sleep` (9)
Sleep for at least the specified duration.

```
arg0: ms — milliseconds to sleep
returns: 0
```

#### `IrqBind` (10)
Register a kernel-level IRQ handler (privileged; only allowed to ring-0 tasks).

```
arg0: irq_nr     — interrupt request number
arg1: handler_fn — function pointer (fn(u32))
returns: 0 on success, Error::Perm if unprivileged
```

---

## Error Codes

All error codes are negative `isize` values (matching the `i64` repr of `Error`):

| Code | Name          | Meaning                                  |
|------|---------------|------------------------------------------|
| -1   | `Perm`        | Permission denied                        |
| -2   | `NotFound`    | Resource not found                       |
| -3   | `InvalidArg`  | Invalid argument (null ptr, bad range)   |
| -4   | `OutOfMemory` | Physical memory exhausted                |
| -5   | `WouldBlock`  | Operation would block (non-blocking mode)|
| -6   | `Io`          | I/O error                                |
| -7   | `NotImpl`     | Syscall not yet implemented              |
| -8   | `Busy`        | Resource temporarily busy                |

---

## IPC Protocol

IPC messages start with a fixed `MessageHeader`:

```rust
#[repr(C)]
pub struct MessageHeader {
    pub src_pid:     u32,   // filled by kernel on receive
    pub dst_pid:     u32,   // destination task PID
    pub msg_type:    u32,   // application-defined message type
    pub payload_len: u32,   // bytes of payload following the header
}
```

The payload immediately follows the header in memory. Total message size = `size_of::<MessageHeader>() + payload_len`.

**Send example:**
```rust
let header = MessageHeader {
    src_pid: 0,       // ignored; kernel fills it
    dst_pid: 42,
    msg_type: 0x100,  // app-defined "PING"
    payload_len: 4,
};
let payload: [u8; 4] = [1, 2, 3, 4];
// Pack into contiguous buffer:
let mut buf = [0u8; size_of::<MessageHeader>() + 4];
buf[..16].copy_from_slice(unsafe {
    core::slice::from_raw_parts(&header as *const _ as *const u8, 16)
});
buf[16..].copy_from_slice(&payload);
sys_ipc_send(42, buf.as_ptr(), buf.len());
```

---

## ABI Stability Guarantees

**Version 0.x (current):** ABI is **unstable**. Syscall numbers may change between releases.

**Version 1.0 (planned):**
- Syscall numbers are frozen. Existing syscalls will not be renumbered.
- New syscalls are added with new numbers only.
- `MessageHeader` layout is frozen (C-compatible).
- Error codes are frozen.

**Versioning Strategy:**
- The `spiritos-abi` crate version tracks the ABI version.
- Userspace binaries embed the ABI version they were compiled against.
- The kernel checks at task load time and refuses to run incompatible binaries (future).

---

## Example: Complete Userspace Task (Rust, no_std)

```rust
#![no_std]
#![no_main]

use core::arch::asm;

fn sys_write(buf: &[u8]) -> isize {
    let ret: isize;
    unsafe {
        asm!(
            "syscall",
            in("rax") 1usize,
            in("rdi") 1usize,
            in("rsi") buf.as_ptr(),
            in("rdx") buf.len(),
            lateout("rax") ret,
            out("rcx") _, out("r11") _,
        );
    }
    ret
}

fn sys_exit(code: usize) -> ! {
    unsafe {
        asm!("syscall", in("rax") 0usize, in("rdi") code, options(noreturn));
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    sys_write(b"Hello from userspace!\n");
    sys_exit(0);
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }
```
