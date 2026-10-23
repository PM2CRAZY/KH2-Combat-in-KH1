//! Movement and player preferences: the `pref` entry of `03system.bin`.
//!
//! Layout per OpenKH `docs/kh2/file/type/preferences.md` and
//! `OpenKh.Kh2/SystemData/Pref/*.cs` (Apache-2.0). `pref` is a BAR holding
//! `plyr`, `fmab`, `prty`, `sstm`, `magi`. Each is `count u32 | count x
//! offset u32` followed by float records; several pointers may share one
//! record. The pointer index is the "moveset" id (index 0 is filler).

use crate::bar::Bar;
use crate::reader::Reader;
use crate::{bad, Result};

/// A pointer table of fixed-size float records.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrefTable {
    /// Offset per pointer index.
    pub pointers: Vec<u32>,
    /// Records, as raw floats, keyed by offset.
    records: Vec<(u32, Vec<f32>)>,
}

impl PrefTable {
    fn parse(buf: &[u8], floats: usize) -> Result<PrefTable> {
        let mut r = Reader::new(buf);
        let count = r.u32()? as usize;
        if count > 4096 {
            return bad("pref: implausible pointer count");
        }
        let pointers = (0..count).map(|_| r.u32()).collect::<Result<Vec<_>>>()?;
        let mut records: Vec<(u32, Vec<f32>)> = Vec::new();
        for &p in &pointers {
            if p == 0 || records.iter().any(|(o, _)| *o == p) {
                continue;
            }
            // Records can be shorter than the documented field list at the
            // end of a table; read what fits (missing fields read as absent).
            let avail = buf.len().saturating_sub(p as usize) / 4;
            let mut rr = Reader::at(buf, p as usize);
            let rec = (0..floats.min(avail)).map(|_| rr.f32()).collect::<Result<Vec<_>>>()?;
            records.push((p, rec));
        }
        Ok(PrefTable { pointers, records })
    }

    /// Raw floats for pointer index `i`.
    pub fn get(&self, i: usize) -> Option<&[f32]> {
        let p = *self.pointers.get(i)?;
        self.records.iter().find(|(o, _)| *o == p).map(|(_, v)| v.as_slice())
    }

    /// Distinct records in file order.
    pub fn unique(&self) -> impl Iterator<Item = &[f32]> {
        self.records.iter().map(|(_, v)| v.as_slice())
    }
}

/// Field names for each record kind, in file order.
pub const PRTY_FIELDS: &[&str] = &[
    "walk_speed", "run_speed", "jump_height", "turn_speed", "hang_height", "hang_margin", "stun_time",
    "mp_drive", "up_down_speed", "dash_speed", "acceleration", "brake", "subjective_offset",
];
pub const FMAB_FIELDS: &[&str] = &[
    "high_jump_height", "air_dodge_height", "air_dodge_speed", "air_slide_time", "air_slide_speed",
    "air_slide_brake", "air_slide_stop_brake", "air_slide_star_time", "glide_speed", "glide_fall_ratio",
    "glide_fall_height", "glide_fall_max", "glide_acceleration", "glide_start_height", "glide_end_height",
    "glide_turn_speed", "dodge_roll_star_time",
];
pub const PLYR_FIELDS: &[&str] = &[
    "attack_y_offset", "attack_radius", "attack_min_h", "attack_max_h", "attack_v_angle", "attack_v_range",
    "attack_s_range", "attack_h_angle", "attack_u_min_h", "attack_u_max_h", "attack_u_range", "attack_j_front",
    "attack_air_min_h", "attack_air_max_h", "attack_air_big_h_offset", "attack_uv0", "attack_jv0",
    "attack_first_v0", "attack_combo_v0", "attack_finish_v0", "blow_recovery_h", "blow_recovery_v",
    "blow_recovery_time", "auto_lock_on_range", "auto_lock_on_min_h", "auto_lock_on_max_h",
    "auto_lock_on_time", "auto_lock_on_h_adjust", "auto_lock_on_inner_adjust",
];
/// Only the movement-relevant head of SSTM is named; the rest is kept raw.
pub const SSTM_FIELDS: &[&str] = &[
    "ceiling_stop", "ceiling_disable_command_time", "hang_range_h", "hang_range_l", "hang_range_xz",
    "fall_max", "blow_brake_xz", "blow_min_xz", "blow_brake_up", "blow_up", "blow_speed", "blow_to_hit_back",
    "hit_back", "hit_back_small", "hit_back_to_jump", "fly_blow_brake", "fly_blow_stop", "fly_blow_up_adjust",
    "magic_jump", "lock_on_range", "lock_on_release_range",
];

