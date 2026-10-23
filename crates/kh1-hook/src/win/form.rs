//! Wisdom Form (the user: "lets add wisdom form", from the second command page's Drive, Sora
//! recoloured blue). Chosen from `drive`'s list, it plays out as KH2's Drive does
//! (`kh2-bevy`'s `drive.rs`, from the exe):
//! - Sora's Drive motion (52, invulnerable) with base Sora's moveset; when it ends, the swap:
//!   the form's moveset, motions and combo rows (`P_EX100_MAGF`, moveset 3), his facing and
//!   lock kept, and its arrival (53). Its abilities: Sora's support abilities, not his action
//!   or growth ones, and the form's own growth ability, Quick Run, one level above Sora's (the user);
//! - the form runs on KH2's form gauge (`gauge`; the user, s318: "lets incorporate the drive
//!   gauge"; it ran on Sora's MP before): `(level + 3) × 600` frames, drained as KH2 drains it,
//!   the level the form's own growth ability's (`form_level`, s325: 1 to 4, so 4 to 7 bars);
//!   when it runs out (`drain`), or on the Revert command (the Drive row itself in the form), back
//!   to Sora, at once, in the air too:
//!   what was playing is cut and he is free again with his movement kept (walking, running or
//!   idle by the stick; in the air, falling). Knocked out: back at once;
//! - Drive and Revert are taken only on a frame Sora can change (`Character::can_change_form`:
//!   free, jumping, falling, landing, or at the end of a move or a cast), never buffered: a
//!   press before that does nothing (the menu refuses it, `combat::run_entry`);
//! - the change takes 45 frames (the user): Sora's Drive motion for `DRIVE_FRAMES`, then the
//!   swap and the form's short arrival; in the air he is held where he is through it.
//! - its shots and its spells: `formmagic`.
//! The cost is KH2's, in bars of KH2's Drive gauge (`gauge::cost`: Valor and Wisdom 3, Limit and
//! Master 4, Final 5, Anti 9; it was MP until s317). MP is for magic only, in a form too.
//!
//! Valor Form (stage 211, the user: "i want to try to add valor form"): the same change and
//! revert, `P_EX100_BTLF` (moveset 2, its two Keyblades' swings), its growth ability High Jump
//! (at MAX, as Wisdom's Quick Run), no magic (KH2's Valor can't cast), and its second Keyblade
//! (`valor`): entered only with an eligible one (the user: "If no keyblade is eligible to be in
//! that slot, dont let players go into valor form").
//!
//! Final Form (stage 231, the user: "lets work on implementing final form ... make it a secret"):
//! the same change and revert, `P_EX100_ULTF` (moveset 5), its growth ability Glide at MAX, its
//! two Keyblades floating where KH2's own weapon motions put them (the second always the Kingdom
//! Key, no menu slot: the user, "don't add the final form keyblade selection into the menu like we
//! did for valor, just default it to kingdom key"). Not in the Drive list: a secret button
//! code on the pad (`pad`) asks for it, as the Drive list would. Its magic is Wisdom's (the user:
//! "bring it to final form, with final form's animations"): `Kind::magic_sub`, `formmagic`.
//! (Since s271 Final is in the Drive list and the code asks for Anti Form.)
//!
//! Anti Form (the user: "that cheat code we were using ... I want you to add anti form. I want
//! 1:1 combat parity. I dont want it to cost any MP to enter, and I dont want sora to be able to
//! revert out of it"; s285 to s317 it took half his MP, "Antiform should take half of your MP"; since s318 it empties the Drive gauge): reached only by the secret code (`pad`), never from the Drive list
//! (`Kind::EVERY`, not `Kind::ALL`). `P_EX100_HTLF` (moveset 6): its own moveset, motions, combo
//! rows, movement numbers, effects and sounds, no Keyblade (KH2's Anti holds none). As KH2's:
//! - no magic, items, summons or KH1 specials (`combat`), no guard (no motion 42 in its set), no
//!   growth ability (00battle `fmlv`'s Anti rows give none, so `growth_up` doesn't apply);
//! - 1.5 times the damage taken (`hurt`, `character_core::drive::DriveParams::anti_damage`);
//! - no Revert: the Drive row greyed and refused (`usable`, `choose`, `drive`);
//! - Square (the user: "lets add anti-form's square move"): Anti Glide (row 11, motion 173) on the
//!   ground, standing or moving; Side Spin (row 12, motion 161) in a ground combo
//!   (`ANTI_SIDE_SPIN_ROW`). Neither guards.
//! It empties the Drive gauge (KH2's cost of 9; s285 to s317 it took half his MP) and wants 3 bars
//! to start (`drivegauge::ANTI_NEEDS`); it runs on its 6-bar form gauge, orbs draining it as in
//! KH2, and when that runs out (or knocked out) it is back to Sora.
//!
//! Limit Form (the user: "lets begin working on limit form"; `docs/drive-model.md`, "Limit Form in
//! the KH1 port"): in the Drive list after Wisdom, the same change and revert, `P_EX100_KH1F`
//! (moveset 0x42): its own moveset, motions, combo rows (Zantetsuken, Ripple Drive and Hurricane
//! Period as its finishers, the Zantetsu Counter from its guard), effects and sounds, and one
//! Keyblade (Sora's own, as in base form). Its growth ability is Dodge Roll, one level above
//! Sora's (as the others'). It keeps KH1 Sora's own look (no tint: the model drawn is KH1's
//! already). No magic: KH2's Limit Form has its Limit commands in place of Magic (Sonic Rave, Last
//! Arcanum, Strike Raid, Infinity), run by its model's own script (`limi`): `limits`, the Limit
//! menu in Magic's place. That menu is off for now (`limits::KH2_LIMITS`; the user: "lets comment
//! the KH2 limits, and for the moment put in the equivalent KH1 limits"): Magic is the Limit menu
//! of KH1's own five limits instead (equipped or not; its shortcuts KH2 FM's Limit Form layout),
//! run as KH1 does them, KH1 having him for each, the form on and its gauge draining all through.
//! It costs 4 bars and opens by the user's table (`unlocked`, `crate::story`).
//!
//! Master Form (the user: "lets take the rules and procedures we've done for wisdom form and valor
//! form, and lets make master form"; `docs/drive-model.md`, "Master Form in the KH1 port"): in the
//! Drive list after Limit, before Final (KH2's order), the same change and revert, `P_EX100_TRIF`
//! (moveset 4): its own moveset, motions, combo rows, effects and sounds, two Keyblades (the right
//! one in his hand, the second floating by his left, `leftblade`), its second Keyblade from its own
//! slot (`valor::master_keyblade`, the Equipment page's "M:" row; the Kingdom Key when it holds
//! none, as Final's). Its growth ability is Aerial Dodge, one level above Sora's; no guard (its
//! motion 42 is the repel's clip, with no guard window). It casts as Wisdom and Final do, on its
//! own cast motions, at KH2's Master sub (+ 20, `Kind::magic_sub`, `formmagic`). It costs
//! 4 bars and opens once Anti-Sora is beaten (`unlocked`, `crate::story`).
//!
//! Partners (the user: "Let's make drive forms require their specific party members to cast. For
//! example, wisdom form needs donald alive to activate in KH2"; `docs/drive-model.md`, "Partners"):
//! Valor needs Goofy, Wisdom Donald, Master and Final both, Limit no one, each in KH1's field party
//! and not knocked out (`crate::partners`, `field_party`); a guest in his place doesn't count.
//! Without them the Drive row is greyed (`usable`) and the choice refused with the reason
//! (`choose`). Only the start asks: a partner going down in the form doesn't end it.
//! While the form lasts (from the change's start) the partners it takes vanish, their HUD entries
//! too, and come back beside Sora when it ends (`vanish`; Anti Form takes both, if there).

