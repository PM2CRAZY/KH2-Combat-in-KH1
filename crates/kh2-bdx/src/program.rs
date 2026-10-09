//! The BDX file: a 16-byte name, three sizes, a trigger table and the code.
//!
//! Layout (OpenKH docs, `docs/kh2/file/ai`; checked against the game's
//! script loader):
//!
//! ```text
//! 0x00  char  name[16]
//! 0x10  i32   work size   (bytes of shared work memory)
//! 0x14  i32   stack size  (bytes of local-variable stack)
//! 0x18  i32   temp size   (bytes of evaluation stack)
//! 0x1c  (i32 key, i32 entry)*   ending with an entry of 0
//! ```
//!
//! Code addresses ("pc") count 16-bit words from byte 16, so the header
//! itself occupies the first words of the address space.

use crate::BdxError;

#[derive(Clone, Debug)]
pub struct Program {
    pub name: String,
    pub work_size: u32,
    pub stack_size: u32,
    pub temp_size: u32,
    /// `(key, entry pc)` in file order.
    pub triggers: Vec<(i32, u32)>,
    bytes: Vec<u8>,
}

impl Program {
    pub fn parse(bytes: &[u8]) -> Result<Program, BdxError> {
        if bytes.len() < 0x24 {
            return Err(BdxError::Format("shorter than the header".into()));
        }
        let rd = |o: usize| i32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
        let name_end = bytes[..16].iter().position(|b| *b == 0).unwrap_or(16);
        let name = String::from_utf8_lossy(&bytes[..name_end]).into_owned();
        let (work, stack, temp) = (rd(0x10), rd(0x14), rd(0x18));
        if work < 0 || stack < 0 || temp < 0 {
            return Err(BdxError::Format("negative size".into()));
        }
        // The game walks the table until an entry address of 0.
        let mut triggers = Vec::new();
        let mut o = 0x1c;
        while o + 8 <= bytes.len() {
            let (key, entry) = (rd(o), rd(o + 4));
            if entry == 0 {
                break;
            }
            triggers.push((key, entry as u32));
            o += 8;
        }
        Ok(Program { name, work_size: work as u32, stack_size: stack as u32, temp_size: temp as u32, triggers, bytes: bytes.to_vec() })
    }

    /// The entry pc for a trigger key (the first match, as in the game).
    pub fn trigger(&self, key: i32) -> Option<u32> {
        self.triggers.iter().find(|(k, _)| *k == key).map(|(_, e)| *e)
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Number of addressable code words.
    pub fn words(&self) -> u32 {
        (self.bytes.len().saturating_sub(16) / 2) as u32
    }

    pub fn word(&self, pc: u32) -> Option<u16> {
        let o = 16 + pc as usize * 2;
        self.bytes.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_and_triggers() {
        let mut b = vec![0u8; 0x40];
        b[..4].copy_from_slice(b"test");
        b[0x10..0x14].copy_from_slice(&48i32.to_le_bytes());
        b[0x14..0x18].copy_from_slice(&512i32.to_le_bytes());
        b[0x18..0x1c].copy_from_slice(&512i32.to_le_bytes());
        b[0x1c..0x20].copy_from_slice(&21i32.to_le_bytes());
        b[0x20..0x24].copy_from_slice(&18i32.to_le_bytes());
        let p = Program::parse(&b).unwrap();
        assert_eq!(p.name, "test");
        assert_eq!((p.work_size, p.stack_size, p.temp_size), (48, 512, 512));
        assert_eq!(p.triggers, vec![(21, 18)]);
        assert_eq!(p.trigger(21), Some(18));
        assert_eq!(p.trigger(20), None);
    }
}
