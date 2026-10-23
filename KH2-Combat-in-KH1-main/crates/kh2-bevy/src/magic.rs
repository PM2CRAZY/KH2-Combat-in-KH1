//! Spells in the playground. Each spell's behaviour comes from its own
//! file's script, run by `kh2-magic`; this module answers the scripts'
//! questions about the arena, Sora and the Soldiers, and applies what the
//! spells' attacks do (damage, reactions, Magnet's pull, Cure's healing,
//! Reflect's barrier).
//!
//! Particles are not drawn: each effect is a small glowing orb at the
//! effect's real position, and each spell attack is drawn as its real hit
//! volume.

use std::collections::HashMap;

use bevy::prelude::*;
use combat_core::{first_overlap, HitRecord, Shape as CShape};
use kh2_data::kit::MenuKit;
use kh2_formats::battle::{Atkp, Battle};
use kh2_magic::{Event, Magic, ObjId, SpellFile, World};

use crate::rig::RigAnim;
use crate::world::Arena;
use crate::SCALE;

/// Sora's object id in the spell scripts' world. Soldiers use their own ids.
pub const SORA: ObjId = 1;

pub struct Spells {
    pub magic: Magic,
    /// Kit spell index -> loaded spell file.
    pub file_of: Vec<Option<usize>>,
    /// Loaded spell file -> MAGC element (0 Fire .. 5 Reflect).
    pub element_of: Vec<u8>,
    pub magi: kh2_formats::pref::MagiTable,
    pub rng: u32,
    /// Sora's recoil slide: direction and per-frame speed.
    pub recoil: Option<(Vec3, f32)>,
    /// Drive Form id -> its object script (Wisdom Form's shots).
    pub object_scripts: HashMap<u8, usize>,
}

impl Spells {
    /// Loads every spell file the kit lists (`read` returns a file's bytes).
    pub fn load(kit: &MenuKit, magi: kh2_formats::pref::MagiTable, read: impl Fn(&str) -> Option<Vec<u8>>) -> Spells {
        let mut magic = Magic::new();
        let mut loaded: HashMap<String, usize> = HashMap::new();
        let mut element_of = Vec::new();
        let mut file_of = Vec::new();
        for s in &kit.spells {
            let idx = match loaded.get(&s.file) {
                Some(i) => Some(*i),
                None => match read(&s.file).map(|b| SpellFile::parse(&s.file, &b)) {
                    Some(Ok(f)) => {
                        let i = magic.add_spell(f);
                        element_of.push(s.element);
                        loaded.insert(s.file.clone(), i);
                        Some(i)
                    }
                    Some(Err(e)) => {
                        eprintln!("note: {} not usable ({e})", s.file);
                        None
                    }
                    None => {
                        eprintln!("note: {} not found", s.file);
                        None
                    }
                },
            };
            file_of.push(idx);
        }
        let mut sp = Spells { magic, file_of, element_of, magi, rng: 0x1234_5678, recoil: None, object_scripts: HashMap::new() };
        // Forms whose model carries a script (Wisdom: `wisd`), with the
        // effects from their effect archive.
        for (form, model) in [(2u8, "P_EX100_MAGF")] {
            let script = read(&format!("obj/{model}.mdlx")).and_then(|b| SpellFile::bar_entry(&b, 3));
            let pax = read(&format!("obj/{model}.a.us")).and_then(|b| SpellFile::bar_entry(&b, 18));
            match (script, pax) {
                (Some(s), Some(p)) => match SpellFile::from_parts(model, &s, &p) {
                    Ok(f) => {
                        let i = sp.magic.add_spell(f);
                        sp.element_of.push(255);
                        sp.object_scripts.insert(form, i);
                    }
                    Err(e) => eprintln!("note: {model} script not usable ({e})"),
                },
                _ => eprintln!("note: {model} script or effects not found (Wisdom Form fires no shots)"),
            }
        }
        sp
    }

}

pub fn rand(rng: &mut u32) -> f32 {
    *rng ^= *rng << 13;
    *rng ^= *rng >> 17;
    *rng ^= *rng << 5;
    (*rng >> 8) as f32 / (1u32 << 24) as f32
}

/// An object as the scripts see it this frame.
#[derive(Clone)]
pub struct Snap {
    pub id: ObjId,
    pub pos: Vec3,
    pub facing: f32,
    pub motion: u16,
    pub alive: bool,
    pub airborne: bool,
    pub jumping: bool,
    /// The target-type shape's centre (where to aim).
    pub target: Vec3,
    /// The object-type shape's centre (the body centre).
    pub center: Vec3,
    pub bones: Option<Bones>,
}

