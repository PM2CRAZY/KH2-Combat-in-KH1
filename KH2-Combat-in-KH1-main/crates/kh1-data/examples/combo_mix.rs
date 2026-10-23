//! The user (KH1, stage 37): make sure KH2's combo doesn't repeat moves
//! before b48. Runs KH2's controller with the user's KH1 menu (Combo Plus x4, Air Combo Plus
//! x2, Combo Master and the mapped modifiers), every swing connecting, against foes near, far,
//! around and above, and prints each combo's attack motions; repeats in a row are flagged.
//!
//! `cargo run -p kh1-data --example combo_mix -- <KH2 dir>`
use character_core::{Abilities, Button, Env, Input, TargetInfo};

fn main() {
    let dir = std::env::args().nth(1).expect("KH2 dir");
    let data = kh2_data::sora::load_sora_dir(std::path::Path::new(&dir)).unwrap();
    let sets: Vec<(&str, Abilities)> = vec![
        ("no modifiers", Abilities { combo_master: true, ..Abilities::max_growth() }),
        (
            "the user's KH1 menu",
            Abilities {
                guard: true,
                combo_plus: 4,
                air_combo_plus: 2,
                combo_master: true,
                // Flash Step, Aerial Sweep, Counterguard, Finishing Leap, Slapshot, Slide Dash,
                // Vicinity Break, Guard Break, Aerial Finish, Guard.
                equipped: vec![177, 97, 96, 95, 90, 92, 180, 93, 100, 1],
                ..Abilities::max_growth()
            },
        ),
    ];
    let foe = |id: u32, x: f32, y: f32, z: f32| TargetInfo { id, pos: [x, y, z], radius: 40.0, height: 160.0 };
    let scenes: Vec<(&str, Vec<TargetInfo>, bool)> = vec![
        ("close", vec![foe(1, 0.0, 0.0, 120.0)], false),
        ("at 4 m", vec![foe(1, 0.0, 0.0, 400.0)], false),
        ("at 7 m", vec![foe(1, 0.0, 0.0, 700.0)], false),
        ("surrounded", vec![foe(1, 0.0, 0.0, 120.0), foe(2, 120.0, 0.0, 0.0), foe(3, -120.0, 0.0, 0.0), foe(4, 0.0, 0.0, -120.0)], false),
        ("foe 4 m up", vec![foe(1, 0.0, 400.0, 200.0)], false),
        ("air, level", vec![foe(1, 0.0, 300.0, 120.0)], true),
        ("air, at 4 m", vec![foe(1, 0.0, 300.0, 400.0)], true),
        ("air, below", vec![foe(1, 0.0, 0.0, 250.0)], true),
    ];
    for (label, abilities) in &sets {
        println!("== {label}");
        for (name, targets, air) in &scenes {
            let mut ch = data.character.clone();
            ch.abilities = abilities.clone();
            ch.set_lock(Some(1));
            let mut pos = [0.0f32, 0.0, 0.0];
            let mut grounded = true;
            let mut combos: Vec<Vec<u16>> = vec![vec![]];
            let mut last = 0u16;
            let mut live = std::collections::HashSet::new();
            let mut idle = 0;
            for f in 0..900 {
                let jump = *air && f % 240 < 10 && f % 240 < 10;
                let attack = !(*air && f % 240 < 14) && f % 8 < 4;
                let input = Input { stick: [0.0, 0.0], jump: Button { pressed: jump && f % 240 == 0, held: jump }, attack: Button { pressed: attack && f % 8 == 0, held: attack }, action: Button::default(), lock: Button::default() };
                let env = Env { pos, grounded, height_above_ground: pos[1].max(0.0), targets, ledges: &[] };
                let out = ch.step(&input, &env);
                for k in 0..3 {
                    pos[k] += out.displacement[k];
                }
                if pos[1] <= 0.0 {
                    pos[1] = 0.0;
                    grounded = true;
                } else {
                    grounded = false;
                }
                // Every swing connects (the combo counter only advances on hits).
                for h in &out.hitboxes {
                    if live.insert(h.swing) {
                        ch.register_hit();
                    }
                }
                if out.motion != last && (150..=210).contains(&out.motion) {
                    combos.last_mut().unwrap().push(out.motion);
                    idle = 0;
                } else {
                    idle += 1;
                    if idle == 40 && !combos.last().unwrap().is_empty() {
                        combos.push(vec![]);
                    }
                }
                last = out.motion;
            }
            let shown: Vec<String> = combos
                .iter()
                .filter(|c| !c.is_empty())
                .map(|c| {
                    let rep = c.windows(2).any(|w| w[0] == w[1]);
                    format!("{c:?}{}", if rep { " REPEAT" } else { "" })
                })
                .collect();
            println!("{name}: {}", shown.join(" "));
        }
    }
}
