// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

#![cfg(target_os = "linux")]

use std::io::Error;

use super::kexec::Kexec;
use super::{config::Config, Command};

use anyhow::Result;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Boot {
    /// Don't display the progress bar
    #[structopt(short, long)]
    quiet: bool,

    /// Number of retries for network failures.
    #[structopt(short, long, default_value = "5")]
    tries: u32,
}

impl Boot {
    const NOIMG: &'static str = r###"
No container image target (wyr.img=IMG) could be found!

You can use the following kernel cmdline arguments to control Wyrcan:

  * wyr.img=IMG - Specifies which container will be booted. IMG should be a
    container name in the usual format. For example:

      wyr.img=registry.gitlab.com/wyrcan/debian:latest

  * wyr.arg=ARG - Passes the specified cmdline arguments to the container's
    kernel. This argument may be specified multiple times and may be quoted to
    include spaces. The arguments passed within will be ignored by the Wyrcan
    kernel. For example, the following is valid:

      wyr.arg="quiet log-buf-len=1M" wyr.arg=print-fatal-signals=1

    The container's kernel will receive the following cmdline:

      quiet log-buf-len=1M print-fatal-signals=1

  * wyr.net.[KIND.]FILE.SECTION.KEY=VAL - Allows you to specify custom
    networking parameters. All arguments of this type are grouped into files.
    Then a systemd.KIND file is created with the specified contents. If KIND
    is not specified, it defaults to "network".  For example, the previously
    outlined config would produce /etc/systemd/network/FILE.KIND with the
    following contents:

      [SECTION]
      KEY=VAL

    See the systemd-networkd documentation for the full range of configuration
    possibilities.

  * wyr.efi=write - Saves the wyr.img and wyr.arg parameters to EFI NVRAM.
    This enables persistent, automated boot.

  * wyr.efi=clear - Removes all previously stored values from EFI NVRAM. This
    disables persistent, automated boot.
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

        // Prepare the cmdline.
        let cmdline = if cfg.cmdline.is_empty() {
            None
        } else {
            Some(format!(r#""{}""#, cfg.cmdline.join(r#"" ""#)))
        };

        // Do the kexec.
        let kexec = Kexec {
            quiet: self.quiet,
            tries: self.tries,
            image: img.clone(),
            cmdline,
        };

        kexec.execute()
    }
}
