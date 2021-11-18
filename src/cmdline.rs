// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2021 Profian, Inc.

use std::str::from_utf8;

pub struct CmdLine<'a>(&'a [u8]);

impl<'a> CmdLine<'a> {
    pub fn new(value: &'a str) -> Option<Self> {
        if value.is_ascii() {
            Some(Self(value.as_bytes()))
        } else {
            None
        }
    }
}

impl<'a> Iterator for CmdLine<'a> {
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
    use super::CmdLine;

    #[test]
    fn empty() {
        let cmdline = CmdLine::new("").unwrap();
        assert_eq!(cmdline.collect::<Vec<_>>(), []);
    }

    #[test]
    fn noquotes() {
        let cmdline = CmdLine::new(" \t foo=bar bat\tbaz=qux quz\t").unwrap();
        assert_eq!(
            cmdline.collect::<Vec<_>>(),
            [
                (Some("foo"), "bar"),
                (None, "bat"),
                (Some("baz"), "qux"),
                (None, "quz")
            ]
        );
    }

    #[test]
    fn quotes() {
        let cmdline = CmdLine::new("\t  foo=\"bar bat\" \"baz=qux\tquz\"  \t").unwrap();
        assert_eq!(
            cmdline.collect::<Vec<_>>(),
            [(Some("foo"), "bar bat"), (Some("baz"), "qux\tquz"),]
        );
    }
}
