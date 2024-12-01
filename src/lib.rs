#![feature(anonymous_pipe)]

use std::{
    io::Read,
    os::fd::{IntoRawFd, OwnedFd},
    pipe::{PipeReader, pipe},
    sync::{LockResult, Mutex, MutexGuard, PoisonError},
};

unsafe extern "C" {
    static mut stdout: *mut libc::FILE;
    static mut stderr: *mut libc::FILE;
}

pub struct LentFile {
    file: &'static mut *mut libc::FILE,
    guard: MutexGuard<'static, ()>,
}

pub fn stdout_mut() -> Result<LentFile, PoisonError<MutexGuard<'static, ()>>> {
    static MUTEX: Mutex<()> = Mutex::new(());

    #[allow(static_mut_refs)]
    Ok(LentFile {
        file: unsafe { &mut stdout },
        guard: MUTEX.lock()?,
    })
}

pub fn stderr_mut() -> Result<LentFile, PoisonError<MutexGuard<'static, ()>>> {
    static MUTEX: Mutex<()> = Mutex::new(());

    #[allow(static_mut_refs)]
    Ok(LentFile {
        file: unsafe { &mut stderr },
        guard: MUTEX.lock()?,
    })
}

struct SwapFile {
    swapped: *mut libc::FILE,
    target: LentFile,
}

impl SwapFile {
    fn fdopen(fd: i32) -> *mut libc::FILE {
        unsafe { libc::fdopen(fd, "wb".as_bytes().as_ptr() as _) }
    }

    pub fn new(fd: OwnedFd, target: LentFile) -> Self {
        let mut file = Self::fdopen(fd.into_raw_fd());

        unsafe extern "C" {
            fn flockfile(file: *mut libc::FILE);
        }

        unsafe {
            flockfile(file);
        } // lock file to prevent other threads from writing to it

        core::mem::swap(target.file, &mut file);

        Self {
            swapped: file,
            target,
        }
    }
}

impl Drop for SwapFile {
    fn drop(&mut self) {
        // restore stdout, stderr
        core::mem::swap(self.target.file, &mut self.swapped);

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

        // drop guard
    }
}

pub fn capture<F: FnOnce()>(f: F, target: LentFile) -> std::io::Result<PipeReader> {
    let (reader, writer) = pipe()?;

    let swap_file = SwapFile::new(writer.into(), target);

    f();

    drop(swap_file);

    Ok(reader)
}

pub fn cap_string<F: FnOnce()>(f: F, target: LentFile) -> std::io::Result<String> {
    let mut string = String::new();
    capture(f, target)?.read_to_string(&mut string)?;

    Ok(string)
}

pub fn cap_stdout<F: FnOnce()>(f: F) -> std::io::Result<String> {
    cap_string(f, stdout_mut().map_err(|_| std::io::ErrorKind::Deadlock)?)
}

pub fn cap_stderr<F: FnOnce()>(f: F) -> std::io::Result<String> {
    cap_string(f, stderr_mut().map_err(|_| std::io::ErrorKind::Deadlock)?)
}
