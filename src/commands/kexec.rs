// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

use std::ffi::{CStr, CString};
use std::fmt::Arguments;
use std::fs::File;
use std::os::unix::prelude::*;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::commands::extract::{Extract, LookAside};

use super::Command;

use structopt::StructOpt;

/// Load a container to be executed on reboot
#[derive(StructOpt, Debug)]
pub struct Kexec {
    /// Don't display the progress bar
    #[structopt(short, long)]
    pub quiet: bool,

    /// The container image (format: [source]name[:tag|@digest])
    pub image: String,

    /// The kernel command line to use after reboot
    #[structopt(long, short)]
    pub cmdline: Option<String>,

    /// Number of retries for network failures.
    #[structopt(short, long, default_value = "5")]
    pub tries: u32,
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
            std::arch::asm!(
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

    fn write_fmt(&self, args: Arguments<'_>) -> Result<(), std::io::Error> {
        if !self.quiet {
            eprintln!("● {}", args);
        }

        Ok(())
    }

    fn run(&self, kernel: &Path, initrd: &Path) -> anyhow::Result<()> {
        write!(self, "Getting: {}", self.image)?;

        // Download and extract the specified container image.
        let mut extra = Vec::new();
        for tries in 0.. {
            extra.truncate(0);

            let extract = Extract {
                kernel: LookAside::kernel(File::create(kernel)?),
                initrd: File::create(initrd)?,
                cmdline: LookAside::cmdline(&mut extra),
                progress: true,
                image: self.image.clone(),
            };

            match extract.execute() {
                Err(e) if tries < self.tries => write!(self, "Failure: {}", e)?,
                Err(e) => return Err(e),
                Ok(()) => break,
            }

            std::thread::sleep(Duration::from_secs(2u64.pow(tries)));
        }
        let extra = String::from_utf8(extra)?;
        let extra = extra.trim();

        // Merge the extra arguments with the specified arguments.
        let all = format!(r#"{} {}"#, extra, self.cmdline.as_deref().unwrap_or(""));
        let all = all.trim();

        // Do the kexec.
        write!(self, "Loading: {} ({})", self.image, all)?;
        let all = CString::new(all)?;
        Self::kexec(File::open(kernel)?, File::open(initrd)?, &all)?;

        // Wait for the kernel to tell us it is ready.
        while std::fs::read("/sys/kernel/kexec_loaded")? != [b'1', b'\n'] {
            std::thread::sleep(Duration::from_millis(100));
        }

        Ok(())
    }
}

impl Command for Kexec {
    fn execute(self) -> anyhow::Result<()> {
        let kernel = PathBuf::from(format!("/tmp/wyrcan.{}.kernel", std::process::id()));
        let initrd = PathBuf::from(format!("/tmp/wyrcan.{}.initrd", std::process::id()));

        let result = self.run(&kernel, &initrd);

        if kernel.exists() {
            std::fs::remove_file(kernel)?;
        }

        if initrd.exists() {
            std::fs::remove_file(initrd)?;
        }

        result
    }
}