use super::{log, pose};
use character_core::{Abilities, Character, EventKind};
use kh2_formats::battle::Battle;
use kh2_rig::RigAnim;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};
use std::sync::Arc;

pub use super::formmagic::{element_sounds, package_spell, shots_step, spell_file, LAST_PACKAGE, SPELL_NAMES};

pub const DRIVE_MOTION: u16 = 52;
pub const ARRIVE_MOTION: u16 = 53;
// (No revert motion: base Sora's motion 4, which the old revert played, is his fall loop (A004),
// and left him in KH2's fall on the ground, the user.)
/// Frames of Sora's Drive motion before the swap (its own runs 160; the user: "it should only
/// take 45 frames ... from start to end"): with the form's arrival (motion 53, 3 frames at 30
/// fps: 6), 45 in all.
const DRIVE_FRAMES: u32 = 39;
/// The revert's light and sound: Sora's package elements 15..27 (no motion of his starts them;
/// they mirror the Drive motion's 1..13, element 15 with sound 115 as element 1 has the change's
/// 114; Data, KH2's own call for them not traced), started on him as he comes back (the user:
/// "missing its sfx sound and effect when reverting back from a drive form").
pub const REVERT_ELEMENTS: [i16; 13] = [15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27];
/// The motion id the revert's effects and sound are started under (no motion of Sora's has it).
pub const REVERT_FX: u16 = u16::MAX - 2;
/// Anti Form's Side Spin: its PTYA row 12 (motion 161), Square in a ground combo (Data).
const ANTI_SIDE_SPIN_ROW: u8 = 12;


/// Upper Slash's ability (KH2's id): Sora's Square launch, given to Valor (`pose::port_moves`).
const UPPER_SLASH: u16 = 95;

/// The forms (`drive::FORMS` in the same order).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Valor,
    Wisdom,
    /// `P_EX100_KH1F`, KH1 Sora's look.
    Limit,
    /// `P_EX100_TRIF`, its second Keyblade floating.
    Master,
    Final,
    /// Only by the secret code (`pad`), never in the Drive list.
    Anti,
}

impl Kind {
    /// The Drive list's (Final too now, the user: "put it into the drive menu"; Limit after
    /// Wisdom and Master after Limit, as in KH2's list).
    pub const ALL: [Kind; 5] = [Kind::Valor, Kind::Wisdom, Kind::Limit, Kind::Master, Kind::Final];
    /// Every form (Anti too, the secret code's), in the enum's order (`code`).
    const EVERY: [Kind; 6] = [Kind::Valor, Kind::Wisdom, Kind::Limit, Kind::Master, Kind::Final, Kind::Anti];
    pub fn name(self) -> &'static str {
        match self {
            Kind::Valor => "Valor Form",
            Kind::Wisdom => "Wisdom Form",
            Kind::Limit => "Limit Form",
            Kind::Master => "Master Form",
            Kind::Final => "Final Form",
            Kind::Anti => "Anti Form",
        }
    }
    fn code(self) -> u8 {
        self as u8 + 1
    }
    fn from_code(c: u8) -> Option<Kind> {
        Kind::EVERY.get((c as usize).checked_sub(1)?).copied()
    }
    /// Its KH2 data, if it loaded.
    pub fn data(self, kh2: &pose::Kh2) -> Option<&pose::Form> {
        match self {
            Kind::Valor => kh2.valor.as_ref(),
            Kind::Wisdom => kh2.wisdom.as_ref(),
            Kind::Limit => kh2.limit.as_ref(),
            Kind::Master => kh2.master.as_ref(),
            Kind::Final => kh2.final_form.as_ref(),
            Kind::Anti => kh2.anti.as_ref(),
        }
    }
    /// With a second Keyblade (its form slot's: Valor's, Master's, Final's).
    pub fn two_blades(self) -> bool {
        matches!(self, Kind::Valor | Kind::Master | Kind::Final)
    }
    /// Its second Keyblade floats away from his hand (Master's left one, both of Final's), placed
    /// by KH2's blade bone (`leftblade::note_pose`).
    pub fn floats(self) -> bool {
        matches!(self, Kind::Master | Kind::Final)
    }
    /// KH2's magic sub offset in the form (`docs/drive-model.md`: + 10 in Wisdom, + 30 in Final;
    /// 03system `magi` and the spells' ATKP rows have subs 30..35 for Final), for a form that casts
    /// as Wisdom does (`formmagic`); `None` for one that can't cast (Valor, Limit, Anti: the
    /// command's form mask; Limit has its Limit commands instead, `limits`).
    /// (Final: the user, "lets try to incorporate our rules, logic, and process from wisdom forms'
    /// magic, and bring it to final form, with final form's animations". KH2 has no spell files
    /// of Final's own: MAGC's rows are the same as Sora's, and the form's moveset gives the
    /// motions under its ids, `P_EX100_ULTF`'s E36x / E37x.)
    /// (Master, + 20: the same rules on its own casts, `P_EX100_TRIF`'s D370..D374, with both its
    /// Keyblades' hits in them.)
    pub fn magic_sub(self) -> Option<i32> {
        match self {
            Kind::Valor | Kind::Limit | Kind::Anti => None,
            Kind::Wisdom => Some(10),
            Kind::Master => Some(20),
            Kind::Final => Some(30),
        }
    }
}

/// A request: 1 a form (`WANT`), 2 revert (the menu), 3 knocked out (`hurt`).
static REQUEST: AtomicU8 = AtomicU8::new(0);
/// The form asked for (`Kind::code`).
static WANT: AtomicU8 = AtomicU8::new(0);
/// The form he is in (`Kind::code`, 0 none).
static CURRENT: AtomicU8 = AtomicU8::new(0);
/// In a form (the menu's entry reads "Revert"), or changing into one.
static IN_FORM: AtomicBool = AtomicBool::new(false);
static CHANGING: AtomicBool = AtomicBool::new(false);

