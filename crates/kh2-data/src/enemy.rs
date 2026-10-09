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

/// The Shadow (objentry model name).
pub const SHADOW: &str = "M_EX010";

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
    pub reaction: ReactionParams,
}

impl EnemyData {
    /// Radius of the body (OBJ, type 1) shape, for pushing bodies apart.
    pub fn body_radius(&self) -> f32 {
        self.rig.body_shapes.iter().find(|s| s.kind == 1).map_or(50.0, |s| s.radius)
    }
}

pub(crate) fn load_enemy_with(read: &dyn Fn(&str) -> Result<Vec<u8>>, model: &str, combat: &CombatData) -> Result<EnemyData> {
    let mdlx = read(&format!("obj/{model}.mdlx"))?;
    let mset = Mset::parse(&read(&format!("obj/{model}.mset"))?)?;
    let pref = kh2_formats::pref::Pref::from_03system(&read("03system.bin")?)?;
    let objects = kh2_formats::objentry::parse_objentry(&read("00objentry.bin")?)?;
    let weight = objects.iter().find(|o| o.model == model).map_or(0.0, |o| o.weight);
    let mut clips = HashMap::new();
    for s in &mset.slots {
        if let Some(a) = &s.anb {
            let m = s.index as u16;
            clips.insert(m, clip_from_anb(m, a, None, combat));
        }
    }
    let collision = kh2_formats::motion::parse_model_collision(&mdlx)?;
    let rig = Rig {
        skeleton: parse_skeleton(&mdlx)?,
        motions: pose_motions(&mset),
        weapon_skeleton: Vec::new(),
        weapon_motions: HashMap::new(),
        weapon_bone: None,
        weapon_shapes: Vec::new(),
        body_shapes: bone_shapes(&collision),
    };
    Ok(EnemyData { model: model.to_string(), clips, rig: Arc::new(rig), weight, reaction: reaction_params(&pref) })
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
