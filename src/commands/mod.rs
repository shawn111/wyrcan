// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

mod boot;
mod config;
mod efi;
mod extract;
mod kexec;
mod morph;
mod net;
mod unpack;
mod unpacker;

use structopt::StructOpt;

pub trait Command {
    fn execute(self) -> anyhow::Result<()>;
}

#[derive(StructOpt, Debug)]
#[structopt(about = "The Container Bootloader")]
pub enum Main {
    #[cfg(target_os = "linux")]
    #[structopt(setting(structopt::clap::AppSettings::Hidden))]
    Efi(efi::Efi),

    #[cfg(target_os = "linux")]
    #[structopt(setting(structopt::clap::AppSettings::Hidden))]
    Net(net::Net),

    #[cfg(target_os = "linux")]
    #[structopt(setting(structopt::clap::AppSettings::Hidden))]
    Boot(boot::Boot),

    #[cfg(target_os = "linux")]
    Kexec(kexec::Kexec),

    Morph(morph::Morph),

    Unpack(unpack::Unpack),
}

impl Command for Main {
    fn execute(self) -> anyhow::Result<()> {
        match self {
            #[cfg(target_os = "linux")]
            Self::Efi(cmd) => cmd.execute(),

            #[cfg(target_os = "linux")]
            Self::Net(cmd) => cmd.execute(),

            #[cfg(target_os = "linux")]
            Self::Boot(cmd) => cmd.execute(),

            #[cfg(target_os = "linux")]
            Self::Kexec(cmd) => cmd.execute(),

            Self::Unpack(cmd) => cmd.execute(),

            Self::Morph(cmd) => cmd.execute(),
        }
    }
}
