// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

use std::fs::File;
use std::io::{Error, Write as _};
use std::path::PathBuf;

use super::extract::{Extract, LookAside};
use super::kexec::Kexec;
use super::Command;

use iocuddle::{Group, Ioctl, Read, Write};
use structopt::StructOpt;

const FILE: Group = Group::new(b'f');
const FS_IOC_GETFLAGS: Ioctl<Read, &libc::c_long> = unsafe { FILE.read(1) };
const FS_IOC_SETFLAGS: Ioctl<Write, &libc::c_long> = unsafe { FILE.write(2) };
const FS_IMMUTABLE_FL: libc::c_long = 0x00000010;

fn reboot() -> anyhow::Result<()> {
    unsafe { libc::sync() };
    let ret = unsafe { libc::reboot(libc::LINUX_REBOOT_CMD_RESTART) };
    if ret < 0 {
        return Err(Error::last_os_error().into());
    }

    Ok(())
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
    const NAME: &'static str = "CmdLine";
    const BASE: &'static str = "/sys/firmware/efi/efivars";
    const FLAG: [u8; 4] = 7u32.to_ne_bytes();

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

You can use the following kernel cmdline arguments to control the behavior of
Wyrcan. Except for wyrcan.img=IMG, all cmdline arguments are optional.

  * wyrcan.img=IMG - Specifies which container will be booted. IMG should be
    a container name in the usual format. For example:

      wyrcan.img=registry.gitlab.com/wyrcan/debian:latest

  * wyrcan.efi=write - Saves the cmdline arguments to EFI NVRAM. This enables
    persistent cmdline arguments for automated boot.

  * wyrcan.efi=clear - Removes all cmdline arguments from EFI NVRAM. This
    enables persistent cmdline arguments for automated boot.

  * wyrcan.pass=ARG - Passes a cmdline argument to the container's kernel. The
    argument will be ignored by the Wyrcan kernel. This allows one to specify
    an argument to the inner kernel only. For example, under this config, the
    "quiet" argument will be active for the inner kernel only:

      wyrcan.pass=quiet

  * wyrcan.skip=ARG - Prevents a cmdline argument from being passed to the
    container's kernel. This allows one to specify an argument to the outer
    kernel only. For example, under this configuration, the "quiet" argument
    will be active for the outer kernel only:

      quiet wyrcan.skip=quiet

Press enter or return to reboot.
"###;
}

impl Command for Boot {
    fn execute(self) -> anyhow::Result<()> {
        let var = PathBuf::from(format!("{}/{}-{}", Self::BASE, Self::NAME, Self::UUID));
        let bcl = std::fs::read_to_string("/proc/cmdline")?;

        // Parse the boot cmdline arguments
        let mut cmdline = Vec::<String>::new();
        let mut skip = Vec::<String>::new();
        let mut img: Option<String> = None;
        let mut efi: Option<Efi> = None;
        for arg in bcl.split_whitespace() {
            match arg.find('=').map(|i| arg.split_at(i)) {
                Some(("wyrcan.efi", "=write")) => efi = Some(Efi::Write),
                Some(("wyrcan.efi", "=clear")) => efi = Some(Efi::Clear),
                Some(("wyrcan.pass", v)) => cmdline.push(v[1..].into()),
                Some(("wyrcan.skip", v)) => skip.push(v[1..].into()),
                Some(("wyrcan.img", v)) => img = Some(v[1..].into()),
                Some(("initrd", ..)) => continue,
                _ if arg.starts_with("wyrcan.") => continue,
                _ => cmdline.push(arg.into()),
            }
        }
        cmdline = cmdline.into_iter().filter(|x| !skip.contains(x)).collect();

        // If the cmdline says to clear EFI, do it...
        if let Some(Efi::Clear) = efi {
            if var.exists() {
                println!("{}", Self::WARNING);
                let mut answer = String::new();
                std::io::stdin().read_line(&mut answer)?;

                if answer.trim() == "yes" {
                    // Remove the immutability flag.
                    let mut file = File::open(&var)?;
                    let (.., mut flags) = FS_IOC_GETFLAGS.ioctl(&file)?;
                    flags &= !FS_IMMUTABLE_FL;
                    FS_IOC_SETFLAGS.ioctl(&mut file, &flags)?;

                    // Remove the file.
                    std::fs::remove_file(&var)?;
                }
            }

            return reboot();
        }

        // If no boot image was specified, look in EFI.
        if img.is_none() && var.exists() {
            println!("Reading: EFI");
            let bytes = std::fs::read(&var)?;
            let ecl = std::str::from_utf8(&bytes[4..])?; // Skip the prefix
            println!("Scanned: {}", ecl.trim());
            for arg in ecl.split_whitespace() {
                let arg = arg.to_string();
                match arg.find('=').map(|i| arg.split_at(i)) {
                    Some(("wyrcan.img", v)) => img = Some(v[1..].into()),
                    _ if !cmdline.contains(&arg) => cmdline.push(arg),
                    _ => (),
                }
            }
        }

        // If we still have no image, give the user some documentation.
        let img = match img {
            Some(img) => img,
            None => {
                println!("{}", Self::NOIMG);
                let mut answer = String::new();
                std::io::stdin().read_line(&mut answer)?;

                return reboot();
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
            println!("{}", Self::WARNING);
            let mut answer = String::new();
            std::io::stdin().read_line(&mut answer)?;

            if answer.trim() == "yes" {
                // Prepare the args
                let mut args = cmdline.clone();
                args.push(format!("wyrcan.img={}", img));
                let args = args.join(" ");

                // Prepare the output
                let mut data = Vec::new();
                data.write_all(&Self::FLAG)?;
                data.write_all(args.as_bytes())?;

                // Write out the efi variable
                println!("Writing: {}", args);
                std::fs::write(&var, &data)?;
            }

            std::thread::sleep(std::time::Duration::from_millis(3000));
            return reboot();
        }

        // Merge the extra arguments with the specified arguments.
        for (i, arg) in extra.split_whitespace().enumerate() {
            let arg = arg.to_string();
            if !skip.contains(&arg) {
                cmdline.insert(i, arg);
            }
        }
        let args = cmdline.join(" ");

        println!("Booting: {} ({})", img, &args);
        Kexec {
            kernel: PathBuf::from("/tmp/kernel"),
            initrd: PathBuf::from("/tmp/initrd"),
            cmdline: args,
            reboot: true,
        }
        .execute()?;

        Ok(())
    }
}
