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

/// Bounds on the `(attack stat - defense)` term before multipliers.
/// One of two candidate readings of KH2's per-enemy min/max damage fields;
/// see [`HitClamp`] for the other (the current default in `kh2-data`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BaseClamp {
    pub min: i32,
    pub max: i32,
}

/// Per-hit bounds on final damage, as parts-per-10000 of the defender's
/// max HP (10000 = 100%). KH2 enemy tables carry a min/max pair per enemy;
/// the evidence that they mean this is in `docs/combat-model.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HitClamp {
    pub min_permyriad: u32,
    pub max_permyriad: u32,
}

impl HitClamp {
    /// `(min, max)` damage for a defender with `max_hp`.
    pub fn bounds(&self, max_hp: u32) -> (u32, u32) {
        let lo = (max_hp as u64 * self.min_permyriad as u64 / 10_000) as u32;
        let hi = (max_hp as u64 * self.max_permyriad as u64 / 10_000) as u32;
        (lo, hi.max(lo).max(1))
    }
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
    pub base_clamp: Option<BaseClamp>,
    pub hit_clamp: Option<HitClamp>,
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
            base_clamp: None,
            hit_clamp: None,
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

    pub fn with_base_clamp(mut self, clamp: BaseClamp) -> Self {
        self.base_clamp = Some(clamp);
        self
    }

    pub fn with_hit_clamp(mut self, clamp: HitClamp) -> Self {
        self.hit_clamp = Some(clamp);
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
