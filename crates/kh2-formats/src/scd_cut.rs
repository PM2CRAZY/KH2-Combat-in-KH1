//! Cuts one sound out of a KH2 sound bank into a bank of its own, the way KH1 stores its own
//! effects (`docs/kh1-sound-native.md`, "One-sound banks").
//!
//! KH1's PC effects are one-sound `.win32.scd` files (`se021010.win32.scd`: one sound, one
//! track, one audio stream). KH1's create (`SslSeCreate`) plays sound 0 of the bank whose file
//! number it is asked for, so a KH2 sound has to be a bank of its own to go through KH1's queue.
//! Both games use the same SEDBSSCF reader (versions 1..4 accepted by KH1's
//! `GetScdHeaderSize` / `GetSoundBasicDescSize`), so the pieces are copied as they are; only
//! the offset tables are new and the sound's (track, audio) pairs are renumbered.
//!
//! Layout read from KH2's banks and KH1's own one-sound files (Checked (files)):
//!
//! ```text
//! 0x00  "SEDBSSCF", u32 version (3 or 4), ..., u32 file size at 0x10 (0x30 bytes in all)
//! 0x30  u16 sounds, u16 tracks, u16 audio streams, u16 bank number
//! 0x38  u32 track table, u32 audio table, u32 layout table (one per sound), u32 routing (0),
//!       u32 attribute table, u32 end padding (0)
//! 0x50  the sound table (u32 offsets), then the other tables, each 16-aligned
//! sound  byte 0 track count, byte 1 bus, ..., f32 volume at 0x08; its last 4 x count bytes are
//!        (u16 track, u16 audio) pairs
//! track  a command script (no indices of its own: Assumption, none seen in the scripts read)
//! layout u16 size (0x80) first
//! audio  0x20-byte stream header (u32 data size, ..., u32 extra size at 0x18), extra, data
//! ```
//!
//! The output follows KH1's own one-sound files: tables, then the layout, the sound, the
//! tracks, the attributes and the audio streams, each block 16-aligned.

use crate::{bad, Error, Result};

fn u16_at(b: &[u8], o: usize) -> Result<u16> {
    b.get(o..o + 2).map(|s| u16::from_le_bytes([s[0], s[1]])).ok_or_else(|| Error::Format("scd cut: short".into()))
}

fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    b.get(o..o + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]])).ok_or_else(|| Error::Format("scd cut: short".into()))
}

fn align16(v: &mut Vec<u8>) {
    while v.len() % 16 != 0 {
        v.push(0);
    }
}

/// What a cut bank holds, for the log.
#[derive(Clone, Debug, PartialEq)]
pub struct Cut {
    pub bytes: Vec<u8>,
    /// The sound's bus (its record's byte 1).
    pub bus: u8,
    pub tracks: usize,
    /// The audio streams' total size.
    pub audio_bytes: usize,
}

