// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

use std::fs::File;
use std::io::Error;
use std::path::PathBuf;

use super::extract::{Extract, LookAside};
use super::kexec::Kexec;
use super::Command;
use crate::cmdline::CmdLine;

use anyhow::Result;
use structopt::StructOpt;

#[derive(Copy, Clone, Debug)]
enum Efi {
    Write,
    Clear,
}

#[derive(StructOpt, Debug)]
pub struct Boot {}

impl Boot {
    const UUID: &'static str = "6987e713-a5ff-4ec2-ad55-c1fca471ed2d";

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
        let nvr = crate::efi::Store::new(Self::UUID);

        let bcl = std::fs::read_to_string("/proc/cmdline")?;
        let bcl = match CmdLine::new(&bcl) {
            Some(cmdline) => cmdline,
            None => {
                eprintln!("error: kernel cmdline is not ascii");
                return Self::reboot();
            }
        };

        // Parse the boot cmdline arguments
        let mut arg = Vec::new();
        let mut img = None;
        let mut efi = None;
        for (k, v) in bcl {
            match (k, v) {
                (Some("wyrcan.efi"), "write") => efi = Some(Efi::Write),
                (Some("wyrcan.efi"), "clear") => efi = Some(Efi::Clear),
                (Some("wyrcan.img"), v) => img = Some(v.to_string()),
                (Some("wyrcan.arg"), v) => arg.push(v.to_string()),
                _ => (),
            }
        }

        // If the cmdline says to clear EFI, do it...
        if let Some(Efi::Clear) = efi {
            if Self::prompt(Self::WARNING)?.trim() == "yes" {
                if nvr.exists("CmdLine") {
                    nvr.clear("CmdLine")?;
                }

                if nvr.exists("Image") {
                    nvr.clear("Image")?;
                }
            }

            return Self::reboot();
        }

        // If no boot image was specified, look in EFI.
        if img.is_none() && nvr.exists("Image") {
            let image = nvr.read("Image")?;
            println!("Scanned: {}", image);
            img = Some(image);
        }

        // If no arguments were specified, look in EFI.
        if arg.is_empty() && nvr.exists("CmdLine") {
            let cmdline = nvr.read("CmdLine")?;
            println!("Scanned: {}", cmdline);
            arg.push(cmdline);
        }

        // If we still have no image, give the user some documentation.
        let img = match img {
            Some(img) => img,
            None => {
                println!("{}", Self::NOIMG);
                return Self::reboot();
            }
        };

        // Download and extract the specified container image.
        println!("Loading: {}", &img);
        let mut extra = Vec::new();
        Extract {
            kernel: LookAside::kernel(File::create("/tmp/kernel")?),
            initrd: File::create("/tmp/initrd")?,
            cmdline: LookAside::cmdline(&mut extra),
            progress: true,
            name: img.clone(),
        }
        .execute()?;
        let extra = String::from_utf8(extra)?;
        let extra = extra.trim();

        // If specified, save the command line to EFI.
        if let Some(Efi::Write) = efi {
            if Self::prompt(Self::WARNING)?.trim() == "yes" {
                let args = arg.join(" ");
                println!("Writing: {} ({})", &img, &args);
                nvr.write("CmdLine", &args)?;
                nvr.write("Image", &img)?;
            }

            return Self::reboot();
        }

        // Merge the extra arguments with the specified arguments.
        if !extra.is_empty() {
            arg.insert(0, extra.to_string());
        }
        let all = arg.join(" ");

        println!("Booting: {} ({})", img, &all);
        Kexec {
            kernel: PathBuf::from("/tmp/kernel"),
            initrd: PathBuf::from("/tmp/initrd"),
            cmdline: all,
            reboot: true,
        }
        .execute()?;

        Ok(())
    }
}
