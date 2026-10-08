//! Builds a `character-core` controller for Sora from the user's install:
//! movement from `03system.bin` (pref), moves from `obj/P_EX100.mset` plus
//! the Keyblade's `obj/W_EX010.mset`, combo rows from `00battle.bin` PTYA,
//! and Keyblade hitbox shapes from `obj/W_EX010.mdlx`.

use std::collections::HashMap;

use character_core::{
    AttackParams, Character, Clip, ComboRow, Event, EventKind, GrowthLevel, MoveParams, Moveset, Window, WindowKind,
};
use kh2_formats::battle::PtyaEntry;
use kh2_formats::motion::{range_id, Anb, Mset, ObjectCollision};
use kh2_formats::pref::{field, Pref, FMAB_FIELDS, PLYR_FIELDS, PRTY_FIELDS, SSTM_FIELDS};

use crate::{CombatData, Install, Result, Source};

/// Sora's moveset id in the pref and PTYA pointer tables.
pub const SORA_MOVESET: usize = 1;
pub const PLAYER_MSET: &str = "obj/P_EX100.mset";
pub const WEAPON_MSET: &str = "obj/W_EX010.mset";
pub const WEAPON_MDLX: &str = "obj/W_EX010.mdlx";

/// SSTM float index of "attack distance to speed" (the ground lunge factor).
pub const SSTM_ATTACK_DISTANCE_TO_SPEED: usize = 42;

/// One attack collision sphere along the Keyblade (model units).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BladeSphere {
    /// Distance along the blade from the hand.
    pub along: f32,
    pub radius: f32,
}

/// Everything needed to run Sora.
#[derive(Clone, Debug)]
pub struct SoraData {
    pub character: Character,
    pub combat: CombatData,
    /// Attack collision groups -> spheres.
    pub blade_groups: HashMap<u8, Vec<BladeSphere>>,
    /// Motion ids that resolved to a clip.
    pub motions: Vec<u16>,
}

fn rec<'a>(t: &'a kh2_formats::pref::PrefTable, i: usize) -> &'a [f32] {
    t.get(i).unwrap_or(&[])
}

fn f(r: &[f32], names: &[&str], n: &str) -> f32 {
    field(r, names, n).unwrap_or(0.0)
}

pub fn move_params(p: &Pref) -> MoveParams {
    let prty = rec(&p.prty, SORA_MOVESET);
    let sstm = rec(&p.sstm, 0);
    MoveParams {
        walk_speed: f(prty, PRTY_FIELDS, "walk_speed"),
        run_speed: f(prty, PRTY_FIELDS, "run_speed"),
        jump_height: f(prty, PRTY_FIELDS, "jump_height"),
        turn_speed: f(prty, PRTY_FIELDS, "turn_speed"),
        fall_max: f(sstm, SSTM_FIELDS, "fall_max"),
        // Gravity and air control are constants in the executable, not
        // table values; MoveParams::default() carries them.
        ..MoveParams::default()
    }
}

pub fn growth_levels(p: &Pref) -> Vec<GrowthLevel> {
    p.fmab
        .unique()
        .take(4)
        .map(|r| {
            let g = |n: &str| f(r, FMAB_FIELDS, n);
            GrowthLevel {
                high_jump_height: g("high_jump_height"),
                air_dodge_height: g("air_dodge_height"),
                air_dodge_speed: g("air_dodge_speed"),
                quick_run_time: g("air_slide_time"),
                quick_run_speed: g("air_slide_speed"),
                quick_run_brake: g("air_slide_brake"),
                quick_run_stop_brake: g("air_slide_stop_brake"),
                quick_run_star_time: g("air_slide_star_time"),
                glide_speed: g("glide_speed"),
                glide_fall_ratio: g("glide_fall_ratio"),
                glide_fall_height: g("glide_fall_height"),
                glide_fall_max: g("glide_fall_max"),
                glide_acceleration: g("glide_acceleration"),
                glide_start_height: g("glide_start_height"),
                glide_end_height: g("glide_end_height"),
                glide_turn_speed: g("glide_turn_speed"),
                dodge_roll_star_time: g("dodge_roll_star_time"),
            }
        })
        .collect()
}

pub fn attack_params(p: &Pref) -> AttackParams {
    let plyr = rec(&p.plyr, SORA_MOVESET);
    let sstm = rec(&p.sstm, 0);
    let g = |n: &str| f(plyr, PLYR_FIELDS, n);
    AttackParams {
        auto_lock_on_range: g("auto_lock_on_range"),
        lock_on_range: f(sstm, SSTM_FIELDS, "lock_on_range"),
        lock_on_release_range: f(sstm, SSTM_FIELDS, "lock_on_release_range"),
        distance_to_speed: sstm.get(SSTM_ATTACK_DISTANCE_TO_SPEED).copied().unwrap_or(0.0),
    }
}

pub fn combo_row(e: &PtyaEntry) -> ComboRow {
    ComboRow {
        id: e.id,
        kind: e.kind,
        sub: e.sub,
        combo_offset: e.combo_offset,
        flag: e.flag,
        motion: e.motion,
        next_motion: e.next_motion,
        jump: e.jump,
        jump_min: e.jump_min,
        jump_max: e.jump_max,
        speed_min: e.speed_min,
        speed_max: e.speed_max,
        near: e.near,
        far: e.far,
        low: e.low,
        high: e.high,
        inner_min: e.inner_min,
        inner_max: e.inner_max,
        blend_time: e.blend_time,
        distance_adjust: e.distance_adjust,
        ability: e.ability,
        score: e.score,
    }
}

