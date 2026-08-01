#![no_main]
mod buf;
mod cached;
mod sys;

use crate::sys::{SysInfo, collect};
use core::fmt::Write;

struct Sink<const N: usize> {
    buf: [u8; N],
    len: usize,
    ok: bool, // sticky overflow flag
}

impl<const N: usize> Sink<N> {
    const fn new() -> Self {
        Self {
            buf: [0; N],
            len: 0,
            ok: true,
        }
    }

    /// One write, looping only if the kernel short-writes.
    fn flush(&self) {
        let mut off = 0;
        while off < self.len {
            let n = unsafe { libc::write(1, self.buf[off..].as_ptr().cast(), self.len - off) };
            if n <= 0 {
                return;
            } // EOF/EPIPE
            off += n as usize;
        }
    }
}

impl<const N: usize> core::fmt::Write for Sink<N> {
    #[inline]
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let b = s.as_bytes();
        let end = self.len + b.len();
        if end > N {
            self.ok = false;
            return Err(core::fmt::Error);
        }
        self.buf[self.len..end].copy_from_slice(b);
        self.len = end;
        Ok(())
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn main() {
    let mut info = SysInfo::new();
    collect(&mut info);

    let mut sink = Sink::<1024>::new();

    let _ = write!(
        &mut sink,
        "
     ---(_)\t\tos:       {}
 _/  ---  \\\t\tkernel:   {}
(_) |   |\t\tuptime:   {}
  \\  --- _/\t\tdisk:     {}/{}
     ---(_)\t\tmem:      {}/{}     

",
        info.os.name.as_str(),
        info.os.kernel.as_str(),
        info.uptime.uptime.as_str(),
        info.disk.used,
        info.disk.total,
        info.mem.used_gb.as_str(),
        info.mem.total_gb.as_str(),
    );

    sink.flush();
}