#[derive(Clone)]
pub struct Bones {
    pub to_world: Mat4,
    pub bones: Vec<Mat4>,
    pub weapons: Vec<Vec<Mat4>>,
}

impl Bones {
    pub fn of(anim: &RigAnim) -> Bones {
        Bones { to_world: anim.to_world, bones: anim.bones.clone(), weapons: anim.weapons.clone() }
    }
}

impl Snap {
    pub fn from_anim(id: ObjId, pos: [f32; 3], facing: f32, motion: u16, alive: bool, airborne: bool, jumping: bool, anim: Option<&RigAnim>, fallback_height: f32) -> Snap {
        let pos = Vec3::from(pos);
        let shape_center = |kind: u8| anim.and_then(|a| a.shapes_of_kind(kind).into_iter().find(|(_, w)| *w == 0).map(|(s, _)| Vec3::from(s.center)));
        let mid = pos + Vec3::Y * fallback_height;
        Snap {
            id,
            pos,
            facing,
            motion,
            alive,
            airborne,
            jumping,
            target: shape_center(crate::rig::kind::TARGET).unwrap_or(mid),
            center: shape_center(crate::rig::kind::OBJ).unwrap_or(mid),
            bones: anim.map(Bones::of),
        }
    }

    fn frame(&self, at: Vec3) -> Mat4 {
        Mat4::from_rotation_translation(Quat::from_rotation_y(self.facing), at)
    }

    /// A bone or special anchor's world transform (see `kh2_magic::World`).
    fn bone(&self, code: u16) -> Mat4 {
        // Odd codes are the right-hand weapon, even ones the left (falling
        // back to the right when there is one weapon).
        let weapon_ends = || {
            let b = self.bones.as_ref()?;
            let side = (code - 0x4003) as usize % 2;
            let w = b.weapons.get(side).filter(|w| !w.is_empty()).or_else(|| b.weapons.first())?;
            let first = w.first()?.w_axis.truncate();
            let last = w.last()?.w_axis.truncate();
            Some((b.to_world.transform_point3(first), b.to_world.transform_point3(last)))
        };
        match code {
            0x4000 => self.frame(self.pos),
            0x4001 => self.frame(self.center),
            0x4002 => self.bones.as_ref().map_or(self.frame(self.pos), |b| b.to_world),
            0x4003..=0x4008 => match weapon_ends() {
                Some((base, tip)) => {
                    let at = match code {
                        0x4003 | 0x4004 => tip,
                        0x4005 | 0x4006 => base,
                        _ => (base + tip) * 0.5,
                    };
                    self.frame(at)
                }
                None => self.frame(self.center),
            },
            0x4009 => self.frame(self.target),
            c if c < 0x4000 => match self.bones.as_ref().and_then(|b| b.bones.get(c as usize).map(|m| b.to_world * *m)) {
                Some(m) => m,
                None => self.frame(self.center),
            },
            _ => self.frame(self.center),
        }
    }
}

pub struct PlayWorld<'a> {
    pub objs: Vec<Snap>,
    pub arena: &'a Arena,
    /// Ability numbers Sora has equipped.
    pub abilities: &'a [u16],
    /// Sora's current target.
    pub target: Option<ObjId>,
    pub rng: &'a mut u32,
    pub hitbacks: Vec<(ObjId, Vec3, f32)>,
}

impl PlayWorld<'_> {
    fn get(&self, o: ObjId) -> Option<&Snap> {
        self.objs.iter().find(|s| s.id == o)
    }
}

