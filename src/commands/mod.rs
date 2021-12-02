// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

#[cfg(target_os = "linux")]
mod boot;
#[cfg(target_os = "linux")]
mod efi;
#[cfg(target_os = "linux")]
mod kexec;
#[cfg(target_os = "linux")]
mod net;

mod extract;
mod morph;
mod unpack;
mod unpacker;

use crate::cmdline::{Args, CmdLine};
use crate::efi::Store;

use std::collections::HashMap;

use regex::Regex;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

type IniFile = HashMap<String, HashMap<String, String>>;

#[derive(Debug, Default, Serialize, Deserialize)]
struct Config {
    /// Network files to write into /etc/systemd/network/
    pub network: HashMap<String, IniFile>,

    /// Post-kexec cmdline arguments
    pub cmdline: Vec<String>,

    /// The container image to boot
    pub image: Option<String>,
}

impl<'a> From<Args<'a>> for Config {
    fn from(args: Args<'a>) -> Self {
        let re = Regex::new(Self::RE).unwrap();

        let mut net = HashMap::new();
        let mut img = None;
        let mut arg = Vec::new();
        for (k, v) in args {
            match k {
                Some("wyrcan.img") | Some("wyr.img") => img = Some(v.into()),
                Some("wyrcan.arg") | Some("wyr.arg") => arg.push(v.into()),
                Some(k) if let Some(cap) = re.captures(k) => {
                    let kind = cap.get(1).map(|m| m.as_str()).unwrap_or("network");
                    let file = format!("{}.{}", &cap[2], kind);
                    let sect = cap[3].into();
                    let name = cap[4].into();
                    let data = v.into();

                    let f = net.entry(file).or_insert_with(HashMap::new);
                    let s = f.entry(sect).or_insert_with(HashMap::new);
                    s.insert(name, data);
                }

                _ => continue,
            }
        }

        Self {
            network: net,
            cmdline: arg,
            image: img,
        }
    }
}

impl Config {
    const UUID: &'static str = "6987e713-a5ff-4ec2-ad55-c1fca471ed2d";
    const RE: &'static str = concat!(
        "^(?:wyrcan|wyr)\\.",
        "net\\.",
        "(?:(link|netdev|network)\\.)?",
        "([a-zA-Z0-9_-]+)\\.",
        "([a-zA-Z0-9]+)\\.",
        "([a-zA-Z0-9]+)$",
    );

    pub fn scan() -> Self {
        // Check the kernel cmdline
        let cfg: Self = CmdLine::scan().args().into();
        if cfg.image.is_some() {
            return cfg;
        }

        // Check EFI NVRAM
        let nvr = Store::new(Self::UUID);
        if let Ok(val) = nvr.read("Wyrcan") {
            if let Ok(cfg) = serde_json::from_slice(&val) {
                return cfg;
            }
        }

        Self::default()
    }

    pub fn wipe() -> anyhow::Result<()> {
        let nvr = Store::new(Self::UUID);
        if nvr.exists("Wyrcan") {
            nvr.clear("Wyrcan")?;
        }

        Ok(())
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let nvr = Store::new(Self::UUID);
        let val = serde_json::to_vec(self)?;
        nvr.write("Wyrcan", val)
    }
}

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
