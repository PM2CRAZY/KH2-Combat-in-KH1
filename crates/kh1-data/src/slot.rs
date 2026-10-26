//! New motions for KH1 Sora's motion table, in KH1's own format (`docs/kh1-anim-slots.md`).
//!
//! KH1 plays a motion by its slot (`actor + 0x1D4`, a table of handles), reached from a request id
//! through the request table (`actor + 0x1D8`, the mset's section 1). A motion is one block whose
//! sub-tables are offsets from its start in the file; KH1's loader (`0x1401D5CE0`) turns the
//! fields in [`OFFSET_FIELDS`] and each event group's key offset into handles. This module builds
//! file-layout motions (for the hook to place in memory and turn into handles the same way):
//! a KH1 motion taken as a template, retimed to a new length, with a new event table.
//!
//! Events (`+0x3C`): 12-byte groups `id u16, type u16, count u32, keys offset u32`, ended by
//! `0xFFFFFFFF`; keys of 16 bytes `time f32, a u16, b u16, c u32, d u32`. Types read by KH1 (from
//! the skeleton pass `0x140391E20` and the motion step `0x14029D9A0`): 0 its action may be left
//! (`+0x1CC` bit 20), 1.n the hit window of attack group n (bit 16 + n), 5 the combo window (bit
//! 22), 7 the weapon trail (bit 23), 2 a face / texture change, 4.n a foot effect (`a` the kind,
//! `b` the bone), 6.n a sound (`a` 0: sound `b`; 0x19: voice `b`; else a material sound).

use kh2_formats::{Error, Result};

/// Motion header fields that hold offsets from the motion's start (handles once loaded).
pub const OFFSET_FIELDS: [usize; 7] = [0x14, 0x1C, 0x24, 0x28, 0x30, 0x38, 0x3C];
/// The event table's field.
pub const EVENTS: usize = 0x3C;
/// The end of an event group list.
pub const END: u32 = 0xFFFF_FFFF;

/// KH1's level event types (`EventTrack::kind`).
pub const CAN_LEAVE: u16 = 0;
pub const HIT: u16 = 1;
pub const FACE: u16 = 2;
pub const FOOT: u16 = 4;
pub const COMBO: u16 = 5;
pub const SOUND: u16 = 6;
pub const TRAIL: u16 = 7;

pub(crate) fn u32_at(b: &[u8], at: usize) -> Result<u32> {
    b.get(at..at + 4).map(|s| u32::from_le_bytes(s.try_into().unwrap())).ok_or_else(|| Error::Format(format!("motion: read past the end at {at:#x}")))
}

