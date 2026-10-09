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
    assert!(rig.weapons[0].shapes.iter().any(|s| s.kind == 6));
    // Guard keeps its collision group (the game builds guard shapes from it).
    let guard = &sora.character.moveset.clips[&character_core::motion::GUARD];
    assert!(guard.windows.iter().any(|w| matches!(w.kind, WindowKind::Guard { weapon: 1, .. })));
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

/// Every Drive Form loads from extracted files: its own motions, combo
/// rows and weapons (two for Valor, Master and Final).
#[test]
fn drive_forms_load() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../local/re");
    if !dir.join("P_EX100_BTLF.mset").exists() {
        eprintln!("skipped: no extracted form files");
        return;
    }
    for spec in kh2_data::sora::FORMS {
        let d = kh2_data::sora::load_player_dir(&dir, spec).unwrap_or_else(|e| panic!("{}: {e}", spec.model));
        let rig = d.rig.as_ref().unwrap_or_else(|| panic!("{}: {:?}", spec.model, d.rig_error));
        let hands = match spec.form {
            1 | 4 | 5 => 2,
            6 => 0,
            _ => 1,
        };
        assert_eq!(rig.weapons.len(), hands, "{}", spec.model);
        assert!(!d.character.moveset.combo_rows.is_empty(), "{}: combo rows", spec.model);
        assert!(d.character.moveset.clips.contains_key(&53), "{}: arrival motion", spec.model);
        eprintln!("{}: {} clips, {} combo rows, run {}", spec.model, d.motions.len(), d.character.moveset.combo_rows.len(), d.character.params.run_speed);
    }
}

/// Sora's out-of-combat motions: the field (no weapon) slots of idle, walk
/// and run hold their own data; jump/fall/land share one slot.
#[test]
fn sora_field_motions() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../local/re");
    if !dir.join("P_EX100.mset").exists() {
        eprintln!("skipped: no extracted files");
        return;
    }
    let d = kh2_data::sora::load_player_dir(&dir, kh2_data::sora::SORA).unwrap();
    let rig = d.rig.as_ref().unwrap();
    let mut ids: Vec<u16> = rig.field_motions.keys().copied().collect();
    ids.sort();
    assert_eq!(ids, [0, 1, 2]);
    // Field idle is its own 90-frame loop (battle idle runs 150).
    assert_eq!(rig.field_motions[&0].frame_end, 90.0);
    assert_eq!(rig.motions[&0].frame_end, 150.0);
}