/// `magi`: one 124-byte record of spell parameters per magic "sub" (level
/// x2 + finisher, + 10 per Drive form). Kept raw because the spell scripts
/// read it directly by byte offset; field names are in OpenKH's
/// preferences doc (Fire radius/height/time, Blizzard speed, ...).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MagiTable {
    pub pointers: Vec<u32>,
    data: Vec<u8>,
}

impl MagiTable {
    pub const RECORD: usize = 124;

    /// The record for `sub`, falling back to record 0 for an out-of-range
    /// or empty index, as the game does.
    pub fn get(&self, sub: i32) -> Option<&[u8]> {
        let ok = |i: usize| self.pointers.get(i).copied().filter(|p| *p != 0);
        let p = usize::try_from(sub).ok().and_then(ok).or_else(|| ok(0))? as usize;
        self.data.get(p..p + Self::RECORD)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pref {
    pub plyr: PrefTable,
    pub fmab: PrefTable,
    pub prty: PrefTable,
    pub sstm: PrefTable,
    pub magi: MagiTable,
}

impl Pref {
    /// Parses `03system.bin` and returns its `pref` tables.
    pub fn from_03system(system: &[u8]) -> Result<Pref> {
        let bar = Bar::parse(system)?;
        let pref = bar.get("pref").ok_or_else(|| crate::Error::Format("03system.bin has no pref entry".into()))?;
        Pref::parse(pref.data)
    }

    pub fn parse(pref: &[u8]) -> Result<Pref> {
        let bar = Bar::parse(pref)?;
        let t = |name: &str, n: usize| -> Result<PrefTable> {
            match bar.get(name) {
                Some(e) => PrefTable::parse(e.data, n),
                None => bad(format!("pref: missing {name}")),
            }
        };
        Ok(Pref {
            plyr: t("plyr", PLYR_FIELDS.len())?,
            fmab: t("fmab", FMAB_FIELDS.len())?,
            prty: t("prty", PRTY_FIELDS.len())?,
            sstm: t("sstm", 95)?,
            magi: match bar.get("magi") {
                Some(e) => {
                    let mut r = Reader::new(e.data);
                    let n = (r.u32()? as usize).min(1024);
                    let pointers = (0..n).map(|_| r.u32()).collect::<Result<Vec<_>>>()?;
                    MagiTable { pointers, data: e.data.to_vec() }
                }
                None => MagiTable::default(),
            },
        })
    }
}

/// Looks a named field up in a record.
pub fn field(record: &[f32], names: &[&str], name: &str) -> Option<f32> {
    names.iter().position(|n| *n == name).and_then(|i| record.get(i).copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_pointers() {
        let mut b = Vec::new();
        b.extend_from_slice(&3u32.to_le_bytes());
        for p in [0u32, 16, 16] {
            b.extend_from_slice(&p.to_le_bytes());
        }
        for f in [1.5f32, 2.5] {
            b.extend_from_slice(&f.to_le_bytes());
        }
        let t = PrefTable::parse(&b, 2).unwrap();
        assert_eq!(t.get(1), Some(&[1.5f32, 2.5][..]));
        assert_eq!(t.get(2), t.get(1));
        assert_eq!(t.get(0), None);
        assert_eq!(t.unique().count(), 1);
    }
}
