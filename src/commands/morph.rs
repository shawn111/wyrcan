// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

use super::extract::{Extract, LookAside};
use super::Command;
use crate::iotools::Either;

use std::fs::File;
use std::io::{sink, Sink};
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

/// Morphs a container into the files necessary for boot
#[derive(Parser, Debug)]
pub struct Morph {
    /// The path to store the kernel
    #[clap(short, long)]
    kernel: Option<PathBuf>,

    /// The path to store the initrd
    #[clap(short, long)]
    initrd: Option<PathBuf>,

    /// The path to store the cmdline
    #[clap(short, long)]
    cmdline: Option<PathBuf>,

    /// Don't display the progress bar
    #[clap(short, long)]
    quiet: bool,

    /// The container image (format: [source]name[:tag|@digest])
    image: String,
}

impl Command for Morph {
    fn execute(self) -> anyhow::Result<()> {
        fn create(value: Option<&PathBuf>) -> Result<Either<File, Sink>> {
            Ok(if let Some(path) = value {
                Either::One(File::create(path)?)
            } else {
                Either::Two(sink())
            })
        }

        let extract = Extract {
            kernel: LookAside::kernel(create(self.kernel.as_ref())?),
            initrd: create(self.initrd.as_ref())?,
            cmdline: LookAside::cmdline(create(self.cmdline.as_ref())?),
            image: self.image,
            progress: !self.quiet,
        };

        let result = extract.execute();

        if result.is_err() {
            if let Some(path) = self.kernel {
                std::fs::remove_file(path).unwrap();
            }
            if let Some(path) = self.initrd {
                std::fs::remove_file(path).unwrap();
            }
            if let Some(path) = self.cmdline {
                std::fs::remove_file(path).unwrap();
            }
        }

        result
    }
}