/// Sora went down (`hurt`): out of the form at once, when KH2 next has him.
pub fn knocked_out() {
    if in_form() || CHANGING.load(Ordering::Relaxed) {
        REQUEST.store(3, Ordering::Relaxed);
    }
}

pub fn in_form() -> bool {
    IN_FORM.load(Ordering::Relaxed)
}

/// The form he is in.
pub fn current() -> Option<Kind> {
    if !in_form() {
        return None;
    }
    Kind::from_code(CURRENT.load(Ordering::Relaxed))
}

/// In Wisdom Form (its shots, its spells, its look).
pub fn in_wisdom() -> bool {
    current() == Some(Kind::Wisdom)
}

/// In Valor Form (no magic).
pub fn in_valor() -> bool {
    current() == Some(Kind::Valor)
}

/// In Limit Form (no magic; its own effects and sounds).
pub fn in_limit() -> bool {
    current() == Some(Kind::Limit)
}

/// In Master Form.
pub fn in_master() -> bool {
    current() == Some(Kind::Master)
}

/// In Final Form.
pub fn in_final() -> bool {
    current() == Some(Kind::Final)
}

/// In a form whose second Keyblade floats (Master, Final: `Kind::floats`).
pub fn floating_blade() -> bool {
    current().is_some_and(Kind::floats)
}

/// In Anti Form (no magic, items, summons or specials, no Revert; 1.5 times the damage).
pub fn in_anti() -> bool {
    current() == Some(Kind::Anti)
}

/// The form under way for its partners (`vanish`): the one he is in, or changing into.
pub fn vanishing() -> Option<Kind> {
    if in_form() {
        return current();
    }
    if CHANGING.load(Ordering::Relaxed) {
        return Kind::from_code(WANT.load(Ordering::Relaxed));
    }
    None
}

/// The partners form `kind` takes while it lasts (`vanish`; `crate::partners::taken`).
pub fn takes_partners(kind: Kind) -> &'static [crate::partners::Partner] {
    crate::partners::taken(story_form(kind))
}

/// The form he is in, if it casts as Wisdom does (Wisdom, Final: `Kind::magic_sub`): its Fire,
/// Blizzard and Thunder KH2's (`formmagic`), its Cure, Gravity, Stop and Aero KH1's on its own
/// cast motions (`takeover::command_started`, `pose::follow`).
pub fn casting_form() -> Option<Kind> {
    current().filter(|k| k.magic_sub().is_some())
}

/// In a form with a second Keyblade (Valor, Master, Final).
pub fn two_blades() -> bool {
    current().is_some_and(|k| k.two_blades())
}

/// The secret code, kept unreadable on purpose (the user: it stays a secret): `code_bit`.
const CODE: [u8; 10] = [94, 94, 92, 92, 93, 95, 93, 95, 87, 84];

/// The pad bit of the code's `i`th button.
fn code_bit(i: usize) -> u16 {
    1 << (CODE[i] ^ 0x5A)
}
/// The buttons the code watches (any other pressed starts it over), and the longest gap.
const CODE_BUTTONS: u16 = (1 << 4) | (1 << 5) | (1 << 6) | (1 << 7) | (1 << 13) | (1 << 14) | (1 << 12) | (1 << 15);
const CODE_GAP_MS: u64 = 1500;
/// The code so far: buttons held last time, how far in, when the last one came; and whether it was
/// just completed (taken on the game thread, `before_step`).
static CODE_STATE: std::sync::Mutex<(u16, usize, Option<std::time::Instant>)> = std::sync::Mutex::new((0, 0, None));
static CODE_DONE: AtomicBool = AtomicBool::new(false);

/// The pad as pressed, every read (`probe::pad`): the secret code, once for Final Form (the user:
/// a button sequence, "in that exact sequence, put them into final form"), now for Anti Form
/// (the user: "that cheat code we were using ... I want you to add anti form").
pub fn pad(buttons: u16) {
    let Ok(mut g) = CODE_STATE.try_lock() else { return };
    let (held, at, last) = &mut *g;
    let pressed = buttons & !*held & CODE_BUTTONS;
    *held = buttons;
    if pressed == 0 {
        return;
    }
    if last.is_some_and(|t| t.elapsed().as_millis() as u64 > CODE_GAP_MS) {
        *at = 0;
    }
    *last = Some(std::time::Instant::now());
    for bit in (0..16).map(|i| 1u16 << i).filter(|b| pressed & b != 0) {
        if bit == code_bit(*at) {
            *at += 1;
        } else {
            *at = usize::from(bit == code_bit(0));
        }
        if *at == CODE.len() {
            *at = 0;
            CODE_DONE.store(true, Ordering::Relaxed);
            log("Anti Form: the secret code entered");
        }
    }
}

pub fn busy() -> bool {
    CHANGING.load(Ordering::Relaxed) || REQUEST.load(Ordering::Relaxed) != 0
}