/// Cuts sound `sound` (a KH2 sound id's index in its bank) of `data` into a one-sound bank
/// whose header bank number is `bank_number`. A sound that plays nothing (no tracks) is an
/// error.
pub fn cut(data: &[u8], sound: usize, bank_number: u16) -> Result<Cut> {
    if data.get(0..8) != Some(b"SEDBSSCF") {
        return bad("scd cut: not SEDBSSCF");
    }
    let version = u32_at(data, 8)?;
    if !(1..=4).contains(&version) {
        return bad(format!("scd cut: version {version}"));
    }
    let n_sounds = u16_at(data, 0x30)? as usize;
    let n_tracks = u16_at(data, 0x32)? as usize;
    let n_audio = u16_at(data, 0x34)? as usize;
    let t_track = u32_at(data, 0x38)? as usize;
    let t_audio = u32_at(data, 0x3C)? as usize;
    let t_layout = u32_at(data, 0x40)? as usize;
    let t_routing = u32_at(data, 0x44)? as usize;
    let t_attr = u32_at(data, 0x48)? as usize;
    if t_routing != 0 {
        return bad("scd cut: the bank has a routing table");
    }
    let table = |o: usize, n: usize| -> Result<Vec<usize>> { (0..n).map(|i| Ok(u32_at(data, o + i * 4)? as usize)).collect() };
    let sounds = table(0x50, n_sounds)?;
    let tracks = table(t_track, n_tracks)?;
    let audio = table(t_audio, n_audio)?;
    let layouts = if t_layout != 0 { table(t_layout, n_sounds)? } else { Vec::new() };
    // Every block start, to find where one ends (the next start after it).
    let mut starts: Vec<usize> = sounds.iter().chain(&tracks).chain(&audio).chain(&layouts).copied().filter(|&o| o != 0).collect();
    // The attribute table: no count of its own; its entries run from it to the first block
    // after it (KH2's effect banks have none: the table's offset is the first layout's).
    let first_after_attr = starts.iter().copied().filter(|&o| o > t_attr).min().unwrap_or(data.len());
    let attrs: Vec<usize> = if t_attr != 0 && !starts.contains(&t_attr) && t_attr < first_after_attr {
        table(t_attr, (first_after_attr - t_attr) / 4)?.into_iter().take_while(|&o| o != 0).collect()
    } else {
        Vec::new()
    };
    starts.extend(&attrs);
    starts.push(data.len());
    starts.sort_unstable();
    starts.dedup();
    let end_of = |o: usize| -> usize { starts.iter().copied().find(|&s| s > o).unwrap_or(data.len()) };

    let &s = sounds.get(sound).ok_or_else(|| Error::Format(format!("scd cut: no sound {sound}")))?;
    let s_end = end_of(s);
    let count = *data.get(s).ok_or_else(|| Error::Format("scd cut: short".into()))? as usize;
    if count == 0 || s_end < s + 0x10 + 4 * count {
        return bad(format!("scd cut: sound {sound} plays nothing"));
    }
    let mut record = data[s..s_end].to_vec();
    let bus = record[1];
    let pairs_at = record.len() - 4 * count;
    let mut my_tracks: Vec<usize> = Vec::new();
    let mut my_audio: Vec<usize> = Vec::new();
    for k in 0..count {
        let p = pairs_at + 4 * k;
        let (t, a) = (u16_at(&record, p)? as usize, u16_at(&record, p + 2)? as usize);
        let ti = my_tracks.iter().position(|&x| x == t).unwrap_or_else(|| {
            my_tracks.push(t);
            my_tracks.len() - 1
        });
        let ai = my_audio.iter().position(|&x| x == a).unwrap_or_else(|| {
            my_audio.push(a);
            my_audio.len() - 1
        });
        record[p..p + 2].copy_from_slice(&(ti as u16).to_le_bytes());
        record[p + 2..p + 4].copy_from_slice(&(ai as u16).to_le_bytes());
    }
    let block = |o: usize, what: &str| -> Result<&[u8]> {
        let e = end_of(o);
        data.get(o..e).filter(|b| !b.is_empty()).ok_or_else(|| Error::Format(format!("scd cut: bad {what} at {o:#x}")))
    };
    let track_blocks: Vec<&[u8]> = my_tracks
        .iter()
        .map(|&t| tracks.get(t).ok_or_else(|| Error::Format(format!("scd cut: no track {t}"))).and_then(|&o| block(o, "track")))
        .collect::<Result<_>>()?;
    let audio_blocks: Vec<&[u8]> = my_audio
        .iter()
        .map(|&a| -> Result<&[u8]> {
            let &o = audio.get(a).ok_or_else(|| Error::Format(format!("scd cut: no audio {a}")))?;
            let len = 0x20 + u32_at(data, o + 0x18)? as usize + u32_at(data, o)? as usize;
            data.get(o..o + len).ok_or_else(|| Error::Format(format!("scd cut: audio {a} runs past the bank")))
        })
        .collect::<Result<_>>()?;
    let layout_block: Option<&[u8]> = match layouts.get(sound) {
        Some(&o) if o != 0 => {
            let b = block(o, "layout")?;
            let size = u16_at(data, o)? as usize;
            Some(if size >= 4 && size <= b.len() { &b[..size] } else { b })
        }
        _ => None,
    };
    let attr_blocks: Vec<&[u8]> = attrs.iter().map(|&o| block(o, "attribute")).collect::<Result<_>>()?;

    // The tables: sounds (1), tracks, audio, layouts (1), attributes; then the data.
    let nt = track_blocks.len();
    let na = audio_blocks.len();
    let mut out = data[..0x30].to_vec();
    out.extend_from_slice(&[0u8; 0x20]);
    let place = |out: &mut Vec<u8>, n: usize| -> usize {
        let at = out.len();
        out.extend(std::iter::repeat(0u8).take(4 * n.max(1)));
        align16(out);
        at
    };
    let tab_sound = place(&mut out, 1);
    let tab_track = place(&mut out, nt);
    let tab_audio = place(&mut out, na);
    let tab_layout = if layout_block.is_some() { place(&mut out, 1) } else { 0 };
    let tab_attr = place(&mut out, attr_blocks.len());
    let put = |out: &mut Vec<u8>, b: &[u8]| -> usize {
        let at = out.len();
        out.extend_from_slice(b);
        align16(out);
        at
    };
    let set = |out: &mut Vec<u8>, at: usize, v: usize| out[at..at + 4].copy_from_slice(&(v as u32).to_le_bytes());
    if let Some(l) = layout_block {
        let at = put(&mut out, l);
        set(&mut out, tab_layout, at);
    }
    let at = put(&mut out, &record);
    set(&mut out, tab_sound, at);
    for (k, t) in track_blocks.iter().enumerate() {
        let at = put(&mut out, t);
        set(&mut out, tab_track + 4 * k, at);
    }
    for (k, a) in attr_blocks.iter().enumerate() {
        let at = put(&mut out, a);
        set(&mut out, tab_attr + 4 * k, at);
    }
    for (k, a) in audio_blocks.iter().enumerate() {
        let at = put(&mut out, a);
        set(&mut out, tab_audio + 4 * k, at);
    }
    let size = out.len();
    out[0x10..0x14].copy_from_slice(&(size as u32).to_le_bytes());
    out[0x30..0x32].copy_from_slice(&1u16.to_le_bytes());
    out[0x32..0x34].copy_from_slice(&(nt as u16).to_le_bytes());
    out[0x34..0x36].copy_from_slice(&(na as u16).to_le_bytes());
    out[0x36..0x38].copy_from_slice(&bank_number.to_le_bytes());
    set(&mut out, 0x38, tab_track);
    set(&mut out, 0x3C, tab_audio);
    set(&mut out, 0x40, tab_layout);
    set(&mut out, 0x44, 0);
    set(&mut out, 0x48, tab_attr);
    set(&mut out, 0x4C, 0);
    Ok(Cut { bytes: out, bus, tracks: nt, audio_bytes: audio_blocks.iter().map(|a| a.len()).sum() })
}

