//! Reaction commands, the way the game offers and runs them (see
//! `docs/reaction-commands.md`):
//! - **Offer.** Every frame the best single candidate is rebuilt from
//!   scratch. Enemies' reaction-type collision shapes (type 5; the shape's
//!   argument is the command) count while their group is switched on, and
//!   when they overlap Sora's body shape. The highest priority wins, then
//!   the nearest. With nothing else and HP at a quarter or less, a Drive
//!   Form is offered.
//! - **Run.** The enemy's script does the work: it makes Sora play a
//!   "captured" motion stored in the enemy's own motion set, which carries
//!   Sora's animation, invulnerability and hitboxes.

use bevy::prelude::*;
use character_core::Clip;
use combat_core::first_overlap;
use kh2_data::enemy::EnemyData;
use kh2_data::sora::SoraData;

use crate::rig::kind;
use crate::{drive, Dummy, Game, SCALE};

/// Captured motions are registered on the player as motion `1000 + slot`.
pub const CAPTURE_BASE: u16 = 1000;
/// The low-HP Drive offers: Final, Master, Limit, Wisdom, Valor (cmd ids).
const LOW_HP_DRIVES: [u16; 5] = [553, 552, 674, 551, 550];
/// cmd flags.
const LAND: i32 = 0x2;

/// Adds an enemy's capture motions to a player object (Sora or a form):
/// clips for the controller, poses for the rig.
pub fn register_captures(data: &mut SoraData, enemy: &EnemyData, who: &str) {
    for &slot in enemy.captures.keys() {
        let id = CAPTURE_BASE + slot as u16;
        let Some(c) = enemy.capture(slot, who, id, &data.combat) else { continue };
        data.character.moveset.clips.insert(id, c.clip);
        if let Some(rig) = data.rig.as_mut() {
            let rig = std::sync::Arc::make_mut(rig);
            if let Some(b) = c.body {
                rig.motions.insert(id, b);
            }
            for (w, m) in rig.weapons.iter_mut().zip(c.weapons) {
                if let Some(m) = m {
                    w.motions.insert(id, m);
                }
            }
        }
    }
}

/// The command shown in the reaction slot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Offer {
    pub cmd: u16,
    /// The enemy it acts on (0: none, e.g. a Drive).
    pub target: u32,
    /// Shown but not usable (greyed).
    pub greyed: bool,
    /// Kept on screen while it runs (not usable).
    pub running: bool,
}

/// A reaction command in progress (the enemy script's side).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Running {
    pub cmd: u16,
    pub foe: u32,
    pub stage: u8,
}

#[derive(Default)]
pub struct RcState {
    pub offer: Option<Offer>,
    pub running: Option<Running>,
    rng: u32,
}

/// Rebuilds the offer for this frame.
pub fn select(game: &mut Game, dummies: &Query<&mut Dummy>) {
    let Some(kit) = game.kit.as_ref() else { return };
    let prev = game.rc.offer;
    if let Some(r) = game.rc.running {
        game.rc.offer = Some(Offer { cmd: r.cmd, target: r.foe, greyed: true, running: true });
        return;
    }
    let mut best: Option<(u8, f32, Offer)> = None;
    let sora_shape = game.anim.as_ref().and_then(|a| a.shapes_of_kind(kind::OBJ).into_iter().find(|(_, w)| *w == 0).map(|(s, _)| s));
    let airborne = !game.grounded;
    if let (Some(me), Some(fk)) = (sora_shape, game.foe_kit.as_ref()) {
        for dm in dummies.iter() {
            let Some(st) = dm.foe.as_ref() else { continue };
            if dm.respawn > 0 || st.npc.is_dead() {
                continue;
            }
            let groups = enabled_groups(&fk.data, st.out.as_ref().map(|o| (o.motion, o.clip_frame)));
            for s in fk.data.rig.body_shapes.iter().filter(|s| s.kind == 5 && s.argument > 0) {
                if groups & (1u32 << (s.group & 31)) == 0 {
                    continue;
                }
                let cmd = s.argument as u16;
                let Some((row, _)) = kit.commands.get(&cmd) else { continue };
                let Some(shape) = st.anim.world_shape(s, 0) else { continue };
                if first_overlap(&[shape], &[me]).is_none() {
                    continue;
                }
                let (dx, dz) = (shape.center[0] - me.center[0], shape.center[2] - me.center[2]);
                let mut score = (dx * dx + dz * dz).sqrt();
                if game.character.lock() == Some(dm.id) {
                    score *= 0.5;
                } else if prev.is_some_and(|p| p.target == dm.id) {
                    score *= 0.8;
                }
                let greyed = row.flags & LAND != 0 && airborne;
                let o = Offer { cmd, target: dm.id, greyed, running: false };
                let better = match &best {
                    None => true,
                    Some((p, sc, _)) => row.priority > *p || (row.priority == *p && score < *sc),
                };
                if better {
                    best = Some((row.priority, score, o));
                }
            }
        }
    }
    let mut offer = best.map(|b| b.2);
    // Fallback: a Drive Form at low HP (in the game's order; a random one,
    // kept while it stays available).
    if offer.is_none() && game.sora.hp * 4 <= game.sora.max_hp && game.drive.form() == 0 {
        let ok: Vec<u16> = LOW_HP_DRIVES.iter().copied().filter(|c| kit.commands.get(c).is_some_and(|(r, _)| drive::available(game, r.argument as u8))).collect();
        if !ok.is_empty() {
            let keep = prev.filter(|p| ok.contains(&p.cmd)).map(|p| p.cmd);
            let cmd = keep.unwrap_or_else(|| {
                game.rc.rng = game.rc.rng.wrapping_mul(0x10dcd).wrapping_add(1);
                ok[(game.rc.rng >> 8) as usize % ok.len()]
            });
            offer = Some(Offer { cmd, target: 0, greyed: false, running: false });
        }
    }
    game.rc.offer = offer;
}

