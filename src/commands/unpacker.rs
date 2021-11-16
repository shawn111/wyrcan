// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

use crate::api::{Image, Layer};
use crate::iotools::threaded;

use std::collections::HashSet;
use std::ffi::OsString;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::thread::spawn;

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use tar::{Archive, Entry};

pub struct Bundle<'a, T: Read> {
    unpacker: &'a Unpacker,
    archive: Archive<T>,
    level: usize,
}

impl<'a, T: Read> Bundle<'a, T> {
    pub fn entries<'b>(&'b mut self) -> Result<impl Iterator<Item = Result<Entry<'b, impl Read>>>> {
        Ok(self
            .archive
            .entries()?
            .map(|entry| {
                let entry = entry?;
                let path: PathBuf = entry.path()?.into();
                Ok((entry, path))
            })
            .filter_map(|x| {
                x.map(|(entry, path)| {
                    if self.unpacker.skip(self.level, path) {
                        return None;
                    }

                    Some(entry)
                })
                .transpose()
            }))
    }
}

pub struct Unpacker {
    progress: bool,
    already: RwLock<Vec<HashSet<PathBuf>>>,
    layers: Vec<Layer>,
}

impl Unpacker {
    pub fn new(image: &Image, progress: bool) -> Result<Self> {
        let layers = image.clone().layers()?;
        let already = RwLock::new(Vec::new());

        Ok(Self {
            progress,
            already,
            layers,
        })
    }

    pub fn bundles(&self) -> Result<Vec<Bundle<impl Read>>> {
        // Start ALL downloads in separate threads
        let threads = self
            .layers
            .iter()
            .rev()
            .cloned()
            .map(|layer| spawn(move || layer.download()))
            .collect::<Vec<_>>();

        let mut bundles = Vec::new();

        // Create the progress bar
        let progress = if self.progress {
            let tmpl = "{elapsed:>4} {eta:>4} {wide_bar} {bytes:>12} {bytes_per_sec:>12}";
            let pb = ProgressBar::new(0);
            pb.set_style(ProgressStyle::default_bar().template(tmpl));
            pb
        } else {
            ProgressBar::hidden()
        };

        // Set up the reader chain for each bundle
        let all = threads
            .into_iter()
            .zip(self.layers.iter().rev())
            .enumerate();
        for (level, (thread, layer)) in all {
            let (size, src) = thread.join().unwrap()?;
            progress.inc_length(size);

            let src = progress.wrap_read(src);
            let src = threaded::Reader::new(src);
            let src = layer.decompressor(BufReader::new(src))?;
            let src = threaded::Reader::new(src);

            bundles.push(Bundle {
                unpacker: self,
                archive: Archive::new(src),
                level,
            })
        }

        Ok(bundles)
    }

    fn seen(&self, level: usize, path: impl AsRef<Path>) -> bool {
        let layers = self.already.read().unwrap();
        for layer in &layers[..=level] {
            if layer.contains(path.as_ref()) {
                return true;
            }
        }

        false
    }

    fn skip(&self, level: usize, path: PathBuf) -> bool {
        // Add a new level the first time it is encountered
        if level == self.already.read().unwrap().len() {
            self.already.write().unwrap().push(HashSet::new());
        }

        // We already unpacked this file.
        if level > 0 && self.seen(level - 1, &path) {
            return true;
        }

        // This path or one of its parents is opaqued.
        for ancestor in path.ancestors() {
            let opaque = ancestor.join(".wh..wh..opq");
            if level > 0 && self.seen(level - 1, &opaque) {
                return true;
            }
        }

        // This file was moved or renamed.
        if let Some(filename) = path.file_name() {
            let mut mask = OsString::new();
            mask.push(".wh.");
            mask.push(filename);
            let mask = Path::new(&mask);

            let mask = match path.parent() {
                Some(parent) => parent.join(mask),
                None => mask.into(),
            };

            if level > 0 && self.seen(level - 1, &mask) {
                return true;
            }
        }

        // Mark the file as seen.
        self.already.write().unwrap()[level].insert(path.clone());

        // Skip the opqued files.
        if let Some(filename) = path.file_name() {
            if filename == ".wh..wh..opq" {
                return true;
            }
        }

        false
    }
}
