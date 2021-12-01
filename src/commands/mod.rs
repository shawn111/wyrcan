// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

mod boot;
mod convert;
mod extract;
mod kexec;
mod net;
mod tags;
mod unpack;
mod unpacker;

use crate::cmdline::{Args, CmdLine};
use crate::efi::Store;

use std::collections::HashMap;

use regex::Regex;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    /// Network files to write into /etc/systemd/network/
    pub network: HashMap<String, HashMap<String, HashMap<String, String>>>,

    /// Post-kexec cmdline arguments
    pub cmdline: Vec<String>,

    /// The container image to boot
    pub image: String,
}

impl<'a> From<Args<'a>> for Option<Config> {
    fn from(args: Args<'a>) -> Self {
        let re = Regex::new(Config::RE).unwrap();

        let mut net = HashMap::new();
        let mut img = None;
        let mut arg = Vec::new();
        for (k, v) in args {
            match k {
                Some("wyrcan.img") | Some("wyr.img") => img = Some(v.into()),
                Some("wyrcan.arg") | Some("wyr.arg") => arg.push(v.into()),
                Some(k) if re.is_match(k) => {
                    let cap = re.captures(k).unwrap();

                    let file = format!("{}.network", &cap[2]);
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

        Some(Config {
            network: net,
            cmdline: arg,
            image: img?,
        })
    }
}

impl Config {
    const UUID: &'static str = "6987e713-a5ff-4ec2-ad55-c1fca471ed2d";
    const RE: &'static str = concat!(
        "^(wyrcan|wyr)\\.net\\.",
        "([a-zA-Z0-9]+)\\.([a-zA-Z0-9]+)\\.([a-zA-Z0-9]+)$"
    );

    pub fn scan() -> Option<Self> {
        // Check the kernel cmdline
        if let Some(cfg) = CmdLine::scan().args().into() {
            return Some(cfg);
        }

        // Check EFI NVRAM
        let nvr = Store::new(Self::UUID);
        if let Ok(val) = nvr.read("Wyrcan") {
            if let Ok(cfg) = serde_json::from_slice(&val) {
                return Some(cfg);
            }
        }

        None
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
    #[structopt(setting(structopt::clap::AppSettings::Hidden))]
    Net(net::Net),
    #[structopt(setting(structopt::clap::AppSettings::Hidden))]
    Boot(boot::Boot),
    Tags(tags::Tags),
    Kexec(kexec::Kexec),
    Unpack(unpack::Unpack),
    Convert(convert::Convert),
}

impl Command for Main {
    fn execute(self) -> anyhow::Result<()> {
        match self {
            Self::Net(cmd) => cmd.execute(),
            Self::Boot(cmd) => cmd.execute(),
            Self::Tags(cmd) => cmd.execute(),
            Self::Kexec(cmd) => cmd.execute(),
            Self::Unpack(cmd) => cmd.execute(),
            Self::Convert(cmd) => cmd.execute(),
        }
    }
}
