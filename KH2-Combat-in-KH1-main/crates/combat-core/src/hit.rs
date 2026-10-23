use std::collections::HashMap;

use crate::{compute_damage, compute_healing, AttackDef, Combatant, DamageBreakdown, DamageRules};

/// Why a hit did nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocked {
    NotATarget,
    AlreadyDead,
    Invincible,
    Guarded,
    TooSoon,
}

/// Everything a single hit produced. The host applies the parts it cares
/// about (play the reaction, push the target, add drive gauge, ...).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HitOutcome {
    pub blocked: Option<Blocked>,
    pub damage: u32,
    pub healed: u32,
    pub breakdown: DamageBreakdown,
    /// Raw reaction id from the attack, for the host to play.
    pub reaction: u8,
    pub knockback: (f32, f32),
    pub killed: bool,
    pub revenge_triggered: bool,
    /// Drive gauge the attacker should gain.
    pub attacker_drive_gain: u8,
    /// HP the attacker should regain.
    pub attacker_hp_drain: u8,
}

/// Applies one hit of `attack` from `attacker` onto `defender`, mutating the
/// defender. Does not do multi-hit gating; see [`HitTracker`] for that.
pub fn resolve_hit(
    rules: &DamageRules,
    attack: &AttackDef,
    attacker: &Combatant,
    defender: &mut Combatant,
) -> HitOutcome {
    let blocked = |b| HitOutcome { blocked: Some(b), ..HitOutcome::default() };

    if attack.kind.heals() {
        if defender.team != attacker.team {
            return blocked(Blocked::NotATarget);
        }
        if defender.is_dead() {
            return blocked(Blocked::AlreadyDead);
        }
        let amount = compute_healing(attack, attacker, defender);
        let before = defender.hp;
        defender.hp = (defender.hp + amount).min(defender.max_hp);
        return HitOutcome { healed: defender.hp - before, ..HitOutcome::default() };
    }

    if !attack.kind.deals_damage() {
        return blocked(Blocked::NotATarget);
    }
    if !defender.is_valid_target_of(attacker, attack.targets) {
        return blocked(Blocked::NotATarget);
    }
    if defender.is_dead() {
        return blocked(Blocked::AlreadyDead);
    }
    if defender.invincible {
        return blocked(Blocked::Invincible);
    }
    if defender.guarding && attack.kind != crate::AttackKind::PierceArmor {
        return blocked(Blocked::Guarded);
    }

    let breakdown = compute_damage(rules, attack, attacker, defender);
    let damage = breakdown.final_damage.min(defender.hp);
    defender.hp -= damage;

    let mut revenge_triggered = false;
    if attack.revenge > 0 {
        defender.revenge += attack.revenge as u32;
        if let Some(limit) = defender.revenge_limit {
            if defender.revenge >= limit {
                defender.revenge = 0;
                revenge_triggered = true;
            }
        }
    }

    HitOutcome {
        blocked: None,
        damage,
        healed: 0,
        breakdown,
        reaction: attack.reaction,
        knockback: attack.knockback,
        killed: defender.hp == 0,
        revenge_triggered,
        attacker_drive_gain: attack.drive_gain,
        attacker_hp_drain: attack.hp_drain,
    }
}

/// Gates repeated hits. A hitbox that overlaps a target on many frames
/// should only damage it once, or once every `hit_interval` frames.
///
/// The host gives each swing a unique `swing` id (a counter is fine) and
/// calls [`HitTracker::try_hit`] every frame the hitbox overlaps.
#[derive(Clone, Debug, Default)]
pub struct HitTracker {
    last_hit: HashMap<(u64, u32), u64>,
}

