//! Builds a `character-core` controller for Sora from the user's install:
//! movement from `03system.bin` (pref), moves from `obj/P_EX100.mset` plus
//! the Keyblade's `obj/W_EX010.mset`, combo rows from `00battle.bin` PTYA,
//! and Keyblade hitbox shapes from `obj/W_EX010.mdlx`.

use std::collections::HashMap;
use std::sync::Arc;

use character_core::{
    reaction::ReactionParams, AttackParams, HangParams, Character, Clip, ComboRow, Event, EventKind, GrowthLevel, MoveParams, Moveset, Window, WindowKind,
};
use kh2_formats::anim::{parse_skeleton, Bone, Interpolated};
use kh2_formats::battle::PtyaEntry;
use kh2_formats::motion::{range_id, Anb, Mset, ObjectCollision};
use kh2_formats::pref::{field, Pref, FMAB_FIELDS, PLYR_FIELDS, PRTY_FIELDS, SSTM_FIELDS};

use crate::{CombatData, Install, Result, Source};

/// Sora's moveset id in the pref and PTYA pointer tables.
pub const SORA_MOVESET: usize = 1;
pub const PLAYER_MSET: &str = "obj/P_EX100.mset";
pub const WEAPON_MSET: &str = "obj/W_EX010.mset";
pub const WEAPON_MDLX: &str = "obj/W_EX010.mdlx";
pub const PLAYER_MDLX: &str = "obj/P_EX100.mdlx";
/// Sora's object-table entry (its weapon-joint id picks the `sklt` row).
pub const PLAYER_OBJECT: &str = "P_EX100";

/// SSTM float index of "attack distance to speed" (the ground lunge factor).
pub const SSTM_ATTACK_DISTANCE_TO_SPEED: usize = 42;

/// One attack collision sphere along the Keyblade (model units).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BladeSphere {
    /// Distance along the blade from the hand.
    pub along: f32,
    pub radius: f32,
}

/// One collision shape fixed to a bone (an MDLX collision entry), in model
/// units. See `combat_core::Shape` for how shapes overlap.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneShape {
    /// Collision group: attack windows switch groups on by number.
    pub group: u8,
    /// Collision type: 2 hit (hurt), 6 attack, 13 guard, ... (OpenKH table).
    pub kind: u8,
    /// 0 ellipsoid, 1 column, 2 cube, 3 sphere.
    pub shape: u8,
    /// Free value per entry (on Sora's hurt spheres: 1 for legs and hips).
    pub argument: i16,
    /// Bone index, or `None` for the object's root (feet, facing). Other
    /// special anchors (0x4001 and up) are also treated as the root.
    pub bone: Option<usize>,
    /// Offset in the bone's space.
    pub offset: [f32; 3],
    /// Raised this far (world up) after the offset is placed.
    pub lift: f32,
    pub radius: f32,
    /// Vertical half-size (columns, ellipsoids).
    pub height: f32,
}

/// Sora's and the Keyblade's skeletons and motions, for posing a rig. Read
/// from the user's install; nothing here is a mesh or texture.
#[derive(Clone, Debug)]
pub struct Rig {
    pub skeleton: Vec<Bone>,
    /// Motion id → keyframed motion (battle-with-weapon slot, with the game's
    /// fallback order).
    pub motions: HashMap<u16, Arc<Interpolated>>,
    pub weapon_skeleton: Vec<Bone>,
    pub weapon_motions: HashMap<u16, Arc<Interpolated>>,
    /// Sora bone the Keyblade's root follows (`sklt` right-hand bone).
    pub weapon_bone: Option<usize>,
    /// Every collision shape on the Keyblade's bones and on Sora's.
    pub weapon_shapes: Vec<BoneShape>,
    pub body_shapes: Vec<BoneShape>,
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
    /// Skeletons and keyframed motions (absent in synthetic test data).
    pub rig: Option<Arc<Rig>>,
    /// Why the rig failed to load, if it did.
    pub rig_error: Option<String>,
    /// objentry weight (body pushing; 0 = immovable).
    pub weight: f32,
    /// Spell parameters (`03system.bin` pref `magi`), read by the spell
    /// scripts.
    pub magi: kh2_formats::pref::MagiTable,
}

