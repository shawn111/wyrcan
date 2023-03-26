// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

mod unpack;
mod unpacker;

use clap::Parser;

pub trait Command {
    fn execute(self) -> anyhow::Result<()>;
}

#[derive(Parser, Debug)]
#[clap(about = "The Container Bootloader")]
pub enum Main {
    Unpack(unpack::Unpack),
}

impl Command for Main {
    fn execute(self) -> anyhow::Result<()> {
        match self {
            Self::Unpack(cmd) => cmd.execute(),
        }
    }
}
