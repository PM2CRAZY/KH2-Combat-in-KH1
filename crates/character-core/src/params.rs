//! Tunable numbers. Every field can be filled from any source; `kh2-data`
//! fills them from the player's install (`03system.bin` pref tables).
//! Units: KH2 world units (about 1 cm) and 60 Hz frames.

/// Base movement (KH2 `prty`, `sstm`).
#[derive(Clone, Debug, PartialEq)]
pub struct MoveParams {
    /// Units per frame at partial stick tilt.
    pub walk_speed: f32,
    /// Units per frame at full tilt.
    pub run_speed: f32,
    /// Apex height of a normal jump.
    pub jump_height: f32,
    /// Max yaw change per frame (radians).
    pub turn_speed: f32,
    /// Terminal fall speed (units per frame).
    pub fall_max: f32,
    /// Downward acceleration (units per frame^2). Not found in any table;
    /// see `docs/movement-model.md`.
    pub gravity: f32,
    /// 0 means instant (KH2's value for Sora). Otherwise the velocity lerp
    /// factor per frame towards the stick target.
    pub acceleration: f32,
    pub brake: f32,
    /// Stick magnitude at or above which the character runs.
    pub run_threshold: f32,
}

impl Default for MoveParams {
    /// Placeholder values for hosts that bring their own data. These are
    /// not KH2's numbers.
    fn default() -> Self {
        MoveParams {
            walk_speed: 2.0,
            run_speed: 8.0,
            jump_height: 150.0,
            turn_speed: 0.25,
            fall_max: 16.0,
            gravity: 0.9,
            acceleration: 0.0,
            brake: 0.0,
            run_threshold: 0.5,
        }
    }
}

/// One level of the growth abilities (KH2 `fmab` record).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GrowthLevel {
    pub high_jump_height: f32,
    pub air_dodge_height: f32,
    pub air_dodge_speed: f32,
    /// Quick Run ("air slide" in the tables).
    pub quick_run_time: f32,
    pub quick_run_speed: f32,
    pub quick_run_brake: f32,
    pub quick_run_stop_brake: f32,
    pub quick_run_star_time: f32,
    pub glide_speed: f32,
    pub glide_fall_ratio: f32,
    pub glide_fall_height: f32,
    pub glide_fall_max: f32,
    pub glide_acceleration: f32,
    pub glide_start_height: f32,
    pub glide_end_height: f32,
    pub glide_turn_speed: f32,
    /// Invulnerable frames at the start of a dodge roll.
    pub dodge_roll_star_time: f32,
}

/// Attack targeting numbers (KH2 `plyr`, `sstm`).
#[derive(Clone, Debug, PartialEq)]
pub struct AttackParams {
    /// Height of the attack origin above the feet; combo-row height
    /// windows are measured from here.
    pub attack_y_offset: f32,
    pub first_v0: f32,
    pub combo_v0: f32,
    pub finish_v0: f32,
    pub auto_lock_on_range: f32,
    pub lock_on_range: f32,
    pub lock_on_release_range: f32,
    /// Stop approaching a target when this close (hypothesis).
    pub contact_distance: f32,
}

impl Default for AttackParams {
    fn default() -> Self {
        AttackParams {
            attack_y_offset: 60.0,
            first_v0: 8.0,
            combo_v0: 6.0,
            finish_v0: 12.0,
            auto_lock_on_range: 300.0,
            lock_on_range: 1200.0,
            lock_on_release_range: 1500.0,
            contact_distance: 90.0,
        }
    }
}

/// Which abilities the character has. Growth abilities use level 0 (not
/// owned) to 4 (MAX).
#[derive(Clone, Debug, PartialEq)]
pub struct Abilities {
    pub high_jump: u8,
    pub quick_run: u8,
    pub aerial_dodge: u8,
    pub glide: u8,
    pub dodge_roll: u8,
    pub guard: bool,
    pub combo_plus: u8,
    pub air_combo_plus: u8,
    pub finishing_plus: u8,
    pub negative_combo: u8,
    /// Item ids of equipped action abilities (for combo rows that need one).
    pub equipped: Vec<u16>,
}

impl Default for Abilities {
    fn default() -> Self {
        Abilities {
            high_jump: 0,
            quick_run: 0,
            aerial_dodge: 0,
            glide: 0,
            dodge_roll: 0,
            guard: true,
            combo_plus: 0,
            air_combo_plus: 0,
            finishing_plus: 0,
            negative_combo: 0,
            equipped: Vec::new(),
        }
    }
}

impl Abilities {
    /// Every growth ability at MAX.
    pub fn max_growth() -> Self {
        Abilities { high_jump: 4, quick_run: 4, aerial_dodge: 4, glide: 4, dodge_roll: 4, ..Default::default() }
    }
}

/// Combo lengths. KH2: base combo of 3 (two hits and a finisher);
/// Combo Plus adds one each, Negative Combo removes one (community).
#[derive(Clone, Debug, PartialEq)]
pub struct ComboRules {
    pub ground_base: u8,
    pub air_base: u8,
}

impl Default for ComboRules {
    fn default() -> Self {
        ComboRules { ground_base: 3, air_base: 3 }
    }
}