/// Converts an ANB into a clip. Trigger frames are 60 Hz game frames
/// relative to the clip start; root motion is sampled from the motion's
/// curves at local frame `start + frame * fps / 60`.
pub fn clip_from_anb(motion: u16, player: &Anb, weapon: Option<&Anb>, combat: &CombatData) -> Clip {
    let (len, root) = match &player.motion {
        Some(m) if m.fps > 0.0 && m.frame_end > m.frame_start => {
            let len = ((m.frame_end - m.frame_start) / m.fps * 60.0).round().max(1.0) as u32;
            let base = m.root_at(m.frame_start);
            let root = (0..=len)
                .map(|g| {
                    let p = m.root_at(m.frame_start + g as f32 * m.fps / 60.0);
                    [p[0] - base[0], p[1] - base[1], p[2] - base[2]]
                })
                .collect();
            (len, root)
        }
        _ => {
            let last = player.triggers.ranges.iter().map(|r| r.end.max(r.start)).max().unwrap_or(1).max(1) as u32;
            (last, vec![[0.0; 3]; last as usize + 1])
        }
    };
    let mut clip = Clip { motion, length: len, root, windows: Vec::new(), events: Vec::new() };
    for anb in std::iter::once(player).chain(weapon) {
        for r in &anb.triggers.ranges {
            let p = |i: usize| r.params.get(i).copied().unwrap_or(0);
            let deg = |v: i16| (v as f32).to_radians();
            let kind = match r.id {
                range_id::ATTACK_HITBOX | range_id::ATTACK_HITBOX_COMBO => {
                    let attack = p(0) as u16;
                    let is_guard = combat.battle.attack_for_level(attack, 0, 99).map_or(false, |a| matches!(a.kind, 2 | 3));
                    if is_guard {
                        WindowKind::Guard
                    } else {
                        let combo = (r.id == range_id::ATTACK_HITBOX_COMBO).then(|| p(2));
                        WindowKind::Hitbox { attack, group: p(1) as u8, combo }
                    }
                }
                range_id::ALLOW_COMBO => WindowKind::Combo,
                range_id::ALLOW_COMBO_MAGIC => WindowKind::ComboMagic,
                range_id::ALLOW_COMBO_FINISHER => WindowKind::Finisher,
                range_id::TURN_TO_TARGET => WindowKind::TurnToTarget(deg(p(0))),
                range_id::TURN_TO_LOCK => WindowKind::TurnToLock(deg(p(0))),
                range_id::HURTBOX_OFF => WindowKind::Invulnerable,
                3 | 24 => WindowKind::NoGravity,
                0 => WindowKind::Grounded,
                1 => WindowKind::Air,
                range_id::ALLOW_MOVEMENT => WindowKind::AllowMovement,
                42 => WindowKind::KeepMomentum,
                range_id::ALLOW_RC => WindowKind::AllowReactionCommand,
                range_id::WEAPON_TRAIL => WindowKind::WeaponTrail,
                _ => continue,
            };
            let end = if r.end < 0 { None } else { Some(r.end as u32) };
            clip.windows.push(Window { start: r.start.max(0) as u32, end, kind });
        }
        for e in &anb.triggers.frames {
            let kind = match e.id {
                0 => EventKind::Jump,
                17 => EventKind::MagicCast(e.params.first().copied().unwrap_or(0)),
                28 => EventKind::RevengeCheck,
                34 => EventKind::QuickRunCheck,
                35 => EventKind::FallCheck,
                other => EventKind::Other(other),
            };
            clip.events.push(Event { frame: e.frame.max(0) as u32, kind });
        }
    }
    clip
}

/// Picks the battle-with-weapon slot for a motion (with the game's
/// documented fallback order) and the matching Keyblade slot.
fn moves(player: &Mset, weapon: &Mset, combat: &CombatData) -> HashMap<u16, Clip> {
    let mut clips = HashMap::new();
    for m in 0..=230u16 {
        let Some(ps) = player.slot_for(m as usize, true, true) else { continue };
        let Some(pa) = &ps.anb else { continue };
        let wa = weapon.slot_for(m as usize, true, true).and_then(|s| s.anb.as_ref());
        clips.insert(m, clip_from_anb(m, pa, wa, combat));
    }
    clips
}

pub fn blade_groups(collision: &[ObjectCollision]) -> HashMap<u8, Vec<BladeSphere>> {
    let mut g: HashMap<u8, Vec<BladeSphere>> = HashMap::new();
    for c in collision.iter().filter(|c| c.kind == 6) {
        g.entry(c.group).or_default().push(BladeSphere { along: (c.position[0] as f32).abs(), radius: c.radius as f32 });
    }
    g
}

impl Install {
    pub fn load_sora(&self, source: Source) -> Result<SoraData> {
        let combat = self.load_from(source)?;
        let pref = self.load_pref(source)?;
        let player = self.load_mset(source, PLAYER_MSET)?;
        let weapon = self.load_mset(source, WEAPON_MSET)?;
        let mdlx = self.read_any(source, WEAPON_MDLX)?;
        let collision = kh2_formats::motion::parse_model_collision(&mdlx)?;

        let clips = moves(&player, &weapon, &combat);
        let mut motions: Vec<u16> = clips.keys().copied().collect();
        motions.sort();
        let combo_rows = combat.battle.ptya.set(SORA_MOVESET).unwrap_or(&[]).iter().map(combo_row).collect();
        let moveset = Moveset { clips, combo_rows };
        let character = Character::new(moveset, move_params(&pref), growth_levels(&pref), attack_params(&pref));
        Ok(SoraData { character, combat, blade_groups: blade_groups(&collision), motions })
    }
}
