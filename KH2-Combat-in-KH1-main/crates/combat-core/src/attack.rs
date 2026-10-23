use crate::Element;

/// What an attack does when it connects. Mirrors KH2's attack "type" field.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AttackKind {
    /// Deals damage; blocked by guard.
    #[default]
    Normal,
    /// Deals damage and ignores guard / armor.
    PierceArmor,
    /// A guard hitbox: deals no damage, blocks incoming attacks.
    Guard,
    /// A strong guard hitbox.
    SuperGuard,
    /// Scripted special hit. Treated like `Normal` for damage.
    Special,
    /// Heals the target instead of hurting it.
    Cure,
    /// A stronger cure (KH2 "C Cure"). Heals.
    CureStrong,
}

impl AttackKind {
    pub fn from_id(id: u8) -> AttackKind {
        match id {
            1 => AttackKind::PierceArmor,
            2 => AttackKind::Guard,
            3 => AttackKind::SuperGuard,
            4 => AttackKind::Special,
            5 => AttackKind::Cure,
            6 => AttackKind::CureStrong,
            _ => AttackKind::Normal,
        }
    }

    pub fn deals_damage(self) -> bool {
        matches!(self, AttackKind::Normal | AttackKind::PierceArmor | AttackKind::Special)
    }

    pub fn heals(self) -> bool {
        matches!(self, AttackKind::Cure | AttackKind::CureStrong)
    }

    pub fn is_guard(self) -> bool {
        matches!(self, AttackKind::Guard | AttackKind::SuperGuard)
    }
}

/// Which attacker stat an attack's damage scales from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatSource {
    #[default]
    Strength,
    Magic,
    /// No stat: the stat term is 0 (damage then comes only from the
    /// defender's minimum-damage bound).
    None,
}

impl StatSource {
    pub fn from_id(id: u8) -> StatSource {
        match id {
            0 => StatSource::Strength,
            1 => StatSource::Magic,
            _ => StatSource::None,
        }
    }
}

/// What happens when this attack hits something that reflects or guards.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Deflect {
    Reflect,
    Guard,
    #[default]
    Nothing,
}

impl Deflect {
    pub fn from_id(id: u8) -> Deflect {
        match id {
            0 => Deflect::Reflect,
            1 => Deflect::Guard,
            _ => Deflect::Nothing,
        }
    }
}

/// Which side an attack is allowed to hurt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TargetFilter {
    /// Only the attacker's opponents.
    #[default]
    Opponents,
    /// Opponents and the attacker's allies.
    Everyone,
}

/// One attack's parameters. Every field is plain data so the struct can be
/// filled from any source. KH2-specific values that have no general meaning
/// yet (reaction ids, effect ids) are carried as raw numbers for the host
/// game to interpret.
#[derive(Clone, Debug, PartialEq)]
pub struct AttackDef {
    /// Stable id for lookup and logging.
    pub id: u32,
    pub kind: AttackKind,
    /// Damage multiplier on (attack stat - defense), in percent (100 = x1).
    pub power: u16,
    pub element: Element,
    pub targets: TargetFilter,
    pub stat: StatSource,
    /// Host-defined reaction id (flinch, knock-up, ...). KH2 raw value.
    pub reaction: u8,
    /// Horizontal / vertical knockback strength. The host divides by weight.
    pub knockback: (f32, f32),
    /// Minimum frames between two hits of the same swing on the same target.
    /// 0 means the swing can hit each target once.
    pub hit_interval: u16,
    /// Drive gauge gained by the attacker per hit (raw KH2 units).
    pub drive_gain: u8,
    /// HP gained by the attacker per hit (raw KH2 units).
    pub hp_drain: u8,
    /// Revenge build-up added to the defender per hit.
    pub revenge: u8,
    /// Behaviour against reflecting / guarding targets.
    pub on_reflect_self: Deflect,
    pub on_reflect_other: Deflect,
    pub combo_finisher: bool,
    pub air_combo_finisher: bool,
    pub reaction_command: bool,
    /// Grouping used by the host for combo rules.
    pub combo_group: u8,
}

impl Default for AttackDef {
    fn default() -> Self {
        AttackDef {
            id: 0,
            kind: AttackKind::Normal,
            power: 100,
            element: Element::Physical,
            targets: TargetFilter::Opponents,
            stat: StatSource::Strength,
            reaction: 0,
            knockback: (0.0, 0.0),
            hit_interval: 0,
            drive_gain: 0,
            hp_drain: 0,
            revenge: 0,
            on_reflect_self: Deflect::Nothing,
            on_reflect_other: Deflect::Nothing,
            combo_finisher: false,
            air_combo_finisher: false,
            reaction_command: false,
            combo_group: 0,
        }
    }
}
