//! KH2 HD sound banks (`se/seNNN.win32.scd`): find a sound effect and decode it to PCM.
//!
//! Layout as read from the user's install (the SQEX "SEDBSSCF" container); only what playing
//! one sound needs is read. Our reading, Checked against the files:
//!
//! ```text
//! 0x30  u16 entries, u16 streams, u16 layouts, u16 0
//! 0x38  u32 entry offsets, u32 stream offsets, u32 layout offsets, ...
//! entry  either a small command script (one per sound; its stream is the stream of the
//!        same index) or, in banks with more entries than scripts, a stream header itself
//! stream 0x20-byte header: i32 size, i32 channels, i32 rate, i32 codec, i32 loop start,
//!        i32 loop end, i32 extra size, i32 aux count; then `extra` (for codec 12, a
//!        WAVEFORMATEX for Microsoft ADPCM), then `size` bytes of data
//! ```
//!
//! A KH2 sound id is `bank * 1000 + index` (PAX element sounds, e.g. 999038 = bank 999,
//! index 38). Only Microsoft ADPCM (codec 12) is decoded; it is what the effect banks hold.

use crate::{bad, Error, Result};

/// MS ADPCM step adaptation (the codec's published table).
const ADAPT: [i32; 16] = [230, 230, 230, 230, 307, 409, 512, 614, 768, 614, 512, 409, 307, 230, 230, 230];

/// Decoded sound: interleaved 16-bit samples.
#[derive(Clone, Debug, PartialEq)]
pub struct Pcm {
    pub rate: u32,
    pub channels: u16,
    pub samples: Vec<i16>,
}

pub struct Scd<'a> {
    data: &'a [u8],
    entries: Vec<usize>,
    streams: Vec<usize>,
}

fn u16_at(b: &[u8], o: usize) -> Result<u16> {
    b.get(o..o + 2).map(|s| u16::from_le_bytes([s[0], s[1]])).ok_or_else(|| Error::Format("scd: short".into()))
}

fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    b.get(o..o + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]])).ok_or_else(|| Error::Format("scd: short".into()))
}

fn i32_at(b: &[u8], o: usize) -> Result<i32> {
    Ok(u32_at(b, o)? as i32)
}

/// A KH2 sound id → (bank number, index in the bank).
pub fn split_id(id: u32) -> (u32, usize) {
    (id / 1000, (id % 1000) as usize)
}

/// The archive file name of a bank.
pub fn bank_file(bank: u32) -> String {
    format!("se/se{bank:03}.win32.scd")
}

impl<'a> Scd<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Scd<'a>> {
        if data.get(0..8) != Some(b"SEDBSSCF") {
            return bad("scd: not SEDBSSCF");
        }
        let (n_entries, n_streams) = (u16_at(data, 0x30)? as usize, u16_at(data, 0x32)? as usize);
        let (o_entries, o_streams) = (u32_at(data, 0x38)? as usize, u32_at(data, 0x3C)? as usize);
        let table = |o: usize, n: usize| -> Result<Vec<usize>> { (0..n).map(|i| Ok(u32_at(data, o + i * 4)? as usize)).collect() };
        Ok(Scd { data, entries: table(o_entries, n_entries)?, streams: table(o_streams, n_streams)? })
    }

    /// Whether `at` holds a stream header this reader understands.
    fn is_stream(&self, at: usize) -> bool {
        let ok = |o| i32_at(self.data, at + o).unwrap_or(-1);
        let (size, ch, rate, codec) = (ok(0), ok(4), ok(8), ok(12));
        size > 0 && (1..=8).contains(&ch) && (4000..=96000).contains(&rate) && codec == 12
    }

    /// The stream that sound `index` plays.
    fn stream_of(&self, index: usize) -> Option<usize> {
        let e = *self.entries.get(index)?;
        if self.is_stream(e) {
            return Some(e);
        }
        self.streams.get(index).copied().filter(|&s| self.is_stream(s))
    }

    /// Decodes sound `index` of this bank.
    pub fn sound(&self, index: usize) -> Result<Pcm> {
        let at = self.stream_of(index).ok_or_else(|| Error::Format(format!("scd: no stream for sound {index}")))?;
        let d = self.data;
        let size = i32_at(d, at)? as usize;
        let channels = i32_at(d, at + 4)? as u16;
        let rate = i32_at(d, at + 8)? as u32;
        let extra = i32_at(d, at + 0x18)? as usize;
        let fmt = d.get(at + 0x20..at + 0x20 + extra).ok_or_else(|| Error::Format("scd: short format".into()))?;
        let body = d.get(at + 0x20 + extra..at + 0x20 + extra + size).ok_or_else(|| Error::Format("scd: short data".into()))?;
        let samples = decode_ms_adpcm(fmt, body, channels as usize)?;
        Ok(Pcm { rate, channels, samples })
    }
}

