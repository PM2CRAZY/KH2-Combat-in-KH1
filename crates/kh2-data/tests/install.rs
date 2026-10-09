//! Checks against a real install. Skipped unless `KH2_GAME` points at the
//! KH2 executable or install directory, e.g.
//! `KH2_GAME="D:\epic\KH_1.5_2.5\KINGDOM HEARTS II FINAL MIX.exe" cargo test -p kh2-data`.

use kh2_data::{Install, Source};

fn install() -> Option<Install> {
    let path = std::env::var("KH2_GAME").ok()?;
    Some(Install::locate(path).expect("KH2_GAME is set but the install was not found"))
}

#[test]
fn loads_battle_tables() {
    let Some(install) = install() else { return };
    let data = install.load_from(Source::Live).unwrap();
    let b = &data.battle;
    // Entry counts documented by OpenKH for KH2FM.
    assert_eq!(b.atkp.len(), 2713);
    assert_eq!(b.enmp.len(), 229);
    assert_eq!(b.lvpm.len(), 99);
    assert_eq!(b.lvup.len(), 13);
    assert!(b.lvup[0].len() >= 99);
}

#[test]
fn power_is_a_percentage() {
    let Some(install) = install() else { return };
    let data = install.load().unwrap();
    // Thunder-element spell rows carry 375 (= x3.75, the community value).
    assert!(data.battle.atkp.iter().any(|a| a.element == 3 && a.power == 375));
    let most_common = {
        let mut m = std::collections::HashMap::new();
        for a in &data.battle.atkp {
            *m.entry(a.power).or_insert(0) += 1;
        }
        m.into_iter().max_by_key(|&(_, c)| c).unwrap().0
    };
    assert_eq!(most_common, 100);
}

#[test]
fn lvpm_level_99_defense_matches_community_example() {
    let Some(install) = install() else { return };
    let data = install.load().unwrap();
    // The forum example subtracts 51 Defense for a level-99 enemy.
    assert_eq!(data.battle.lvpm_for(99).unwrap().defense, 51);
}

/// Sora's rig: skeleton, motions, Keyblade attached to the `sklt` hand bone,
/// and every motion poses without NaNs.
#[test]
fn sora_rig_loads_and_poses() {
    let Some(i) = install() else { return };
    let sora = i.load_sora(kh2_data::Source::Live).unwrap();
    let rig = sora.rig.expect("rig");
    assert_eq!(rig.skeleton.len(), 256);
    assert_eq!(rig.weapons.first().and_then(|w| w.bone), Some(178));
    for (id, m) in &rig.motions {
        let t = kh2_anim::key_time(m, 10.0, true);
        let p = kh2_anim::evaluate(&rig.skeleton, m, t, Default::default());
        assert!(p.bones.iter().all(|b| b.is_finite()), "motion {id}");
    }
}