/// KH2's magic sub offset now (`reflect`, `formmagic`): the form's (`Kind::magic_sub`), else 0.
pub fn magic_sub() -> i32 {
    current().and_then(Kind::magic_sub).unwrap_or(0)
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

/// KH1's HP change `(actor, amount)` (`0x1402A2790`: clamped to the maximum, its HUD mirror kept).
const HP_ADD_RVA: super::addr::Rva = super::addr::Rva(0x2A2790);

/// Sora's HP back to full as a Drive starts (the user: "activating a drive form should restore
/// soras health entirely", as KH2's does).
unsafe fn heal_full(sora: usize) {
    let stats = pose::ptr(rd::<u32>(sora + 0x6C));
    if stats < 0x10000 {
        return;
    }
    let (now, max) = (rd::<i32>(stats + 0x3C), rd::<i32>(stats + 0x40));
    if max > now && now > 0 {
        let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
        let add: unsafe extern "system" fn(usize, i32) -> u32 = std::mem::transmute(base + HP_ADD_RVA);
        add(sora, max - now);
        log(&format!("Drive: HP {now} -> {} of {max}", rd::<i32>(stats + 0x3C)));
    }
}

unsafe fn hp(sora: usize) -> i32 {
    let stats = pose::ptr(rd::<u32>(sora + 0x6C));
    if stats < 0x10000 {
        return 1;
    }
    rd::<i32>(stats + 0x3C)
}

/// Whether a Drive or Revert pressed now is taken (`combat::run_entry`; the user: "return
/// should only be accepted when you press return on an end frame", no queue): KH2 has Sora and
/// he can change on this frame.
pub fn accept_now() -> bool {
    // (Or in one of KH1's moves at its cut frame, out of a form: the user, "goes into the drive
    // submenu and selects a drive" from frame 49 of KH1's Fire.)
    !busy() && (super::takeover::with_kh2(|c, _| c.can_change_form()).unwrap_or(false) || !in_form() && super::takeover::kh1_cut_window())
}

/// Whether form `kind`'s entry can be chosen now (`drive`'s menu): reverting in a form (not in
/// Anti Form, which has no Revert), else the Drive gauge's bars for it (`gauge::can_pay`; the
/// list greys a form the gauge is short of) and KH2 having Sora, its data loaded, and what
/// the form itself needs (Valor's second Keyblade, `valor`; its partners, `own_needs`).
pub unsafe fn usable(sora: usize, kind: Kind) -> bool {
    if busy() {
        return false;
    }
    if in_form() {
        // (The user: "I dont want sora to be able to revert out of it".)
        return !in_anti();
    }
    unlocked(kind) && pose::kh2().is_some_and(|k| kind.data(k).is_some()) && (super::takeover::kh2_has() || super::takeover::kh1_cut_window()) && sora > 0x10000 && super::gauge::can_pay(kind) && own_needs(kind).is_ok()
}

/// The Drive list's form as `story`'s (Anti has none: the secret code's, always open).
fn story_form(kind: Kind) -> Option<crate::story::Form> {
    use crate::story::Form;
    match kind {
        Kind::Valor => Some(Form::Valor),
        Kind::Wisdom => Some(Form::Wisdom),
        Kind::Limit => Some(Form::Limit),
        Kind::Master => Some(Form::Master),
        Kind::Final => Some(Form::Final),
        Kind::Anti => None,
    }
}

/// The form is open to Sora (the Drive list's; Anti is the code's, always). The user's table
/// (`crate::story::FORM_TABLE`): by the Dive to the Heart's choice (`save + 0x48C`, set by
/// `0x1402835B0`: 0 Sword, 1 Shield, 2 Rod), the first form after Guard Armor, the second after
/// Opposite Armor and the third after the Pegasus Cup; Master Form after Anti-Sora, Final Form
/// after Riku 2 (Ansem-Riku). The flags are read from the save (`menu::story_flags`) and logged
/// at the start and whenever they change.
pub unsafe fn unlocked(kind: Kind) -> bool {
    let Some(form) = story_form(kind) else { return true };
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    let Some(save) = super::menu::save_block(base) else { return false };
    let Some(flags) = super::menu::story_flags_at(save) else { return false };
    let choice = rd::<u8>(save + crate::story::DIVE_CHOICE);
    static LAST: AtomicU32 = AtomicU32::new(u32::MAX);
    let key = (choice as u32) << 8 | flags.bits() as u32;
    if LAST.swap(key, Ordering::Relaxed) != key {
        let open: Vec<&str> = Kind::ALL.iter().filter(|&&k| story_form(k).is_some_and(|f| crate::story::unlocked(f, choice, &flags))).map(|k| k.name()).collect();
        log(&format!(
            "Drive forms: Dive choice {choice} ({}); {}: open {}",
            crate::story::choice_name(choice),
            flags.describe(),
            if open.is_empty() { "none".to_string() } else { open.join(", ") }
        ));
    }
    crate::story::unlocked(form, choice, &flags)
}

/// What a form needs besides its Drive bars: Valor its second Keyblade (`valor::keyblade`), and each Drive
/// list form its partners (`partners`).
unsafe fn own_needs(kind: Kind) -> Result<(), String> {
    match kind {
        Kind::Valor => super::valor::keyblade().map(|_| ()).ok_or_else(|| String::from("no Keyblade for the second hand (one Sora owns that isn't equipped)"))?,
        // (Master and Final fall back to the Kingdom Key: never refused.)
        Kind::Wisdom | Kind::Limit | Kind::Master | Kind::Final | Kind::Anti => {}
    }
    // (Anti, the secret code's, needs no one: `story_form` has none for it.)
    match story_form(kind) {
        Some(form) => crate::partners::check(form, kind.name(), &field_party()),
        None => Ok(()),
    }
}

/// KH1's field party (`0x142D37C80`: Sora, then the two others).
const PARTY_RVA: super::addr::Rva = super::addr::Rva(0x2D37C80);

/// The two actors after Sora in KH1's field party, each as his party member id and whether he is
/// down (`docs/drive-model.md`, "Partners"). The id is the actor's object's `+0x4C`
/// (`actor + 0x130`, a handle): KH1's own party code reads it as the member, the stats refresh
/// (`0x1402A2CC0`) taking his save record and the HUD code (`0x1402873A0`, through `0x140282EC0`)
/// finding him in the save's party list (`save + 0x48E`) by it. Down: KH1's knocked-out bits
/// (`+0x88 & 0x30`, the test `0x140299A30`: 0x10 set as his HP runs out, 0x20 while he lies down
/// waiting to get up) or no HP left (stats `+0x3C`).
unsafe fn field_party() -> [Option<crate::partners::Member>; 2] {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    let mut out = [None; 2];
    for (slot, m) in out.iter_mut().enumerate() {
        let a = rd::<u64>(base + PARTY_RVA + (slot + 1) * 8) as usize;
        if a < 0x10000 || !super::crash::readable(a) || !super::crash::readable(a + 0x378) {
            continue;
        }
        let obj = pose::ptr(rd::<u32>(a + 0x130));
        if obj < 0x10000 || !super::crash::readable(obj + 0x4D) {
            continue;
        }
        let stats = pose::ptr(rd::<u32>(a + 0x6C));
        let hp = if stats > 0x10000 && super::crash::readable(stats + 0x3F) { rd::<i32>(stats + 0x3C) } else { 1 };
        *m = Some(crate::partners::Member { id: rd::<u16>(obj + 0x4C), down: rd::<u32>(a + 0x88) & 0x30 != 0 || hp <= 0 });
    }
    note_party(base, &out);
    out
}

/// Logs the field party whenever it changes (who is in each slot, down or not), with the save's
/// party list (`save + 0x48E`: Sora, the two in the field, a fourth) beside it.
unsafe fn note_party(base: usize, party: &[Option<crate::partners::Member>; 2]) {
    let mut key = 0u64;
    for (i, m) in party.iter().enumerate() {
        let v = m.map_or(0xFFFF, |m| (m.id as u64) << 1 | m.down as u64);
        key |= v << (i * 16);
    }
    let list = super::menu::save_block(base).map(|s| rd::<[u8; 4]>(s + 0x48E));
    key |= (list.map_or(0xFFFF_FFFF, u32::from_le_bytes) as u64) << 32;
    static LAST: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);
    if LAST.swap(key, Ordering::Relaxed) == key {
        return;
    }
    let who: Vec<String> = party
        .iter()
        .map(|m| match m {
            Some(m) => format!("{} (member {}{})", crate::partners::member_name(m.id), m.id, if m.down { ", knocked out" } else { "" }),
            None => "none".into(),
        })
        .collect();
    let open: Vec<&str> = Kind::ALL.iter().filter(|&&k| story_form(k).is_some_and(|f| crate::partners::check(f, "", party).is_ok())).map(|k| k.name()).collect();
    log(&format!(
        "Drive partners: {}; save party list {}; partners there for {}",
        who.join(", "),
        list.map_or("-".into(), |l| format!("{l:02x?}")),
        if open.is_empty() { "none".to_string() } else { open.join(", ") }
    ));
}

