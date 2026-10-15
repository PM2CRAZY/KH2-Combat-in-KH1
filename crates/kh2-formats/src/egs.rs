//! Epic Games Store archives: `<name>.hed` (index) + `<name>.pkg` (data).
//!
//! Layout and cipher are ported from OpenKH's `OpenKh.Egs` (`Hed.cs`,
//! `EgsHdAsset.cs`, `EgsEncryption.cs`), Apache-2.0, Copyright OpenKH Team
//! and Contributors. Changes from the original: rewritten in Rust; the
//! repeated per-round XOR is folded into one 16-byte mask; the substitution
//! table is located in the user's executable at runtime instead of being
//! embedded.
//!
//! ## HED
//! A flat array of 0x20-byte entries:
//! `md5(file name)[16] | pkg offset i64 | stored length i32 | real length i32`.
//!
//! ## PKG entry at `offset`
//! `header[16]` = `decompressed i32 | remastered count i32 | compressed i32 | date i32`,
//! then `remastered count` x 0x30-byte HD asset headers, then the original
//! (PS2-format) file data. `compressed` is -2 for plain data, -1 for
//! encrypted-only, otherwise the zlib-compressed size. Only the first 0x100
//! bytes of a stream are encrypted; the 16-byte header doubles as the seed.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use md5::{Digest, Md5};
use sha2::Sha256;

use crate::reader::Reader;
use crate::{bad, Error, Result};

/// SHA-256 of the 256-byte substitution table used by the PKG key schedule.
/// The table itself is not stored in this repository.
pub const KEY_TABLE_SHA256: &str =
    "759b2f22f7e0bfd257063a01f3de0c0835905217aa926df6def134a14313672d";

/// AES round constants, as used by the PKG key schedule (a public constant,
/// not game data).
const RCON: [u8; 10] = [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x1B, 0x36];
const PASS_COUNT: usize = 10;

/// The 256-byte substitution table needed to decrypt PKG entries.
#[derive(Clone)]
pub struct KeyTable([u8; 256]);

impl std::fmt::Debug for KeyTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KeyTable(<verified>)")
    }
}

fn sha256_hex(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}

impl KeyTable {
    /// Accepts a table only if its hash matches [`KEY_TABLE_SHA256`].
    pub fn from_bytes(b: &[u8]) -> Result<KeyTable> {
        if b.len() != 256 || sha256_hex(b) != KEY_TABLE_SHA256 {
            return bad("key table hash mismatch");
        }
        Ok(KeyTable(b.try_into().unwrap()))
    }

    /// Finds the table inside a game executable (any of the 1.5+2.5 exes
    /// that read PKG files carry it). The table is a permutation of 0..=255,
    /// so a sliding window tracks distinct byte values and only hashes
    /// windows that are full permutations.
    pub fn from_executable(path: impl AsRef<Path>) -> Result<KeyTable> {
        let data = std::fs::read(path.as_ref())?;
        Self::find_in(&data).ok_or_else(|| {
            Error::Format(format!(
                "PKG key table not found in {} (unexpected executable version?)",
                path.as_ref().display()
            ))
        })
    }

    pub fn find_in(data: &[u8]) -> Option<KeyTable> {
        if data.len() < 256 {
            return None;
        }
        let mut counts = [0u32; 256];
        let mut distinct = 0usize;
        for (i, &b) in data.iter().enumerate() {
            if counts[b as usize] == 0 {
                distinct += 1;
            }
            counts[b as usize] += 1;
            if i >= 256 {
                let out = data[i - 256] as usize;
                counts[out] -= 1;
                if counts[out] == 0 {
                    distinct -= 1;
                }
            }
            if i >= 255 && distinct == 256 {
                let w = &data[i - 255..=i];
                if let Ok(t) = KeyTable::from_bytes(w) {
                    return Some(t);
                }
            }
        }
        None
    }

