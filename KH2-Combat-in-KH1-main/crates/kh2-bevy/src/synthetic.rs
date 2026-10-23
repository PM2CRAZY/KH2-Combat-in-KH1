//! `--synthetic`: made-up numbers for smoke-testing the playground without
//! an install. None of these values come from KH2; they only exercise the
//! code paths (moves, windows, combos, hits).

use std::collections::HashMap;

use character_core::*;
use kh2_data::sora::{BladeSphere, SoraData};
use kh2_data::{CombatData, Source};
use kh2_formats::battle::{Atkp, Battle, Enmp, Lvpm, LvupLevel};

fn attack_clip(motion: u16, len: u32, hit: (u32, u32), attack: u16, combo_at: u32, finisher: bool, lunge: f32) -> Clip {
    let mut c = Clip::plain(motion, len);
    c.root = (0..=len).map(|f| [0.0, 0.0, lunge * (f as f32 / len as f32).min(1.0)]).collect();
    c.windows.push(Window { start: hit.0, end: Some(hit.1), kind: WindowKind::Hitbox { attack, group: 1, combo: None, weapon: 1 } });
    c.windows.push(Window { start: 0, end: Some(12), kind: WindowKind::TurnToLock(0.26) });
    let kind = if finisher { WindowKind::Finisher } else { WindowKind::Combo };
    c.windows.push(Window { start: combo_at, end: None, kind });
    c
}

fn row(id: u8, motion: u16, finisher: bool, air: bool) -> ComboRow {
    ComboRow {
        id, kind: 3, sub: -1, combo_offset: 0,
        flag: if finisher { FLAG_FINISHER } else { 0 } | if air { FLAG_AIR } else { 0 },
        motion, next_motion: 0, jump: if air { 20.0 } else { 0.0 }, jump_min: 0.0, jump_max: 0.0,
        speed_min: 0.0, speed_max: 0.0, near: 0.0, far: 0.0, low: 0.0, high: 0.0, inner_min: -2.0,
        inner_max: 2.0, blend_time: 0.0, distance_adjust: 0.0, ability: 0, score: 1,
    }
}

fn atkp(id: u16, power: u16) -> Atkp {
    Atkp {
        sub_id: 0, id, kind: 0, stat_source: 0, power, team: 0, element: 0, enemy_reaction: 0, effect_on_hit: 0,
        knockback_1: 100, knockback_2: 0, unknown_16: 0, flags: 0, refact_self: 0, refact_other: 0, reflected_motion: 0,
        reflect_hit_back: 0, reflect_action: 0, hit_sound: 0, reflect_rc: 0, reflect_range: 0, reflect_angle: 0,
        damage_effect: 0, switch: 0, interval: 0, floor_check: 0, drive_drain: 0, revenge_damage: 0, tr_reaction: 0,
        combo_group: 0, random_effect: 0, attack_kind_flags: 0, hp_drain: 0,
    }
}

pub fn data() -> SoraData {
    let mut clips = HashMap::new();
    let mut jump = Clip::plain(character_core::motion::JUMP, 10);
    jump.events.push(Event { frame: 4, kind: EventKind::Jump });
    clips.insert(character_core::motion::JUMP, jump);
    let mut land = Clip::plain(character_core::motion::LAND, 20);
    land.windows.push(Window { start: 8, end: None, kind: WindowKind::AllowReactionCommand });
    clips.insert(character_core::motion::LAND, land);
    let mut roll = Clip::plain(character_core::motion::DODGE_ROLL, 50);
    roll.root = (0..=50).map(|f| [0.0, 0.0, 420.0 * (1.0 - (1.0 - f as f32 / 50.0).powi(2))]).collect();
    roll.events.push(Event { frame: 34, kind: EventKind::FallCheck });
    clips.insert(character_core::motion::DODGE_ROLL, roll);
    let mut guard = Clip::plain(character_core::motion::GUARD, 60);
    guard.windows.push(Window { start: 0, end: Some(40), kind: WindowKind::Guard { attack: 0, group: 0, weapon: 0 } });
    clips.insert(character_core::motion::GUARD, guard);
    clips.insert(900, attack_clip(900, 48, (15, 23), 1, 23, false, 90.0));
    clips.insert(901, attack_clip(901, 80, (25, 38), 2, 40, true, 120.0));
    clips.insert(910, attack_clip(910, 60, (10, 22), 3, 22, false, 0.0));
    clips.insert(911, attack_clip(911, 66, (12, 24), 4, 30, true, 0.0));
    let moveset = Moveset {
        clips,
        // Finishers first: the first valid row in table order wins.
        combo_rows: vec![row(1, 901, true, false), row(0, 900, false, false), row(3, 911, true, true), row(2, 910, false, true)],
    };
    let growth = (1..=4)
        .map(|l| GrowthLevel {
            high_jump_height: 200.0 + 100.0 * l as f32,
            air_dodge_height: 150.0,
            air_dodge_speed: 20.0,
            quick_run_time: 15.0,
            quick_run_speed: 30.0,
            quick_run_brake: 0.94,
            quick_run_stop_brake: 0.93,
            quick_run_star_time: 8.0,
            glide_speed: 14.0 + 4.0 * l as f32,
            glide_fall_ratio: 0.015,
            glide_fall_height: 0.0,
            glide_fall_max: 3.5 - 0.5 * l as f32,
            glide_acceleration: 0.92,
            glide_start_height: 60.0,
            glide_end_height: 10.0,
            glide_turn_speed: 0.05,
            dodge_roll_star_time: 8.0 * l as f32,
        })
        .collect();
    let character = Character::new(moveset, MoveParams::default(), growth, AttackParams::default());

    let mut battle = Battle::default();
    battle.atkp = vec![atkp(1, 100), atkp(2, 200), atkp(3, 100), atkp(4, 150)];
    battle.enmp = vec![Enmp {
        id: 34, level: 0, health: [100; 32], max_damage: 10000, min_damage: 500, weakness: [100; 7], exp: 0, prize: 0, bonus_level: 0,
    }];
    battle.lvpm = (1..=99u16).map(|l| Lvpm { hp: 20 + 2 * l, strength: 5 + l, defense: 2 + l / 2, max_strength: 9999, min_strength: 1, exp: 0 }).collect();
    battle.lvup = vec![(1..=99u8)
        .map(|l| LvupLevel { exp: 0, strength: 2 + l, magic: 2 + l, defense: 2 + l / 2, ap: 0, sword_ability: 0, shield_ability: 0, staff_ability: 0 })
        .collect()];
    let combat = CombatData { battle, source: Source::Live };
    let blade_groups = HashMap::from([(1u8, vec![BladeSphere { along: 0.0, radius: 40.0 }, BladeSphere { along: 40.0, radius: 40.0 }])]);
    SoraData { character, combat, blade_groups, motions: vec![], rig: None, rig_error: None, weight: 100.0, magi: Default::default(), spec: kh2_data::sora::SORA }
}
