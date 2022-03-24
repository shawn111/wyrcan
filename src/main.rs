// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

#![warn(clippy::all)]
#![allow(clippy::useless_conversion)]

mod api;
mod cmdline;
mod commands;
mod efi;
mod formats;
mod iotools;

use clap::Parser;
use commands::Command;

fn main() -> anyhow::Result<()> {
    commands::Main::parse().execute()
}
