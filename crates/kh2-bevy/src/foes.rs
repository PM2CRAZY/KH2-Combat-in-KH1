//! Shadows read from the user's install: their own skeleton, motions,
//! collision shapes, attacks and stats, reacting to hits with the game's
//! generic reaction and attacking Sora. What they decide to do is ours (a
//! small timer-driven script); KH2 drives enemies with per-enemy scripts
//! that this project doesn't run.

use std::collections::{HashMap, HashSet};

use bevy::prelude::*;
use character_core::npc::{Npc, NpcCommand, NpcOutput};
use character_core::reaction::{self, Hit};
use character_core::ActiveHitbox;
use combat_core::{first_overlap, AttackDef, AttackVolume, Shape as CShape};
use kh2_data::enemy::EnemyData;
use kh2_formats::battle::Atkp;

use crate::rig::{Anchor, Paint, Part, Piece, RigAnim, Shape};
use crate::world::Arena;
use crate::{Dummy, SCALE};

/// What a Shadow needs at run time: its data plus the ATKP rows its attacks
/// use, at its level.
pub struct FoeKit {
    pub data: EnemyData,
    pub attacks: HashMap<u16, AttackDef>,
}

impl FoeKit {
    pub fn new(data: EnemyData, combat: &kh2_data::CombatData, level: u16) -> FoeKit {
        let mut attacks = HashMap::new();
        for c in data.clips.values() {
            for w in &c.windows {
                let id = match w.kind {
                    character_core::WindowKind::Hitbox { attack, .. } | character_core::WindowKind::Guard { attack, .. } => attack,
                    _ => continue,
                };
                if let Some(a) = combat.attack_at_level(id, 0, level as u8) {
                    attacks.insert(id, a);
                }
            }
        }
        FoeKit { data, attacks }
    }
}

/// Per-Shadow runtime state (on the [`Dummy`] component).
pub struct FoeState {
    pub npc: Npc,
    pub anim: RigAnim,
    pub out: Option<NpcOutput>,
    pub grounded: bool,
    pub cooldown: u32,
    rng: u32,
    pub volumes: HashMap<(u64, bool), AttackVolume>,
    /// Own swings that already landed or clashed.
    pub spent: HashSet<u64>,
    /// This frame's live attack (and guard) windows with their shapes.
    pub live: Vec<(ActiveHitbox, Vec<CShape>)>,
    pub guards: Vec<(ActiveHitbox, Vec<CShape>)>,
}

impl FoeState {
    pub fn new(kit: &FoeKit, seed: u32, facing: f32) -> FoeState {
        let mut npc = Npc::new(kit.data.clips.clone(), kit.data.reaction.clone());
        npc.set_facing(facing);
        FoeState {
            npc,
            anim: RigAnim::new(kit.data.rig.clone()),
            out: None,
            grounded: true,
            cooldown: 60 + seed % 90,
            rng: seed.wrapping_mul(2654435761).max(1),
            volumes: HashMap::new(),
            spent: HashSet::new(),
            live: Vec::new(),
            guards: Vec::new(),
        }
    }

    fn rand(&mut self, n: u32) -> u32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        self.rng % n.max(1)
    }

    pub fn hurt_shapes(&self) -> Vec<CShape> {
        self.anim.hurt_shapes()
    }
}

/// Shadow motion ids (slot numbers in `M_EX010.mset`).
pub mod shadow {
    /// A swipe.
    pub const SWIPE: u16 = 201;
    /// The sinking dash: guarded while sliding in, then a launch.
    pub const DASH: u16 = 203;
    /// Vanishing (used on KO).
    pub const DEATH: u16 = 44;
}

/// Our script: walk up to Sora, swipe when close, sometimes dash from
/// further away, then wait a while.
fn decide(st: &mut FoeState, pos: [f32; 3], sora: [f32; 3]) -> NpcCommand {
    let d = [sora[0] - pos[0], sora[2] - pos[2]];
    let dist = (d[0] * d[0] + d[1] * d[1]).sqrt();
    let dir = if dist > 1e-3 { [d[0] / dist, d[1] / dist] } else { [0.0, 1.0] };
    if st.npc.busy() {
        return NpcCommand::default();
    }
    st.cooldown = st.cooldown.saturating_sub(1);
    if dist > 900.0 {
        return NpcCommand::default();
    }
    if st.cooldown == 0 {
        if dist < 170.0 {
            st.cooldown = 90 + st.rand(90);
            return NpcCommand { face: Some(dir), action: Some(shadow::SWIPE), ..Default::default() };
        }
        if (300.0..550.0).contains(&dist) && st.rand(3) == 0 {
            st.cooldown = 150 + st.rand(90);
            return NpcCommand { face: Some(dir), action: Some(shadow::DASH), ..Default::default() };
        }
    }
    if dist > 150.0 {
        NpcCommand { walk: Some(dir), ..Default::default() }
    } else {
        NpcCommand { face: Some(dir), ..Default::default() }
    }
}