/// Chosen from the Drive list (`drive::chosen`), or Anti Form by the secret code. Returns why
/// not, if refused.
pub unsafe fn choose(_sora: usize, kind: Kind) -> Result<&'static str, String> {
    if busy() {
        return Err("a change is under way".into());
    }
    if in_anti() {
        // No Revert in Anti Form (the user: "I dont want sora to be able to revert out of it").
        return Err("Anti Form has no Revert".into());
    }
    if in_form() {
        REQUEST.store(2, Ordering::Relaxed);
        return Ok("Revert");
    }
    if !pose::kh2().is_some_and(|k| kind.data(k).is_some()) {
        return Err(format!("{}'s files didn't load", kind.name()));
    }
    own_needs(kind)?;
    if !unlocked(kind) {
        return Err(format!("{} isn't unlocked yet", kind.name()));
    }
    let cut = super::takeover::kh1_cut_window();
    if !super::takeover::kh2_has() && !cut {
        return Err("KH1 has Sora".into());
    }
    if !super::gauge::can_pay(kind) {
        return Err(if kind == Kind::Anti {
            format!("the Drive gauge ({}) is short: Anti Form empties it and wants {} bars", super::gauge::describe(), crate::drivegauge::ANTI_NEEDS)
        } else {
            format!("the Drive gauge ({}) is short: it costs {} bars", super::gauge::describe(), super::gauge::cost(kind))
        });
    }
    WANT.store(kind.code(), Ordering::Relaxed);
    REQUEST.store(1, Ordering::Relaxed);
    if cut && !super::takeover::kh2_has() {
        // (KH2 takes him out of KH1's move this frame and changes him.)
        super::takeover::DRIVE_CUT.store(true, Ordering::Relaxed);
    }
    Ok(kind.name())
}

/// The Driver's form state (`takeover`).
#[derive(Default)]
pub struct Drive {
    /// Changing: Sora's Drive motion playing, its frames.
    change: Option<u32>,
    /// In the form: Sora's own abilities (back on revert), the form's (its gauge is `gauge`'s).
    form: Option<InForm>,
    /// The form gauge ran out: a revert as soon as he can change.
    revert: bool,
    /// The form the change under way goes to.
    changing_to: Option<Kind>,
    /// A revert (the form gauge out, the Revert command, a scene) put off because a limit was under way
    /// (`limit_under_way`): logged once when put off, and again when taken.
    waited: Option<&'static str>,
}

/// A limit is under way in Limit Form: one of KH1's (KH1 has him, `limits::kh1_limit_holds`), or
/// KH2's own (`limits::running`, with `limits::KH2_LIMITS` on). KH2 never reverts a form in the
/// middle of a limit: its gauge out, the form ends when the limit has (the user: "Limit from has a
/// bug where sora will be reverted to base sora in the middle of a KH1 Limit"). Only a knock-out
/// or a new world reverts meanwhile.
pub fn limit_under_way() -> bool {
    super::limits::kh1_limit_holds() || in_limit() && super::limits::running()
}

/// Puts off a revert for `why` while a limit is under way (`limit_under_way`), logged once.
fn defer(d: &mut Drive, why: &'static str) {
    if d.waited.is_none() {
        log(&format!("Drive: the revert ({why}) waits: a limit is under way in {}", d.form.as_ref().map_or("the form", |f| f.kind.name())));
    }
    d.waited = Some(why);
}

struct InForm {
    kind: Kind,
    sora: Abilities,
    own: Abilities,
    /// KH1's world when the form started (`WORLD_RVA`): in another one, back to Sora.
    world: u32,
}

/// KH1's current world (u32; `takeover`'s room line reads it too).
const WORLD_RVA: super::addr::Rva = super::addr::Rva(0x2340E5C);

unsafe fn world_now() -> u32 {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    rd::<u32>(base + WORLD_RVA)
}

/// The form's controller data while in it, else Sora's.
pub fn character<'a>(d: &Drive, kh2: &'a pose::Kh2) -> &'a Character {
    match d.form.as_ref().and_then(|f| f.kind.data(kh2)) {
        Some(f) => &f.character,
        None => &kh2.character,
    }
}

pub fn battle<'a>(d: &Drive, kh2: &'a pose::Kh2) -> &'a Arc<Battle> {
    match d.form.as_ref().and_then(|f| f.kind.data(kh2)) {
        Some(f) => &f.battle,
        None => &kh2.battle,
    }
}

/// A growth ability's level (0 none, 1..3, 4 MAX) in a form: one above base Sora's, MAX at most
/// (the user: "have the grown abilities that are associated with each drive form ... always be
/// one level higher than the level unlocked by base sora": none -> LV1, LV1 -> LV2, MAX stays
/// MAX).
fn growth_up(level: u8) -> u8 {
    level.saturating_add(1).min(GROWTH_MAX)
}
const GROWTH_MAX: u8 = 4;

/// The form's level (s325, the user: "when in a form, have the drive capacity be raised to
/// whatever level the form is. For example, if sora has High Jump level 2, this means Valor Form
/// has high jump level 3 Which makes Valor Form level 3"): its own growth ability's level in
/// `own` (`form_abilities`: base Sora's learned level + 1, 4 = MAX), so 1..=4
/// (`drivegauge::form_level`). Its form gauge is `(level + 3) × 600` frames. Anti has no growth
/// ability: 1, which KH2's Anti gauge ignores (its own 6 bars).
pub fn form_level(kind: Kind, own: &Abilities) -> u8 {
    let growth = match kind {
        Kind::Valor => own.high_jump,
        Kind::Wisdom => own.quick_run,
        Kind::Limit => own.dodge_roll,
        Kind::Master => own.aerial_dodge,
        Kind::Final => own.glide,
        Kind::Anti => 1,
    };
    crate::drivegauge::form_level(growth)
}

