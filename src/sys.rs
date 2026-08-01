//! System data collection

use crate::buf::Buf;
use crate::cached::CachePadded;

const P_OSREL: *const u8 = b"/etc/os-release\0".as_ptr();
const P_ROOT: *const u8 = b"/\0".as_ptr();

/// Read a file into `buf` via libc::open/read/close. Returns bytes read.
#[inline]
pub fn raw_read(path: *const u8, buf: &mut [u8]) -> usize {
    unsafe {
        let fd = libc::open(path as *const libc::c_char, libc::O_RDONLY);
        if fd < 0 {
            return 0;
        }
        let n = libc::read(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len());
        libc::close(fd);
        if n > 0 { n as usize } else { 0 }
    }
}

#[inline]
fn is_horizontal_space(byte: u8) -> bool {
    byte == b' ' || byte == b'\t'
}

#[inline]
fn trim_value(mut value: &[u8]) -> &[u8] {
    while let Some(&byte) = value.first() {
        if is_horizontal_space(byte) || byte == b'"' {
            value = &value[1..];
        } else {
            break;
        }
    }

    while let Some(&byte) = value.last() {
        if is_horizontal_space(byte) || byte == b'\r' || byte == b'"' {
            value = &value[..value.len() - 1];
        } else {
            break;
        }
    }

    value
}

pub fn find_val<'a>(content: &'a [u8], key: &[u8], sep: u8) -> Option<&'a [u8]> {
    for line in content.split(|&byte| byte == b'\n') {
        let Some(mut remainder) = line.strip_prefix(key) else {
            continue;
        };

        // Permit spaces or tabs between the key and separator.
        while let Some(&byte) = remainder.first() {
            if is_horizontal_space(byte) {
                remainder = &remainder[1..];
            } else {
                break;
            }
        }

        let Some((&separator, value)) = remainder.split_first() else {
            continue;
        };

        if separator != sep {
            continue;
        }

        let value = trim_value(value);

        if !value.is_empty() {
            return Some(value);
        }
    }

    None
}

fn capitalize_into<const N: usize>(src: &[u8], dst: &mut Buf<N>) {
    if src.is_empty() {
        return;
    }
    if src[0].is_ascii_lowercase() {
        dst.push_byte(src[0] - 32);
        dst.push_bytes(&src[1..]);
    } else {
        dst.push_bytes(src);
    }
}

/// Linux utmp record layout (x86_64). 384 bytes per entry.
#[repr(C)]
struct UtmpEntry {
    ut_type: i16,
    _pad: i16,
    ut_pid: i32,
    ut_line: [u8; 32],
    ut_id: [u8; 4],
    ut_user: [u8; 32],
    ut_host: [u8; 256],
    ut_exit: [i16; 2],
    ut_session: i32,
    ut_tv_sec: i32,
    ut_tv_usec: i32,
    ut_addr_v6: [i32; 4],
    __reserved: [u8; 20],
}

const _: () = assert!(std::mem::size_of::<UtmpEntry>() == 384);

pub struct OsData {
    pub name: Buf<128>,
    pub kernel: Buf<128>,
}

pub struct DiskData {
    pub used: u64,
    pub total: u64,
}

pub struct MemData {
    pub used_gb: Buf<16>,
    pub total_gb: Buf<16>,
}

pub struct LoginData {
    pub uptime: Buf<32>,
}

pub struct SysInfo {
    pub os: CachePadded<OsData>,
    pub disk: CachePadded<DiskData>,
    pub mem: CachePadded<MemData>,
    pub uptime: CachePadded<LoginData>,
}

impl SysInfo {
    pub fn new() -> Self {
        Self {
            os: CachePadded::new(OsData {
                name: Buf::new(),
                kernel: Buf::new(),
            }),
            disk: CachePadded::new(DiskData { used: 0, total: 1 }),
            mem: CachePadded::new(MemData {
                used_gb: Buf::new(),
                total_gb: Buf::new(),
            }),
            uptime: CachePadded::new(LoginData { uptime: Buf::new() }),
        }
    }
}

/// Collect all system data (one-shot).
pub fn collect(info: &mut SysInfo) {
    let mut buf = [0u8; 2048];
    let n = raw_read(P_OSREL, &mut buf);
    let content = &buf[..n];

    let id = find_val(content, b"ID", b'=').unwrap_or(b"linux");
    let ver = find_val(content, b"VERSION", b'=').unwrap_or(b"");
    let code = find_val(content, b"VERSION_CODENAME", b'=').unwrap_or(b"");

    capitalize_into(id, &mut info.os.name);
    info.os.name.push_byte(b' ');
    info.os.name.push_bytes(ver);
    info.os.name.push_byte(b' ');
    capitalize_into(code, &mut info.os.name);

    unsafe {
        let mut u: libc::utsname = std::mem::zeroed();
        if libc::uname(&mut u) == 0 {
            let sn = std::ffi::CStr::from_ptr(u.sysname.as_ptr()).to_bytes();
            let rel = std::ffi::CStr::from_ptr(u.release.as_ptr()).to_bytes();
            info.os.kernel.push_bytes(sn);
            info.os.kernel.push_byte(b' ');
            info.os.kernel.push_bytes(rel);
        }
    }
    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(P_ROOT as *const libc::c_char, &mut stat) == 0 {
            let block = stat.f_frsize as u64;
            let total_bytes = stat.f_blocks as u64 * block;
            let used_bytes = total_bytes - (stat.f_bfree as u64 * block);
            let total_mb = total_bytes / (1024 * 1024);
            let used_mb = used_bytes / (1024 * 1024);
            info.disk.used = used_mb;
            info.disk.total = total_mb;
        }
    }
    let mut si: libc::sysinfo = unsafe { std::mem::zeroed() };
    unsafe { libc::sysinfo(&mut si) };

    let total = si.totalram;
    let avail = si.freeram;
    let used = total.saturating_sub(avail);

    info.mem
        .used_gb
        .push_f64_2dp(used as f64 / (1024.0 * 1024.0));
    info.mem
        .total_gb
        .push_f64_2dp(total as f64 / (1024.0 * 1024.0));

    let secs = si.uptime as u64;
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let mins = (secs % 3600) / 60;
    if days > 0 {
        info.uptime.uptime.push_u64(days);
        info.uptime.uptime.push_str("d ");
    }
    if hours > 0 || days > 0 {
        info.uptime.uptime.push_u64(hours);
        info.uptime.uptime.push_str("h ");
    }
    info.uptime.uptime.push_u64(mins);
    info.uptime.uptime.push_byte(b'm');
}