    /// Expands a 16-byte seed into the XOR mask applied to each 16-byte
    /// block. The original applies 11 round keys in sequence; XOR is
    /// associative, so their XOR is equivalent.
    fn mask(&self, seed: &[u8; 16]) -> [u8; 16] {
        let mut k = [0u8; 16 * (PASS_COUNT + 1)];
        for i in 0..16 {
            k[i] = if seed[i] == 0 { i as u8 } else { seed[i] };
        }
        for i in 0..PASS_COUNT * 4 {
            let p = 0x0C + i * 4;
            let mut f = [k[p], k[p + 1], k[p + 2], k[p + 3]];
            if i % 4 == 0 {
                // The original indexes a byte array of little-endian u32
                // round constants, so only the first byte is non-zero.
                let r = if i % 4 == 0 && (i / 4) < RCON.len() { RCON[i / 4] } else { 0 };
                f = [self.0[f[1] as usize] ^ r, self.0[f[2] as usize], self.0[f[3] as usize], self.0[f[0] as usize]];
            }
            for j in 0..4 {
                k[0x10 + i * 4 + j] = k[i * 4 + j] ^ f[j];
            }
        }
        let mut m = [0u8; 16];
        for round in 0..=PASS_COUNT {
            for j in 0..16 {
                m[j] ^= k[round * 16 + j];
            }
        }
        m
    }

