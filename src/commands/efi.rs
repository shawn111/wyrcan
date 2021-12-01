// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

use super::{Command, Config};
use crate::cmdline::CmdLine;

use structopt::StructOpt;

#[derive(Copy, Clone, Debug)]
enum Arg {
    Write,
    Clear,
}

impl Arg {
    pub fn scan() -> Option<Self> {
        let mut efi = None;

        for (k, v) in CmdLine::scan().args() {
            match k {
                Some("wyrcan.efi") | Some("wyr.efi") => match v {
                    "write" => efi = Some(Arg::Write),
                    "clear" => efi = Some(Arg::Clear),
                    _ => continue,
                },
                _ => continue,
            }
        }

        efi
    }
}

/// Load a kernel to be executed on reboot
#[derive(StructOpt, Debug)]
pub struct Efi {}

impl Efi {
    const WARNING: &'static str = r###"
⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠ WARNING ⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠

On some buggy hardware, modifying an EFI variable can cause the hardware to
become unresponsive. Proceeding with this action could cause irreversible
damage to your hardware. The developers of Wyrcan are not liable for any
hardware defects triggered by this action.

⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠ WARNING ⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠⚠

Would you like to proceed? [yes/no]
"###;

    fn prompt() -> std::io::Result<bool> {
        println!("{}", Self::WARNING);
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer)?;
        Ok(answer.trim() == "yes")
    }
}

impl Command for Efi {
    fn execute(self) -> anyhow::Result<()> {
        match Arg::scan() {
            Some(Arg::Write) => {
                let cfg = Config::scan();

                if cfg.image.is_some() && Self::prompt()? {
                    cfg.save()?;
                }

                Ok(())
            }

            Some(Arg::Clear) => {
                if Self::prompt()? {
                    Config::wipe()?;
                }

                unsafe { libc::sync() };
                let ret = unsafe { libc::reboot(libc::LINUX_REBOOT_CMD_RESTART) };
                if ret < 0 {
                    return Err(std::io::Error::last_os_error().into());
                }

                Ok(())
            }

            None => Ok(()),
        }
    }
}
