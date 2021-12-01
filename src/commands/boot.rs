// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

use std::fs::File;
use std::io::Error;
use std::path::PathBuf;
use std::time::Duration;

use super::extract::{Extract, LookAside};
use super::kexec::Kexec;
use super::{Command, Config};

use anyhow::Result;
use indicatif::ProgressBar;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Boot {
    #[structopt(default_value = "5")]
    tries: u32,
}

impl Boot {
    const NOIMG: &'static str = r###"
No container image target (wyrcan.img=IMG) could be found!

You can use the following kernel cmdline arguments to control Wyrcan:

  * wyrcan.img=IMG - Specifies which container will be booted. IMG should be
    a container name in the usual format. For example:

      wyrcan.img=registry.gitlab.com/wyrcan/debian:latest

  * wyrcan.arg=ARG - Passes the specified cmdline arguments to the container's
    kernel. This argument may be specified multiple times and may be quoted to
    include spaces. The arguments passed within will be ignored by the Wyrcan
    kernel. For example, the following is valid:

      wyrcan.arg="quiet log-buf-len=1M" wyrcan.arg=print-fatal-signals=1

    The container's kernel will receive the following cmdline:

      quiet log-buf-len=1M print-fatal-signals=1

  * wyrcan.efi=write - Saves the wyrcan.img and wyrcan.arg parameters to EFI
    NVRAM. This enables persistent, automated boot.

  * wyrcan.efi=clear - Removes all previously stored values from EFI NVRAM.
    This disables persistent, automated boot.
"###;

    const REBOOT: &'static str = "Press enter or return to reboot.";

    fn prompt(message: &str) -> Result<String> {
        println!("{}", message);
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        Ok(answer)
    }

    fn reboot() -> Result<()> {
        Self::prompt(Self::REBOOT)?;

        unsafe { libc::sync() };
        let ret = unsafe { libc::reboot(libc::LINUX_REBOOT_CMD_RESTART) };
        if ret < 0 {
            return Err(Error::last_os_error().into());
        }

        Ok(())
    }
}

impl Command for Boot {
    fn execute(self) -> Result<()> {
        let cfg = Config::scan();

        // If we have no image, give the user some documentation.
        let img = match &cfg.image {
            Some(img) => img,
            None => {
                println!("{}", Self::NOIMG);
                return Self::reboot();
            }
        };

        // Download and extract the specified container image.
        eprintln!("* Getting: {}", img);
        let mut extra = Vec::new();
        for tries in 0.. {
            extra.truncate(0);

            let extract = Extract {
                kernel: LookAside::kernel(File::create("/tmp/kernel")?),
                initrd: File::create("/tmp/initrd")?,
                cmdline: LookAside::cmdline(&mut extra),
                progress: true,
                name: img.clone(),
            };

            match extract.execute() {
                Err(e) if tries < self.tries => eprintln!("* Failure: {}", e),
                Err(e) => return Err(e),
                Ok(()) => break,
            }

            std::thread::sleep(Duration::from_secs(2u64.pow(tries)));
        }
        let extra = String::from_utf8(extra)?;
        let extra = extra.trim();

        // Merge the extra arguments with the specified arguments.
        let all = format!(r#"{} "{}""#, extra, cfg.cmdline.join(r#"" ""#));
        let all = all.trim();

        {
            // Set up the spinner
            let pb = ProgressBar::new_spinner();
            pb.set_message(format!("Loading: {} ({})", img, all));
            pb.enable_steady_tick(100);

            // Load the kernel and initrd.
            Kexec {
                kernel: PathBuf::from("/tmp/kernel"),
                initrd: PathBuf::from("/tmp/initrd"),
                cmdline: all.to_string(),
            }
            .execute()?;
        }

        // Remove files and exit.
        eprintln!("* Booting: {} ({})", img, all);
        std::fs::remove_file("/tmp/kernel")?;
        std::fs::remove_file("/tmp/initrd")?;
        Ok(())
    }
}
