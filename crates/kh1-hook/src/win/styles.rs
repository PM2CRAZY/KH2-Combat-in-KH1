//! Which game's moves base Sora uses, per kind (the user: "toggle what sora uses for his ground
//! and aerial moves": Ground Combo, Aerial Combo, then each spell, each KH1 | KH2). Read
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
    /// Which Cure base Sora casts (the tester, thread kh2-magic-and-reflect-native, answer 5:
    /// "make a menu in the magic config to change what gets casted kh1 cure or kh2"; m4: KH1's is
    /// KH1's whole Cure): KH2's (its script, `formmagic`) or KH1's own, on the ground and in the
    /// air (since the per-spell rows, m19, no longer only with Magic at KH2).
    Cure,
    /// Sora's hit reactions (the tester, thread soras-hit-reactions-native: KH2's reaction
    /// motions on KH1's reaction requests, `kh2slots::REACTIONS`; KH1 still runs every reaction:
    /// its no-damage timer, HUD, MP, voices and tallies). Base Sora on KH1's states only.
    HitReactions,
    /// Sora's recoil rules when his attack meets an enemy's attack or guard (the tester, thread
    /// kh2-recoil-and-parry-system, answer 4: "if you switch to kh1 all moves including kh2 uses
    /// kh1's rules exactly and vice versa"): KH2's recoil motions, push, hop and no protection on
    /// KH1's recoil requests (`recoil`, `kh2slots::RECOILS`), KH2's clash rules on our attack data.
    Recoil,
    /// Which Fire, Blizzard and Thunder base Sora casts, KH1's or KH2's, on the ground and in the
    /// air (the tester, thread kh2-effects-in-kh1s-own-effect-system m19: "each magic should have
    /// its own settings of which magic you can choose, e.g fire: kh1 | kh2 and that would change
    /// what gets casted"). Cure's is `Cure`. These replace the Magic / Aerial Magic rows
    /// (`Magic` / `AirMagic` stay as the cast slots' kind, `kh2slots`, and read the old ini keys).
    Fire,
    Blizzard,
    Thunder,
    /// The cast animation of a spell base Sora casts (KH1's own cast motion, or KH2's cast motion
    /// as a slot on KH1's request, `kh2slots::CASTS`): the tester, m19, "you can also change the
    /// animation of magic with the exception being reflect, and fire". KH1's spell or KH2's alike
    /// (m20: each casts on that animation's cast frame, `formmagic::anim_kh2`); Fire's animation
    /// is its spell's, Reflect's always KH2's.
    BlizzardAnim,
    ThunderAnim,
    CureAnim,
    GravityAnim,
    StopAnim,
    AeroAnim,
    /// How base Sora moves while he casts in the air, every spell (KH1's or KH2's, Reflect too):
    /// KH1's air cast (its 30 hop, half the sideways speed, KH1's fall, no hover) or KH2's (its 40
    /// hop, the speed kept, KH2's gravity and drag, the cast motion's hover with the arc's clock
    /// running), whatever the Physics row says (`shared::physics::cast_rule`; the tester, m19:
    /// "change air magic to air physics kh1 | kh2 ... remove anything related to the main physics
    /// setting that has magic in it and put it in this new air setting").
    AirPhysics,
    /// Whether base Sora's grounded casts of KH1's own spells take a step of the combo, and his
    /// grounded casts on KH1's cast animation (KH1's spell, or KH2's on KH1's animation row) open
    /// a combo window by KH2's Fire rule (KH2: on; KH1: KH1's casts as they are). The tester, combo row selection after
    /// m29: "make a config for magic that allows you to combo out of grounded magic like kh2 ...
    /// for kh1 magics logic you just apply kh2s fire combo logic on the first shot and the repeated
    /// shot animation" (`formmagic::kh1_cast_window`).
    MagicCombo,
}

