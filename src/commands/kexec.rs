// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::Error;
use std::os::unix::prelude::*;
use std::path::PathBuf;
use std::time::Duration;

use super::Command;

use structopt::StructOpt;

/// Load a kernel to be executed on reboot
#[derive(StructOpt, Debug)]
pub struct Kexec {
    /// The path to the kernel to load
    #[structopt(long, short)]
    pub kernel: PathBuf,

    /// The path to the initrd to load
    #[structopt(long, short)]
    pub initrd: PathBuf,

    /// The kernel command line to use after reboot
    #[structopt(long, short)]
    pub cmdline: String,

    /// Reboot immediately (does not do a clean shutdown)
    #[structopt(long, short)]
    pub reboot: bool,
}

impl Kexec {
    pub fn kexec(kernel: File, initrd: File, cmdline: &CStr) -> std::io::Result<()> {
        let kernel = kernel.as_raw_fd() as usize;
        let initrd = initrd.as_raw_fd();
        let cmdline = cmdline.to_bytes_with_nul();
        let retval: usize;

        #[cfg(target_arch = "aarch64")]
        unsafe {
            asm!(
                "svc #0",
                in("w8") libc::SYS_kexec_file_load,
                inout("x0") kernel => retval,
                in("x1") initrd,
                in("x2") cmdline.len(),
                in("x3") cmdline.as_ptr(),
                in("x4") 0,
                in("x5") 0,
            );
        }

        #[cfg(target_arch = "x86_64")]
        unsafe {
            asm!(
                "syscall",
                inout("rax") libc::SYS_kexec_file_load => retval,
                in("rdi") kernel,
                in("rsi") initrd,
                in("rdx") cmdline.len(),
                in("r10") cmdline.as_ptr(),
                in("r8") 0,
                in("r9") 0,
            );
        }

        if retval > -4096isize as usize {
            let code = -(retval as isize) as i32;
            return Err(std::io::Error::from_raw_os_error(code));
        }

        Ok(())
    }

    pub fn reboot() -> std::io::Result<()> {
        unsafe { libc::sync() };

        let ret = unsafe { libc::reboot(libc::LINUX_REBOOT_CMD_KEXEC) };
        if ret < 0 {
            return Err(Error::last_os_error());
        }

        Ok(())
    }
}

impl Command for Kexec {
    fn execute(self) -> anyhow::Result<()> {
        let kernel = File::open(self.kernel)?;
        let initrd = File::open(self.initrd)?;
        let cmdline = CString::new(self.cmdline)?;

        Self::kexec(kernel, initrd, &cmdline)?;

        // Wait for the kernel to tell us it is ready.
        while std::fs::read("/sys/kernel/kexec_loaded")? != [b'1', b'\n'] {
            std::thread::sleep(Duration::from_millis(100));
        }

        if self.reboot {
            Self::reboot()?;
        }

        Ok(())
    }
}