pub(crate) fn put(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

/// The mset's section offsets (from the file's start).
pub fn sections(mset: &[u8]) -> Result<Vec<usize>> {
    let n = u32_at(mset, 0)? as usize;
    if n == 0 || n > 64 {
        return Err(Error::Format(format!("mset: {n} sections")));
    }
    (0..=n).map(|i| u32_at(mset, 4 + i * 4).map(|v| v as usize)).collect()
}

/// The request table (section 1): one entry per request id (bits 0..11 the slot, `0xFFF` none;
/// 12..14 a kind; 15 a flag; the high half an extra id).
pub fn request_table(mset: &[u8]) -> Result<Vec<u32>> {
    let s = sections(mset)?;
    let (a, b) = (*s.get(1).ok_or_else(|| Error::Format("mset: no request table".into()))?, *s.get(2).unwrap_or(&mset.len()));
    if b <= a || b > mset.len() {
        return Err(Error::Format("mset: bad request table".into()));
    }
    (a..b).step_by(4).map(|p| u32_at(mset, p)).collect()
}

/// The motion table's offsets (from the table at MMTN + 0x40).
fn motion_offsets(mset: &[u8]) -> Result<(usize, Vec<usize>)> {
    let s = sections(mset)?;
    let table = s[0] + 0x40;
    let mut v = Vec::new();
    for i in 0..4096 {
        let o = u32_at(mset, table + i * 4)?;
        if o == END {
            break;
        }
        v.push(o as usize);
    }
    Ok((table, v))
}

/// A slot's motion as one block (its start to the next motion's, or to the request table).
pub fn motion_bytes(mset: &[u8], slot: usize) -> Result<Vec<u8>> {
    let (table, offs) = motion_offsets(mset)?;
    let start = table + *offs.get(slot).ok_or_else(|| Error::Format(format!("mset: no slot {slot}")))?;
    let s = sections(mset)?;
    let mut end = s.get(1).copied().unwrap_or(mset.len());
    for &o in &offs {
        let p = table + o;
        if p > start && p < end {
            end = p;
        }
    }
    mset.get(start..end).map(|b| b.to_vec()).ok_or_else(|| Error::Format("mset: motion past the end".into()))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EventKey {
    pub time: f32,
    pub a: u16,
    pub b: u16,
    pub c: u32,
    pub d: u32,
}

impl EventKey {
    pub fn level(time: f32, on: bool) -> EventKey {
        EventKey { time, a: on as u16, b: 0, c: 0, d: 0 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventTrack {
    pub id: u16,
    pub kind: u16,
    pub keys: Vec<EventKey>,
}

impl EventTrack {
    /// A level track on over `windows` (start, end frames; sorted, merged where they touch).
    pub fn level(kind: u16, id: u16, windows: &[(f32, f32)]) -> EventTrack {
        let mut w: Vec<(f32, f32)> = windows.iter().copied().filter(|(a, b)| b > a).collect();
        w.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut merged: Vec<(f32, f32)> = Vec::new();
        for (a, b) in w {
            match merged.last_mut() {
                Some(l) if a <= l.1 => l.1 = l.1.max(b),
                _ => merged.push((a, b)),
            }
        }
        let keys = merged.iter().flat_map(|&(a, b)| [EventKey::level(a, true), EventKey::level(b, false)]).collect();
        EventTrack { id, kind, keys }
    }
}

/// A motion's event tracks (`block`: a motion in file layout).
pub fn events(block: &[u8]) -> Result<Vec<EventTrack>> {
    let mut p = u32_at(block, EVENTS)? as usize;
    let mut out = Vec::new();
    for _ in 0..256 {
        if u32_at(block, p)? == END {
            return Ok(out);
        }
        let id = u32_at(block, p)? as u16;
        let kind = (u32_at(block, p)? >> 16) as u16;
        let count = u32_at(block, p + 4)? as usize;
        let at = u32_at(block, p + 8)? as usize;
        if count > 4096 {
            return Err(Error::Format("motion: event group too long".into()));
        }
        let keys = (0..count)
            .map(|k| {
                let q = at + k * 16;
                let ab = u32_at(block, q + 4)?;
                Ok(EventKey { time: f32::from_bits(u32_at(block, q)?), a: ab as u16, b: (ab >> 16) as u16, c: u32_at(block, q + 8)?, d: u32_at(block, q + 12)? })
            })
            .collect::<Result<Vec<_>>>()?;
        out.push(EventTrack { id, kind, keys });
        p += 12;
    }
    Err(Error::Format("motion: event list not ended".into()))
}

/// The number of keys the motion's channels and helper channels use.
fn key_count(block: &[u8]) -> Result<usize> {
    let mut n = 0;
    for (count_at, list_at) in [(0x18, 0x1C), (0x20, 0x24)] {
        let count = u32_at(block, count_at)? as usize;
        let list = u32_at(block, list_at)? as usize;
        if count > 8192 {
            return Err(Error::Format("motion: too many channels".into()));
        }
        for c in 0..count {
            let e = list + c * 6;
            let s = block.get(e..e + 6).ok_or_else(|| Error::Format("motion: channel past the end".into()))?;
            n = n.max(u16::from_le_bytes([s[4], s[5]]) as usize + s[3] as usize);
        }
    }
    Ok(n)
}

/// `block` (a motion in file layout) lasting `frames` (60 fps) instead: every key time and event
/// time scaled by `frames / old`, the tangents (per frame) by the inverse; its events replaced by
/// `events`, written after the motion (16-byte aligned).
pub fn retimed(block: &[u8], frames: u32, events: &[EventTrack]) -> Result<Vec<u8>> {
    let old = u32_at(block, 4)?;
    if old == 0 || frames == 0 {
        return Err(Error::Format("motion: no length".into()));
    }
    let r = frames as f32 / old as f32;
    let mut b = block.to_vec();
    put(&mut b, 4, frames);
    let keys = u32_at(&b, 0x28)? as usize;
    for k in 0..key_count(&b)? {
        let p = keys + k * 16;
        if p + 16 > b.len() {
            return Err(Error::Format("motion: key past the end".into()));
        }
        let head = u32_at(&b, p)? as i32;
        let t = ((head >> 8) as f32 * r).round() as i32;
        put(&mut b, p, ((t << 8) | (head & 0xFF)) as u32);
        for q in [p + 8, p + 12] {
            let v = f32::from_bits(u32_at(&b, q)?) / r;
            put(&mut b, q, v.to_bits());
        }
    }
    append_events(&mut b, events);
    Ok(b)
}

/// `events` written after the motion `b` (16-byte aligned), its `+0x3C` pointed at them.
pub(crate) fn append_events(b: &mut Vec<u8>, events: &[EventTrack]) {
    b.resize((b.len() + 15) & !15, 0);
    let groups = b.len();
    let mut keys_at = groups + events.len() * 12 + 4;
    keys_at = (keys_at + 15) & !15;
    let mut table = vec![0u8; keys_at - groups];
    let mut key_bytes = Vec::new();
    for (i, t) in events.iter().enumerate() {
        let g = i * 12;
        table[g..g + 4].copy_from_slice(&(t.id as u32 | (t.kind as u32) << 16).to_le_bytes());
        table[g + 4..g + 8].copy_from_slice(&(t.keys.len() as u32).to_le_bytes());
        table[g + 8..g + 12].copy_from_slice(&((keys_at + key_bytes.len()) as u32).to_le_bytes());
        for k in &t.keys {
            key_bytes.extend_from_slice(&k.time.to_le_bytes());
            key_bytes.extend_from_slice(&(k.a as u32 | (k.b as u32) << 16).to_le_bytes());
            key_bytes.extend_from_slice(&k.c.to_le_bytes());
            key_bytes.extend_from_slice(&k.d.to_le_bytes());
        }
    }
    let e = events.len() * 12;
    table[e..e + 4].copy_from_slice(&END.to_le_bytes());
    b.extend_from_slice(&table);
    b.extend_from_slice(&key_bytes);
    b.resize((b.len() + 15) & !15, 0);
    put(b, EVENTS, groups as u32);
}

/// `tracks` with every key time scaled by `r` (a template's events on a retimed motion).
pub fn scaled(tracks: &[EventTrack], r: f32) -> Vec<EventTrack> {
    tracks.iter().map(|t| EventTrack { keys: t.keys.iter().map(|k| EventKey { time: k.time * r, ..*k }).collect(), ..t.clone() }).collect()
}

/// KH1's edge events are read into a buffer of 8 a call (Steam `0x14029FB30` gives
/// `0x140394900` room for 8 and then walks every one it counted, Checked (decomp)): no more
/// than that may fall in one call's window.
pub const EDGES_PER_CALL: usize = 8;
/// The widest window one of KH1's edge calls covers in play: one game frame at 30 fps (frame
/// scale 2). (A blend's end covers its overshoot, under a frame.)
pub const EDGE_WINDOW: f32 = 2.0;

/// Codes of ours for a sound event's `a` (type 6), the KH2 frame triggers of a KH2 motion baked
/// into a KH1 slot: KH1's sound event handler (Steam `0x140295420 (actor, a, b, c)`, wrapped by
/// the hook) takes `a` 0 (a sound by id), 0x19 (his voice) or a material sound's index (its
/// table `0x140528A87[a]`, `a` 1..0x1D, Checked (exe)); ours are past those. `b` is the KH2 motion,
/// `c` the trigger's value (`d` unused: KH1's handler gets only `a`, `b`, `c`).
pub const KH2_EFFECT: u16 = 0x80;
pub const KH2_SOUND: u16 = 0x81;
pub const KH2_VOICE: u16 = 0x82;
/// Every effect element (KH2 frame trigger 1) of the KH2 motion `b` at one frame, in one key
/// (KH2 fires all of a frame's triggers in one update; KH1 reads 8 edges a call): `c` from
/// [`kh2_effects_value`], the frame and which moveset's package (Sora's, or the form's).
pub const KH2_EFFECTS: u16 = 0x83;

/// `c` of a [`KH2_EFFECTS`] key: the KH2 frame in the low 16 bits, bit 16 set for the form's
/// moveset (its own effect package).
pub fn kh2_effects_value(frame: u32, form: bool) -> u32 {
    (frame & 0xFFFF) | (form as u32) << 16
}

/// A [`KH2_EFFECTS`] key's `c` back as (frame, form).
pub fn kh2_effects_of(c: u32) -> (u32, bool) {
    (c & 0xFFFF, c >> 16 & 1 != 0)
}

/// The time a KH2 frame trigger at `frame` gets as a KH1 edge key. KH2 fires a trigger at F when
/// the motion's time goes from `from` to `to` with `from < F <= to` (and F = the start when a
/// motion starts, Steam KH2 `0x14040E960` from `0x1403C6740` / `0x1403C8A40`, Checked (decomp));
/// KH1 fires a key at t when `from <= t < to` (`0x140394900`, Checked (decomp)). With whole-frame
/// steps both fire F on the same update when the key lies just under F; a start trigger (0) stays
/// at 0, which KH1 fires on the motion's first update (or its blend's end, as KH2 does).
pub fn kh2_trigger_time(frame: u32) -> f32 {
    if frame == 0 {
        0.0
    } else {
        frame as f32 - 1.0 / 64.0
    }
}

/// `keys` (sound events, type 6) added to `tracks`. KH1 reads only the first group of a type and
/// id (`0x140394900` looks each id 0..3 up from the list's start, Checked (decomp)), so they go into
/// the first sound group there is (else a new 6.0), sorted by time. Keys that would put more than
/// [`EDGES_PER_CALL`] foot (4) and sound (6) edges in one [`EDGE_WINDOW`] are left out (KH1's own
/// first); their number is returned.
pub fn add_sound_keys(tracks: &mut Vec<EventTrack>, keys: &[EventKey]) -> usize {
    if keys.is_empty() {
        return 0;
    }
    // (KH1's edges as KH1 reads them: the first group of each type 4 / 6 and id 0..3.)
    let read = |tracks: &[EventTrack]| -> Vec<f32> {
        let mut seen: Vec<(u16, u16)> = Vec::new();
        let mut v = Vec::new();
        for t in tracks {
            if (t.kind == FOOT || t.kind == SOUND) && t.id < 4 && !seen.contains(&(t.kind, t.id)) {
                seen.push((t.kind, t.id));
                v.extend(t.keys.iter().map(|k| k.time));
            }
        }
        v
    };
    let fits = |times: &[f32], t: f32| -> bool {
        // (Every window holding `t` that starts at a key time.)
        times.iter().chain(std::iter::once(&t)).filter(|&&s| s <= t && t < s + EDGE_WINDOW).all(|&s| times.iter().filter(|&&u| u >= s && u < s + EDGE_WINDOW).count() < EDGES_PER_CALL)
    };
    let mut times = read(tracks);
    let mut taken = Vec::new();
    let mut dropped = 0;
    for k in keys {
        if fits(&times, k.time) {
            times.push(k.time);
            taken.push(*k);
        } else {
            dropped += 1;
        }
    }
    let at = match tracks.iter().position(|t| t.kind == SOUND && t.id < 4) {
        Some(i) => i,
        None => {
            tracks.push(EventTrack { id: 0, kind: SOUND, keys: Vec::new() });
            tracks.len() - 1
        }
    };
    let t = &mut tracks[at];
    t.keys.extend(taken);
    t.keys.sort_by(|a, b| a.time.total_cmp(&b.time));
    dropped
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small motion block: 2 channels with 3 keys, events with one hit window.
    fn block() -> Vec<u8> {
        let mut b = vec![0u8; 0x200];
        put(&mut b, 4, 30);
        put(&mut b, 8, 60f32.to_bits());
        put(&mut b, 0x18, 2);
        put(&mut b, 0x1C, 0x40);
        put(&mut b, 0x24, 0x50);
        put(&mut b, 0x28, 0x60);
        // channels: bone 0 rx with keys 0..2, bone 1 tx with key 2
        b[0x40..0x46].copy_from_slice(&[0, 0, 4, 2, 0, 0]);
        b[0x46..0x4C].copy_from_slice(&[1, 0, 7, 1, 2, 0]);
        for (k, (t, v)) in [(0i32, 0.0f32), (30, 1.0), (15, 2.0)].iter().enumerate() {
            let p = 0x60 + k * 16;
            put(&mut b, p, ((t * 256) << 8 | 2) as u32);
            put(&mut b, p + 4, v.to_bits());
            put(&mut b, p + 8, 0.5f32.to_bits());
            put(&mut b, p + 12, 0.25f32.to_bits());
        }
        put(&mut b, EVENTS, 0x100);
        put(&mut b, 0x100, (HIT as u32) << 16);
        put(&mut b, 0x104, 2);
        put(&mut b, 0x108, 0x120);
        put(&mut b, 0x10C, END);
        put(&mut b, 0x120, 10f32.to_bits());
        put(&mut b, 0x124, 1);
        put(&mut b, 0x130, 20f32.to_bits());
        b
    }

    #[test]
    fn retimes_keys_and_replaces_events() {
        let b = block();
        let ev = events(&b).unwrap();
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].kind, HIT);
        assert_eq!(ev[0].keys[0], EventKey::level(10.0, true));
        let new = vec![EventTrack::level(HIT, 0, &[(20.0, 32.0)]), EventTrack::level(COMBO, 0, &[(30.0, 60.0)])];
        let r = retimed(&b, 60, &new).unwrap();
        assert_eq!(u32_at(&r, 4).unwrap(), 60);
        // Key 1 (time 30) is at 60 now, its tangents halved; the interpolation byte kept.
        let h = u32_at(&r, 0x70).unwrap() as i32;
        assert_eq!(h >> 8, 60 * 256);
        assert_eq!(h & 0xFF, 2);
        assert_eq!(f32::from_bits(u32_at(&r, 0x78).unwrap()), 0.25);
        assert_eq!(events(&r).unwrap(), new);
        assert_eq!(r.len() % 16, 0);
    }

    #[test]
    fn level_tracks_merge() {
        let t = EventTrack::level(HIT, 0, &[(30.0, 40.0), (10.0, 20.0), (18.0, 25.0)]);
        let times: Vec<(f32, u16)> = t.keys.iter().map(|k| (k.time, k.a)).collect();
        assert_eq!(times, vec![(10.0, 1), (25.0, 0), (30.0, 1), (40.0, 0)]);
    }
    #[test]
    fn kh2_triggers_join_the_first_sound_group() {
        let mut tracks = vec![EventTrack::level(HIT, 0, &[(1.0, 5.0)]), EventTrack { id: 1, kind: SOUND, keys: vec![EventKey { time: 10.0, a: 0, b: 7, c: 0, d: 0 }] }, EventTrack { id: 2, kind: SOUND, keys: vec![] }];
        let k = |t: u32| EventKey { time: kh2_trigger_time(t), a: KH2_SOUND, b: 151, c: 999019, d: 0 };
        assert_eq!(add_sound_keys(&mut tracks, &[k(12), k(0)]), 0);
        let times: Vec<f32> = tracks[1].keys.iter().map(|k| k.time).collect();
        assert_eq!(times, vec![0.0, 10.0, 12.0 - 1.0 / 64.0]);
        assert!(tracks[2].keys.is_empty());
        // (None there: a new 6.0.)
        let mut none = vec![EventTrack::level(HIT, 0, &[(1.0, 5.0)])];
        add_sound_keys(&mut none, &[k(3)]);
        assert_eq!((none[1].kind, none[1].id, none[1].keys.len()), (SOUND, 0, 1));
    }

    #[test]
    fn no_more_edges_than_kh1_reads_at_once() {
        let mut tracks = vec![EventTrack { id: 0, kind: FOOT, keys: vec![EventKey { time: 5.0, a: 0, b: 22, c: 0, d: 0 }; 3] }];
        let keys: Vec<EventKey> = (0..8).map(|_| EventKey { time: 5.5, a: KH2_EFFECT, b: 166, c: 1, d: 0 }).collect();
        assert_eq!(add_sound_keys(&mut tracks, &keys), 3);
        assert_eq!(tracks[1].keys.len(), 5);
        // (Two frames on, room again.)
        assert_eq!(add_sound_keys(&mut tracks, &[EventKey { time: 7.5, a: KH2_EFFECT, b: 166, c: 1, d: 0 }]), 0);
    }

    #[test]
    fn kh2_effects_value_round_trips() {
        assert_eq!(kh2_effects_of(kh2_effects_value(0, true)), (0, true));
        assert_eq!(kh2_effects_of(kh2_effects_value(76, false)), (76, false));
        assert!(KH2_EFFECTS > KH2_VOICE);
    }
}