/// One frame for one Shadow: script, controller, movement, pose, shapes.
pub fn step(dm: &mut Dummy, kit: &FoeKit, sora_pos: [f32; 3], arena: &Arena, sora_radius: f32) {
    let Some(st) = dm.foe.as_mut() else { return };
    let cmd = decide(st, dm.pos, sora_pos);
    let out = st.npc.step(&cmd, st.grounded);
    let r = kit.data.body_radius();
    let mut p = dm.pos;
    p[0] += out.displacement[0];
    p[2] += out.displacement[2];
    p = arena.push_out(p, r, 30.0);
    // Keep out of Sora's body.
    let (dx, dz) = (p[0] - sora_pos[0], p[2] - sora_pos[2]);
    let dist = (dx * dx + dz * dz).sqrt();
    let min = r + sora_radius;
    if dist < min && (p[1] - sora_pos[1]).abs() < 120.0 {
        let (nx, nz) = if dist > 1e-3 { (dx / dist, dz / dist) } else { (0.0, 1.0) };
        p[0] = sora_pos[0] + nx * min;
        p[2] = sora_pos[2] + nz * min;
    }
    let from_y = p[1];
    p[1] += out.displacement[1];
    let ground = arena.ground_crossed(from_y, p, r);
    st.grounded = p[1] <= ground + 0.5 && out.displacement[1] <= 0.5;
    if st.grounded {
        p[1] = ground;
    }
    dm.pos = p;
    st.anim.update_npc(&out, &kit.data.clips, p);

    // Live attack shapes with their sweep copies.
    st.live.clear();
    st.guards.clear();
    let keys: HashSet<(u64, bool)> = out.hitboxes.iter().chain(&out.guard_boxes).map(|h| (h.swing, h.weapon)).collect();
    st.volumes.retain(|k, _| keys.contains(k));
    st.spent.retain(|s| keys.iter().any(|k| k.0 == *s));
    for (list, guard) in [(&out.hitboxes, false), (&out.guard_boxes, true)] {
        for h in list {
            let shapes = st.anim.attack_shapes(h.group, false);
            let swept = st.volumes.entry((h.swing, h.weapon)).or_default().update(&shapes);
            if guard {
                st.guards.push((*h, swept));
            } else {
                st.live.push((*h, swept));
            }
        }
    }
    st.out = Some(out);
}

/// Sora's swing connected with a Shadow: the reaction (damage is applied by
/// the caller).
pub fn react(dm: &mut Dummy, atk: &Atkp, attacker_facing: f32, killed: bool) {
    let Some(st) = dm.foe.as_mut() else { return };
    let hit = Hit {
        reaction: atk.enemy_reaction,
        knockback: (atk.knockback_1, atk.knockback_2),
        dir: [attacker_facing.sin(), attacker_facing.cos()],
        part: 0,
    };
    st.npc.take_hit(&hit);
    if killed {
        st.npc.die(shadow::DEATH);
    }
}

/// The two sides of a clash: each owner's reaction, from the game's rules.
/// Returns what Sora should do: `Some((mine_self, other_other))`.
pub fn foe_clash_reaction(st: &mut FoeState, own: &Atkp, sora_facing: f32) {
    // An enemy attack with "refact self" 2 carries on; otherwise its
    // "reflected" motion plays with a push of `reflect_hit_back` (-1 = 50).
    if own.refact_self == 2 {
        return;
    }
    if own.reflected_motion != 0 {
        let dist = if own.reflect_hit_back < 0 { 50.0 } else { own.reflect_hit_back as f32 };
        st.npc.repel(own.reflected_motion as u16, dist, [sora_facing.sin(), sora_facing.cos()]);
    }
}

pub fn clashes(a: &Atkp, b: &Atkp) -> bool {
    reaction::clashes(a.kind, b.kind)
}

pub fn overlap(a: &[CShape], b: &[CShape]) -> bool {
    first_overlap(a, b).is_some()
}

/// The mannequin for an enemy: a dark capsule on every bone segment and two
/// yellow eyes on the head (our own design).
pub fn pieces(kit: &FoeKit) -> Vec<Piece> {
    let mut out: Vec<Piece> = crate::rig::pieces(&kit.data.rig).into_iter().map(|p| Piece { paint: Paint::Shade, ..p }).collect();
    // Eyes: the bone with the highest bind position, offset forward.
    let bind = kh2_anim::Pose::bind(&kit.data.rig.skeleton);
    if let Some(head) = (0..kit.data.rig.skeleton.len()).max_by(|a, b| bind.bone_position(*a).y.total_cmp(&bind.bone_position(*b).y)) {
        for side in [-1.0f32, 1.0] {
            out.push(Piece { part: Part::Body, shape: Shape::Sphere(Anchor::Offset(head, Vec3::new(side * 7.0, -6.0, 9.0)), 3.2), paint: Paint::Eye });
        }
    }
    out
}

/// Links a mannequin piece to its Shadow.
#[derive(Component)]
pub struct FoePiece {
    pub owner: Entity,
    pub piece: Piece,
}

pub fn sync_pieces(dummies: Query<&Dummy>, mut pieces: Query<(&FoePiece, &mut Transform, &mut Visibility)>) {
    for (fp, mut t, mut vis) in &mut pieces {
        let Ok(dm) = dummies.get(fp.owner) else { continue };
        let Some(st) = &dm.foe else { continue };
        // Dead: shown while the vanish motion plays, then hidden until respawn.
        *vis = if dm.respawn > 0 && dm.respawn < 70 { Visibility::Hidden } else { Visibility::Inherited };
        let anim = &st.anim;
        match fp.piece.shape {
            Shape::Capsule(a, b, _) => {
                let (Some(a), Some(b)) = (anim.anchor(Part::Body, a), anim.anchor(Part::Body, b)) else { continue };
                let (a, b) = (a * SCALE, b * SCALE);
                *t = Transform { translation: (a + b) * 0.5, rotation: Quat::from_rotation_arc(Vec3::Y, (b - a).normalize_or(Vec3::Y)), scale: Vec3::ONE };
            }
            Shape::Sphere(c, r) => {
                let Some(c) = anim.anchor(Part::Body, c) else { continue };
                *t = Transform::from_translation(c * SCALE).with_scale(Vec3::splat(r * 2.0 * SCALE));
            }
        }
    }
}