impl World for PlayWorld<'_> {
    fn exists(&self, o: ObjId) -> bool {
        self.get(o).is_some_and(|s| s.alive)
    }
    fn pos(&self, o: ObjId) -> Vec3 {
        self.get(o).map_or(Vec3::ZERO, |s| s.pos)
    }
    fn facing(&self, o: ObjId) -> f32 {
        self.get(o).map_or(0.0, |s| s.facing)
    }
    fn motion(&self, o: ObjId) -> u16 {
        self.get(o).map_or(0, |s| s.motion)
    }
    fn airborne(&self, o: ObjId) -> bool {
        self.get(o).is_some_and(|s| s.airborne)
    }
    fn jumping(&self, o: ObjId) -> bool {
        self.get(o).is_some_and(|s| s.jumping)
    }
    fn bone(&self, o: ObjId, code: u16) -> Option<Mat4> {
        self.get(o).map(|s| s.bone(code))
    }
    fn target_point(&self, o: ObjId) -> Vec3 {
        self.get(o).map_or(Vec3::ZERO, |s| s.target)
    }
    fn target_of(&self, o: ObjId) -> Option<ObjId> {
        if o == SORA {
            self.target
        } else {
            Some(SORA)
        }
    }
    fn party(&self, n: u32) -> Option<ObjId> {
        (n == 0).then_some(SORA)
    }
    fn has_ability(&self, o: ObjId, n: u32) -> bool {
        o == SORA && self.abilities.contains(&(n as u16))
    }
    fn ground(&self, from: Vec3, depth: f32) -> Option<Vec3> {
        self.arena.raycast(from, from - Vec3::Y * depth)
    }
    fn bg_hit(&self, from: Vec3, to: Vec3) -> bool {
        self.arena.raycast(from, to).is_some()
    }
    fn hitback(&mut self, o: ObjId, dir: Vec3, dist: f32) {
        self.hitbacks.push((o, dir, dist));
    }
    fn random(&mut self) -> f32 {
        rand(self.rng)
    }
}

/// The ATKP row an attack uses on its `count`-th hit in a row (exe).
pub fn row(battle: &Battle, id: u16, sub: u8, count: u32) -> Option<&Atkp> {
    battle.attack_for_level(id, sub, count.min(255) as u8)
}

/// Spell attacks' keys in hit records (kept apart from Sora's and the
/// Soldiers' swing ids).
pub const SPELL_KEY: u64 = 1 << 60;

/// What a spell attack did to a Soldier, for the log and the reaction.
pub struct SpellHit {
    /// The attack object.
    pub key: u64,
    pub spell: usize,
    pub foe: u32,
    pub atk: u16,
    /// Hits in a row from this attack's group (0-based).
    pub count: u32,
    pub row: Atkp,
    /// From the attack's centre toward the Soldier (x, z).
    pub dir: [f32; 2],
}

/// Tests every live spell attack against one Soldier's hurt shapes using
/// its hit record (once per attack until the row's re-hit interval runs
/// out; rows picked by hits in a row from the same group). Returns the
/// hits that count.
pub fn hits_on(magic: &mut Magic, battle: &Battle, foe: u32, foe_pos: Vec3, hurt: &[CShape], record: &mut HitRecord) -> Vec<SpellHit> {
    let mut out = Vec::new();
    for a in magic.attacks.iter_mut().filter(|a| !a.freed && a.owner == SORA) {
        if record.has_hit(a.id | SPELL_KEY) {
            continue;
        }
        let shapes = a.shapes();
        if shapes.is_empty() || first_overlap(&shapes, hurt).is_none() {
            continue;
        }
        let Some(first) = row(battle, a.atk, a.sub, 0) else { continue };
        // Reflect barriers (kind 3) and guards don't hurt.
        if matches!(first.kind, 2 | 3) {
            continue;
        }
        let group = if first.combo_group != 0 { first.combo_group as i64 } else { a.group };
        let adm = record.admit(group, matches!(first.kind, 5 | 6), false);
        let Some(r) = row(battle, a.atk, a.sub, adm.count).cloned() else { continue };
        record.mark(a.id | SPELL_KEY, r.interval);
        a.hit = true;
        if !adm.effective {
            continue;
        }
        let c = a.pos.unwrap_or(foe_pos);
        let d = Vec2::new(foe_pos.x - c.x, foe_pos.z - c.z).normalize_or(Vec2::Y);
        out.push(SpellHit { key: a.key, spell: a.spell, foe, atk: a.atk, count: adm.count, row: r, dir: [d.x, d.y] });
    }
    out
}

/// Marks spell attacks that ran into walls or the floor this frame (ATKP
/// flag bit 0, like the Keyblade's wall strikes).
pub fn bg_hits(magic: &mut Magic, battle: &Battle, arena: &Arena) {
    for a in magic.attacks.iter_mut().filter(|a| !a.freed && a.moved && !a.hit_bg) {
        let (Some(p), Some(q)) = (a.prev, a.pos) else { continue };
        let strikes = row(battle, a.atk, a.sub, 0).is_some_and(|r| r.flags & 1 != 0);
        if strikes && arena.raycast(p, q).is_some() {
            a.hit_bg = true;
        }
    }
}

