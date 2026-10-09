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

#[test]
fn the_shadow_loads_with_its_attacks_and_hurt_shapes() {
    let Some(d) = std::env::var_os("KH2_LOCAL_DATA") else { return };
    let dir = std::path::Path::new(&d);
    if !dir.join("M_EX010.mdlx").exists() {
        return;
    }
    let sora = kh2_data::sora::load_sora_dir(dir).unwrap();
    let e = kh2_data::enemy::load_enemy_dir(dir, kh2_data::enemy::SHADOW, &sora.combat).unwrap();
    let swipe = &e.clips[&201];
    assert!(swipe.windows.iter().any(|w| matches!(w.kind, WindowKind::Hitbox { attack: 5, group: 1, .. })));
    assert_eq!(e.rig.body_shapes.iter().filter(|s| s.kind == 2).count(), 3);
    for m in [0u16, 1, 10, 12, 14, 15, 16, 17, 18, 19] {
        assert!(e.clips.contains_key(&m) && e.rig.motions.contains_key(&m), "motion {m}");
    }
    assert_eq!(e.reaction.hit_back, 200.0);
}