/// Base Sora's growth levels for a form's own (`growth_up`): what he has learned, equipped or not
/// (the user: "wisdom form should have lv2 EVEN IF SORA DOESNT ACTIVELY HAVE QUICK RUN EQUIPPED";
/// `menu::LEARNED`), at least what he has on.
fn growth_of(sora: &Abilities) -> Abilities {
    let mut a = sora.clone();
    if let Some(l) = super::menu::LEARNED.lock().ok().and_then(|l| l.clone()) {
        a.high_jump = a.high_jump.max(l.high_jump);
        a.quick_run = a.quick_run.max(l.quick_run);
        a.aerial_dodge = a.aerial_dodge.max(l.aerial_dodge);
        a.glide = a.glide.max(l.glide);
        a.dodge_roll = a.dodge_roll.max(l.dodge_roll);
    }
    a
}

/// The form's abilities (KH2's, exe): Sora's support abilities carry over, his action and
/// growth ones don't; the form's own growth ability at its level; every combo row of the form's
/// table unlocked.
fn form_abilities(sora: &Abilities, form: &Character, kind: Kind) -> Abilities {
    let learned = growth_of(sora);
    let mut a = Abilities {
        // (Never in Anti Form: the user, "you shouldnt be able to guard in anti form".)
        // (Only with a guard window in its motion 42: Master's 42 is its repel's clip, D350, as its
        // 41 and 43, with none in its body's or either Keyblade's set, so KH2's Master has no
        // guard. Valor's and Limit's have one, ATKP 162; Wisdom and Final have no 42.)
        guard: kind != Kind::Anti
            && form.moveset.clips.get(&character_core::motion::GUARD).is_some_and(|c| c.windows.iter().any(|w| matches!(w.kind, character_core::WindowKind::Guard { .. }))),
        // (Sora's Aerial Recovery in the form, with its own recovery motion: the user, Wisdom and
        // Valor were "missing aerial recovery".)
        aerial_recovery: sora.aerial_recovery && form.moveset.clips.contains_key(&character_core::motion::AIR_RECOVERY),
        combo_plus: sora.combo_plus,
        air_combo_plus: sora.air_combo_plus,
        finishing_plus: sora.finishing_plus,
        negative_combo: sora.negative_combo,
        combo_master: sora.combo_master,
        one_swing_on_miss: sora.one_swing_on_miss,
        // Each form's growth ability (Valor High Jump, Wisdom Quick Run, Limit Dodge Roll, Master
        // Aerial Dodge, Final Glide) one level above base Sora's learned level, equipped or not
        // (`growth_of`), MAX at most (`growth_up`). Anti has none (00battle `fmlv`'s Anti
        // rows give no ability at any level), so none of Sora's growth abilities either.
        high_jump: if kind == Kind::Valor { growth_up(learned.high_jump) } else { 0 },
        quick_run: if kind == Kind::Wisdom { growth_up(learned.quick_run) } else { 0 },
        aerial_dodge: if kind == Kind::Master { growth_up(learned.aerial_dodge) } else { 0 },
        glide: if kind == Kind::Final { growth_up(learned.glide) } else { 0 },
        dodge_roll: if kind == Kind::Limit { growth_up(learned.dodge_roll) } else { 0 },
        ..sora.clone()
    };
    a.equipped = form.moveset.combo_rows.iter().map(|r| r.ability).filter(|&x| x != 0).collect();
    // (Sora's own moves added to the form, `pose::port_moves`: only while he has them equipped.)
    if kind == Kind::Valor {
        a.equipped.retain(|&x| x != UPPER_SLASH || sora.equipped.contains(&UPPER_SLASH));
    }
    a
}

/// Replaces the controller (keeps facing, lock-on, the glide button, and his movement: speed,
/// fall, in the air or not).
fn swap(ch: &mut Character, anim: &mut RigAnim, data: &Character, rig: &Arc<kh2_data::sora::Rig>, abilities: Abilities) {
    let (facing, lock, glide) = (ch.facing(), ch.lock(), ch.glide_button);
    let mut c = data.clone();
    c.carry_movement(ch);
    c.abilities = abilities;
    c.glide_button = glide;
    c.air_magic_falls = ch.air_magic_falls;
    c.air_magic_gravity = ch.air_magic_gravity;
    c.set_facing(facing);
    c.set_lock(lock);
    *ch = c;
    let field = anim.field;
    *anim = RigAnim::new(rig.clone());
    anim.queued_blend = 14.0;
    anim.field = field;
}

/// Before KH2's step: the menu's request, the change under way, the form's end.
pub unsafe fn before_step(d: &mut Drive, ch: &mut Character, anim: &mut RigAnim, kh2: &pose::Kh2, actor: usize) {
    // Knocked out in the form: Sora again at once (KH2: a KO reverts).
    let knocked = REQUEST.load(Ordering::Relaxed) == 3 && REQUEST.swap(0, Ordering::Relaxed) == 3;
    if (d.form.is_some() || d.change.is_some()) && (knocked || hp(actor) <= 0) {
        revert(d, ch, anim, kh2, "knocked out");
        return;
    }
    // Landed in another world in the form: Sora again at once, wherever he lands (the user: "when
    // leaving a world in a drive form, wherever world sora lands, he should be automatically
    // reverted"). Anti Form too.
    if let Some(w) = d.form.as_ref().map(|f| f.world) {
        let now = world_now();
        if now != w {
            log(&format!("Drive: a new world ({w} -> {now}): back to Sora"));
            revert(d, ch, anim, kh2, "a new world");
            return;
        }
    }
    // The secret code (`pad`): Anti Form (it asked for Final Form until s271), as if chosen from
    // the Drive list (KH2 must be able to change him now, as for the list: `accept_now`: free,
    // jumping, falling, landing or at a move's end; not in a form).
    if CODE_DONE.swap(false, Ordering::Relaxed) {
        if d.form.is_some() || d.change.is_some() || !ch.can_change_form() {
            log("Anti Form: not now (in a form, changing, or mid-move)");
        } else {
            match choose(actor, Kind::Anti) {
                Ok(_) => log("Anti Form: asked for by the secret code"),
                Err(e) => log(&format!("Anti Form: refused ({e})")),
            }
        }
    }
    match REQUEST.swap(0, Ordering::Relaxed) {
        1 if d.form.is_none() && d.change.is_none() => {
            let Some(kind) = Kind::from_code(WANT.load(Ordering::Relaxed)) else { return };
            // (Taken on the frame it was pressed in, `accept_now`; in the air too, held there.)
            if !super::gauge::can_pay(kind) {
                log(&format!("Drive: the Drive gauge ({}) is short of {} by now", super::gauge::describe(), kind.name()));
                return;
            }
            let air = ch.airborne();
            if !ch.play_held(DRIVE_MOTION, true) {
                log("Drive: Sora has no Drive motion (52)");
                return;
            }
            // KH2's cost in bars (`gauge::pay`: a part-filled bar survives unless the gauge was full).
            let (before, after) = super::gauge::pay(kind);
            heal_full(actor);
            // KH1's "Give me strength!" (the user: "add the battle quote from kh1 where he says
            // "give me strength" when your transforming into a drive").
            super::kh1voice::give_me_strength(actor);
            d.change = Some(0);
            d.changing_to = Some(kind);
            CHANGING.store(true, Ordering::Relaxed);
            // KH2's grey screen tint through the change (`kh2fx::drive_tint`).
            super::kh2fx::drive_tint(true);
            log(&format!(
                "Drive: {} for {} bars (Drive gauge {before} -> {after}), Sora's Drive motion{}",
                kind.name(),
                super::gauge::cost(kind),
                if air { ", held in the air" } else { "" }
            ));
        }
        // (Never in Anti Form, which has no Revert: `choose` refuses it already.)
        2 if d.form.as_ref().is_some_and(|f| f.kind != Kind::Anti) => {
            // (Not mid-limit: kept for its end.)
            if limit_under_way() {
                REQUEST.store(2, Ordering::Relaxed);
                defer(d, "the Revert command");
                return;
            }
            revert(d, ch, anim, kh2, "the Revert command");
            return;
        }
        _ => {}
    }
    if let Some(n) = d.change {
        let n = n + 1;
        d.change = Some(n);
        if !ch.scripted() || n >= DRIVE_FRAMES {
            d.change = None;
            arrive(d, ch, anim, kh2);
        }
        return;
    }
    // (A scene's revert put off during a limit, the scene gone by its end: the form stays.)
    if d.waited.is_some() && !d.revert && !limit_under_way() {
        log(&format!("Drive: the limit over, the revert put off during it ({}) no longer asks: the form stays", d.waited.unwrap_or("?")));
        d.waited = None;
    }
    if d.revert && ch.can_change_form() {
        // (Not mid-limit: KH2's own limits run under KH2's step, `limits`; KH1's have him away
        // from it, so this comes on the first frame after one anyway.)
        if limit_under_way() {
            defer(d, "the form gauge ran out");
            return;
        }
        d.revert = false;
        revert(d, ch, anim, kh2, "the form gauge ran out");
    }
}

