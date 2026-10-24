//! Wisdom Form (the user: "lets add wisdom form", from the second command page's Drive, costing
//! MP, Sora recoloured blue). Chosen from `drive`'s list, it plays out as KH2's Drive does
//! (`kh2-bevy`'s `drive.rs`, from the exe):
//! - Sora's Drive motion (52, invulnerable) with base Sora's moveset; when it ends, the swap:
//!   the form's moveset, motions and combo rows (`P_EX100_MAGF`, moveset 3), his facing and
//!   lock kept, and its arrival (53). Its abilities: Sora's support abilities, not his action
//!   or growth ones, and the form's own growth ability, Quick Run, at MAX (the user);
//! - the form runs on Sora's MP (the user): its yellow part drained bit by bit, a blue bar
//!   broken into it when it runs out (KH1's own payment; `after_step`); out of MP, or on the
//!   Revert command (the Drive row itself in the form), back to Sora, at once, in the air too:
//!   what was playing is cut and he is free again with his movement kept (walking, running or
//!   idle by the stick; in the air, falling). Knocked out: back at once;
//! - Drive and Revert are taken only on a frame Sora can change (`Character::can_change_form`:
//!   free, jumping, falling, landing, or at the end of a move or a cast), never buffered: a
//!   press before that does nothing (the menu refuses it, `combat::run_entry`);
//! - the change takes 45 frames (the user): Sora's Drive motion for `DRIVE_FRAMES`, then the
//!   swap and the form's short arrival; in the air he is held where he is through it.
//! - its shots and its spells: `formmagic`.
//! The cost is MP (`COST`, KH1's own payment), not a Drive gauge, which KH1 has none of.
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
//! code on the pad (`pad`) asks for it, as the Drive list would.

use super::{log, pose};
use character_core::{Abilities, Character, EventKind};
use kh2_formats::battle::Battle;
use kh2_rig::RigAnim;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
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
/// MP to change (KH1's payment takes hundredths of a bar from 100 up).
pub const COST_MP: i32 = 3;
const COST: i32 = COST_MP * 100;
/// The form's level (KH1 has no form experience): Lv 1, 40 s of battle.
const LEVEL: u32 = 1;
/// Frames of battle per unit of yellow MP the form uses up (half as fast out of battle). A blue
/// bar breaks into its number × 30 units (KH1's own conversion), so the top bar lasts that many
/// units.
/// (3: the user, "lets also have drive forms drain a little slower"; it was 2.)
const DRAIN_FRAMES: u32 = 3;
/// Upper Slash's ability (KH2's id): Sora's Square launch, given to Valor (`pose::port_moves`).
const UPPER_SLASH: u16 = 95;
/// KH2's magic sub offset in Wisdom Form.
pub const MAGIC_SUB: i32 = 10;

/// The forms (`drive::FORMS` in the same order).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Valor,
    Wisdom,
    Final,
}

impl Kind {
    /// The Drive list's (Final is secret, `pad`).
    pub const ALL: [Kind; 2] = [Kind::Valor, Kind::Wisdom];
    const EVERY: [Kind; 3] = [Kind::Valor, Kind::Wisdom, Kind::Final];
    pub fn name(self) -> &'static str {
        match self {
            Kind::Valor => "Valor Form",
            Kind::Wisdom => "Wisdom Form",
            Kind::Final => "Final Form",
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
            Kind::Final => kh2.final_form.as_ref(),
        }
    }
    /// With a second Keyblade (Valor's slot's).
    pub fn two_blades(self) -> bool {
        matches!(self, Kind::Valor | Kind::Final)
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

/// The form he is changing into: asked for (the command), or his Drive motion playing.
pub fn arriving() -> Option<Kind> {
    if in_form() || !(CHANGING.load(Ordering::Relaxed) || REQUEST.load(Ordering::Relaxed) == 1) {
        return None;
    }
    Kind::from_code(WANT.load(Ordering::Relaxed))
}

/// In Wisdom Form (its shots, its spells, its look).
pub fn in_wisdom() -> bool {
    current() == Some(Kind::Wisdom)
}

/// In Valor Form (no magic).
pub fn in_valor() -> bool {
    current() == Some(Kind::Valor)
}

/// In Final Form.
pub fn in_final() -> bool {
    current() == Some(Kind::Final)
}

/// In a form with a second Keyblade (Valor, Final).
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

/// The pad as pressed, every read (`probe::pad`): the secret code for Final Form (the user: a
/// button sequence, "in that exact sequence, put them into final form").
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
            log("Final Form: the secret code entered");
        }
    }
}