/// Collision groups switched on for an enemy in a motion at a frame: the
/// model's defaults, then the motion's enable/disable windows.
fn enabled_groups(data: &EnemyData, at: Option<(u16, u32)>) -> u32 {
    let mut g = data.groups;
    if let Some(clip) = at.and_then(|(m, f)| data.clips.get(&m).map(|c| (c, f))) {
        let (c, f): (&Clip, u32) = clip;
        for w in c.windows.iter().filter(|w| w.contains(f)) {
            match w.kind {
                character_core::WindowKind::EnableGroup(n) => g |= 1 << (n & 31),
                character_core::WindowKind::DisableGroup(n) => g &= !(1 << (n & 31)),
                _ => {}
            }
        }
    }
    g
}

/// The reaction button: runs the offered command if it can start now.
pub fn press(game: &mut Game, battle: bool, log: &mut Vec<String>) {
    let Some(o) = game.rc.offer else { return };
    if o.greyed || o.running || !game.character.can_take_command() || game.drive.busy() {
        return;
    }
    let Some((row, name)) = game.kit.as_ref().and_then(|k| k.commands.get(&o.cmd)).cloned() else { return };
    match row.execute {
        // A Drive (the low-HP offer).
        4 => {
            drive::request(game, row.argument as u8, battle, log);
        }
        _ => {
            // The enemy's script runs it (trigger 11). Ours: the Soldier's
            // Cyclone, as its script does.
            if !start_enemy_rc(game, o.cmd, o.target) {
                log.push(format!("{name}: no handler for command {}", o.cmd));
                return;
            }
            log.push(format!("Reaction Command: {name}"));
        }
    }
    game.rc.offer = None;
}

/// The Soldier's trigger-11 handler for Cyclone (cmd 34), from its script:
/// capture slot 252 onto Sora, slide him sideways around the Soldier at
/// 30 per frame while it plays, then capture slot 253 (the spin attack).
fn start_enemy_rc(game: &mut Game, cmd: u16, foe: u32) -> bool {
    if cmd != 34 {
        return false;
    }
    if !game.character.play(CAPTURE_BASE + 252, false) {
        return false;
    }
    game.rc.running = Some(Running { cmd, foe, stage: 0 });
    true
}

/// While an RC runs: Sora's extra movement this frame, and the next stage
/// when a captured motion ends.
pub fn frame(game: &mut Game, dummies: &Query<&mut Dummy>, last_motion: u16) -> Vec3 {
    let Some(mut r) = game.rc.running else { return Vec3::ZERO };
    let playing = |m: u16| game.character.scripted() && last_motion == m;
    let mut push = Vec3::ZERO;
    match r.stage {
        // Interrupted (the capture is no longer Sora's motion): stop.
        0 if last_motion != CAPTURE_BASE + 252 => {
            game.rc.running = None;
            return push;
        }
        0 if playing(CAPTURE_BASE + 252) => {
            // dir = normalize(roty(foe − Sora, 90°)) at 30 per frame (the
            // game's move request adds dir × speed × frame time).
            if let Some(dm) = dummies.iter().find(|d| d.id == r.foe) {
                let d = Vec3::new(dm.pos[0] - game.pos[0], 0.0, dm.pos[2] - game.pos[2]);
                let rot = Vec3::new(d.z, 0.0, -d.x).normalize_or_zero();
                push = rot * 30.0;
            }
        }
        0 => {
            r.stage = 1;
            if !game.character.play(CAPTURE_BASE + 253, false) {
                game.rc.running = None;
                return push;
            }
        }
        _ if playing(CAPTURE_BASE + 253) => {}
        _ => {
            game.rc.running = None;
            return push;
        }
    }
    game.rc.running = Some(r);
    push
}

/// The mark over the enemy a reaction command acts on (our look).
pub fn draw_mark(game: Res<Game>, dummies: Query<&Dummy>, mut gizmos: Gizmos) {
    let Some(o) = game.rc.offer.filter(|o| !o.running && o.target != 0) else { return };
    let Some(dm) = dummies.iter().find(|d| d.id == o.target) else { return };
    let c = Vec3::new(dm.pos[0], dm.pos[1] + 190.0, dm.pos[2]) * SCALE;
    let r = 0.18;
    let col = if o.greyed { Color::srgb(0.5, 0.5, 0.5) } else { Color::srgb(0.3, 1.0, 0.4) };
    let pts = [c + Vec3::Y * r, c + Vec3::X * r, c - Vec3::Y * r, c - Vec3::X * r];
    for i in 0..4 {
        gizmos.line(pts[i], pts[(i + 1) % 4], col);
    }
}