unsafe fn arrive(d: &mut Drive, ch: &mut Character, anim: &mut RigAnim, kh2: &pose::Kh2) {
    CHANGING.store(false, Ordering::Relaxed);
    // (The tint fades out after the swap.)
    super::kh2fx::drive_tint(false);
    let kind = d.changing_to.take().unwrap_or(Kind::Wisdom);
    let Some(f) = kind.data(kh2) else { return };
    let sora = ch.abilities.clone();
    let own = form_abilities(&sora, &f.character, kind);
    swap(ch, anim, &f.character, &f.rig, own.clone());
    // Valor: Square in a combo goes into its finisher, row 12 (155) on the ground, 38 (183) in the
    // air (its kind-0 rows, reached only by id; the user: "valor forms ability to go into a
    // finisher by pressing square during a combo").
    if kind == Kind::Valor {
        ch.square_finishers = Some((12, 38));
    }
    // Anti: Square in a ground combo is its Side Spin, row 12 (motion 161; PTYA ability 0, so it
    // needs none), one swing of the combo as Upper Slash is (KH Wiki: "pressing Square in the
    // middle of a ground combo"; the user: "lets add anti-form's square move"). Square otherwise is
    // its Anti Glide, row 11 (motion 173), from the controller's guard-row path.
    if kind == Kind::Anti {
        ch.square_swing = Some(ANTI_SIDE_SPIN_ROW);
    }
    // (s220 showed Valor's stance in place of its landing, whose clip is Sora's own; the user,
    // s229: "valor form is currently missing kh2's landing animation": KH2's landing plays.)
    super::kh2fx::owner_gone();
    // (Still held in the air through it; then he falls.)
    if ch.clip_exists(ARRIVE_MOTION) {
        ch.play_held(ARRIVE_MOTION, true);
    }
    let lv = (growth_of(&sora), own.clone());
    let level = form_level(kind, &own);
    d.form = Some(InForm { kind, sora, own, world: world_now() });
    // KH2's form gauge, `(level + 3) × 600` frames (Anti 6 bars), from now (`gauge`).
    let frames = super::gauge::enter(kind, level);
    CURRENT.store(kind.code(), Ordering::Relaxed);
    IN_FORM.store(true, Ordering::Relaxed);
    if kind.two_blades() {
        super::valor::started();
    }
    let (growth, sora_lv, form_lv) = match kind {
        Kind::Valor => ("High Jump", lv.0.high_jump, lv.1.high_jump),
        Kind::Wisdom => ("Quick Run", lv.0.quick_run, lv.1.quick_run),
        Kind::Limit => ("Dodge Roll", lv.0.dodge_roll, lv.1.dodge_roll),
        Kind::Master => ("Aerial Dodge", lv.0.aerial_dodge, lv.1.aerial_dodge),
        Kind::Final => ("Glide", lv.0.glide, lv.1.glide),
        Kind::Anti => {
            log(&format!(
                "Anti Form: entered; its form gauge {frames:.0} frames ({:.0} bars, 1 a frame in battle, half out of it; Drive orbs drain it), back to Sora when it runs out; no growth ability, no guard ({}), no Revert, 1.5x damage taken",
                frames / 600.0,
                if lv.1.guard { "it has a guard motion!" } else { "no motion 42" }
            ));
            return;
        }
    };
    let learned = match sora_lv {
        0 => format!("no {growth} learned"),
        l if l >= GROWTH_MAX => format!("{growth} MAX learned"),
        l => format!("{growth} LV{l} learned"),
    };
    log(&format!(
        "{} level {level} ({learned}): form gauge {frames:.0} frames ({:.0} bars, 1 a frame in battle, half out of it; hits drain it, Drive orbs refill it); its {growth} level {form_lv} (one above Sora's, equipped or not; 4 = MAX)",
        kind.name(),
        frames / 600.0
    ));
}

/// Back to Sora: what played is cut, he is free with his movement (the user: on the ground
/// "return you to whatever animation is being called from the control stick and not halt his
/// movement"; in the air, falling).
unsafe fn revert(d: &mut Drive, ch: &mut Character, anim: &mut RigAnim, kh2: &pose::Kh2, why: &str) {
    CHANGING.store(false, Ordering::Relaxed);
    super::kh2fx::drive_tint(false);
    d.change = None;
    d.revert = false;
    d.changing_to = None;
    if let Some(waited) = d.waited.take() {
        log(&format!("Drive: the revert put off during a limit ({waited}) taken now"));
    }
    // The form gauge is gone; the Drive gauge is as the cost left it (KH2: no refill on the way out).
    super::gauge::revert();
    let Some(f) = d.form.take() else {
        // (Out of the change: its partners back.)
        super::vanish::frame();
        return;
    };
    IN_FORM.store(false, Ordering::Relaxed);
    CURRENT.store(0, Ordering::Relaxed);
    if f.kind.two_blades() {
        super::valor::ended();
    }
    swap(ch, anim, &kh2.character, &kh2.rig, f.sora);
    super::formmagic::end_all();
    super::limits::end_all();
    super::kh2fx::owner_gone();
    for el in REVERT_ELEMENTS {
        super::kh2fx::event(REVERT_FX, EventKind::Effect(el));
        super::kh2sound::event(REVERT_FX, EventKind::Effect(el));
    }
    log(&format!("Revert ({why}){}", if ch.airborne() { ", falling" } else { "" }));
    // Its partners back beside him now (`vanish`).
    super::vanish::frame();
}

