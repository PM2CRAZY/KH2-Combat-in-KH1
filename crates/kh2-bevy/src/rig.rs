//! Sora as an asset-free mannequin: his real skeleton (bones only) posed by
//! his real motions every frame, both read from the user's install. The
//! visible shapes are our own capsules between joints; no mesh or texture
//! is loaded.

use bevy::prelude::*;

use kh2_data::sora::Rig;

pub use kh2_rig::{kind, Anchor, Part, RigAnim};

#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Capsule(Anchor, Anchor, f32),
    Sphere(Anchor, f32),
}

/// Palette slots (our own styling).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Paint {
    Jacket,
    Pants,
    Skin,
    Shoe,
    Steel,
    Gold,
    Grip,
    /// Enemies: near-black body, glowing yellow eyes.
    Shade,
    Eye,
}

impl Paint {
    pub const ALL: [Paint; 9] = [Paint::Jacket, Paint::Pants, Paint::Skin, Paint::Shoe, Paint::Steel, Paint::Gold, Paint::Grip, Paint::Shade, Paint::Eye];

    pub fn material(self) -> StandardMaterial {
        let (c, rough, metal) = match self {
            Paint::Jacket => (Color::srgb(0.16, 0.17, 0.22), 0.7, 0.0),
            Paint::Pants => (Color::srgb(0.24, 0.32, 0.52), 0.75, 0.0),
            Paint::Skin => (Color::srgb(0.93, 0.88, 0.82), 0.55, 0.0),
            Paint::Shoe => (Color::srgb(0.82, 0.24, 0.22), 0.5, 0.0),
            Paint::Steel => (Color::srgb(0.82, 0.84, 0.9), 0.25, 0.8),
            Paint::Gold => (Color::srgb(0.95, 0.75, 0.3), 0.3, 0.9),
            Paint::Grip => (Color::srgb(0.2, 0.2, 0.25), 0.6, 0.2),
            Paint::Shade => (Color::srgb(0.06, 0.05, 0.09), 0.8, 0.0),
            Paint::Eye => (Color::srgb(1.0, 0.9, 0.2), 0.4, 0.0),
        };
        let emissive = if matches!(self, Paint::Eye) { LinearRgba::rgb(2.0, 1.6, 0.2) } else { LinearRgba::BLACK };
        StandardMaterial { base_color: c, perceptual_roughness: rough, metallic: metal, emissive, ..default() }
    }
}

/// One mannequin piece, posed every frame.
#[derive(Component, Clone, Copy, Debug)]
pub struct Piece {
    pub part: Part,
    pub shape: Shape,
    pub paint: Paint,
}

/// Sora's bone roles (indices into `P_EX100`'s skeleton, found from the bind
/// pose's hierarchy; only the bones the mannequin needs).
mod sora_bones {
    pub const PELVIS: usize = 3;
    pub const SPINE: usize = 67;
    pub const CHEST: usize = 68;
    pub const NECK: usize = 138;
    pub const HEAD_TOP: usize = 141;
    /// (shoulder, elbow, wrist, hand, middle-finger tip)
    pub const ARMS: [[usize; 5]; 2] = [[76, 79, 80, 86, 99], [168, 171, 172, 178, 191]];
    /// (hip, knee, ankle, heel, toe)
    pub const LEGS: [[usize; 5]; 2] = [[11, 12, 13, 16, 17], [40, 41, 42, 45, 46]];
    pub const COUNT: usize = 256;
}

/// A simple, rounded mannequin on Sora's skeleton (our own design: torso,
/// head, limbs, mitten hands, big shoes) and a Keyblade-shaped prop on the
/// weapon skeleton. Falls back to one capsule per bone for other skeletons.
pub fn pieces(rig: &Rig) -> Vec<Piece> {
    use sora_bones::*;
    use Anchor::*;
    use Shape::*;
    let mut out = Vec::new();
    let mut add = |part, shape, paint| out.push(Piece { part, shape, paint });
    if rig.skeleton.len() == COUNT {
        let b = Part::Body;
        add(b, Sphere(Lerp(PELVIS, SPINE, 0.15), 10.5), Paint::Pants);
        add(b, Capsule(Lerp(PELVIS, SPINE, 0.35), Bone(CHEST), 10.0), Paint::Jacket);
        add(b, Capsule(Bone(CHEST), Bone(NECK), 3.6), Paint::Skin);
        add(b, Sphere(Lerp(NECK, HEAD_TOP, 0.45), 10.5), Paint::Skin);
        for [sh, el, wr, hand, tip] in ARMS {
            add(b, Sphere(Bone(sh), 5.2), Paint::Jacket);
            add(b, Capsule(Bone(sh), Bone(el), 4.0), Paint::Jacket);
            add(b, Capsule(Bone(el), Bone(wr), 3.5), Paint::Skin);
            add(b, Sphere(Lerp(hand, tip, 0.4), 4.6), Paint::Skin);
        }
        for [hip, knee, ankle, heel, toe] in LEGS {
            add(b, Capsule(Bone(hip), Bone(knee), 6.0), Paint::Pants);
            add(b, Capsule(Bone(knee), Bone(ankle), 4.8), Paint::Pants);
            add(b, Capsule(Lerp(heel, ankle, 0.35), Bone(toe), 5.6), Paint::Shoe);
        }
    } else {
        for s in segments(&rig.skeleton) {
            add(Part::Body, Capsule(Bone(s.0), Bone(s.1), s.2), Paint::Jacket);
        }
    }
    for (i, wr) in rig.weapons.iter().enumerate() {
        if wr.skeleton.len() < 12 {
            continue;
        }
        // The blade is rigid on bone 2 and points along its -X (where the
        // attack spheres sit); bones 3..11 are the dangling keychain.
        let w = Part::Weapon(i as u8);
        let along = |d: f32| Offset(2, Vec3::new(-d, 0.0, 0.0));
        add(w, Capsule(Bone(2), Bone(3), 1.7), Paint::Grip);
        add(w, Capsule(along(3.0), along(-3.0), 4.5), Paint::Gold);
        add(w, Capsule(along(4.0), along(70.0), 2.0), Paint::Steel);
        add(w, Sphere(along(66.0), 4.2), Paint::Steel);
        for i in 3..11 {
            add(w, Capsule(Bone(i), Bone(i + 1), 0.6), Paint::Gold);
        }
    }
    out
}

/// (child, parent, radius) for every visible bone segment.
fn segments(skeleton: &[kh2_formats::anim::Bone]) -> Vec<(usize, usize, f32)> {
    let bind = kh2_anim::Pose::bind(skeleton);
    let mut out = Vec::new();
    for (i, b) in skeleton.iter().enumerate() {
        if b.parent < 0 {
            continue;
        }
        let p = b.parent as usize;
        let len = bind.bone_position(i).distance(bind.bone_position(p));
        if len >= 1.0 {
            out.push((i, p, (len * 0.2).clamp(0.8, 5.5)));
        }
    }
    out
}
