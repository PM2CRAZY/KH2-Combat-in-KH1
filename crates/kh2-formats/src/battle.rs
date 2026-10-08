//! Tables inside `00battle.bin` (a BAR file).
//!
//! Field layouts follow OpenKH's documentation
//! (<https://openkh.dev/kh2/file/type/00battle.html>) and the readers in
//! `OpenKh.Kh2/Battle/*.cs` (Apache-2.0). Where the docs and code disagree,
//! the code was followed and the choice is noted. Field names marked
//! `unknown_*` are undocumented upstream too.

use crate::bar::Bar;
use crate::reader::Reader;
use crate::{bad, Result};

/// `version u32 | count u32 | count * entry`, used by most sub-tables.
fn table<T>(name: &str, buf: &[u8], entry_size: usize, mut f: impl FnMut(&mut Reader) -> Result<T>) -> Result<Vec<T>> {
    let mut r = Reader::new(buf);
    let _version = r.u32()?;
    let count = r.u32()? as usize;
    if count.saturating_mul(entry_size) > buf.len().saturating_sub(8) {
        return bad(format!("{name}: {count} entries of {entry_size} bytes exceed {} bytes", buf.len()));
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let mut er = Reader::at(buf, 8 + i * entry_size);
        out.push(f(&mut er)?);
        debug_assert!(er.pos() <= 8 + (i + 1) * entry_size);
    }
    Ok(out)
}

/// ATKP: one row per attack hitbox parameter set. 48 bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct Atkp {
    /// Low byte: lowest attacker level this row applies to. High byte: the
    /// sub id. (Read as one u16 here; see [`Atkp::min_level`] and
    /// [`Atkp::sub`]. Checked in the executable's ATKP lookup.)
    pub sub_id: u16,
    pub id: u16,
    pub kind: u8,
    /// Which attacker stat the damage scales from: 0 Strength, 1 Magic,
    /// anything else none. (OpenKH calls this byte "critical adjust"; the
    /// executable's damage routine reads it as the stat source.)
    pub stat_source: u8,
    pub power: u16,
    pub team: u8,
    pub element: u8,
    pub enemy_reaction: u8,
    pub effect_on_hit: u8,
    pub knockback_1: i16,
    pub knockback_2: i16,
    pub unknown_16: i16,
    pub flags: u8,
    pub refact_self: u8,
    pub refact_other: u8,
    pub reflected_motion: u8,
    pub reflect_hit_back: i16,
    pub reflect_action: i32,
    pub hit_sound: i32,
    pub reflect_rc: u16,
    pub reflect_range: u8,
    pub reflect_angle: i8,
    pub damage_effect: u8,
    pub switch: u8,
    pub interval: u16,
    pub floor_check: u8,
    pub drive_drain: u8,
    pub revenge_damage: u8,
    pub tr_reaction: u8,
    pub combo_group: u8,
    pub random_effect: u8,
    pub attack_kind_flags: u8,
    pub hp_drain: u8,
}

impl Atkp {
    pub const SIZE: usize = 48;

    pub fn min_level(&self) -> u8 {
        self.sub_id as u8
    }

    pub fn sub(&self) -> u8 {
        (self.sub_id >> 8) as u8
    }

    fn read(r: &mut Reader) -> Result<Atkp> {
        Ok(Atkp {
            sub_id: r.u16()?,
            id: r.u16()?,
            kind: r.u8()?,
            stat_source: r.u8()?,
            power: r.u16()?,
            team: r.u8()?,
            element: r.u8()?,
            enemy_reaction: r.u8()?,
            effect_on_hit: r.u8()?,
            knockback_1: r.i16()?,
            knockback_2: r.i16()?,
            unknown_16: r.i16()?,
            flags: r.u8()?,
            refact_self: r.u8()?,
            refact_other: r.u8()?,
            reflected_motion: r.u8()?,
            reflect_hit_back: r.i16()?,
            reflect_action: r.i32()?,
            hit_sound: r.i32()?,
            reflect_rc: r.u16()?,
            reflect_range: r.u8()?,
            reflect_angle: r.i8()?,
            damage_effect: r.u8()?,
            switch: r.u8()?,
            interval: r.u16()?,
            floor_check: r.u8()?,
            drive_drain: r.u8()?,
            revenge_damage: r.u8()?,
            tr_reaction: r.u8()?,
            combo_group: r.u8()?,
            random_effect: r.u8()?,
            attack_kind_flags: r.u8()?,
            hp_drain: r.u8()?,
        })
    }
}

