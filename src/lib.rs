#![feature(anonymous_pipe)]

use std::{
    io::Read,
    os::fd::{IntoRawFd, OwnedFd},
    pipe::{PipeReader, pipe},
};

unsafe extern "C" {
    static mut stdout: *mut libc::FILE;
    static mut stderr: *mut libc::FILE;
}

pub fn stdout_mut() -> &'static mut *mut libc::FILE {
    #[allow(static_mut_refs)]
    unsafe {
        &mut stdout
    }
}

pub fn stderr_mut() -> &'static mut *mut libc::FILE {
    #[allow(static_mut_refs)]
    unsafe {
        &mut stderr
    }
}

struct SwapFile {
    swapped: *mut libc::FILE,
    target: &'static mut *mut libc::FILE,
}

impl SwapFile {
    fn fdopen(fd: i32) -> *mut libc::FILE {
        unsafe { libc::fdopen(fd, "wb".as_bytes().as_ptr() as _) }
    }

    pub fn new(fd: OwnedFd, target: &'static mut *mut libc::FILE) -> Self {
        let mut file = Self::fdopen(fd.into_raw_fd());

        unsafe {
            unsafe extern "C" {
                fn flockfile(file: *mut libc::FILE);
            }

            flockfile(file); // lock other threads
        }

        core::mem::swap(target, &mut file);

        Self {
            swapped: file,
            target,
        }
    }
}

impl Drop for SwapFile {
    fn drop(&mut self) {
        // restore stdout, stderr
        core::mem::swap(self.target, &mut self.swapped);

        unsafe {
            // now, self.target points to file created at `new` method
            // we must fflush them.
            // because, due to the pipes are not tty, FILE->_flags donen't have _IO_LINE_BUF.
            // fclose calls fflush internally. so we don't have to call it.

            // self.target had been locked at `new` method.
            // so, we don't have to call funlockfile it before flocse.

            libc::fclose(self.swapped);
            // self.swapped is FILE that we created at `new` method.
            // `fd` was OwnedFd. so closing it is safe.
        }
    }
}

pub fn capture<F: FnOnce()>(
    f: F,
    target: &'static mut *mut libc::FILE,
) -> std::io::Result<PipeReader> {
    let (reader, writer) = pipe()?;

    let swap_file = SwapFile::new(writer.into(), target);

    f();

    drop(swap_file);

    Ok(reader)
}

pub fn cap_string<F: FnOnce()>(
    f: F,
    target: &'static mut *mut libc::FILE,
) -> std::io::Result<String> {
    let mut string = String::new();
    capture(f, target)?.read_to_string(&mut string)?;

    Ok(string)
}

pub fn cap_stdout<F: FnOnce()>(f: F) -> std::io::Result<String> {
    cap_string(f, stdout_mut())
}

pub fn cap_stderr<F: FnOnce()>(f: F) -> std::io::Result<String> {
    cap_string(f, stderr_mut())
}