    fn decrypt_head(&self, seed: &[u8; 16], data: &mut [u8]) {
        let m = self.mask(seed);
        let end = data.len().min(0x100);
        for (i, b) in data[..end].iter_mut().enumerate() {
            *b ^= m[i % 16];
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HedEntry {
    pub md5: [u8; 16],
    pub offset: u64,
    pub stored_len: u32,
    pub real_len: u32,
}

/// One `.hed`/`.pkg` pair.
#[derive(Debug)]
pub struct Archive {
    pub hed_path: PathBuf,
    pub pkg_path: PathBuf,
    pub entries: Vec<HedEntry>,
}

pub fn name_hash(name: &str) -> [u8; 16] {
    Md5::digest(name.as_bytes()).into()
}

impl Archive {
    /// Opens `<stem>.hed` and remembers `<stem>.pkg`. Accepts either path.
    pub fn open(path: impl AsRef<Path>) -> Result<Archive> {
        let p = path.as_ref();
        let hed_path = p.with_extension("hed");
        let pkg_path = p.with_extension("pkg");
        let hed = std::fs::read(&hed_path)?;
        if hed.len() % 0x20 != 0 {
            return bad(format!("{}: size is not a multiple of 0x20", hed_path.display()));
        }
        let mut r = Reader::new(&hed);
        let mut entries = Vec::with_capacity(hed.len() / 0x20);
        for _ in 0..hed.len() / 0x20 {
            let md5: [u8; 16] = r.bytes(16)?.try_into().unwrap();
            let offset = r.i64()?;
            let stored_len = r.i32()?;
            let real_len = r.i32()?;
            if offset < 0 || stored_len < 0 || real_len < 0 {
                return bad("negative value in hed entry");
            }
            entries.push(HedEntry { md5, offset: offset as u64, stored_len: stored_len as u32, real_len: real_len as u32 });
        }
        Ok(Archive { hed_path, pkg_path, entries })
    }

    pub fn find(&self, name: &str) -> Option<&HedEntry> {
        let h = name_hash(name);
        self.entries.iter().find(|e| e.md5 == h)
    }

    /// Reads, decrypts and decompresses the original-format data of `name`.
    pub fn read(&self, keys: &KeyTable, name: &str) -> Result<Vec<u8>> {
        let e = *self.find(name).ok_or_else(|| Error::NotFound(name.to_string()))?;
        let mut f = File::open(&self.pkg_path)?;
        f.seek(SeekFrom::Start(e.offset))?;
        let mut seed = [0u8; 16];
        f.read_exact(&mut seed)?;
        let mut h = Reader::new(&seed);
        let decompressed = h.i32()?;
        let remastered = h.i32()?;
        let compressed = h.i32()?;
        if decompressed < 0 || !(0..=0x10000).contains(&remastered) {
            return bad(format!("{name}: implausible pkg header"));
        }
        f.seek(SeekFrom::Current(remastered as i64 * 0x30))?;
        let len = if compressed >= 0 { compressed } else { decompressed } as usize;
        if len as u64 > e.stored_len as u64 {
            return bad(format!("{name}: data length exceeds hed entry"));
        }
        let mut data = vec![0u8; len];
        f.read_exact(&mut data)?;
        if compressed > -2 {
            keys.decrypt_head(&seed, &mut data);
        }
        if compressed > -1 {
            if data.len() < 2 {
                return bad(format!("{name}: compressed data too short"));
            }
            let mut out = Vec::with_capacity(decompressed as usize);
            flate2::read::DeflateDecoder::new(&data[2..]).read_to_end(&mut out)?;
            if out.len() != decompressed as usize {
                return bad(format!("{name}: decompressed {} bytes, expected {decompressed}", out.len()));
            }
            Ok(out)
        } else {
            Ok(data)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_hash_is_md5() {
        // RFC 1321 test vector.
        assert_eq!(
            name_hash("abc"),
            [0x90, 0x01, 0x50, 0x98, 0x3c, 0xd2, 0x4f, 0xb0, 0xd6, 0x96, 0x3f, 0x7d, 0x28, 0xe1, 0x7f, 0x72]
        );
    }

    /// Straight transcription of the reference algorithm (11 sequential
    /// XOR passes per block, 4 bytes at a time) to check the folded mask.
    fn reference_decrypt(table: &[u8; 256], seed: &[u8; 16], data: &mut [u8]) {
        let mut sk = [0u8; 40];
        for (i, r) in RCON.iter().enumerate() {
            sk[i * 4] = *r;
        }
        let mut k = [0u8; 0xB0];
        for i in 0..16 {
            k[i] = if seed[i] == 0 { i as u8 } else { seed[i] };
        }
        for i in 0..40 {
            let mut f = [k[0x0C + i * 4], k[0x0D + i * 4], k[0x0E + i * 4], k[0x0F + i * 4]];
            if i % 4 == 0 {
                f = [
                    table[f[1] as usize] ^ sk[i],
                    table[f[2] as usize] ^ sk[i + 1],
                    table[f[3] as usize] ^ sk[i + 2],
                    table[f[0] as usize] ^ sk[i + 3],
                ];
            }
            for j in 0..4 {
                k[0x10 + i * 4 + j] = k[i * 4 + j] ^ f[j];
            }
        }
        let mut idx = 0;
        while idx < data.len().min(0x100) {
            let n = (data.len() - idx).min(16);
            for pass in (0..=10).rev() {
                for j in 0..n {
                    data[idx + j] ^= k[pass * 16 + j];
                }
            }
            idx += 16;
        }
    }

    #[test]
    fn folded_mask_matches_reference() {
        // Any permutation works for this equivalence check; use a made-up one.
        let mut t = [0u8; 256];
        for (i, b) in t.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(167).wrapping_add(13);
        }
        let kt = KeyTable(t);
        for s in 0..4u8 {
            let seed: [u8; 16] = core::array::from_fn(|i| if i % 3 == 0 { 0 } else { (i as u8).wrapping_mul(31).wrapping_add(s) });
            let original: Vec<u8> = (0..300u32).map(|x| (x * 7 + s as u32) as u8).collect();
            let mut a = original.clone();
            let mut b = original.clone();
            kt.decrypt_head(&seed, &mut a);
            reference_decrypt(&t, &seed, &mut b);
            assert_eq!(a, b);
            assert_eq!(&a[0x100..], &original[0x100..]);
        }
    }

    #[test]
    fn rejects_wrong_table() {
        let mut t = [0u8; 256];
        for (i, b) in t.iter_mut().enumerate() {
            *b = i as u8;
        }
        assert!(KeyTable::from_bytes(&t).is_err());
        assert!(KeyTable::find_in(&t).is_none());
    }
}