/// ENMP: enemy stats. 92 bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct Enmp {
    pub id: u16,
    /// 0 = use the world's battle level.
    pub level: u16,
    /// Slot 0 is the main body; other slots are parts (purpose partly unknown).
    pub health: [i16; 32],
    /// Upstream name "MaxDamage" / "Damage Cap".
    pub max_damage: u16,
    pub min_damage: u16,
    /// Physical, Fire, Blizzard, Thunder, Dark, Light, General. Percent.
    pub weakness: [u16; 7],
    pub exp: u16,
    pub prize: u16,
    pub bonus_level: u16,
}

impl Enmp {
    pub const SIZE: usize = 92;
    fn read(r: &mut Reader) -> Result<Enmp> {
        let id = r.u16()?;
        let level = r.u16()?;
        let mut health = [0i16; 32];
        for h in &mut health {
            *h = r.i16()?;
        }
        let max_damage = r.u16()?;
        let min_damage = r.u16()?;
        let mut weakness = [0u16; 7];
        for w in &mut weakness {
            *w = r.u16()?;
        }
        Ok(Enmp { id, level, health, max_damage, min_damage, weakness, exp: r.u16()?, prize: r.u16()?, bonus_level: r.u16()? })
    }
}

/// LVPM: enemy stat scaling per level (index 0 = level 1). 12 bytes, no
/// table header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lvpm {
    /// HP percent: `(base_hp * hp + 99) / 100`.
    pub hp: u16,
    pub strength: u16,
    pub defense: u16,
    pub max_strength: u16,
    pub min_strength: u16,
    pub exp: u16,
}

impl Lvpm {
    pub const SIZE: usize = 12;
    pub fn scale_hp(&self, base: i32) -> i32 {
        (base * self.hp as i32 + 99) / 100
    }
}

/// PLRP: starting stats and items per playable character. 128 bytes.
/// (Upstream docs list the padding at offset 72; the C# reader, followed
/// here, puts 32 items at offset 12, then 52 bytes of padding.)
#[derive(Clone, Debug, PartialEq)]
pub struct Plrp {
    pub id: u16,
    pub character: u8,
    pub hp: u8,
    pub mp: u8,
    pub ap: u8,
    pub strength: u8,
    pub magic: u8,
    pub defense: u8,
    pub armor_slots: u8,
    pub accessory_slots: u8,
    pub item_slots: u8,
    pub items: [u16; 32],
}

impl Plrp {
    pub const SIZE: usize = 128;
    fn read(r: &mut Reader) -> Result<Plrp> {
        let mut p = Plrp {
            id: r.u16()?,
            character: r.u8()?,
            hp: r.u8()?,
            mp: r.u8()?,
            ap: r.u8()?,
            strength: r.u8()?,
            magic: r.u8()?,
            defense: r.u8()?,
            armor_slots: r.u8()?,
            accessory_slots: r.u8()?,
            item_slots: r.u8()?,
            items: [0; 32],
        };
        for it in &mut p.items {
            *it = r.u16()?;
        }
        Ok(p)
    }
}

/// One level of a playable character's LVUP table. 16 bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LvupLevel {
    pub exp: i32,
    pub strength: u8,
    pub magic: u8,
    pub defense: u8,
    pub ap: u8,
    pub sword_ability: u16,
    pub shield_ability: u16,
    pub staff_ability: u16,
}

/// FMLV: drive form / summon levels. 8 bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fmlv {
    /// Final Mix order: 0 Summon, 1 Valor, 2 Wisdom, 3 Limit, 4 Master,
    /// 5 Final, 6 Anti. (High nibble of byte 0, per the C# reader; the docs
    /// show the nibbles swapped.)
    pub form: u8,
    pub level: u8,
    pub anti_rate: u8,
    pub ability_level: u8,
    pub ability: u16,
    pub exp: i32,
}

/// MAGC: spell definitions. 56 bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct Magc {
    pub id: u8,
    pub level: u8,
    pub world: u8,
    pub filename: String,
    pub item: u16,
    pub command: u16,
    pub ground_motion: i16,
    pub ground_blend: i16,
    pub finish_motion: i16,
    pub finish_blend: i16,
    pub air_motion: i16,
    pub air_blend: i16,
    pub voice: u8,
    pub voice_finisher: u8,
    pub voice_self: u8,
}

/// BTLV: world battle level contributions per progress flag. 32 bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct Btlv {
    pub id: i32,
    pub progress_flag: i32,
    /// Indexed by world id (0 = ZZ, 1 = World of Darkness, 2 = Twilight
    /// Town, ... see OpenKH's world list).
    pub world_levels: [u8; 19],
}

