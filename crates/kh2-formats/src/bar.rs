//! BAR containers ("Binary ARchive"), the generic KH2 container.
//!
//! Layout per OpenKH `docs/kh2/file/type/bar.md` and `OpenKh.Kh2/Bar.cs`
//! (Apache-2.0):
//! header `"BAR\x01" | count i32 | 0 i32 | motionset type i32`, then
//! `count` x 0x10-byte entries `type u16 | duplicate i16 | name[4] | offset i32 | size i32`.
//! Offsets are relative to the start of the BAR.

use crate::reader::Reader;
use crate::{bad, Result};

pub const MAGIC: u32 = 0x0152_4142; // "BAR\x01"

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BarEntry<'a> {
    pub kind: u16,
    pub duplicate: i16,
    /// Four-character name, NULs trimmed (e.g. "atkp").
    pub name: String,
    pub data: &'a [u8],
}

#[derive(Debug)]
pub struct Bar<'a> {
    pub motionset: i32,
    pub entries: Vec<BarEntry<'a>>,
}

impl<'a> Bar<'a> {
    pub fn parse(buf: &'a [u8]) -> Result<Bar<'a>> {
        let mut r = Reader::new(buf);
        if r.u32()? != MAGIC {
            return bad("not a BAR file");
        }
        let count = r.i32()?;
        r.skip(4)?;
        let motionset = r.i32()?;
        if !(0..=0x4000).contains(&count) {
            return bad(format!("implausible BAR entry count {count}"));
        }
        let mut entries = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let kind = r.u16()?;
            let duplicate = r.i16()?;
            let name = r.cstr(4)?;
            let offset = r.i32()?;
            let size = r.i32()?;
            let data: &[u8] = if offset < 0 || size <= 0 {
                &[]
            } else {
                let (o, s) = (offset as usize, size as usize);
                match buf.get(o..o.saturating_add(s)) {
                    Some(d) => d,
                    None => return bad(format!("BAR entry '{name}' out of bounds")),
                }
            };
            entries.push(BarEntry { kind, duplicate, name, data });
        }
        Ok(Bar { motionset, entries })
    }

    pub fn get(&self, name: &str) -> Option<&BarEntry<'a>> {
        self.entries.iter().find(|e| e.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_synthetic_bar() {
        let mut b = Vec::new();
        b.extend_from_slice(&MAGIC.to_le_bytes());
        b.extend_from_slice(&1i32.to_le_bytes());
        b.extend_from_slice(&0i32.to_le_bytes());
        b.extend_from_slice(&0i32.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&0i16.to_le_bytes());
        b.extend_from_slice(b"test");
        b.extend_from_slice(&0x20i32.to_le_bytes());
        b.extend_from_slice(&3i32.to_le_bytes());
        b.extend_from_slice(&[9, 8, 7]);
        let bar = Bar::parse(&b).unwrap();
        assert_eq!(bar.get("test").unwrap().data, &[9, 8, 7]);
    }
}
