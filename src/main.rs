// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

#![feature(asm, if_let_guard)]
#![warn(clippy::all)]

mod api;
mod cmdline;
mod commands;
mod efi;
mod formats;
mod iotools;

use commands::Command;
use structopt::StructOpt;

fn main() -> anyhow::Result<()> {
    commands::Main::from_args().execute()
}
