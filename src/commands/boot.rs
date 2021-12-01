// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

use std::fs::File;
use std::io::Error;
use std::path::PathBuf;
use std::time::Duration;

use super::extract::{Extract, LookAside};
use super::kexec::Kexec;
use super::{Command, Config};
use crate::cmdline::CmdLine;

use anyhow::Result;
use indicatif::ProgressBar;
use structopt::StructOpt;

const MAX_TRIES: u32 = 5;

#[derive(Copy, Clone, Debug)]
enum Efi {
    Write,
    Clear,
}

impl Efi {
    pub fn scan() -> Option<Self> {
        let mut efi = None;

        for (k, v) in CmdLine::scan().args() {
            match k {
                Some("wyrcan.efi") | Some("wyr.efi") => match v {
                    "write" => efi = Some(Efi::Write),
                    "clear" => efi = Some(Efi::Clear),
                    _ => continue,
                },
                _ => continue,
            }
        }

        efi
    }
}

#[derive(StructOpt, Debug)]
pub struct Boot {}

impl Boot {
    const WARNING: &'static str = r###"
⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠ WARNING ⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠

On some buggy hardware, modifying an EFI variable can cause the hardware to
become unresponsive. Proceeding with this action could cause irreversible
damage to your hardware. The developers of Wyrcan are not liable for any
hardware defects triggered by this action.

⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠ WARNING ⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠

Would you like to proceed? [yes/no]
"###;

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
        let efi = Efi::scan();

        // If the cmdline says to clear EFI, do it...
        if let Some(Efi::Clear) = efi {
            if Self::prompt(Self::WARNING)?.trim() == "yes" {
                Config::wipe()?;
            }

            return Self::reboot();
        }

        // If we have no config, give the user some documentation.
        let cfg = match cfg {
            Some(cfg) => cfg,
            None => {
                println!("{}", Self::NOIMG);
                return Self::reboot();
            }
        };

        // Download and extract the specified container image.
        eprintln!("* Getting: {}", &cfg.image);
        let mut extra = Vec::new();
        for tries in 0.. {
            extra.truncate(0);

            let extract = Extract {
                kernel: LookAside::kernel(File::create("/tmp/kernel")?),
                initrd: File::create("/tmp/initrd")?,
                cmdline: LookAside::cmdline(&mut extra),
                progress: true,
                name: cfg.image.clone(),
            };

            match extract.execute() {
                Err(e) if tries < MAX_TRIES => eprintln!("* Failure: {}", e),
                Err(e) => return Err(e),
                Ok(()) => break,
            }

            std::thread::sleep(Duration::from_secs(2u64.pow(tries)));
        }
        let extra = String::from_utf8(extra)?;
        let extra = extra.trim();

        // If specified, save the command line to EFI.
        if let Some(Efi::Write) = efi {
            if Self::prompt(Self::WARNING)?.trim() == "yes" {
                let args = cfg.cmdline.join(" ");
                eprintln!("* Writing: {} ({})", cfg.image, args);
                cfg.save()?;
            }

            return Self::reboot();
        }

        // Merge the extra arguments with the specified arguments.
        let all = format!(r#"{} "{}""#, extra, cfg.cmdline.join(r#"" ""#));
        let all = all.trim();

        {
            // Set up the spinner
            let pb = ProgressBar::new_spinner();
            pb.set_message(format!("Loading: {} ({})", cfg.image, all));
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
        eprintln!("* Booting: {} ({})", cfg.image, all);
        std::fs::remove_file("/tmp/kernel")?;
        std::fs::remove_file("/tmp/initrd")?;
        Ok(())
    }
}
