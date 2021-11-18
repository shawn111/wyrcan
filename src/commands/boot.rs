// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

use std::fs::File;
use std::io::{Error, Write as _};
use std::path::PathBuf;
use std::str::from_utf8;

use super::extract::{Extract, LookAside};
use super::kexec::Kexec;
use super::Command;

use anyhow::Result;
use iocuddle::{Group, Ioctl, Read, Write};
use structopt::StructOpt;

const FILE: Group = Group::new(b'f');
const FS_IOC_GETFLAGS: Ioctl<Read, &libc::c_long> = unsafe { FILE.read(1) };
const FS_IOC_SETFLAGS: Ioctl<Write, &libc::c_long> = unsafe { FILE.write(2) };
const FS_IMMUTABLE_FL: libc::c_long = 0x00000010;

struct CmdLine<'a>(&'a [u8]);

impl<'a> CmdLine<'a> {
    pub fn new(value: &'a str) -> Option<Self> {
        if value.is_ascii() {
            Some(Self(value.as_bytes()))
        } else {
            None
        }
    }
}

impl<'a> Iterator for CmdLine<'a> {
    type Item = (Option<&'a str>, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        while self.0[0].is_ascii_whitespace() {
            self.0 = &self.0[1..];
        }

        if self.0.is_empty() {
            return None;
        }

        let mut quoted = false;
        let mut equals = 0;
        let mut end = 0;

        while end < self.0.len() && (!self.0[end].is_ascii_whitespace() || quoted) {
            match self.0[end] {
                b'"' => quoted = !quoted,
                b'=' if equals == 0 => equals = end,
                _ => (),
            }

            end += 1;
        }

        let (lhs, rhs) = self.0.split_at(end);
        self.0 = rhs;

        let (mut lhs, mut rhs) = lhs.split_at(equals);

        if lhs.starts_with(b"\"") {
            lhs = &lhs[1..];

            if rhs.ends_with(b"\"") {
                rhs = &rhs[..rhs.len() - 1];
            }
        }

        if lhs.is_empty() {
            Some((None, from_utf8(rhs).unwrap()))
        } else {
            rhs = &rhs[1..];

            if rhs.starts_with(b"\"") {
                rhs = &rhs[1..];
                if rhs.ends_with(b"\"") {
                    rhs = &rhs[..rhs.len() - 1];
                }
            }

            Some((Some(from_utf8(lhs).unwrap()), from_utf8(rhs).unwrap()))
        }
    }
}

struct EfiStore<'a>(&'a str);

impl<'a> EfiStore<'a> {
    const BASE: &'static str = "/sys/firmware/efi/efivars";
    const FLAG: [u8; 4] = 7u32.to_ne_bytes();

    fn path(&self, name: &str) -> PathBuf {
        PathBuf::from(format!("{}/{}-{}", Self::BASE, name, self.0))
    }

    pub fn new(uuid: &'a str) -> Self {
        Self(uuid)
    }

    pub fn exists(&self, name: &str) -> bool {
        self.path(name).exists()
    }

    pub fn read(&self, name: &str) -> Result<String> {
        let bytes = std::fs::read(self.path(name))?;
        Ok(from_utf8(&bytes[4..])?.to_string())
    }

    pub fn write(&self, name: &str, value: &str) -> Result<()> {
        let mut data = Vec::new();
        data.write_all(&Self::FLAG)?;
        data.write_all(value.as_bytes())?;

        Ok(std::fs::write(self.path(name), data)?)
    }

    pub fn clear(&self, name: &str) -> Result<()> {
        let path = self.path(name);

        // Remove the immutability flag.
        let mut file = File::open(&path)?;
        let (.., mut flags) = FS_IOC_GETFLAGS.ioctl(&file)?;
        flags &= !FS_IMMUTABLE_FL;
        FS_IOC_SETFLAGS.ioctl(&mut file, &flags)?;

        // Remove the file.
        Ok(std::fs::remove_file(path)?)
    }
}

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
    kernel. The arguments will be ignored by the Wyrcan kernel. For example,
    the "quiet" argument will be active for the inner kernel only:

      wyrcan.arg=quiet

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
        unsafe { libc::sync() };
        let ret = unsafe { libc::reboot(libc::LINUX_REBOOT_CMD_RESTART) };
        if ret < 0 {
            return Err(Error::last_os_error().into());
        }

        Ok(())
    }

    fn preboot() -> Result<()> {
        Self::prompt(Self::REBOOT)?;
        Self::reboot()
    }
}

impl Command for Boot {
    fn execute(self) -> Result<()> {
        let nvr = EfiStore::new(Self::UUID);

        let bcl = std::fs::read_to_string("/proc/cmdline")?;
        let bcl = match CmdLine::new(&bcl) {
            Some(cmdline) => cmdline,
            None => {
                eprintln!("error: kernel cmdline is not ascii");
                return Self::preboot();
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
                return Self::preboot();
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

        // If specified, save the command line to EFI.
        if let Some(Efi::Write) = efi {
            if Self::prompt(Self::WARNING)?.trim() == "yes" {
                let args = arg.join(" ");
                nvr.write("CmdLine", &args)?;
                nvr.write("Image", &img)?;
            }

            return Self::preboot();
        }

        // Merge the extra arguments with the specified arguments.
        arg.insert(0, extra);
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