impl HitTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolves the hit if gating allows it, otherwise returns
    /// `Blocked::TooSoon` without touching the defender.
    pub fn try_hit(
        &mut self,
        frame: u64,
        swing: u64,
        rules: &DamageRules,
        attack: &AttackDef,
        attacker: &Combatant,
        defender: &mut Combatant,
    ) -> HitOutcome {
        let key = (swing, defender.id);
        if let Some(&last) = self.last_hit.get(&key) {
            let interval = attack.hit_interval as u64;
            if interval == 0 || frame < last + interval {
                return HitOutcome { blocked: Some(Blocked::TooSoon), ..HitOutcome::default() };
            }
        }
        let out = resolve_hit(rules, attack, attacker, defender);
        if out.blocked.is_none() {
            self.last_hit.insert(key, frame);
        }
        out
    }

    /// Forget a finished swing.
    pub fn end_swing(&mut self, swing: u64) {
        self.last_hit.retain(|(s, _), _| *s != swing);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;

    fn sora() -> Combatant {
        Combatant::new(1, Team::Player, Stats { hp: 100, strength: 30, magic: 20, defense: 5 })
    }
    fn heartless(hp: u32) -> Combatant {
        Combatant::new(2, Team::Enemy, Stats { hp, strength: 10, magic: 10, defense: 4 })
    }

    #[test]
    fn basic_physical_hit() {
        let a = AttackDef { power: 150, ..Default::default() };
        let mut e = heartless(100);
        let out = resolve_hit(&DamageRules::default(), &a, &sora(), &mut e);
        assert_eq!(out.damage, 39); // (30-4) x 1.5
        assert_eq!(e.hp, 61);
        // rounding is up: 26 x 1.25 = 32.5 -> 33
        let a = AttackDef { power: 125, ..Default::default() };
        assert_eq!(resolve_hit(&DamageRules::default(), &a, &sora(), &mut heartless(100)).damage, 33);
    }

    #[test]
    fn magic_and_difficulty() {
        // Thundaga-like row: 375%, Magic, Thunder; 80 Magic vs 51 Defense,
        // 50% thunder weakness. ceil(50*100*375*29/1e6) = 55; Critical x1.25 -> 69.
        let caster = Combatant::new(1, Team::Player, Stats { hp: 1, strength: 0, magic: 80, defense: 0 });
        let mut target = Combatant::new(2, Team::Enemy, Stats { hp: 999, strength: 0, magic: 0, defense: 51 });
        target.resist.0[Element::Thunder.index()] = 50;
        let a = AttackDef { power: 375, element: Element::Thunder, stat: StatSource::Magic, ..Default::default() };
        let rules = DamageRules { difficulty: Difficulty::Critical, ..Default::default() };
        let out = resolve_hit(&rules, &a, &caster, &mut target);
        assert_eq!(out.breakdown.after_power, 55);
        assert_eq!(out.damage, 69);
        // Enemies hit twice as hard on Critical.
        let enemy = Combatant::new(2, Team::Enemy, Stats { hp: 1, strength: 30, magic: 0, defense: 0 });
        let mut p = Combatant::new(1, Team::Player, Stats { hp: 1000, strength: 0, magic: 0, defense: 10 });
        assert_eq!(resolve_hit(&rules, &AttackDef::default(), &enemy, &mut p).damage, 40);
    }

    #[test]
    fn bounds_and_immunity() {
        let a = AttackDef::default();
        // max HP 1000: min 5% = 50, max 2% = 20 (min wins when they cross)
        let mut e = heartless(1000).with_damage_bounds(StatBounds::from_permyriad(1000, 100, 2000));
        assert_eq!(e.damage_bounds, StatBounds { min: 10, max: 200 });
        let strong = Combatant::new(9, Team::Player, Stats { hp: 1, strength: 999, magic: 0, defense: 0 });
        assert_eq!(resolve_hit(&DamageRules::default(), &a, &strong, &mut e).damage, 200);
        e.stats.defense = 2000;
        assert_eq!(resolve_hit(&DamageRules::default(), &a, &sora(), &mut e).damage, 10);
        // attacker cap applies too
        let capped = strong.clone().with_attack_bounds(StatBounds { min: 1, max: 7 });
        e.stats.defense = 0;
        assert_eq!(resolve_hit(&DamageRules::default(), &a, &capped, &mut e).damage, 10);
        e.resist.0[0] = 0;
        assert_eq!(resolve_hit(&DamageRules::default(), &a, &sora(), &mut e).damage, 0);
    }

    #[test]
    fn combo_boost_adds_per_hit() {
        let mut s = sora();
        s.boosts.combo = Some(10);
        s.combo_hits = 3;
        let a = AttackDef { combo_finisher: true, ..Default::default() };
        // 26 + ceil(26 x 30%) = 26 + 8
        assert_eq!(resolve_hit(&DamageRules::default(), &a, &s, &mut heartless(100)).damage, 34);
    }

    #[test]
    fn same_team_is_not_a_target() {
        let a = AttackDef::default();
        let mut ally = Combatant::new(3, Team::Player, Stats { hp: 10, ..Default::default() });
        let out = resolve_hit(&DamageRules::default(), &a, &sora(), &mut ally);
        assert_eq!(out.blocked, Some(Blocked::NotATarget));
        let a = AttackDef { targets: TargetFilter::Everyone, ..Default::default() };
        assert!(resolve_hit(&DamageRules::default(), &a, &sora(), &mut ally).blocked.is_none());
    }

    #[test]
    fn cure_heals_allies_only() {
        // 100 max HP x 30% x 20 Magic / 10000 = 6
        let cure = AttackDef { kind: AttackKind::Cure, power: 30, stat: StatSource::Magic, ..Default::default() };
        let mut ally = Combatant::new(3, Team::Player, Stats { hp: 100, ..Default::default() });
        ally.hp = 10;
        let out = resolve_hit(&DamageRules::default(), &cure, &sora(), &mut ally);
        assert_eq!(out.healed, 6);
        let mut e = heartless(50);
        e.hp = 1;
        assert_eq!(resolve_hit(&DamageRules::default(), &cure, &sora(), &mut e).healed, 0);
    }

    #[test]
    fn interval_gating() {
        let a = AttackDef { hit_interval: 10, ..Default::default() };
        let mut e = heartless(10_000);
        let mut t = HitTracker::new();
        let r = DamageRules::default();
        assert!(t.try_hit(0, 1, &r, &a, &sora(), &mut e).blocked.is_none());
        assert_eq!(t.try_hit(5, 1, &r, &a, &sora(), &mut e).blocked, Some(Blocked::TooSoon));
        assert!(t.try_hit(10, 1, &r, &a, &sora(), &mut e).blocked.is_none());
        // interval 0: once per swing
        let once = AttackDef::default();
        assert!(t.try_hit(0, 2, &r, &once, &sora(), &mut e).blocked.is_none());
        assert_eq!(t.try_hit(99, 2, &r, &once, &sora(), &mut e).blocked, Some(Blocked::TooSoon));
        t.end_swing(2);
        assert!(t.try_hit(100, 2, &r, &once, &sora(), &mut e).blocked.is_none());
    }

    #[test]
    fn revenge_and_guard() {
        let a = AttackDef { revenge: 5, ..Default::default() };
        let mut e = heartless(10_000);
        e.revenge_limit = Some(10);
        let r = DamageRules::default();
        assert!(!resolve_hit(&r, &a, &sora(), &mut e).revenge_triggered);
        assert!(resolve_hit(&r, &a, &sora(), &mut e).revenge_triggered);
        e.guarding = true;
        assert_eq!(resolve_hit(&r, &a, &sora(), &mut e).blocked, Some(Blocked::Guarded));
        let pierce = AttackDef { kind: AttackKind::PierceArmor, ..Default::default() };
        assert!(resolve_hit(&r, &pierce, &sora(), &mut e).blocked.is_none());
    }
}
