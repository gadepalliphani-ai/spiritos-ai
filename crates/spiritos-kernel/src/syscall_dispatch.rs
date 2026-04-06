use spiritos_abi::{syscall::Syscall, error::Error};

pub fn dispatch(nr: usize, a0: usize, a1: usize, a2: usize, _a3: usize) -> isize {
    if nr == Syscall::Exit as usize {
        return 0;
    }
    if nr == Syscall::Write as usize {
        let p = spiritos_hal::platform::get();
        let slice = unsafe { core::slice::from_raw_parts(a1 as *const u8, a2) };
        if let Ok(s) = core::str::from_utf8(slice) {
            p.console.write_str(s);
            return a2 as isize;
        }
        return Error::InvalidArg as isize;
    }
    if nr == Syscall::GetPid as usize { return 0; }
    if nr == Syscall::Yield as usize  { return 0; }
    Error::NotImpl as isize
}