/// One PTYA row: an attack choice in a character's combo tree. 68 bytes.
#[derive(Clone, Debug, PartialEq)]
pub struct PtyaEntry {
    pub id: u8,
    pub kind: u8,
    pub sub: i8,
    pub combo_offset: i8,
    pub flag: u32,
    /// Motion id (player moveset slot = motion * 4 + relative).
    pub motion: u16,
    pub next_motion: u16,
    pub jump: f32,
    pub jump_max: f32,
    pub jump_min: f32,
    pub speed_min: f32,
    pub speed_max: f32,
    pub near: f32,
    pub far: f32,
    pub low: f32,
    pub high: f32,
    pub inner_min: f32,
    pub inner_max: f32,
    pub blend_time: f32,
    pub distance_adjust: f32,
    /// Ability (03system item id) that unlocks this row; 0 = always.
    pub ability: u16,
    pub score: u16,
}

/// PTYA: `type u32 | count u32 | count x set offset u32`, each set
/// `n u32 | n x PtyaEntry`. The pointer index is the moveset id shared with
/// `pref/prty` (1 = Sora).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ptya {
    pub pointers: Vec<u32>,
    sets: Vec<(u32, Vec<PtyaEntry>)>,
}

impl Ptya {
    pub fn set(&self, moveset: usize) -> Option<&[PtyaEntry]> {
        let p = *self.pointers.get(moveset)?;
        self.sets.iter().find(|(o, _)| *o == p).map(|(_, v)| v.as_slice())
    }

    fn parse(d: &[u8]) -> Result<Ptya> {
        let mut r = Reader::new(d);
        let _ty = r.u32()?;
        let n = r.u32()? as usize;
        if n > 1024 {
            return bad("ptya: implausible pointer count");
        }
        let pointers = (0..n).map(|_| r.u32()).collect::<Result<Vec<_>>>()?;
        let mut sets: Vec<(u32, Vec<PtyaEntry>)> = Vec::new();
        for &p in &pointers {
            if p == 0 || sets.iter().any(|(o, _)| *o == p) {
                continue;
            }
            let mut r = Reader::at(d, p as usize);
            let count = r.u32()? as usize;
            if count > 512 {
                return bad("ptya: implausible set size");
            }
            let mut v = Vec::with_capacity(count);
            for _ in 0..count {
                v.push(PtyaEntry {
                    id: r.u8()?,
                    kind: r.u8()?,
                    sub: r.i8()?,
                    combo_offset: r.i8()?,
                    flag: r.u32()?,
                    motion: r.u16()?,
                    next_motion: r.u16()?,
                    jump: r.f32()?,
                    jump_max: r.f32()?,
                    jump_min: r.f32()?,
                    speed_min: r.f32()?,
                    speed_max: r.f32()?,
                    near: r.f32()?,
                    far: r.f32()?,
                    low: r.f32()?,
                    high: r.f32()?,
                    inner_min: r.f32()?,
                    inner_max: r.f32()?,
                    blend_time: r.f32()?,
                    distance_adjust: r.f32()?,
                    ability: r.u16()?,
                    score: r.u16()?,
                });
            }
            sets.push((p, v));
        }
        Ok(Ptya { pointers, sets })
    }
}

/// The combat-relevant tables of `00battle.bin`.
#[derive(Clone, Debug, Default)]
pub struct Battle {
    pub atkp: Vec<Atkp>,
    pub enmp: Vec<Enmp>,
    pub lvpm: Vec<Lvpm>,
    pub plrp: Vec<Plrp>,
    /// Sora/Roxas, Donald, Goofy, Mickey, Auron, Mulan, Aladdin, Sparrow,
    /// Beast, Jack, Simba, Tron, Riku.
    pub lvup: Vec<Vec<LvupLevel>>,
    pub fmlv: Vec<Fmlv>,
    pub magc: Vec<Magc>,
    pub btlv: Vec<Btlv>,
    /// Combo trees (attack selection rules) per moveset id.
    pub ptya: Ptya,
    /// Names of BAR entries present but not parsed by this crate.
    pub unparsed: Vec<String>,
}

