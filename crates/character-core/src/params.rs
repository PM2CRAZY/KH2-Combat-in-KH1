//! Tunable numbers. Every field can be filled from any source; `kh2-data`
//! fills them from the player's install (`03system.bin` pref tables).
//! Units: KH2 world units (about 1 cm) and 60 Hz frames.

/// Base movement (KH2 `prty`, `sstm`) and the physics constants the
/// original keeps in code.
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
    /// Largest downward step per frame while falling.
    pub fall_max: f32,
    /// Gravity for the falling half of a jump (units per frame^2).
    /// KH2: 20/49.
    pub gravity: f32,
    /// Fraction of the current air velocity kept per tick when steering
    /// (KH2: 0.9; the remaining 0.1 is the stick target).
    pub air_keep: f32,
    /// Fraction of the air velocity kept per tick with no stick input
    /// (KH2: 0.95).
    pub air_drag: f32,
    /// Air steering that adds to the velocity instead of easing it (0: KH2's `air_keep`). KH1's
    /// air state (`0x1402AD510`): with the stick, each 60 Hz frame adds `stick direction x the
    /// walk or run speed x air_accel` (its stats' `+0x14`), the length kept to the larger of the
    /// old length and `run_speed` (so speed above the run is kept, never gained). For a host
    /// that gives the controller KH1's physics.
    pub air_accel: f32,
    /// Stick tilt at or above which the character runs (below it, past 0, he walks). KH2: 0.8,
    /// in the code (Steam KH2 `0x1403a8fa0` / `0x1403aa700`: the walk speed `+0x128` under 0.8,
    /// the run speed `+0x12C` from it).
    pub run_threshold: f32,
    /// Ledge grab. `margin == 0` turns it off (as in the game).
    pub hang: HangParams,
    /// Horizontal speed per unit of hop height when repelled in the air
    /// (KH2 `sstm` hit_back_to_jump: 0.048).
    pub hit_back_to_jump: f32,
    /// Frames of no air commands after the head hits a ceiling (KH2
    /// `sstm` ceiling_disable_command_time: 16).
    pub ceiling_lock: f32,
    /// Ground inertia (`prty` acceleration / brake; Wisdom Form 0.85 / 0.97, Sora 0 = none):
    /// steering, the velocity becomes `target × (1 − acceleration) + velocity × acceleration`;
    /// with the stick let go, it is multiplied by `brake` (he slides to a stop).
    pub acceleration: f32,
    pub brake: f32,
    /// First-person eye height above the feet (`prty` subjective_offset; Sora 150). KH2's
    /// first-person camera places the eye there (Steam KH2 `0x1403b5c50`).
    pub subjective_offset: f32,
}

/// Ledge-grab numbers (KH2 `prty` hang_height/hang_margin, `sstm`
/// hang_range_*, and the character's background-collision radius).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HangParams {
    /// How far below the ledge the feet hang, and where the search is
    /// centred above the feet.
    pub height: f32,
    /// The ledge must be at least this high above the ground below.
    pub margin: f32,
    /// Search window above / below the centre.
    pub range_h: f32,
    pub range_l: f32,
    /// Horizontal search reach beyond the body radius.
    pub range_xz: f32,
    /// Background-collision radius: hanging keeps this gap from the wall.
    pub body_radius: f32,
}

impl Default for MoveParams {
    /// Placeholder speeds for hosts that bring their own data; the physics
    /// constants are KH2's.
    fn default() -> Self {
        MoveParams {
            walk_speed: 2.0,
            run_speed: 8.0,
            jump_height: 150.0,
            turn_speed: 0.25,
            fall_max: 16.0,
            gravity: KH2_GRAVITY,
            air_keep: 0.9,
            air_drag: 0.95,
            air_accel: 0.0,
            run_threshold: 0.8,
            hang: HangParams::default(),
            hit_back_to_jump: 0.048,
            ceiling_lock: 16.0,
            acceleration: 0.0,
            brake: 0.0,
            subjective_offset: 150.0,
        }
    }
}

/// KH2's gravity constant (units per frame^2 at 60 Hz).
pub const KH2_GRAVITY: f32 = 20.0 / 49.0;

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
    pub auto_lock_on_range: f32,
    pub lock_on_range: f32,
    pub lock_on_release_range: f32,
    /// Ground lunge speed per unit of distance to the target
    /// (KH2 sstm "attack distance to speed"). Only rows with a nonzero
    /// speed window lunge; Sora's ground rows don't.
    pub distance_to_speed: f32,
    /// The plyr record's `attack_j_front` (`+0x2C`; Sora's -5): with his collision radius, how
    /// far ahead KH2 tests the floor at an attack press (`Character::set_drop_ahead`). The exe's
    /// `+0x150` record there is plyr (Steam KH2, Checked (decomp): its other reads are plyr's by
    /// field order, `+0x50` / `+0x54` / `+0x58` the blow recovery h / v / time `0x1403D6BB0` /
    /// `0x1403D6D50`, `+0x5C` / `+0x60` / `+0x64` / `+0x68` the auto lock-on range, min h, max h
    /// and time `0x1403BE2B0` / `0x1403BE4B0` / `0x1403DCCD0`).
    pub jump_front: f32,
    /// His own target shape (the model's first collision shape of kind 3; Sora's: 70 up, radius
    /// 55): its centre's height above his feet and its radius. An attack row's hop is measured
    /// from it (Steam KH2 `0x1403D78D0` -> `0x1403D7520`: the height from this centre to the
    /// target's, the gap less both radii). 0 without a model.
    pub target_lift: f32,
    pub target_radius: f32,
}

impl Default for AttackParams {
    fn default() -> Self {
        AttackParams { auto_lock_on_range: 300.0, lock_on_range: 1200.0, lock_on_release_range: 1500.0, distance_to_speed: 0.1, jump_front: 0.0, target_lift: 0.0, target_radius: 0.0 }
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
    /// Every swing counts as connecting for the combo counter.
    pub combo_master: bool,
    /// Press jump while launched to right yourself in mid-air.
    pub aerial_recovery: bool,
    /// Hyper Healing (ours, for hosts that buy it): back on your feet in half the time after
    /// a launch, and Aerial Recovery / Retaliating Slash from the launch's 2nd frame.
    pub hyper_healing: bool,
    /// Guard Plus (ours): the guard holds blows from every side, not only the front.
    pub guard_all_round: bool,
    /// Ours (the KH1 port, the user): without Combo Master a swing that hasn't connected is
    /// the only one; the next press starts a new first swing once it ends.
    pub one_swing_on_miss: bool,
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
            combo_master: false,
            aerial_recovery: false,
            hyper_healing: false,
            guard_all_round: false,
            one_swing_on_miss: false,
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

/// Combo lengths. KH2: ground and air combos of 3 and one finisher;
/// Combo Plus / Air Combo Plus add one, Negative Combo removes one from
/// both, Finishing Plus adds a finisher (checked in the executable).
#[derive(Clone, Debug, PartialEq)]
pub struct ComboRules {
    pub ground_base: u8,
    pub air_base: u8,
    pub finisher_base: u8,
}

impl Default for ComboRules {
    fn default() -> Self {
        ComboRules { ground_base: 3, air_base: 3, finisher_base: 1 }
    }
}