impl Kind {
    pub const EVERY: [Kind; COUNT] = [
        Kind::GroundCombo,
        Kind::AirCombo,
        Kind::Magic,
        Kind::AirMagic,
        Kind::Jump,
        Kind::Glide,
        Kind::DodgeRoll,
        Kind::MiscAnims,
        Kind::PlayerModel,
        Kind::BattleQuotes,
        Kind::Cure,
        Kind::HitReactions,
        Kind::Recoil,
        Kind::Fire,
        Kind::Blizzard,
        Kind::Thunder,
        Kind::BlizzardAnim,
        Kind::ThunderAnim,
        Kind::CureAnim,
        Kind::GravityAnim,
        Kind::StopAnim,
        Kind::AeroAnim,
        Kind::AirPhysics,
        Kind::MagicCombo,
    ];

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
            // (Its own key since the spell rows: `cure_style` only counted with Magic at KH2.)
            Kind::Cure => "magic_cure",
            Kind::HitReactions => "hit_reactions",
            Kind::Recoil => "recoil",
            Kind::Fire => "magic_fire",
            Kind::Blizzard => "magic_blizzard",
            Kind::Thunder => "magic_thunder",
            Kind::BlizzardAnim => "anim_blizzard",
            Kind::ThunderAnim => "anim_thunder",
            Kind::CureAnim => "anim_cure",
            Kind::GravityAnim => "anim_gravity",
            Kind::StopAnim => "anim_stop",
            Kind::AeroAnim => "anim_aero",
            Kind::AirPhysics => "magic_air_physics",
            Kind::MagicCombo => "magic_combo",
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
            Kind::HitReactions => "Hit Reactions",
            Kind::Recoil => "Recoil",
            Kind::Fire => "Fire",
            Kind::Blizzard => "Blizzard",
            Kind::Thunder => "Thunder",
            Kind::BlizzardAnim => "Blizzard Animation",
            Kind::ThunderAnim => "Thunder Animation",
            Kind::CureAnim => "Cure Animation",
            Kind::GravityAnim => "Gravity Animation",
            Kind::StopAnim => "Stop Animation",
            Kind::AeroAnim => "Aero Animation",
            Kind::AirPhysics => "Air Physics",
            Kind::MagicCombo => "Magic Combo",
        }
    }

    fn cell(self) -> &'static AtomicBool {
        &KH2[self as usize]
    }
}

/// KH2's (true) or KH1's, per `Kind`: every kind KH1's by default (the user: "kh1 is on by default
/// for all settings").
static KH2: [AtomicBool; COUNT] = [const { AtomicBool::new(false) }; COUNT];

/// How many kinds there are (`Kind::EVERY`, the ini's `settings::styles`).
pub const COUNT: usize = 24;
static INIT: Once = Once::new();

fn init() {
    INIT.call_once(|| {
        let s = super::settings::get();
        for (k, v) in Kind::EVERY.iter().zip(s.styles) {
            k.cell().store(v, Ordering::Relaxed);
        }
    });
}

/// The Config menu's setting for this kind (KH2's or KH1's), as the player chose it.
pub fn setting(kind: Kind) -> bool {
    init();
    kind.cell().load(Ordering::Relaxed)
}

/// Base Sora uses KH2's moves of this kind (ignoring Drive Forms: callers check `form::in_form`):
/// the setting. (t488 made a KH2-only combo ability turn the Base / Aerial Base Combo to KH2's
/// whatever the setting said; the tester, combo row selection m19: "the ability should not ignore
/// the combat settings". At KH1 those abilities now act on KH1's own combo: Negative Combo
/// shortens KH1's length inside its row picker, Finishing Plus chains KH1's finishers,
/// `kh1picker`, `combomods`.)
pub fn kh2(kind: Kind) -> bool {
    setting(kind)
}

/// A combo ability KH1 doesn't have is equipped (`note_abilities`), for the log.
static KH2_COMBO_ABILITY: AtomicBool = AtomicBool::new(false);

/// The combo abilities only KH2 has: Finishing Plus, Negative Combo and Berserk Charge (KH1 has
/// Combo Plus, Air Combo Plus and Combo Master itself).
pub fn kh2_combo_ability(a: &character_core::Abilities) -> bool {
    a.finishing_plus > 0 || a.negative_combo > 0 || a.equipped.contains(&character_core::BERSERK_CHARGE)
}

/// Sora's abilities as KH1's menu gives them (`menu`, base Sora's own: a Drive Form's don't count).
/// Logged when a KH2-only combo ability comes or goes, with what it does at each setting.
pub fn note_abilities(a: &character_core::Abilities) {
    let on = kh2_combo_ability(a);
    if KH2_COMBO_ABILITY.swap(on, Ordering::Relaxed) != on {
        let which = [(a.finishing_plus > 0, "Finishing Plus"), (a.negative_combo > 0, "Negative Combo"), (a.equipped.contains(&character_core::BERSERK_CHARGE), "Berserk Charge")]
            .iter()
            .filter(|w| w.0)
            .map(|w| w.1)
            .collect::<Vec<_>>()
            .join(", ");
        super::log(&format!(
            "move style: a combo ability only KH2 has is {} ({}): it follows the settings (Base Combo {}, Aerial Base Combo {}; at KH1 on KH1's own combo)",
            if on { "equipped" } else { "no longer equipped" },
            if on { which.as_str() } else { "none" },
            if setting(Kind::GroundCombo) { "KH2" } else { "KH1" },
            if setting(Kind::AirCombo) { "KH2" } else { "KH1" }
        ));
    }
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
