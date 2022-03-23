// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

#![cfg(target_os = "linux")]

use std::{io::ErrorKind, path::Path, str::from_utf8};

pub struct CmdLine<T: AsRef<str>>(T);

impl<T: AsRef<str>> CmdLine<T> {
    const PATH: &'static str = "/proc/cmdline";

    pub fn new(cmdline: T) -> std::io::Result<Self> {
        if cmdline.as_ref().is_ascii() {
            return Ok(Self(cmdline));
        }

        Err(ErrorKind::InvalidData.into())
    }

    pub fn args(&self) -> Args {
        Args(self.0.as_ref().as_bytes())
    }
}

impl CmdLine<String> {
    pub fn load(path: impl AsRef<Path>) -> std::io::Result<Self> {
        Self::new(std::fs::read_to_string(path)?)
    }

    pub fn scan() -> Self {
        Self::load(Self::PATH).unwrap_or_else(|_| Self(String::new()))
    }
}

pub struct Args<'a>(&'a [u8]);

impl<'a> Iterator for Args<'a> {
    type Item = (Option<&'a str>, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        while !self.0.is_empty() && self.0[0].is_ascii_whitespace() {
            self.0 = &self.0[1..];
        }

        if self.0.is_empty() {
            return None;
        }

        let mut quoted = false;
        let mut equals = 0;
        let mut end = 0;

        while end < self.0.len() && (!self.0[end].is_ascii_whitespace() || quoted) {
            match self.0[end] {
                b'"' => quoted = !quoted,
                b'=' if equals == 0 => equals = end,
                _ => (),
            }

            end += 1;
        }

        let (lhs, rhs) = self.0.split_at(end);
        self.0 = rhs;

        let (mut lhs, mut rhs) = lhs.split_at(equals);

        if lhs.starts_with(b"\"") {
            lhs = &lhs[1..];

            if rhs.ends_with(b"\"") {
                rhs = &rhs[..rhs.len() - 1];
            }
        }

        if lhs.is_empty() {
            Some((None, from_utf8(rhs).unwrap()))
        } else {
            rhs = &rhs[1..];

            if rhs.starts_with(b"\"") {
                rhs = &rhs[1..];
                if rhs.ends_with(b"\"") {
                    rhs = &rhs[..rhs.len() - 1];
                }
            }

            Some((Some(from_utf8(lhs).unwrap()), from_utf8(rhs).unwrap()))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn empty() {
        let cmdline = CmdLine::new("").unwrap();
        assert_eq!(cmdline.args().next(), None);
    }

    #[test]
    fn noquotes() {
        let cmdline = CmdLine::new(" \t foo=bar bat\tbaz=qux quz\t").unwrap();
        let mut args = cmdline.args();
        assert_eq!(args.next(), Some((Some("foo"), "bar")));
        assert_eq!(args.next(), Some((None, "bat")));
        assert_eq!(args.next(), Some((Some("baz"), "qux")));
        assert_eq!(args.next(), Some((None, "quz")));
        assert_eq!(args.next(), None);
    }

    #[test]
    fn quotes() {
        let cmdline = CmdLine::new("\t  foo=\"bar bat\" \"baz=qux\tquz\"  \t").unwrap();
        let mut args = cmdline.args();
        assert_eq!(args.next(), Some((Some("foo"), "bar bat")));
        assert_eq!(args.next(), Some((Some("baz"), "qux\tquz")));
        assert_eq!(args.next(), None);
    }
}