pub(crate) fn rec<'a>(t: &'a kh2_formats::pref::PrefTable, i: usize) -> &'a [f32] {
    t.get(i).unwrap_or(&[])
}

pub(crate) fn f(r: &[f32], names: &[&str], n: &str) -> f32 {
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
        hang: HangParams {
            height: f(prty, PRTY_FIELDS, "hang_height"),
            margin: f(prty, PRTY_FIELDS, "hang_margin"),
            range_h: f(sstm, SSTM_FIELDS, "hang_range_h"),
            range_l: f(sstm, SSTM_FIELDS, "hang_range_l"),
            range_xz: f(sstm, SSTM_FIELDS, "hang_range_xz"),
            // Filled from the model's background-collision shape once the
            // rig loads (see load_sora_with).
            body_radius: 0.0,
        },
        hit_back_to_jump: f(sstm, SSTM_FIELDS, "hit_back_to_jump"),
        // Gravity and air control are constants in the executable, not
        // table values; MoveParams::default() carries them.
        ..MoveParams::default()
    }
}

/// Hit reaction numbers (`sstm`).
pub fn reaction_params(p: &Pref) -> ReactionParams {
    let s = rec(&p.sstm, 0);
    let g = |n: &str| f(s, SSTM_FIELDS, n);
    ReactionParams {
        hit_back: g("hit_back"),
        hit_back_small: g("hit_back_small"),
        blow_up: g("blow_up"),
        blow_speed: g("blow_speed"),
        blow_to_hit_back: g("blow_to_hit_back"),
        blow_brake_xz: g("blow_brake_xz"),
        blow_min_xz: g("blow_min_xz"),
        blow_brake_up: g("blow_brake_up"),
        hit_back_to_jump: g("hit_back_to_jump"),
        fall_max: g("fall_max"),
        ..ReactionParams::default()
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
/// curves on the 60 Hz key timeline, from the clip's start.
pub fn clip_from_anb(motion: u16, player: &Anb, weapon: Option<&Anb>, combat: &CombatData) -> Clip {
    let (len, root) = match &player.motion {
        Some(m) if m.fps > 0.0 && m.frame_end > m.frame_start => {
            let len = ((m.frame_end - m.frame_start) / m.fps * 60.0).round().max(1.0) as u32;
            // Key times are 60 Hz frames; the frame range is in the motion's
            // own rate.
            let k0 = m.frame_start * 60.0 / m.fps;
            let base = m.root_at(k0);
            let root = (0..=len)
                .map(|g| {
                    let p = m.root_at(k0 + g as f32);
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
    for (on_weapon, anb) in std::iter::once((false, player)).chain(weapon.map(|w| (true, w))) {
        for r in &anb.triggers.ranges {
            let p = |i: usize| r.params.get(i).copied().unwrap_or(0);
            let deg = |v: i16| (v as f32).to_radians();
            let kind = match r.id {
                range_id::ATTACK_HITBOX | range_id::ATTACK_HITBOX_COMBO => {
                    let attack = p(0) as u16;
                    let is_guard = combat.battle.attack_for_level(attack, 0, 99).map_or(false, |a| matches!(a.kind, 2 | 3));
                    if is_guard {
                        WindowKind::Guard { attack, group: p(1) as u8, weapon: on_weapon }
                    } else {
                        let combo = (r.id == range_id::ATTACK_HITBOX_COMBO).then(|| p(2));
                        WindowKind::Hitbox { attack, group: p(1) as u8, combo, weapon: on_weapon }
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

/// Collision entries as bone-attached shapes.
pub fn bone_shapes(collision: &[ObjectCollision]) -> Vec<BoneShape> {
    collision
        .iter()
        .map(|c| BoneShape {
            group: c.group,
            kind: c.kind,
            shape: c.shape,
            argument: c.argument,
            bone: (c.bone >= 0 && c.bone & 0x4000 == 0).then_some(c.bone as usize),
            offset: [c.position[0] as f32, c.position[1] as f32, c.position[2] as f32],
            lift: c.position_height as f32,
            radius: c.radius as f32,
            height: c.height as f32,
        })
        .collect()
}

pub(crate) fn pose_motions(mset: &Mset) -> HashMap<u16, Arc<Interpolated>> {
    let mut out = HashMap::new();
    for m in 0..=255u16 {
        if let Some(p) = mset.slot_for(m as usize, true, true).and_then(|s| s.anb.as_ref()).and_then(|a| a.pose.clone()) {
            out.insert(m, Arc::new(p));
        }
    }
    out
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
        load_sora_with(source, &|name| self.read_any(source, name))
    }
}

/// Loads Sora from files already extracted into `dir` (by their base names,
/// e.g. `P_EX100.mset`, `00battle.bin`). For local research and tests only.
pub fn load_sora_dir(dir: &std::path::Path) -> Result<SoraData> {
    load_sora_with(Source::Live, &|name| {
        let base = name.rsplit('/').next().unwrap_or(name);
        Ok(std::fs::read(dir.join(base))?)
    })
}

fn load_sora_with(source: Source, read: &dyn Fn(&str) -> Result<Vec<u8>>) -> Result<SoraData> {
    let combat = CombatData { battle: kh2_formats::battle::Battle::parse(&read("00battle.bin")?)?, source };
    let pref = kh2_formats::pref::Pref::from_03system(&read("03system.bin")?)?;
    let player = Mset::parse(&read(PLAYER_MSET)?)?;
    let weapon = Mset::parse(&read(WEAPON_MSET)?)?;
    let mdlx = read(WEAPON_MDLX)?;
    let collision = kh2_formats::motion::parse_model_collision(&mdlx)?;

    let clips = moves(&player, &weapon, &combat);
    let mut motions: Vec<u16> = clips.keys().copied().collect();
    motions.sort();
    let combo_rows = combat.battle.ptya.set(SORA_MOVESET).unwrap_or(&[]).iter().map(combo_row).collect();
    let moveset = Moveset { clips, combo_rows };
    let character = Character::new(moveset, move_params(&pref), growth_levels(&pref), attack_params(&pref));
    let (rig, rig_error) = match load_rig(read, &player, &weapon, &collision) {
        Ok(r) => (Some(Arc::new(r)), None),
        Err(e) => (None, Some(e.to_string())),
    };
    let mut character = character;
    character.reaction = reaction_params(&pref);
    // The background-collision radius is the model's type-4 (BG_PLAYER)
    // shape; the game reads it the same way.
    if let Some(r) = rig.as_ref().and_then(|r| r.body_shapes.iter().find(|s| s.kind == 4)) {
        character.params.hang.body_radius = r.radius;
    }
    let weight = read("00objentry.bin")
        .ok()
        .and_then(|b| kh2_formats::objentry::parse_objentry(&b).ok())
        .and_then(|o| o.into_iter().find(|o| o.model == PLAYER_OBJECT))
        .map_or(100.0, |o| o.weight);
    Ok(SoraData { character, combat, blade_groups: blade_groups(&collision), motions, rig, rig_error, weight, magi: pref.magi.clone() })
}

fn load_rig(read: &dyn Fn(&str) -> Result<Vec<u8>>, player: &Mset, weapon: &Mset, weapon_collision: &[ObjectCollision]) -> Result<Rig> {
    let player_mdlx = read(PLAYER_MDLX)?;
    let skeleton = parse_skeleton(&player_mdlx)?;
    let weapon_skeleton = parse_skeleton(&read(WEAPON_MDLX)?)?;
    let body = kh2_formats::motion::parse_model_collision(&player_mdlx)?;
    let objects = kh2_formats::objentry::parse_objentry(&read("00objentry.bin")?)?;
    let sklt = kh2_formats::objentry::parse_sklt(&read("03system.bin")?)?;
    let weapon_bone = objects
        .iter()
        .find(|o| o.model == PLAYER_OBJECT)
        .and_then(|o| sklt.iter().find(|s| s.id == o.weapon_joint as u32))
        .and_then(|s| (s.bones[0] >= 0).then_some(s.bones[0] as usize))
        .filter(|&b| b < skeleton.len());
    Ok(Rig {
        motions: pose_motions(player),
        weapon_motions: pose_motions(weapon),
        weapon_bone,
        weapon_shapes: bone_shapes(weapon_collision),
        body_shapes: bone_shapes(&body),
        skeleton,
        weapon_skeleton,
    })
}
