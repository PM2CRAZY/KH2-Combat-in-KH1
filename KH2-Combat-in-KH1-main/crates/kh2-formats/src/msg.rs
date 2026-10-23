//! KH2 message files (`msg/<lang>/*.bar`): id -> encoded string, and a
//! decoder for the plain-text subset (letters, digits, space, common
//! punctuation). The character codes were identified on the user's install
//! by matching known names; control codes are dropped. OpenKH documents the
//! full encoding (Apache-2.0).

use std::collections::HashMap;

use crate::bar::Bar;
use crate::reader::Reader;
use crate::Result;

#[derive(Clone, Debug, Default)]
pub struct Messages {
    pub by_id: HashMap<u32, Vec<u8>>,
}

impl Messages {
    pub fn parse(bar_bytes: &[u8]) -> Result<Messages> {
        let bar = Bar::parse(bar_bytes)?;
        let mut by_id = HashMap::new();
        for e in &bar.entries {
            if e.data.len() < 8 {
                continue;
            }
            let mut r = Reader::new(e.data);
            let _version = r.u32()?;
            let count = r.u32()? as usize;
            for k in 0..count {
                let mut h = Reader::at(e.data, 8 + k * 8);
                let id = h.u32()?;
                let off = h.u32()? as usize;
                if off >= e.data.len() {
                    continue;
                }
                let end = e.data[off..].iter().position(|&b| b == 0).map_or(e.data.len(), |p| off + p);
                by_id.insert(id, e.data[off..end].to_vec());
            }
        }
        Ok(Messages { by_id })
    }

    /// Decoded text of a message id (the 0x8000 "system" bit is ignored).
    pub fn text(&self, id: u32) -> Option<String> {
        self.by_id.get(&(id & 0x7fff)).map(|b| decode(b))
    }
}

/// Decodes the plain-text subset of KH2's US text encoding.
pub fn decode(bytes: &[u8]) -> String {
    let mut s = String::new();
    for &c in bytes {
        match c {
            0x01 => s.push(' '),
            0x2e..=0x47 => s.push((b'A' + (c - 0x2e)) as char),
            0x9a..=0xb3 => s.push((b'a' + (c - 0x9a)) as char),
            0x90..=0x99 => s.push((b'0' + (c - 0x90)) as char),
            0x86 => s.push('&'),
            0x66 => s.push('-'),
            0xec => s.push('\''),
            0x50 => s.push(','),
            _ => {}
        }
    }
    s.trim().to_string()
}

#[cfg(test)]
mod tests {
    #[test]
    fn decodes_letters() {
        // "Combo Plus"
        assert_eq!(super::decode(&[0x30, 0xa8, 0xa6, 0x9b, 0xa8, 0x01, 0x3d, 0xa5, 0xae, 0xac]), "Combo Plus");
    }
}