/// The sound record of a cut bank (its first sound), for small changes after the cut.
fn record_at(bank: &[u8]) -> Result<usize> {
    let r = u32_at(bank, 0x50)? as usize;
    if r + 0x10 > bank.len() {
        return bad("scd cut: no record");
    }
    Ok(r)
}

/// Scales a cut bank's sound volume (its record's f32 at 0x08).
pub fn scale_volume(bank: &mut [u8], by: f32) -> Result<()> {
    let r = record_at(bank)?;
    let v = f32::from_bits(u32_at(bank, r + 8)?) * by;
    bank[r + 8..r + 12].copy_from_slice(&v.to_bits().to_le_bytes());
    Ok(())
}

/// Sets a cut bank's sound bus (its record's byte 1).
pub fn set_bus(bank: &mut [u8], bus: u8) -> Result<()> {
    let r = record_at(bank)?;
    bank[r + 1] = bus;
    Ok(())
}

/// How long a cut bank's sound plays (ms), read from its first audio stream's header without
/// decoding: Microsoft ADPCM (codec 12) only, whole blocks of the WAVEFORMATEX's samples per block
/// and a part block for the rest. `None` for any other codec (Vorbis has no sample count in its
/// header) or a header that doesn't read.
pub fn length_ms(bank: &[u8]) -> Option<u32> {
    let audio = u32_at(bank, u32_at(bank, 0x3C).ok()? as usize).ok()? as usize;
    adpcm_ms(bank.get(audio..)?)
}