/// Microsoft ADPCM: `fmt` is the WAVEFORMATEX (+ADPCM fields), `data` whole blocks.
pub fn decode_ms_adpcm(fmt: &[u8], data: &[u8], channels: usize) -> Result<Vec<i16>> {
    let block = u16_at(fmt, 12)? as usize;
    let per_block = u16_at(fmt, 18)? as usize;
    let n_coef = u16_at(fmt, 20)? as usize;
    let coef: Vec<(i32, i32)> = (0..n_coef).map(|i| Ok((u16_at(fmt, 22 + i * 4)? as i16 as i32, u16_at(fmt, 24 + i * 4)? as i16 as i32))).collect::<Result<_>>()?;
    if block == 0 || channels == 0 || coef.is_empty() || block < 7 * channels {
        return bad("scd: bad ADPCM format");
    }
    let mut out = Vec::with_capacity(data.len() / block * per_block * channels);
    for b in data.chunks_exact(block) {
        let mut pred = vec![(0i32, 0i32); channels];
        let mut delta = vec![0i32; channels];
        let mut s1 = vec![0i32; channels];
        let mut s2 = vec![0i32; channels];
        let mut o = 0;
        for ch in 0..channels {
            pred[ch] = coef[(b[o] as usize).min(coef.len() - 1)];
            o += 1;
        }
        let rd = |o: usize| i16::from_le_bytes([b[o], b[o + 1]]) as i32;
        for ch in 0..channels {
            delta[ch] = rd(o);
            o += 2;
        }
        for ch in 0..channels {
            s1[ch] = rd(o);
            o += 2;
        }
        for ch in 0..channels {
            s2[ch] = rd(o);
            o += 2;
        }
        // The two header samples come out oldest first.
        for ch in 0..channels {
            out.push(s2[ch] as i16);
        }
        for ch in 0..channels {
            out.push(s1[ch] as i16);
        }
        let mut ch = 0;
        let mut made = 2 * channels;
        let want = per_block * channels;
        for &byte in &b[o..] {
            for nib in [byte >> 4, byte & 0x0F] {
                if made >= want {
                    break;
                }
                let n = if nib & 8 != 0 { nib as i32 - 16 } else { nib as i32 };
                let (c1, c2) = pred[ch];
                // Divided (toward zero), as the reference decoder does.
                let p = (s1[ch] * c1 + s2[ch] * c2) / 256 + n * delta[ch];
                let p = p.clamp(-32768, 32767);
                s2[ch] = s1[ch];
                s1[ch] = p;
                delta[ch] = ((ADAPT[nib as usize] * delta[ch]) >> 8).max(16);
                out.push(p as i16);
                made += 1;
                ch = (ch + 1) % channels;
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_ids_split() {
        assert_eq!(split_id(999038), (999, 38));
        assert_eq!(split_id(169), (0, 169));
        assert_eq!(bank_file(0), "se/se000.win32.scd");
    }

    #[test]
    fn adpcm_block_header_and_one_nibble() {
        // Mono, block of 7 header bytes + 1 data byte, 4 samples per block, the standard
        // first coefficient pair (256, 0).
        let mut fmt = vec![0u8; 22 + 4];
        fmt[12..14].copy_from_slice(&8u16.to_le_bytes());
        fmt[18..20].copy_from_slice(&4u16.to_le_bytes());
        fmt[20..22].copy_from_slice(&1u16.to_le_bytes());
        fmt[22..24].copy_from_slice(&256u16.to_le_bytes());
        let mut block = vec![0u8];
        block.extend_from_slice(&16i16.to_le_bytes()); // delta
        block.extend_from_slice(&100i16.to_le_bytes()); // sample 1
        block.extend_from_slice(&50i16.to_le_bytes()); // sample 2
        block.push(0x1F); // nibbles +1, -1
        let pcm = decode_ms_adpcm(&fmt, &block, 1).unwrap();
        // 50, 100, then 100 + 1*16 = 116, then 116 - 1*16 = 100 (delta stays at its floor).
        assert_eq!(pcm, vec![50, 100, 116, 100]);
    }
}