pub fn busy() -> bool {
    CHANGING.load(Ordering::Relaxed) || REQUEST.load(Ordering::Relaxed) != 0
}

/// KH2's magic sub offset now (`reflect`).
pub fn magic_sub() -> i32 {
    if in_wisdom() {
        MAGIC_SUB
    } else {
        0
    }
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn mp(sora: usize) -> i32 {
    let stats = pose::ptr(rd::<u32>(sora + 0x6C));
    if stats < 0x10000 {
        return 0;
    }
    rd::<i32>(stats + 0x44)
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

/// Whether form `kind`'s entry can be chosen now (`drive`'s menu): reverting in a form, else MP
/// for it and KH2 having Sora, its data loaded, and what the form itself needs (Valor's second
/// Keyblade, `valor`).
pub unsafe fn usable(sora: usize, kind: Kind) -> bool {
    if busy() {
        return false;
    }
    if in_form() {
        return true;
    }
    pose::kh2().is_some_and(|k| kind.data(k).is_some()) && (super::takeover::kh2_has() || super::takeover::kh1_cut_window() || super::shared::active()) && sora > 0x10000 && mp(sora) > COST_MP && own_needs(kind).is_ok()
}

/// The forms Sora could change into from now on (`kh2dpx`, to have their effects built before
/// he does): out of a form and not changing, each form whose data loaded, with MP for it and what
/// it needs itself (`usable` without the frame's own conditions: KH2 having him, KH1's cut window;
/// Valor's second Keyblade read without `valor::keyblade`'s save write and log).
pub unsafe fn possible(sora: usize) -> Vec<Kind> {
    if in_form() || busy() || sora < 0x10000 || mp(sora) <= COST_MP {
        return Vec::new();
    }
    let Some(k) = pose::kh2() else { return Vec::new() };
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    let needs = |kind: Kind| kind != Kind::Valor || super::equip::valor_keyblade(base).is_some();
    Kind::EVERY.into_iter().filter(|&kind| kind.data(k).is_some() && needs(kind)).collect()
}

/// What a form needs besides MP: Valor its second Keyblade (`valor::keyblade`).
unsafe fn own_needs(kind: Kind) -> Result<(), String> {
    match kind {
        Kind::Valor => super::valor::keyblade().map(|_| ()).ok_or_else(|| "no Keyblade for the second hand (one Sora owns that isn't equipped)".into()),
        Kind::Wisdom | Kind::Final => Ok(()),
    }
}

/// Chosen from the Drive list (`drive::chosen`). Returns why not, if refused.
pub unsafe fn choose(sora: usize, kind: Kind) -> Result<&'static str, String> {
    if busy() {
        return Err("a change is under way".into());
    }
    if in_form() {
        REQUEST.store(2, Ordering::Relaxed);
        return Ok("Revert");
    }
    if !pose::kh2().is_some_and(|k| kind.data(k).is_some()) {
        return Err(format!("{}'s files didn't load", kind.name()));
    }
    own_needs(kind)?;
    let cut = super::takeover::kh1_cut_window();
    if !super::takeover::kh2_has() && !cut {
        return Err("KH1 has Sora".into());
    }
    let have = mp(sora);
    if have <= COST_MP {
        return Err(format!("MP {have}: it costs {COST_MP} and needs some left to run on"));
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
    /// In the form: battle frames left, Sora's own abilities (back on revert), the form's.
    form: Option<InForm>,
    /// Out of MP: a revert as soon as he can change.
    revert: bool,
    /// The form the change under way goes to.
    changing_to: Option<Kind>,
}

struct InForm {
    kind: Kind,
    /// Half frames toward the next MP drained (2 a frame in battle, 1 out of it).
    left: u32,
    sora: Abilities,
    own: Abilities,
    logged_s: u32,
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

/// The form's abilities (KH2's, exe): Sora's support abilities carry over, his action and
/// growth ones don't; the form's own growth ability at its level; every combo row of the form's
/// table unlocked.
fn form_abilities(sora: &Abilities, form: &Character, kind: Kind) -> Abilities {
    let mut a = Abilities {
        guard: form.moveset.clips.contains_key(&character_core::motion::GUARD),
        // (Sora's Aerial Recovery in the form, with its own recovery motion: the user, Wisdom and
        // Valor were "missing aerial recovery".)
        aerial_recovery: sora.aerial_recovery && form.moveset.clips.contains_key(&character_core::motion::AIR_RECOVERY),
        combo_plus: sora.combo_plus,
        air_combo_plus: sora.air_combo_plus,
        finishing_plus: sora.finishing_plus,
        negative_combo: sora.negative_combo,
        combo_master: sora.combo_master,
        one_swing_on_miss: sora.one_swing_on_miss,
        // (Each form's growth ability at MAX: the user, Wisdom with Quick Run MAX; Valor's High
        // Jump the same.)
        high_jump: if kind == Kind::Valor { 4 } else { 0 },
        quick_run: if kind == Kind::Wisdom { 4 } else { 0 },
        aerial_dodge: 0,
        // (Final's growth ability: Glide.)
        glide: if kind == Kind::Final { 4 } else { 0 },
        dodge_roll: 0,
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
    // The secret code (`pad`): Final Form, as if chosen from the Drive list (KH2 must be able to
    // change him now, as for the list: `accept_now`).
    if CODE_DONE.swap(false, Ordering::Relaxed) {
        if d.form.is_some() || d.change.is_some() || !ch.can_change_form() {
            log("Final Form: not now (in a form, changing, or mid-move)");
        } else {
            match choose(actor, Kind::Final) {
                Ok(_) => log("Final Form: asked for by the secret code"),
                Err(e) => log(&format!("Final Form: refused ({e})")),
            }
        }
    }
    match REQUEST.swap(0, Ordering::Relaxed) {
        1 if d.form.is_none() && d.change.is_none() => {
            let Some(kind) = Kind::from_code(WANT.load(Ordering::Relaxed)) else { return };
            // (Taken on the frame it was pressed in, `accept_now`; in the air too, held there.)
            if mp(actor) <= COST_MP {
                log(&format!("Drive: MP {} short of {COST_MP} by now", mp(actor)));
                return;
            }
            let air = ch.airborne();
            if !ch.play_held(DRIVE_MOTION, true) {
                log("Drive: Sora has no Drive motion (52)");
                return;
            }
            let before = mp(actor);
            super::reflect::pay_mp(actor, COST);
            heal_full(actor);
            // KH1's "Give me strength!" (the user: "add the battle quote from kh1 where he says
            // "give me strength" when your transforming into a drive").
            super::kh1voice::give_me_strength(actor);
            d.change = Some(0);
            d.changing_to = Some(kind);
            CHANGING.store(true, Ordering::Relaxed);
            log(&format!("Drive: {} (MP {before} -> {}), Sora's Drive motion{}", kind.name(), mp(actor), if air { ", held in the air" } else { "" }));
        }
        2 if d.form.is_some() => {
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
    if d.revert && ch.can_change_form() {
        d.revert = false;
        revert(d, ch, anim, kh2, "out of MP");
    }
}

unsafe fn arrive(d: &mut Drive, ch: &mut Character, anim: &mut RigAnim, kh2: &pose::Kh2) {
    CHANGING.store(false, Ordering::Relaxed);
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
    // (s220 showed Valor's stance in place of its landing, whose clip is Sora's own; the user,
    // s229: "valor form is currently missing kh2's landing animation": KH2's landing plays.)
    super::kh2fx::owner_gone();
    // (Still held in the air through it; then he falls.)
    if ch.clip_exists(ARRIVE_MOTION) {
        ch.play_held(ARRIVE_MOTION, true);
    }
    d.form = Some(InForm { kind, left: 0, sora, own, logged_s: 0 });
    CURRENT.store(kind.code(), Ordering::Relaxed);
    IN_FORM.store(true, Ordering::Relaxed);
    if kind.two_blades() {
        super::valor::started();
    }
    let growth = match kind {
        Kind::Valor => "High Jump",
        Kind::Wisdom => "Quick Run",
        Kind::Final => "Glide",
    };
    log(&format!("{} (Lv{LEVEL}): yellow MP drained 1 every {DRAIN_FRAMES} frames of battle (half as fast out of it), {growth} MAX", kind.name()));
}

/// Back to Sora: what played is cut, he is free with his movement (the user: on the ground
/// "return you to whatever animation is being called from the control stick and not halt his
/// movement"; in the air, falling).
unsafe fn revert(d: &mut Drive, ch: &mut Character, anim: &mut RigAnim, kh2: &pose::Kh2, why: &str) {
    CHANGING.store(false, Ordering::Relaxed);
    d.change = None;
    d.revert = false;
    d.changing_to = None;
    let Some(f) = d.form.take() else { return };
    IN_FORM.store(false, Ordering::Relaxed);
    CURRENT.store(0, Ordering::Relaxed);
    if f.kind.two_blades() {
        super::valor::ended();
    }
    swap(ch, anim, &kh2.character, &kh2.rig, f.sora);
    super::formmagic::end_all();
    super::kh2fx::owner_gone();
    for el in REVERT_ELEMENTS {
        super::kh2fx::event(REVERT_FX, EventKind::Effect(el));
        super::kh2sound::event(REVERT_FX, EventKind::Effect(el));
    }
    log(&format!("Revert ({why}){}", if ch.airborne() { ", falling" } else { "" }));
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
            f.own = own.clone();
            ch.abilities = own;
        }
    }
}

/// After KH2's step: the look (`tint`), the MP the form uses up, its end.
///
/// The form's gauge is Sora's MP (the user: "drive forms consume mp over time"), drained the
/// way KH1 takes a cost under one bar (the user: "drain yellow mp incrementally, when the value
/// that stores yellow mp reaches zero then we take away one bar of blue mp"): KH1's payment
/// (`0x1402A39D0`) with a cost below 100 takes it from the yellow part (stats `+0xBE`) and, when
/// that runs out, breaks the top blue bar into yellow. One unit every `DRAIN_FRAMES` in battle,
/// half as often out of it (KH2's gauge drains so); with none left, back to Sora (on the
/// ground, `before_step`).
pub unsafe fn after_step(d: &mut Drive, actor: usize, battle: bool) {
    super::tint::refresh(
        actor,
        d.form.as_ref().and_then(|f| match f.kind {
            Kind::Wisdom => Some(super::tint::Look::Wisdom),
            Kind::Valor => Some(super::tint::Look::Valor),
            Kind::Final => None,
        }),
    );
    // Valor's second Keyblade's Strength and MP (and back to Sora's own after).
    super::valorstats::step(actor);
    let Some(f) = d.form.as_mut() else { return };
    // (Valor: its Keyblade checked every 10 frames, so a change made in the menu reaches his
    // hand, `valor::keyblade`.)
    if f.kind == Kind::Valor {
        static TICK: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        if TICK.fetch_add(1, Ordering::Relaxed) % 10 == 0 {
            super::valor::keyblade();
        }
    }
    f.left += if battle { 2 } else { 1 };
    if f.left >= DRAIN_FRAMES * 2 {
        f.left = 0;
        if super::reflect::has_mp(actor, 1) {
            let before = (mp(actor), yellow(actor));
            super::reflect::pay_mp(actor, 1);
            if mp(actor) != before.0 && f.logged_s < 40 {
                f.logged_s += 1;
                log(&format!("{}: a blue bar broken into yellow: MP {} -> {} (yellow {} -> {})", f.kind.name(), before.0, mp(actor), before.1, yellow(actor)));
            }
        }
    }
    if !super::reflect::has_mp(actor, 1) && !d.revert {
        d.revert = true;
        log(&format!("{}: out of MP", f.kind.name()));
    }
}

/// The yellow part of Sora's MP (stats `+0xBE`).
unsafe fn yellow(sora: usize) -> u16 {
    let stats = pose::ptr(rd::<u32>(sora + 0x6C));
    if stats < 0x10000 {
        return 0;
    }
    rd::<u16>(stats + 0xBE)
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

/// A frame trigger 1 of the form's motion: Wisdom's shots, Valor's own elements.
pub fn effect(motion: u16, n: i16) {
    if in_wisdom() {
        super::formmagic::effect(n);
    } else if in_valor() || in_final() {
        super::kh2fx::form_event(motion, n);
    }
}
