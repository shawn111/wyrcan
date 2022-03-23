// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

#![cfg(target_os = "linux")]

use std::fs::File;
use std::io::Write as _;
use std::path::PathBuf;

use anyhow::Result;
use iocuddle::{Group, Ioctl, Read, Write};

const FILE: Group = Group::new(b'f');
const FS_IOC_GETFLAGS: Ioctl<Read, &libc::c_long> = unsafe { FILE.read(1) };
const FS_IOC_SETFLAGS: Ioctl<Write, &libc::c_long> = unsafe { FILE.write(2) };
const FS_IMMUTABLE_FL: libc::c_long = 0x00000010;

pub struct Store<'a>(&'a str);

impl<'a> Store<'a> {
    const BASE: &'static str = "/sys/firmware/efi/efivars";
    const FLAG: [u8; 4] = 7u32.to_ne_bytes();

    fn path(&self, name: &str) -> PathBuf {
        PathBuf::from(format!("{}/{}-{}", Self::BASE, name, self.0))
    }

    fn mutate(&self, name: &str) -> Result<PathBuf> {
        let path = self.path(name);

        // Remove the immutability flag.
        if path.exists() {
            let mut file = File::open(&path)?;
            let (.., mut flags) = FS_IOC_GETFLAGS.ioctl(&file)?;
            flags &= !FS_IMMUTABLE_FL;
            FS_IOC_SETFLAGS.ioctl(&mut file, &flags)?;
        }

        Ok(path)
    }

    pub fn new(uuid: &'a str) -> Self {
        Self(uuid)
    }

    pub fn exists(&self, name: &str) -> bool {
        self.path(name).exists()
    }

    pub fn read(&self, name: &str) -> Result<Vec<u8>> {
        let mut bytes = std::fs::read(self.path(name))?;
        bytes.drain(0..4);
        Ok(bytes)
    }

    pub fn write(&self, name: &str, value: impl AsRef<[u8]>) -> Result<()> {
        let mut data = Vec::new();
        data.write_all(&Self::FLAG)?;
        data.write_all(value.as_ref())?;

        Ok(std::fs::write(self.mutate(name)?, data)?)
    }

    pub fn clear(&self, name: &str) -> Result<()> {
        Ok(std::fs::remove_file(self.mutate(name)?)?)
    }
}