/// KH1 has had Sora for a scene long enough in the form (`takeover::scene_frame`: a cutscene, a
/// scene out of a battle such as a boss falling, any long one): back to Sora there and then, as
/// in a new world (KH2 reverts a form when a battle ends in a scene; the user, s307: Wisdom's look
/// stayed on under KH1's own animations after Opposite Armor fell). His own look and stats come
/// back now, not on KH2's next step (which waits for KH1 to let him go). Anti Form too.
/// Never mid-limit (`limit_under_way`): put off until the limit's end, as KH2 does.
pub unsafe fn scene_revert(d: &mut Drive, ch: &mut Character, anim: &mut RigAnim, kh2: &pose::Kh2, actor: usize, why: &str) {
    if d.form.is_none() && d.change.is_none() {
        return;
    }
    if limit_under_way() {
        defer(d, "a scene");
        return;
    }
    revert(d, ch, anim, kh2, why);
    super::tint::refresh(actor, None);
    super::valorstats::step(actor);
}

/// After KH1's menu set Sora's abilities (`menu::Growth`): in the form, they are his own for
/// after it, and the form's stay.
pub fn abilities(d: &mut Drive, ch: &mut Character) {
    if let Some(f) = d.form.as_mut() {
        if ch.abilities != f.own {
            f.sora = ch.abilities.clone();
            // (Its own growth ability and rows stay.)
            let mut own = f.own.clone();
            own.combo_plus = f.sora.combo_plus;
            own.air_combo_plus = f.sora.air_combo_plus;
            own.finishing_plus = f.sora.finishing_plus;
            own.negative_combo = f.sora.negative_combo;
            own.combo_master = f.sora.combo_master;
            // (Its growth ability follows Sora's: one level above.)
            match f.kind {
                Kind::Valor => own.high_jump = growth_up(growth_of(&f.sora).high_jump),
                Kind::Wisdom => own.quick_run = growth_up(growth_of(&f.sora).quick_run),
                Kind::Limit => own.dodge_roll = growth_up(growth_of(&f.sora).dodge_roll),
                Kind::Master => own.aerial_dodge = growth_up(growth_of(&f.sora).aerial_dodge),
                Kind::Final => own.glide = growth_up(growth_of(&f.sora).glide),
                // (No growth ability of its own.)
                Kind::Anti => {}
            }
            f.own = own.clone();
            ch.abilities = own;
        }
    }
}

/// After KH2's step: the look (`tint`), the form's Keyblades, its end (`drain`).
///
/// (The form gauge itself drains every frame of Sora's, `gauge::frame`.)
pub unsafe fn after_step(d: &mut Drive, actor: usize) {
    super::tint::refresh(
        actor,
        d.form.as_ref().and_then(|f| match f.kind {
            Kind::Wisdom => Some(super::tint::Look::Wisdom),
            Kind::Valor => Some(super::tint::Look::Valor),
            Kind::Final => Some(super::tint::Look::Final),
            // (KH1 Sora's outfit recoloured, the user: "change sora's outfit in limit form to
            // have his jumpsuit be black, his belt yellow, and his shoes black".)
            Kind::Limit => Some(super::tint::Look::Limit),
            // (The user: "I want sora's jumpsuit to be yellow, and the inside of sora's jacket's
            // hood be yellow as well instead of blue. I want soras belt to be black".)
            Kind::Master => Some(super::tint::Look::Master),
            // (Shadow Sora's look, the user: "make sora look like shadow sora in this form".)
            Kind::Anti => Some(super::tint::Look::Anti),
        }),
    );
    // Valor's second Keyblade's Strength and MP (and back to Sora's own after).
    super::valorstats::step(actor);
    let Some(kind) = d.form.as_ref().map(|f| f.kind) else { return };
    // (Valor, Master, Final: its Keyblade checked every 10 frames, so a change made in the menu
    // reaches his hand, `valor::keyblade` / `valor::master_keyblade` / `valor::final_keyblade`.)
    if kind.two_blades() {
        static TICK: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        if TICK.fetch_add(1, Ordering::Relaxed) % 10 == 0 {
            match kind {
                Kind::Final => {
                    super::valor::final_keyblade();
                }
                Kind::Master => {
                    super::valor::master_keyblade();
                }
                _ => {
                    super::valor::keyblade();
                }
            }
        }
    }
    drain(d);
}

/// The form's end when its gauge has run out (`gauge::ran_out`; KH2's gauge empty, FUN_140404600):
/// `revert` set, taken by `before_step` once KH2 has him and he can change. Also while KH1 has him
/// for one of its limits in Limit Form (`takeover`, `limits::kh1_limit_holds`: the form stays on
/// all through the limit, its gauge draining as ever; out meanwhile, the revert waits for the
/// limit's end and KH2, `limit_under_way`).
pub unsafe fn drain(d: &mut Drive) {
    let Some(f) = d.form.as_ref() else { return };
    if super::gauge::ran_out() && !d.revert {
        d.revert = true;
        let mid = limit_under_way();
        log(&format!("{}: back to Sora{}", f.kind.name(), if mid { " once the limit is over (KH2 having him)" } else { "" }));
        if mid {
            defer(d, "the form gauge ran out");
        }
    }
}

/// Sora's Drive motion: its effects and sounds play whatever the ini's lists (`kh2fx`,
/// `kh2sound`).
pub fn own_motion(motion: u16) -> bool {
    matches!(motion, DRIVE_MOTION | REVERT_FX)
}

/// Whether this frame's event goes to the form instead of Sora's effects: Wisdom's shots,
/// Valor's elements (its own archive's, `P_EX100_BTLF.a.us`: `effect`).
pub fn takes(d: &Drive, e: &EventKind) -> bool {
    d.form.is_some() && matches!(e, EventKind::Effect(_))
}

/// A frame trigger 1 of the form's motion: Wisdom's shots, Valor's (Limit's, Master's, Final's,
/// Anti's) own elements.
pub fn effect(motion: u16, n: i16) {
    if in_wisdom() {
        super::formmagic::effect(n);
    } else if in_valor() || in_limit() || in_master() || in_final() || in_anti() {
        super::kh2fx::form_event(motion, n);
    }
}
