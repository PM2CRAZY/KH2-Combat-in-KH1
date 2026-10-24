//! Which game's moves base Sora uses, per kind (the user: "toggle what sora uses for his ground
//! and aerial moves": Ground Combo, Aerial Combo, Magic, Aerial Magic, each KH1 | KH2). Read
//! from `kh1-kh2.ini` at start, switched in game from the Config menu (`config`), kept in the
//! ini. Drive Forms keep their own KH2 movesets whatever these say (the user's choice).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

/// A kind of move whose style can be switched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    GroundCombo,
    AirCombo,
    Magic,
    AirMagic,
    Jump,
    Glide,
    DodgeRoll,
    /// KH2's animations over KH1's own non-combat actions (an item's use), `pose::follow`.
    MiscAnims,
    /// Which model Sora is drawn with: KH1's own, or KH2's (`model`). Not a move: the same in
    /// Drive Forms (their own KH2 models).
    PlayerModel,
    /// Sora's battle quotes (KH2's voice on KH2's moves, or KH1's): everywhere, forms too.
    BattleQuotes,
    /// Which Cure base Sora casts with Magic / Aerial Magic at KH2 (the tester, thread
    /// kh2-magic-and-reflect-native, answer 5: "make a menu in the magic config to change what
    /// gets casted kh1 cure or kh2"): KH2's (its script, `formmagic`) or KH1's own spell on KH2's
    /// Cure cast motion, as KH1's Gravity / Stop / Aero are there.
    Cure,
}

impl Kind {
    pub const EVERY: [Kind; 11] = [Kind::GroundCombo, Kind::AirCombo, Kind::Magic, Kind::AirMagic, Kind::Jump, Kind::Glide, Kind::DodgeRoll, Kind::MiscAnims, Kind::PlayerModel, Kind::BattleQuotes, Kind::Cure];

    /// Its ini key.
    pub fn key(self) -> &'static str {
        match self {
            Kind::GroundCombo => "ground_combo",
            Kind::AirCombo => "air_combo",
            Kind::Magic => "magic",
            Kind::AirMagic => "air_magic",
            Kind::Jump => "jump_style",
            Kind::Glide => "glide_style",
            Kind::DodgeRoll => "roll_style",
            Kind::MiscAnims => "misc_anims",
            Kind::PlayerModel => "player_model",
            Kind::BattleQuotes => "battle_quotes",
            Kind::Cure => "cure_style",
        }
    }

    /// Its Config menu label.
    pub fn label(self) -> &'static str {
        match self {
            Kind::GroundCombo => "Base Combo",
            Kind::AirCombo => "Aerial Base Combo",
            Kind::Magic => "Magic",
            Kind::AirMagic => "Aerial Magic",
            Kind::Jump => "Jump",
            Kind::Glide => "Glide",
            Kind::DodgeRoll => "Dodge Roll",
            Kind::MiscAnims => "Misc Animations",
            Kind::PlayerModel => "Player Model",
            Kind::BattleQuotes => "Battle Quotes",
            Kind::Cure => "Cure",
        }
    }

    fn cell(self) -> &'static AtomicBool {
        &KH2[self as usize]
    }
}

/// KH2's (true) or KH1's, per `Kind`: every kind KH1's by default (the user: "kh1 is on by default
/// for all settings").
static KH2: [AtomicBool; 11] = [const { AtomicBool::new(false) }; 11];
static INIT: Once = Once::new();

fn init() {
    INIT.call_once(|| {
        let s = super::settings::get();
        for (k, v) in Kind::EVERY.iter().zip(s.styles) {
            k.cell().store(v, Ordering::Relaxed);
        }
    });
}

/// Base Sora uses KH2's moves of this kind (ignoring Drive Forms: callers check `form::in_form`).
pub fn kh2(kind: Kind) -> bool {
    init();
    kind.cell().load(Ordering::Relaxed)
}

/// Base Sora uses KH1's moves of this kind now: not in a Drive Form, and set to KH1.
pub fn kh1_now(kind: Kind) -> bool {
    !kh2(kind) && !super::form::in_form()
}

/// Switches a kind (the Config menu), kept in the ini.
pub fn set(kind: Kind, kh2: bool) {
    init();
    kind.cell().store(kh2, Ordering::Relaxed);
    super::settings::save(kind.key(), if kh2 { "kh2" } else { "kh1" });
    super::log(&format!("move style: {} {}", kind.label(), if kh2 { "KH2" } else { "KH1" }));
}
