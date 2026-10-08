use crate::Resistances;

/// Which side a fighter is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Team {
    Player,
    Enemy,
    Neutral,
}

/// The four stats the damage formula reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub hp: u32,
    pub strength: u16,
    pub magic: u16,
    pub defense: u16,
}

/// Bounds on the `(attack stat - defense)` term. KH2 keeps two pairs: one
/// on the attacker (its strength cap and floor) and one on the defender
/// (how little or how much one hit may do to it). The tighter of each pair
/// wins. Players default to `1..=99999`, i.e. no real cap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatBounds {
    pub min: u32,
    pub max: u32,
}

impl Default for StatBounds {
    fn default() -> Self {
        StatBounds { min: 1, max: 99_999 }
    }
}

impl StatBounds {
    /// Defender bounds from parts-per-10000 of max HP, rounded up (how KH2
    /// turns an enemy table's min/max damage pair into absolute values).
    pub fn from_permyriad(max_hp: u32, min_permyriad: u32, max_permyriad: u32) -> StatBounds {
        let f = |p: u32| ((max_hp as u64 * p as u64 + 9_999) / 10_000) as u32;
        StatBounds { min: f(min_permyriad), max: f(max_permyriad) }
    }
}

/// Percent bonuses the attacker adds on top of the base hit. 100 = none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Boosts {
    pub fire: u16,
    pub blizzard: u16,
    pub thunder: u16,
    /// Per combo hit on a combo finisher (Combo Boost). `None` = ability off.
    pub combo: Option<u16>,
    /// Per air-combo hit on an air finisher (Air Combo Boost).
    pub air_combo: Option<u16>,
    /// On reaction commands (Reaction Boost).
    pub reaction: Option<u16>,
}

impl Default for Boosts {
    fn default() -> Self {
        Boosts { fire: 100, blizzard: 100, thunder: 100, combo: None, air_combo: None, reaction: None }
    }
}

/// Damage reduction that kicks in at low HP (a "damage control" style
/// ability): when HP is at most `hp_fraction` of max, damage taken is
/// scaled by `percent`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LowHpGuard {
    pub hp_fraction: f32,
    pub percent: u32,
}

/// A fighter's live combat state.
#[derive(Clone, Debug, PartialEq)]
pub struct Combatant {
    pub id: u32,
    pub team: Team,
    pub stats: Stats,
    pub hp: u32,
    pub max_hp: u32,
    pub resist: Resistances,
    /// Attacker-side bounds on the base term (KH2 LVPM max/min strength).
    pub attack_bounds: StatBounds,
    /// Defender-side bounds on the base term (KH2 ENMP max/min damage).
    pub damage_bounds: StatBounds,
    /// Subtracted from the base term while HP is at or below a quarter of
    /// max (the Defender ability). 0 = off.
    pub low_hp_defense: u16,
    pub low_hp_guard: Option<LowHpGuard>,
    pub boosts: Boosts,
    /// Live combo counters, read by combo/air-combo boosts.
    pub combo_hits: u8,
    pub air_combo_hits: u8,
    /// Revenge build-up. When it reaches `revenge_limit` the defender is
    /// flagged to retaliate (`HitOutcome::revenge_triggered`).
    pub revenge: u32,
    pub revenge_limit: Option<u32>,
    /// While true, damaging hits are ignored entirely.
    pub invincible: bool,
    /// While true, attacks that don't pierce armor are blocked.
    pub guarding: bool,
}

impl Combatant {
    pub fn new(id: u32, team: Team, stats: Stats) -> Combatant {
        Combatant {
            id,
            team,
            stats,
            hp: stats.hp,
            max_hp: stats.hp,
            resist: Resistances::default(),
            attack_bounds: StatBounds::default(),
            damage_bounds: StatBounds::default(),
            low_hp_defense: 0,
            low_hp_guard: None,
            boosts: Boosts::default(),
            combo_hits: 0,
            air_combo_hits: 0,
            revenge: 0,
            revenge_limit: None,
            invincible: false,
            guarding: false,
        }
    }

    pub fn with_resist(mut self, resist: Resistances) -> Self {
        self.resist = resist;
        self
    }

    pub fn with_attack_bounds(mut self, b: StatBounds) -> Self {
        self.attack_bounds = b;
        self
    }

    pub fn with_damage_bounds(mut self, b: StatBounds) -> Self {
        self.damage_bounds = b;
        self
    }

    pub fn is_dead(&self) -> bool {
        self.hp == 0
    }

    /// Whether `self` may be hurt by an attack from `attacker` under `filter`.
    pub fn is_valid_target_of(&self, attacker: &Combatant, filter: crate::TargetFilter) -> bool {
        if self.id == attacker.id {
            return false;
        }
        match filter {
            crate::TargetFilter::Everyone => true,
            crate::TargetFilter::Opponents => self.team != attacker.team,
        }
    }
}
