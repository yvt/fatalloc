use core::fmt;

static mut MUTEX: libc::pthread_mutex_t = libc::PTHREAD_MUTEX_INITIALIZER;

pub fn warn(args: fmt::Arguments<'_>) {
    unsafe { libc::pthread_mutex_lock(core::ptr::addr_of_mut!(MUTEX)) };
    let _ = fmt::Write::write_str(&mut Stderr, "fatalloc: ");
    let _ = fmt::Write::write_fmt(&mut Stderr, args);
    let _ = fmt::Write::write_str(&mut Stderr, "\n");
    unsafe { libc::pthread_mutex_unlock(core::ptr::addr_of_mut!(MUTEX)) };
}

struct Stderr;

impl fmt::Write for Stderr {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let stderr = 2;
        let mut s = s.as_bytes();
        while !s.is_empty() {
            let written = unsafe { libc::write(stderr, s.as_ptr().cast(), s.len()) };
            if written < 0 {
                break;
            }
            s = &s[written as usize..];
        }
        Ok(())
    }
}

macro_rules! warn {
    ($($tt:tt)*) => {
        crate::logger::warn(format_args!($($tt)*))
    }
}