/// Live Reflect barriers: (attack key, its ATKP row, shapes).
pub fn barriers(magic: &Magic, battle: &Battle) -> Vec<(u64, Atkp, Vec<CShape>)> {
    magic
        .attacks
        .iter()
        .filter(|a| !a.freed && a.owner == SORA)
        .filter_map(|a| {
            let r = row(battle, a.atk, a.sub, 0)?;
            (r.kind == 3).then(|| (a.key, r.clone(), a.shapes()))
        })
        .filter(|(_, _, s)| !s.is_empty())
        .collect()
}

pub fn handle_events(events: Vec<Event>, log: &mut Vec<String>) -> Vec<(u64, ObjId)> {
    let mut strikes = Vec::new();
    for e in events {
        match e {
            Event::Strike { attack, target } => strikes.push((attack, target)),
            Event::Missing(n) => log.push(format!("spell script used {n} (not modelled)")),
            Event::Error(s) => log.push(format!("spell script error: {s}")),
        }
    }
    strikes
}

// ---- drawing ----

/// Element colours for orbs and hit volumes.
pub fn color(element: u8) -> Color {
    match element {
        0 => Color::srgb(1.0, 0.45, 0.1),
        1 => Color::srgb(0.55, 0.85, 1.0),
        2 => Color::srgb(1.0, 0.95, 0.3),
        3 => Color::srgb(0.4, 1.0, 0.5),
        4 => Color::srgb(0.75, 0.45, 1.0),
        _ => Color::srgb(0.85, 0.9, 1.0),
    }
}

#[derive(Component)]
pub struct EffectOrb(pub usize);
#[derive(Component)]
pub struct AttackVolume(pub usize);

const ORBS: usize = 48;
const VOLUMES: usize = 16;

pub fn spawn_pool(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    let sphere = meshes.add(Sphere::new(1.0).mesh().uv(16, 10));
    for i in 0..ORBS {
        let m = mats.add(StandardMaterial { base_color: Color::WHITE, emissive: LinearRgba::WHITE, unlit: true, alpha_mode: AlphaMode::Blend, ..default() });
        commands.spawn((Mesh3d(sphere.clone()), MeshMaterial3d(m), Transform::default(), Visibility::Hidden, EffectOrb(i), bevy::pbr::NotShadowCaster));
    }
    for i in 0..VOLUMES {
        let m = mats.add(StandardMaterial { base_color: Color::WHITE, unlit: true, alpha_mode: AlphaMode::Blend, cull_mode: None, ..default() });
        commands.spawn((Mesh3d(sphere.clone()), MeshMaterial3d(m), Transform::default(), Visibility::Hidden, AttackVolume(i), bevy::pbr::NotShadowCaster));
    }
}

pub fn draw(
    game: Res<crate::Game>,
    mut orbs: Query<(&EffectOrb, &mut Transform, &mut Visibility, &MeshMaterial3d<StandardMaterial>), Without<AttackVolume>>,
    mut vols: Query<(&AttackVolume, &mut Transform, &mut Visibility, &MeshMaterial3d<StandardMaterial>), Without<EffectOrb>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let Some(sp) = game.spells.as_ref() else { return };
    let effects: Vec<_> = sp.magic.effects.iter().filter(|e| e.alive()).collect();
    for (o, mut t, mut vis, mat) in &mut orbs {
        match effects.get(o.0) {
            Some(e) => {
                let el = sp.element_of.get(e.spell).copied().unwrap_or(5);
                let s = 14.0 * e.scale.max_element().clamp(0.3, 4.0);
                *t = Transform::from_translation(e.pos * SCALE).with_scale(Vec3::splat(s * SCALE));
                *vis = Visibility::Inherited;
                if let Some(m) = mats.get_mut(&mat.0) {
                    let c = color(el);
                    m.base_color = c.with_alpha(0.85 * e.alpha());
                    m.emissive = c.to_linear() * 2.0;
                }
            }
            None => *vis = Visibility::Hidden,
        }
    }
    let attacks: Vec<_> = sp.magic.attacks.iter().filter(|a| !a.freed && a.pos.is_some() && a.radius > 0.0).collect();
    for (v, mut t, mut vis, mat) in &mut vols {
        match attacks.get(v.0) {
            Some(a) => {
                let el = sp.element_of.get(a.spell).copied().unwrap_or(5);
                let p = a.pos.unwrap_or_default();
                *t = Transform::from_translation(p * SCALE).with_scale(Vec3::new(a.radius, a.height, a.radius) * SCALE);
                *vis = Visibility::Inherited;
                if let Some(m) = mats.get_mut(&mat.0) {
                    m.base_color = color(el).with_alpha(if a.hit { 0.2 } else { 0.12 });
                }
            }
            None => *vis = Visibility::Hidden,
        }
    }
}