/// `length_ms` of one audio stream (its 0x20-byte header, then its extra data).
fn adpcm_ms(a: &[u8]) -> Option<u32> {
    let (size, channels, rate, codec, extra) = (u32_at(a, 0).ok()? as u64, u32_at(a, 4).ok()? as u64, u32_at(a, 8).ok()? as u64, u32_at(a, 0x0C).ok()?, u32_at(a, 0x18).ok()? as usize);
    if codec != 12 || channels == 0 || rate == 0 || extra < 20 {
        return None;
    }
    let fmt = a.get(0x20..0x20 + extra)?;
    let (block, per_block) = (u16_at(fmt, 12).ok()? as u64, u16_at(fmt, 18).ok()? as u64);
    if block < 7 * channels || per_block < 2 {
        return None;
    }
    // (A part block holds its two header samples and two a byte per channel after its header.)
    let whole = size / block * per_block;
    let rest = size % block;
    let part = if rest >= 7 * channels { 2 + (rest - 7 * channels) * 2 / channels } else { 0 };
    Some(((whole + part.min(per_block)) * 1000 / rate) as u32)
}

/// Writes into a bank what KH1's engine writes into it as it loads it (Steam KH1 `0x1400748F0`,
/// the bank's set-up: byte `+0xF` of every sound record set to 0xFF), so the bytes are the same
/// before a load and after a release (`kh1bank` checks a bank it registers again against them).
pub fn as_loaded(bank: &mut [u8]) {
    let Ok(header) = u16_at(bank, 0x0E) else { return };
    let header = header as usize;
    let Ok(n) = u16_at(bank, header) else { return };
    let table = header + if u32_at(bank, 8).unwrap_or(0) == 1 { 0x10 } else { 0x20 };
    for i in 0..n as usize {
        let Ok(r) = u32_at(bank, table + 4 * i) else { return };
        if let Some(b) = bank.get_mut(r as usize + 0x0F) {
            *b = 0xFF;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A two-sound v3 bank in KH2's own order (layouts, sounds, tracks, audio), no attributes.
    fn bank() -> Vec<u8> {
        let mut d = vec![0u8; 0x30];
        d[..8].copy_from_slice(b"SEDBSSCF");
        d[8] = 3;
        d[0x0D] = 4;
        d[0x0E] = 0x30;
        let mut off = vec![0u8; 0x20];
        off[0..2].copy_from_slice(&2u16.to_le_bytes());
        off[2..4].copy_from_slice(&2u16.to_le_bytes());
        off[4..6].copy_from_slice(&2u16.to_le_bytes());
        off[6..8].copy_from_slice(&999u16.to_le_bytes());
        d.extend(off);
        // Tables at 0x50 (sounds), 0x60 (tracks), 0x70 (audio), 0x80 (layouts); attributes at
        // 0x90 = the first layout (none).
        d.resize(0x90, 0);
        let w = |d: &mut Vec<u8>, at: usize, v: u32| d[at..at + 4].copy_from_slice(&v.to_le_bytes());
        w(&mut d, 0x38, 0x60);
        w(&mut d, 0x3C, 0x70);
        w(&mut d, 0x40, 0x80);
        w(&mut d, 0x48, 0x90);
        // Layouts 0x90, 0x110 (0x80 each).
        for (k, at) in [0x90usize, 0x110].iter().enumerate() {
            d.resize(at + 0x80, 0);
            d[*at] = 0x80;
            d[at + 8] = k as u8 + 1;
            w(&mut d, 0x80 + 4 * k, *at as u32);
        }
        // Sounds 0x190, 0x1A4 (20 bytes: 0x10 + one pair); sound 1 plays track 1, audio 0.
        for (k, at) in [0x190usize, 0x1A4].iter().enumerate() {
            d.resize(at + 20, 0);
            d[*at] = 1;
            d[at + 1] = 2;
            d[at + 8..at + 12].copy_from_slice(&0.8f32.to_bits().to_le_bytes());
            let (t, a) = if k == 0 { (0u16, 1u16) } else { (1, 0) };
            d[at + 16..at + 18].copy_from_slice(&t.to_le_bytes());
            d[at + 18..at + 20].copy_from_slice(&a.to_le_bytes());
            w(&mut d, 0x50 + 4 * k, *at as u32);
        }
        // Tracks 0x1B8, 0x1C8 (16 bytes).
        for (k, at) in [0x1B8usize, 0x1C8].iter().enumerate() {
            d.resize(at + 16, 0);
            d[*at] = 0x2F;
            d[at + 15] = 0xA0 + k as u8;
            w(&mut d, 0x60 + 4 * k, *at as u32);
        }
        // Audio 0x1E0 (0x20 + extra 4 + data 8), 0x210 (data 4).
        for (k, (at, size)) in [(0x1E0usize, 8u32), (0x210, 4)].iter().enumerate() {
            d.resize(at + 0x24 + *size as usize, 0);
            w(&mut d, *at, *size);
            w(&mut d, at + 4, 1);
            w(&mut d, at + 8, 48000);
            w(&mut d, at + 12, 12);
            w(&mut d, at + 0x18, 4);
            for i in 0..*size as usize {
                d[at + 0x24 + i] = 0x10 * (k as u8 + 1) + i as u8;
            }
            w(&mut d, 0x70 + 4 * k, *at as u32);
        }
        let n = d.len() as u32;
        w(&mut d, 0x10, n);
        d
    }

    #[test]
    fn cuts_one_sound_with_its_own_pieces() {
        let src = bank();
        let c = cut(&src, 1, 990).unwrap();
        let o = &c.bytes;
        assert_eq!(&o[..8], b"SEDBSSCF");
        assert_eq!(u32_at(o, 0x10).unwrap() as usize, o.len());
        assert_eq!((u16_at(o, 0x30).unwrap(), u16_at(o, 0x32).unwrap(), u16_at(o, 0x34).unwrap(), u16_at(o, 0x36).unwrap()), (1, 1, 1, 990));
        assert_eq!(o.len() % 16, 0);
        let rec = u32_at(o, 0x50).unwrap() as usize;
        // The record, renumbered to (0, 0), volume kept.
        assert_eq!(o[rec], 1);
        assert_eq!(f32::from_bits(u32_at(o, rec + 8).unwrap()), 0.8);
        assert_eq!((u16_at(o, rec + 16).unwrap(), u16_at(o, rec + 18).unwrap()), (0, 0));
        // Track 1 and audio 0 of the source.
        let t = u32_at(o, u32_at(o, 0x38).unwrap() as usize).unwrap() as usize;
        assert_eq!(&o[t..t + 16], &src[0x1C8..0x1D8]);
        let a = u32_at(o, u32_at(o, 0x3C).unwrap() as usize).unwrap() as usize;
        assert_eq!(&o[a..a + 0x2C], &src[0x1E0..0x20C]);
        // Layout 1 (its 0x80 bytes).
        let l = u32_at(o, u32_at(o, 0x40).unwrap() as usize).unwrap() as usize;
        assert_eq!(&o[l..l + 0x80], &src[0x110..0x190]);
        // No attributes: the table's first slot is empty.
        assert_eq!(u32_at(o, u32_at(o, 0x48).unwrap() as usize).unwrap(), 0);
        assert!(crate::scd::Scd::parse(o).is_ok());
    }

    #[test]
    fn silent_sound_is_refused() {
        let mut src = bank();
        src[0x190] = 0;
        assert!(cut(&src, 0, 990).is_err());
    }

    #[test]
    fn bus_sets() {
        let mut c = cut(&bank(), 0, 990).unwrap().bytes;
        set_bus(&mut c, 8).unwrap();
        let rec = u32_at(&c, 0x50).unwrap() as usize;
        assert_eq!(c[rec + 1], 8);
    }

    #[test]
    fn volume_scales() {
        let mut c = cut(&bank(), 0, 990).unwrap().bytes;
        scale_volume(&mut c, 0.5).unwrap();
        let rec = u32_at(&c, 0x50).unwrap() as usize;
        assert!((f32::from_bits(u32_at(&c, rec + 8).unwrap()) - 0.4).abs() < 1e-6);
    }

    /// The user's own banks, when `KH2_SCD_DIR` names a folder of them (local research):
    /// every playable id cuts, and the cut decodes to the same samples as the id in its bank.
    /// (Except where `Scd`'s own reading is off: an audio index past the track count is read
    /// through the track table running on into the audio table, so it plays audio `a - tracks`;
    /// se000's ids on audio 152 and up. The cut keeps the record's own audio index, which is
    /// what the engine reads.)
    #[test]
    fn real_banks_cut_and_decode_the_same() {
        let Ok(dir) = std::env::var("KH2_SCD_DIR") else { return };
        for e in std::fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("scd") {
                continue;
            }
            let data = std::fs::read(&p).unwrap();
            let scd = crate::scd::Scd::parse(&data).unwrap();
            let n = u16_at(&data, 0x30).unwrap() as usize;
            let n_tracks = u16_at(&data, 0x32).unwrap() as usize;
            let sounds: Vec<usize> = (0..n).map(|i| u32_at(&data, 0x50 + 4 * i).unwrap() as usize).collect();
            let (mut ok, mut same, mut past) = (0, 0, 0);
            for id in 0..n {
                let Ok(want) = scd.sound_id(id) else { continue };
                let c = cut(&data, id, 990).unwrap_or_else(|e| panic!("{}: {id}: {e}", p.display()));
                ok += 1;
                let got = crate::scd::Scd::parse(&c.bytes).unwrap().sound_id(0).unwrap();
                let end = sounds.get(id + 1).copied().unwrap_or(u32_at(&data, 0x38).unwrap() as usize);
                let audio = u16_at(&data, end - 2).unwrap() as usize;
                if got.samples == want.samples && got.rate == want.rate && (got.volume - want.volume).abs() < 1e-6 {
                    same += 1;
                } else if audio >= n_tracks {
                    past += 1;
                } else {
                    eprintln!("{}: id {id} decodes differently", p.display());
                }
            }
            eprintln!("{}: {ok} cut, {same} the same, {past} on audio past the track count", p.display());
            assert_eq!(ok, same + past);
        }
    }

    #[test]
    fn adpcm_length_from_the_header() {
        // 48000 Hz mono, 0x24-byte blocks of 64 samples, 10 whole blocks and a part one of 7 + 4.
        let mut a = vec![0u8; 0x20 + 32];
        let w = |a: &mut Vec<u8>, at: usize, v: u32| a[at..at + 4].copy_from_slice(&v.to_le_bytes());
        w(&mut a, 0, 0x24 * 10 + 11);
        w(&mut a, 4, 1);
        w(&mut a, 8, 48000);
        w(&mut a, 0x0C, 12);
        w(&mut a, 0x18, 32);
        a[0x20 + 12..0x20 + 14].copy_from_slice(&0x24u16.to_le_bytes());
        a[0x20 + 18..0x20 + 20].copy_from_slice(&64u16.to_le_bytes());
        assert_eq!(adpcm_ms(&a), Some(((640 + 2 + 8) * 1000 / 48000) as u32));
        w(&mut a, 0x0C, 6);
        assert_eq!(adpcm_ms(&a), None);
    }

    #[test]
    fn as_loaded_marks_each_sound_record() {
        let mut c = cut(&bank(), 1, 990).unwrap().bytes;
        // (The test bank's audio has no ADPCM format: length unknown.)
        assert_eq!(length_ms(&c), None);
        let rec = u32_at(&c, 0x50).unwrap() as usize;
        let before = c.clone();
        as_loaded(&mut c);
        assert_eq!(c[rec + 0x0F], 0xFF);
        let changed: Vec<usize> = (0..c.len()).filter(|&i| c[i] != before[i]).collect();
        assert!(changed.iter().all(|&i| i == rec + 0x0F));
    }
}
