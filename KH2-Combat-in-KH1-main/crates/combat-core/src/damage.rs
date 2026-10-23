use crate::{AttackDef, AttackKind, Combatant, Element, StatSource, Team};

/// Game difficulty.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Difficulty {
    Beginner,
    #[default]
    Standard,
    Proud,
    Critical,
}

/// The difficulty table. Values are percentages applied with rounding up.
/// Defaults are the ones KH2 Final Mix's executable uses (checked).
#[derive(Clone, Debug, PartialEq)]
pub struct DamageRules {
    pub difficulty: Difficulty,
    /// Damage the player side deals, per difficulty (Beginner, Standard,
    /// Proud, Critical).
    pub player_dealt: [u32; 4],
    /// Damage the player side takes, per difficulty.
    pub player_taken: [u32; 4],
}

impl Default for DamageRules {
    fn default() -> Self {
        DamageRules { difficulty: Difficulty::Standard, player_dealt: [150, 100, 100, 125], player_taken: [50, 100, 200, 200] }
    }
}

/// Every intermediate value of one damage calculation, for debugging and
/// for comparing against the real game step by step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DamageBreakdown {
    pub attack_stat: i32,
    pub defense: i32,
    /// `attack_stat - defense` after both sides' bounds.
    pub base: i32,
    /// After power, element weakness and general weakness.
    pub after_power: u32,
    pub after_difficulty: u32,
    pub after_boost: u32,
    pub final_damage: u32,
}

fn ceil_div(n: u64, d: u64) -> u64 {
    (n + d - 1) / d
}

fn pct_up(v: u32, pct: u32) -> u32 {
    ceil_div(v as u64 * pct as u64, 100) as u32
}

/// Computes damage without changing any state. The order follows KH2 Final
/// Mix's damage routine (see `docs/combat-model.md`):
///
/// 1. base = attacker stat (Strength or Magic, per attack) - Defense, minus
///    the Defender bonus at <= 25% HP
/// 2. base capped at the lower of the attacker's and defender's maxima,
///    then raised to the higher of their minima
/// 3. x weakness[element] x weakness[general] x power, /1,000,000 rounded up
/// 4. difficulty percent, rounded up
/// 5. attacker's Fire/Blizzard/Thunder boost, rounded up
/// 6. combo / air-combo / reaction boosts (added on top), rounded up
/// 7. low-HP guard on the defender, rounded up
///
/// There is no minimum of 1: an immune target takes 0.
pub fn compute_damage(rules: &DamageRules, attack: &AttackDef, attacker: &Combatant, defender: &Combatant) -> DamageBreakdown {
    let attack_stat = match attack.stat {
        StatSource::Strength => attacker.stats.strength as i32,
        StatSource::Magic => attacker.stats.magic as i32,
        StatSource::None => 0,
    };
    let defense = defender.stats.defense as i32;
    let mut base = attack_stat - defense;
    if defender.hp <= defender.max_hp / 4 {
        base -= defender.low_hp_defense as i32;
    }
    let hi = attacker.attack_bounds.max.min(defender.damage_bounds.max) as i32;
    let lo = attacker.attack_bounds.min.max(defender.damage_bounds.min) as i32;
    base = base.min(hi).max(lo);

    let weak = defender.resist.percent(attack.element) as u64;
    let general = defender.resist.percent(Element::Neutral) as u64;
    let after_power = if base > 0 {
        ceil_div(weak * general * attack.power as u64 * base as u64, 1_000_000) as u32
    } else {
        0
    };

    let dealt_by_player = attacker.team == Team::Player && defender.team != Team::Player;
    let taken_by_player = defender.team == Team::Player && attacker.team != Team::Player;
    let i = rules.difficulty as usize;
    let mut dmg = after_power;
    if dmg > 0 && rules.difficulty != Difficulty::Standard {
        if taken_by_player {
            dmg = pct_up(dmg, rules.player_taken[i]);
        } else if dealt_by_player {
            dmg = pct_up(dmg, rules.player_dealt[i]);
        }
    }
    let after_difficulty = dmg;

    let b = &attacker.boosts;
    match attack.element {
        Element::Fire => dmg = pct_up(dmg, b.fire as u32),
        Element::Blizzard => dmg = pct_up(dmg, b.blizzard as u32),
        Element::Thunder => dmg = pct_up(dmg, b.thunder as u32),
        _ => {}
    }
    if let (true, Some(p)) = (attack.combo_finisher, b.combo) {
        dmg += pct_up(dmg, attacker.combo_hits as u32 * p as u32);
    } else if let (true, Some(p)) = (attack.air_combo_finisher, b.air_combo) {
        dmg += pct_up(dmg, attacker.air_combo_hits as u32 * p as u32);
    } else if let (true, Some(p)) = (attack.reaction_command, b.reaction) {
        dmg += pct_up(dmg, p as u32);
    }
    let after_boost = dmg;

    if let Some(g) = defender.low_hp_guard {
        if defender.hp as f32 <= defender.max_hp as f32 * g.hp_fraction {
            dmg = pct_up(dmg, g.percent);
        }
    }

    DamageBreakdown { attack_stat, defense, base, after_power, after_difficulty, after_boost, final_damage: dmg }
}

/// Healing for cure-type attacks. `Cure`: max HP x power x stat / 10000;
/// `CureStrong` (KH2 "C-Cure"): power x stat / 100. Both round up.
pub fn compute_healing(attack: &AttackDef, caster: &Combatant, target: &Combatant) -> u32 {
    let stat = match attack.stat {
        StatSource::Strength => caster.stats.strength,
        StatSource::Magic => caster.stats.magic,
        StatSource::None => 0,
    } as u64;
    match attack.kind {
        AttackKind::Cure => ceil_div(target.max_hp as u64 * attack.power as u64 * stat, 10_000) as u32,
        AttackKind::CureStrong => ceil_div(attack.power as u64 * stat, 100) as u32,
        _ => 0,
    }
}
