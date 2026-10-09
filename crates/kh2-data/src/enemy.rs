//! Enemies read from the user's install: an enemy's own moveset (motions
//! with their attack windows), skeleton and collision shapes, plus its
//! stats. Nothing here is a mesh or texture.

use std::collections::HashMap;
use std::sync::Arc;

use character_core::reaction::ReactionParams;
use character_core::Clip;
use kh2_formats::anim::parse_skeleton;
use kh2_formats::motion::Mset;

use crate::sora::{bone_shapes, clip_from_anb, pose_motions, reaction_params, Rig};
use crate::{CombatData, Install, Result, Source};

/// The Soldier (objentry model name; ENMP 33, status 1000). It has the
/// Cyclone reaction command.
pub const SOLDIER: &str = "M_EX010";
/// The Shadow (ENMP 34).
pub const SHADOW: &str = "M_EX020";

/// One enemy kind.
#[derive(Clone, Debug)]
pub struct EnemyData {
    pub model: String,
    /// Motion id -> clip. Enemy movesets index motions directly by slot.
    pub clips: HashMap<u16, Clip>,
    /// Skeleton, motions and collision shapes (no weapon).
    pub rig: Arc<Rig>,
    /// objentry weight.
    pub weight: f32,
    /// objentry enemy status id (Heartless 1000.., Nobodies 3000..).
    pub status: u16,
    pub reaction: ReactionParams,
    /// Collision groups on by default (the model collision header).
    pub groups: u32,
    /// Capture slots (reaction commands): slot -> the motions other
    /// characters play when captured (see [`Capture`]).
    pub captures: HashMap<usize, Vec<(String, kh2_formats::motion::Anb)>>,
}

/// A captured motion as a clip for the captured character: the body
/// motion (tag `<id>0`) with the weapons' windows (`<id>1`, `<id>2`) and
/// their poses. Character ids: "001" Sora and his Drive Forms, "027" Limit
/// Form (objentry byte 7).
#[derive(Clone, Debug)]
pub struct Capture {
    pub clip: Clip,
    pub body: Option<Arc<kh2_formats::anim::Interpolated>>,
    pub weapons: Vec<Option<Arc<kh2_formats::anim::Interpolated>>>,
}

impl EnemyData {
    /// Slot `slot` captured onto a character with id tag `who` ("001"),
    /// as motion `motion`.
    pub fn capture(&self, slot: usize, who: &str, motion: u16, combat: &CombatData) -> Option<Capture> {
        let set = self.captures.get(&slot)?;
        let tag = |n: u8| set.iter().find(|(t, _)| *t == format!("{who}{n}")).map(|(_, a)| a);
        let body = tag(0)?;
        let weapons: Vec<Option<&kh2_formats::motion::Anb>> = vec![tag(1), tag(2)];
        let clip = clip_from_anb(motion, body, &weapons, combat);
        Some(Capture {
            clip,
            body: body.pose.clone().map(Arc::new),
            weapons: weapons.iter().map(|w| w.and_then(|a| a.pose.clone()).map(Arc::new)).collect(),
        })
    }
}

impl EnemyData {
    /// Radius of the body (OBJ, type 1) shape, for pushing bodies apart.
    pub fn body_radius(&self) -> f32 {
        self.rig.body_shapes.iter().find(|s| s.kind == 1).map_or(50.0, |s| s.radius)
    }

    /// The map-collision shape (see [`crate::sora::bg_shape`]).
    pub fn bg_shape(&self) -> Option<&crate::sora::BoneShape> {
        crate::sora::bg_shape(&self.rig.body_shapes)
    }
}

pub(crate) fn load_enemy_with(read: &dyn Fn(&str) -> Result<Vec<u8>>, model: &str, combat: &CombatData) -> Result<EnemyData> {
    let mdlx = read(&format!("obj/{model}.mdlx"))?;
    let mset = Mset::parse(&read(&format!("obj/{model}.mset"))?)?;
    let pref = kh2_formats::pref::Pref::from_03system(&read("03system.bin")?)?;
    let objects = kh2_formats::objentry::parse_objentry(&read("00objentry.bin")?)?;
    let weight = objects.iter().find(|o| o.model == model).map_or(0.0, |o| o.weight);
    let status = objects.iter().find(|o| o.model == model).map_or(0, |o| o.neo_status);
    let mut clips = HashMap::new();
    for s in &mset.slots {
        if let Some(a) = &s.anb {
            let m = s.index as u16;
            clips.insert(m, clip_from_anb(m, a, &[], combat));
        }
    }
    let collision = kh2_formats::motion::parse_model_collision(&mdlx)?;
    let rig = Rig {
        skeleton: parse_skeleton(&mdlx)?,
        motions: pose_motions(&mset),
        field_motions: HashMap::new(),
        weapons: Vec::new(),
        body_shapes: bone_shapes(&collision),
    };
    let groups = kh2_formats::motion::model_collision_groups(&mdlx).unwrap_or(1);
    let captures = mset.slots.iter().filter(|s| !s.nested.is_empty()).map(|s| (s.index, s.nested.clone())).collect();
    Ok(EnemyData { model: model.to_string(), clips, rig: Arc::new(rig), weight, status, reaction: reaction_params(&pref), groups, captures })
}

impl Install {
    pub fn load_enemy(&self, source: Source, model: &str, combat: &CombatData) -> Result<EnemyData> {
        load_enemy_with(&|name| self.read_any(source, name), model, combat)
    }
}

/// From files extracted into `dir` (base names). Research and tests only.
pub fn load_enemy_dir(dir: &std::path::Path, model: &str, combat: &CombatData) -> Result<EnemyData> {
    load_enemy_with(
        &|name| {
            let base = name.rsplit('/').next().unwrap_or(name);
            Ok(std::fs::read(dir.join(base))?)
        },
        model,
        combat,
    )
}
