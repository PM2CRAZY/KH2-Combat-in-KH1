//! Research helper: `form_info <dir> <model>` loads a Drive Form from extracted files (the
//! `obj/` names by their base name in `dir`) and prints its moveset: movement numbers, combo
//! rows, motions (with their windows and events), weapons and their bones.
use character_core::WindowKind;
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let dir = std::path::Path::new(&a[1]);
    let spec = kh2_data::sora::FORMS.into_iter().chain([kh2_data::sora::SORA]).find(|s| s.model == a[2]).expect("model");
    let d = kh2_data::sora::load_player_dir(dir, spec).unwrap();
    let c = &d.character;
    println!("{} moveset {} form {}", spec.model, spec.moveset, spec.form);
    println!("params: {:?}", c.params);
    println!("attack: {:?}", c.attack);
    let rig = d.rig.as_ref().expect("rig");
    println!("weapons: {:?}", rig.weapons.iter().map(|w| (w.bone, w.motions.len(), w.shapes.len())).collect::<Vec<_>>());
    let mut rows = c.moveset.combo_rows.clone();
    rows.sort_by_key(|r| r.id);
    for r in &rows {
        println!(
            "row {:>3} kind {} sub {} flag {:#x} motion {} next {} ability {} jump {} near {} far {} low {} high {} combo_offset {}",
            r.id, r.kind, r.sub, r.flag, r.motion, r.next_motion, r.ability, r.jump, r.near, r.far, r.low, r.high, r.combo_offset
        );
    }
    let mut ms: Vec<_> = c.moveset.clips.keys().copied().collect();
    ms.sort();
    println!("motions {:?}", ms);
    let verbose = a.get(3).map(|s| s.split(',').filter_map(|x| x.parse::<u16>().ok()).collect::<Vec<_>>()).unwrap_or_default();
    for m in ms {
        let cl = &c.moveset.clips[&m];
        let hits: Vec<String> = cl
            .windows
            .iter()
            .filter_map(|w| match w.kind {
                WindowKind::Hitbox { attack, group, weapon, .. } => Some(format!("{}:{}..{:?} g{} w{}", attack, w.start, w.end, group, weapon)),
                WindowKind::Guard { attack, group, weapon } => Some(format!("guard {}:{}..{:?} g{} w{}", attack, w.start, w.end, group, weapon)),
                _ => None,
            })
            .collect();
        let casts: Vec<String> = cl.events.iter().filter(|e| matches!(e.kind, character_core::EventKind::MagicCast(_))).map(|e| format!("{:?}@{}", e.kind, e.frame)).collect();
        println!("m{m:>3} len {:>4} hits [{}] casts [{}]", cl.length, hits.join(", "), casts.join(", "));
        if verbose.contains(&m) {
            for w in &cl.windows {
                println!("    {:?} {}..{:?}", w.kind, w.start, w.end);
            }
            for e in &cl.events {
                println!("    ev {:?} @{}", e.kind, e.frame);
            }
        }
    }
    for (i, w) in rig.weapons.iter().enumerate() {
        let mut wm: Vec<_> = w.motions.keys().copied().collect();
        wm.sort();
        println!("weapon {i} on bone {:?}: motions {:?}", w.bone, wm);
    }
}
