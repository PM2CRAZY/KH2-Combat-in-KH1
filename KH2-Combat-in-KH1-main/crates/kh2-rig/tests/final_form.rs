//! Final Form's floating Keyblades, against the user's extracted files in `local/re` (never
//! committed; skipped without them).

use kh2_rig::MotionHits;

#[test]
fn final_form_keyblades_spin_and_hurt_in_the_glide() {
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../local/re");
    if !dir.join("P_EX100_ULTF.mset").exists() || !dir.join("W_EX010_ULTF_R.mset").exists() {
        eprintln!("skipped: no extracted Final Form files");
        return;
    }
    let spec = kh2_data::sora::FORMS.into_iter().find(|s| s.model == "P_EX100_ULTF").unwrap();
    let d = kh2_data::sora::load_player_dir(&dir, spec).unwrap();
    let rig = d.rig.as_ref().unwrap();
    // The glide's weapon motion spins the blade (bone 2) on its root: 6 turns a 60-frame loop.
    for w in &rig.weapons {
        let m = &w.motions[&character_core::motion::GLIDE];
        let at = |f: f32| kh2_anim::evaluate(&w.skeleton, m, kh2_anim::key_time(m, f, true), kh2_anim::Options::default()).bones;
        let (a, b) = (at(0.0), at(2.0));
        let turn = |p: &[glam::Mat4]| glam::Quat::from_mat4(&(p[0].inverse() * p[2]));
        assert!(turn(&a).angle_between(turn(&b)) > 0.3, "the blade spins on its root");
    }
    // Their attacks: the glide's all through, the jump's from frame 2.
    let clips = &d.character.moveset.clips;
    let mut h = MotionHits::default();
    let glide: Vec<u16> = h.step_at(clips.get(&character_core::motion::GLIDE), 40.0).iter().map(|x| x.attack).collect();
    assert_eq!(glide, vec![1281, 1282]);
    let mut h = MotionHits::default();
    assert!(h.step_at(clips.get(&character_core::motion::JUMP), 1.0).is_empty());
    assert_eq!(h.step_at(clips.get(&character_core::motion::JUMP), 2.0).iter().map(|x| x.attack).collect::<Vec<_>>(), vec![1273, 1274]);
    let row = d.combat.battle.attack_for_level(1281, 0, 0).unwrap();
    eprintln!("glide row 1281: power {}, re-hit interval {}", row.power, row.interval);
}