impl Battle {
    pub fn parse(bytes: &[u8]) -> Result<Battle> {
        let bar = Bar::parse(bytes)?;
        let mut b = Battle::default();
        for e in &bar.entries {
            let d = e.data;
            match e.name.as_str() {
                "atkp" => b.atkp = table("atkp", d, Atkp::SIZE, Atkp::read)?,
                "enmp" => b.enmp = table("enmp", d, Enmp::SIZE, Enmp::read)?,
                "plrp" => b.plrp = table("plrp", d, Plrp::SIZE, Plrp::read)?,
                "lvpm" => b.lvpm = parse_lvpm(d)?,
                "lvup" => b.lvup = parse_lvup(d)?,
                "ptya" => b.ptya = Ptya::parse(d)?,
                "fmlv" => {
                    b.fmlv = table("fmlv", d, 8, |r| {
                        let b0 = r.u8()?;
                        let b1 = r.u8()?;
                        Ok(Fmlv {
                            form: b0 >> 4,
                            level: b0 & 0xF,
                            anti_rate: b1 >> 4,
                            ability_level: b1 & 0xF,
                            ability: r.u16()?,
                            exp: r.i32()?,
                        })
                    })?
                }
                "magc" => {
                    b.magc = table("magc", d, 56, |r| {
                        let id = r.u8()?;
                        let level = r.u8()?;
                        let world = r.u8()?;
                        r.skip(1)?;
                        Ok(Magc {
                            id,
                            level,
                            world,
                            filename: r.cstr(32)?,
                            item: r.u16()?,
                            command: r.u16()?,
                            ground_motion: r.i16()?,
                            ground_blend: r.i16()?,
                            finish_motion: r.i16()?,
                            finish_blend: r.i16()?,
                            air_motion: r.i16()?,
                            air_blend: r.i16()?,
                            voice: r.u8()?,
                            voice_finisher: r.u8()?,
                            voice_self: r.u8()?,
                        })
                    })?
                }
                "btlv" => {
                    b.btlv = table("btlv", d, 32, |r| {
                        let id = r.i32()?;
                        let progress_flag = r.i32()?;
                        let mut world_levels = [0u8; 19];
                        for w in &mut world_levels {
                            *w = r.u8()?;
                        }
                        Ok(Btlv { id, progress_flag, world_levels })
                    })?
                }
                other => b.unparsed.push(other.to_string()),
            }
        }
        if b.atkp.is_empty() || b.enmp.is_empty() {
            return bad("00battle.bin is missing atkp or enmp");
        }
        Ok(b)
    }

    /// Enemy stat scaling for a level (1-based). Levels past the table are
    /// clamped to the last row.
    pub fn lvpm_for(&self, level: u16) -> Option<&Lvpm> {
        if self.lvpm.is_empty() {
            return None;
        }
        let i = (level.max(1) as usize - 1).min(self.lvpm.len() - 1);
        self.lvpm.get(i)
    }

    pub fn enemy(&self, id: u16) -> Option<&Enmp> {
        self.enmp.iter().find(|e| e.id == id)
    }

    pub fn attack(&self, id: u16, sub_id: u16) -> Option<&Atkp> {
        self.atkp.iter().find(|a| a.id == id && a.sub_id == sub_id)
    }

    /// The row the game uses for attack `id`/`sub` at attacker `level`.
    ///
    /// Rows sharing an id and sub are stored in ascending level order. The
    /// game starts from the first and walks forward while the next row's
    /// minimum level is at most `level`. A row with `switch` 0 always
    /// replaces the current pick; a row with `switch` 1 or 2 only does so
    /// when `(level - min_level)` is even (1) or odd (2).
    pub fn attack_for_level(&self, id: u16, sub: u8, level: u8) -> Option<&Atkp> {
        let start = self.atkp.iter().position(|a| a.id == id && a.sub() == sub)?;
        let mut pick = &self.atkp[start];
        for row in &self.atkp[start + 1..] {
            if row.id != id || row.sub() != sub || level < row.min_level() {
                break;
            }
            let parity = ((level - row.min_level()) & 1) + 1;
            if row.switch == 0 || parity == row.switch {
                pick = row;
            }
        }
        Some(pick)
    }
}

fn parse_lvpm(d: &[u8]) -> Result<Vec<Lvpm>> {
    // No header upstream. Read as many whole rows as fit.
    let mut r = Reader::new(d);
    (0..d.len() / Lvpm::SIZE)
        .map(|_| {
            Ok(Lvpm {
                hp: r.u16()?,
                strength: r.u16()?,
                defense: r.u16()?,
                max_strength: r.u16()?,
                min_strength: r.u16()?,
                exp: r.u16()?,
            })
        })
        .collect()
}

/// LVUP: `magic u32 | count u32 | 0x38 bytes`, then per character
/// `levels u32 | levels * 16 bytes` (layout per the C# reader).
fn parse_lvup(d: &[u8]) -> Result<Vec<Vec<LvupLevel>>> {
    let mut r = Reader::new(d);
    r.skip(8 + 0x38)?;
    let mut chars = Vec::new();
    for _ in 0..13 {
        let n = r.i32()?;
        if !(0..=200).contains(&n) {
            return bad(format!("lvup: implausible level count {n}"));
        }
        let mut lv = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let l = LvupLevel {
                exp: r.i32()?,
                strength: r.u8()?,
                magic: r.u8()?,
                defense: r.u8()?,
                ap: r.u8()?,
                sword_ability: r.u16()?,
                shield_ability: r.u16()?,
                staff_ability: r.u16()?,
            };
            r.skip(2)?;
            lv.push(l);
        }
        chars.push(lv);
    }
    Ok(chars)
}
