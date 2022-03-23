// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

use super::config::Config;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use structopt::StructOpt;

/// Generate a network configuration from the kernel cmdline
#[derive(StructOpt, Debug)]
pub struct Net {}

impl Net {
    const OUTDIR: &'static str = "/etc/systemd/network";
}

impl super::Command for Net {
    fn execute(self) -> anyhow::Result<()> {
        let mut cfg = Config::scan();

        // Specify a default config.
        if cfg.network.is_empty() {
            let defaults = [
                ("autoconf.network", "Match", "Type", "ether"),
                ("autoconf.network", "Network", "DHCP", "yes"),
                ("autoconf.network", "Network", "IPv6AcceptRA", "yes"),
            ];

            for (file, sect, name, data) in defaults {
                let f = cfg.network.entry(file.into()).or_insert_with(HashMap::new);
                let s = f.entry(sect.into()).or_insert_with(HashMap::new);
                s.insert(name.into(), data.into());
            }
        }

        // Write out network configuration files.
        for (file, sections) in cfg.network {
            let mut f = BufWriter::new(File::create(Path::new(Self::OUTDIR).join(file))?);

            for (sect, entries) in sections {
                writeln!(f, "[{}]", sect)?;

                for (name, data) in entries {
                    writeln!(f, "{}={}", name, data)?;
                }

                writeln!(f)?;
            }

            f.flush()?;
        }

        Ok(())
    }
}
