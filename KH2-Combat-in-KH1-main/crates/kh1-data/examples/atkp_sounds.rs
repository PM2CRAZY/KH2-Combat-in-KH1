//! Research: the hit sounds (ATKP `hit_sound`) of some attacks.
//! `cargo run -p kh1-data --example atkp_sounds -- <00battle.bin> <atkp ids...>`
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let b = kh2_formats::battle::Battle::parse(&std::fs::read(&a[1]).unwrap()).unwrap();
    for id in a[2..].iter().filter_map(|x| x.parse::<u16>().ok()) {
        if let Some(r) = b.attack_for_level(id, 0, 0) {
            println!("atkp {id}: power {} reaction {} hit_sound {} effect_on_hit {} kind_flags {:#x}", r.power, r.enemy_reaction, r.hit_sound, r.effect_on_hit, r.attack_kind_flags);
        }
    }
}
