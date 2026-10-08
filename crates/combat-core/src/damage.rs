use crate::{AttackDef, Combatant, CriticalAdjust, Element};

/// Difficulty presets. Multipliers live in [`DamageRules`] so they can be
/// tuned; the defaults are community-reported values (see docs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Difficulty {
    Beginner,
    #[default]
    Standard,
    Proud,
    Critical,
}

/// Every tunable constant in the damage formula. Values marked
/// "hypothesis" in `docs/combat-model.md` should be confirmed in-game
/// before you rely on exact numbers.
#[derive(Clone, Debug, PartialEq)]
pub struct DamageRules {
    pub difficulty: Difficulty,
    /// Multiplier on damage the player side deals, per difficulty.
    pub player_dealt: [f32; 4],
    /// Multiplier on damage the player side takes, per difficulty.
    pub player_taken: [f32; 4],
    /// Smallest damage a damaging hit can do after all multipliers.
    pub min_damage: u32,
    /// Extra caller-supplied multiplier (ability boosts such as Fire Boost).
    pub elemental_boost: [f32; 7],
}

impl Default for DamageRules {
    fn default() -> Self {
        DamageRules {
            difficulty: Difficulty::Standard,
            // Beginner, Standard, Proud, Critical
            player_dealt: [1.0, 1.0, 1.0, 1.25],
            player_taken: [0.5, 1.0, 1.5, 2.0],
            min_damage: 1,
            elemental_boost: [1.0; 7],
        }
    }
}

impl DamageRules {
    fn difficulty_index(&self) -> usize {
        self.difficulty as usize
    }
}

/// Every intermediate value of one damage calculation, for debugging and
/// for comparing against the real game step by step.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DamageBreakdown {
    pub attack_stat: i32,
    pub defense: i32,
    /// `attack_stat - defense`, after the defender's clamp.
    pub base: i32,
    pub after_power: u32,
    pub after_element: u32,
    pub after_boost: u32,
    pub after_difficulty: u32,
    /// After the defender's per-hit [`crate::HitClamp`], if any.
    pub after_clamp: u32,
    pub final_damage: u32,
}

/// Computes damage without changing any state.
///
/// Order of operations (sources in `docs/combat-model.md`):
/// 1. base = attack stat (Strength, or Magic for spells) - defense,
///    clamped by the defender's `base_clamp`, floored at 0
/// 2. x power multiplier, truncated
/// 3. x element resistance percent / 100, truncated
/// 4. x elemental boost, truncated
/// 5. x difficulty multiplier and critical adjust, truncated
/// 6. clamped by the defender's `hit_clamp` (fraction of max HP)
/// 7. raised to `rules.min_damage` if the hit is a damaging one
pub fn compute_damage(
    rules: &DamageRules,
    attack: &AttackDef,
    attacker: &Combatant,
    defender: &Combatant,
) -> DamageBreakdown {
    let attack_stat = if attack.element.is_magical() {
        attacker.stats.magic
    } else {
        attacker.stats.strength
    } as i32;
    let defense = defender.stats.defense as i32;

    let mut base = attack_stat - defense;
    if let Some(c) = defender.base_clamp {
        base = base.clamp(c.min, c.max.max(c.min));
    }
    let base = base.max(0);

    let after_power = (base as f32 * attack.power.max(0.0)) as u32;
    let pct = defender.resist.percent(attack.element) as u32;
    let after_element = after_power * pct / 100;
    let after_boost =
        (after_element as f32 * rules.elemental_boost[attack.element.index()]) as u32;

    let i = rules.difficulty_index();
    let mut diff = if attacker.team == crate::Team::Player && defender.team != crate::Team::Player {
        rules.player_dealt[i]
    } else if defender.team == crate::Team::Player && attacker.team != crate::Team::Player {
        rules.player_taken[i]
    } else {
        1.0
    };
    // Hypothesis: the per-attack critical adjust only scales the extra
    // damage the *player side takes* on Critical. Applying it to player
    // attacks too would halve every spell (all of them are flagged "half"),
    // which contradicts the community worked example in the tests.
    let hits_player = defender.team == crate::Team::Player && attacker.team != crate::Team::Player;
    if rules.difficulty == Difficulty::Critical && hits_player {
        diff *= match attack.critical_adjust {
            CriticalAdjust::Normal => 1.0,
            CriticalAdjust::Half => 0.5,
            CriticalAdjust::None => 0.0,
        };
    }
    let after_difficulty = (after_boost as f32 * diff) as u32;

    let after_clamp = match defender.hit_clamp {
        Some(c) => {
            let (lo, hi) = c.bounds(defender.max_hp);
            after_difficulty.clamp(lo, hi)
        }
        None => after_difficulty,
    };

    let final_damage = if pct == 0 || diff == 0.0 {
        0 // immune or explicitly zeroed: no minimum
    } else {
        after_clamp.max(rules.min_damage)
    };

    DamageBreakdown {
        attack_stat,
        defense,
        base,
        after_power,
        after_element,
        after_boost,
        after_difficulty,
        after_clamp,
        final_damage,
    }
}

/// Healing amount for cure-type attacks: power x Magic (hypothesis).
pub fn compute_healing(attack: &AttackDef, caster: &Combatant) -> u32 {
    (caster.stats.magic as f32 * attack.power.max(0.0)) as u32
}

/// Elements that use Magic instead of Strength, exposed for tests and hosts.
pub fn uses_magic(e: Element) -> bool {
    e.is_magical()
}
