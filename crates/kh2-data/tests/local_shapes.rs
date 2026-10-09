//! Runs against Sora's files extracted into a local folder (never
//! committed): set `KH2_LOCAL_DATA` to that folder.

use character_core::WindowKind;

#[test]
fn collision_shapes_and_guard_groups_load() {
    let Some(d) = std::env::var_os("KH2_LOCAL_DATA") else { return };
    let sora = kh2_data::sora::load_sora_dir(std::path::Path::new(&d)).unwrap();
    let rig = sora.rig.as_ref().expect("rig");
    // Every MDLX collision entry becomes a shape, not only attacks/hurts.
    assert!(rig.body_shapes.iter().any(|s| s.kind == 2 && s.bone.is_some()));
    assert!(rig.body_shapes.iter().any(|s| s.kind == 4 && s.bone.is_none() && s.lift > 0.0));
    assert!(rig.weapon_shapes.iter().any(|s| s.kind == 6));
    // Guard keeps its collision group (the game builds guard shapes from it).
    let guard = &sora.character.moveset.clips[&character_core::motion::GUARD];
    assert!(guard.windows.iter().any(|w| matches!(w.kind, WindowKind::Guard { weapon: true, .. })));
}
