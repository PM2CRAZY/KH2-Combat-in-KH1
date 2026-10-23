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
        // Limit Form: its limits' motions past 230 (Strike Raid's, Infinity's), the follow-up
        // windows its script reads (Sonic Rave's K401: script ranges 1 and 2) and the commands
        // its motions offer (Last Arcanum's K410: Bash).
        if spec.form == 3 {
            let clips = &d.character.moveset.clips;
            for m in [231u16, 232, 233, 234, 241, 242, 243, 244, 245, 246, 247, 248] {
                assert!(clips.contains_key(&m), "Limit Form motion {m}");
            }
            let rave = &clips[&212];
            for n in [1, 2] {
                assert!(rave.windows.iter().any(|w| w.kind == WindowKind::ScriptRange(n) && w.start == 16 && w.end == Some(46)), "K401 range {n}");
            }
            assert!(clips[&221].windows.iter().any(|w| w.kind == WindowKind::OfferCommand(702)));
            assert!(rave.windows.iter().any(|w| matches!(w.kind, WindowKind::Hitbox { attack: 1878, weapon: 1, .. })), "Rave's Keyblade hit");
        }
        eprintln!("{}: {} clips, {} combo rows, run {}", spec.model, d.motions.len(), d.character.moveset.combo_rows.len(), d.character.params.run_speed);
    }
}

/// Every attack and guard window of Sora and the forms finds a shape: its collision group has
/// an attack or guard entry on what the window is on (Anti's claws and feet are its own
/// model's groups 1, 4, 6, 7, 8 and 12). Needs unmodded files in `KH2_LOCAL_DATA`: a mod that
/// gives the forms Sora's model gives them his table too, and Anti's groups 1, 4 and 7 go empty.
#[test]
fn every_attack_group_has_a_shape() {
    let Some(d) = std::env::var_os("KH2_LOCAL_DATA") else { return };
    let dir = std::path::Path::new(&d);
    if !dir.join("P_EX100_HTLF.mdlx").exists() {
        return;
    }
    let mut specs = vec![kh2_data::sora::SORA];
    specs.extend(kh2_data::sora::FORMS);
    for spec in specs {
        let d = kh2_data::sora::load_player_dir(dir, spec).unwrap_or_else(|e| panic!("{}: {e}", spec.model));
        let rig = d.rig.as_ref().unwrap_or_else(|| panic!("{}: {:?}", spec.model, d.rig_error));
        let gaps: Vec<_> = rig.shape_groups(&d.character.moveset).into_iter().filter(|g| g.shapes == 0).collect();
        assert!(gaps.is_empty(), "{}: {gaps:?}", spec.model);
    }
    let anti = kh2_data::sora::load_player_dir(dir, kh2_data::sora::FORMS[5]).unwrap();
    let groups: Vec<u8> = anti.rig.unwrap().shape_groups(&anti.character.moveset).iter().map(|g| g.group).collect();
    assert_eq!(groups, [1, 2, 3, 4, 6, 7, 8, 10, 11, 12, 13]);
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
