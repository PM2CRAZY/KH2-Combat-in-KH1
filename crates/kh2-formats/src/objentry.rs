//! `00objentry.bin` (object definitions) and `03system.bin` → `sklt`
//! (weapon attachment bones). Layouts follow OpenKH's docs and
//! `OpenKh.Kh2/Objentry.cs` (Apache-2.0); the `sklt` table was confirmed as
//! the weapon-bone lookup by reading the executable (see
//! `docs/animation.md`).

use crate::bar::Bar;
use crate::reader::Reader;
use crate::{bad, Result};

/// One object definition (96 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct Objentry {
    pub id: u32,
    pub kind: u8,
    pub sub_kind: u8,
    pub draw_priority: u8,
    /// Key into the `sklt` table: which bones hold this object's weapons.
    pub weapon_joint: u8,
    pub model: String,
    pub moveset: String,
    pub flags: u16,
    /// Enemy status id ("neo status", +0x4c): keys the enemy's status and
    /// journal rows; 1000..2011 are Heartless, 3000..4014 Nobodies.
    pub neo_status: u16,
    pub weight: f32,
}

impl Objentry {
    pub const SIZE: usize = 0x60;
}

pub fn parse_objentry(buf: &[u8]) -> Result<Vec<Objentry>> {
    let mut r = Reader::new(buf);
    let _version = r.u32()?;
    let count = r.u32()? as usize;
    if 8 + count * Objentry::SIZE > buf.len() {
        return bad("objentry: table larger than file");
    }
    (0..count)
        .map(|i| {
            let mut e = Reader::at(buf, 8 + i * Objentry::SIZE);
            let id = e.u32()?;
            let kind = e.u8()?;
            let sub_kind = e.u8()?;
            let draw_priority = e.u8()?;
            let weapon_joint = e.u8()?;
            let model = e.cstr(32)?;
            let moveset = e.cstr(32)?;
            let flags = e.u16()?;
            let _target = e.u8()?;
            let _pad = e.u8()?;
            let neo_status = e.u16()?;
            let _neo_moveset = e.u16()?;
            let weight = e.f32()?;
            Ok(Objentry { id, kind, sub_kind, draw_priority, weapon_joint, model, moveset, flags, neo_status, weight })
        })
        .collect()
}

/// `sklt`: weapon joint id → the model bones weapons attach to (right hand,
/// left hand; -1 = none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sklt {
    pub id: u32,
    pub bones: [i16; 2],
}

pub fn parse_sklt(system: &[u8]) -> Result<Vec<Sklt>> {
    let bar = Bar::parse(system)?;
    let Some(e) = bar.entries.iter().find(|e| e.name == "sklt") else { return bad("03system.bin has no sklt") };
    let mut r = Reader::new(e.data);
    let _version = r.u32()?;
    let count = r.u32()? as usize;
    (0..count)
        .map(|i| {
            let mut s = Reader::at(e.data, 8 + i * 8);
            Ok(Sklt { id: s.u32()?, bones: [s.i16()?, s.i16()?] })
        })
        .collect()
}

/// `03system.bin` → `wmst`: weapon motion-set names, 32 bytes each, indexed
/// by `moveset × 2 + hand` (0 right, 1 left). An empty name means none.
/// The playable movesets: 1 Sora, 2 Valor, 3 Wisdom, 4 Master, 5 Final,
/// 6 Anti, 0x42 Limit (checked against the executable's use of the table).
pub fn parse_wmst(system: &[u8]) -> Result<Vec<String>> {
    let bar = Bar::parse(system)?;
    let Some(e) = bar.entries.iter().find(|e| e.name == "wmst") else { return bad("03system.bin has no wmst") };
    Ok(e.data
        .chunks_exact(32)
        .map(|c| {
            let end = c.iter().position(|&b| b == 0).unwrap_or(c.len());
            String::from_utf8_lossy(&c[..end]).into_owned()
        })
        .collect())
}
