//! Wisdom Form (the user: "lets add wisdom form", from the second command page's Drive, costing
//! MP, Sora recoloured blue). Chosen from `drive`'s list, it plays out as KH2's Drive does
//! (`kh2-bevy`'s `drive.rs`, from the exe):
//! - Sora's Drive motion (52, invulnerable) with base Sora's moveset; when it ends, the swap:
//!   the form's moveset, motions and combo rows (`P_EX100_MAGF`, moveset 3), his facing and
//!   lock kept, and its arrival (53). Its abilities: Sora's support abilities, not his action
//!   or growth ones, and the form's own growth ability, Quick Run, at the form's level (KH2's
//!   form levels kept in KH1's save, `formexp`);
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
//! (at Valor's form level, `formexp`), no magic (KH2's Valor can't cast), and its second Keyblade
//! (`valor`): entered only with an eligible one (the user: "If no keyblade is eligible to be in
//! that slot, dont let players go into valor form").
//!
//! Final Form (stage 231, the user: "lets work on implementing final form ... make it a secret"):
//! the same change and revert, `P_EX100_ULTF` (moveset 5), its growth ability Glide at its form level, its
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

pub use super::formmagic::{element_sounds, package_spell, shots_step, LAST_PACKAGE};

pub const DRIVE_MOTION: u16 = 52;
pub const ARRIVE_MOTION: u16 = 53;
/// The form's short stance after its arrival on the ground (motion 6 of its moveset; `kh1rows`
/// "Form stance", `native_arrive`).
pub const STANCE_MOTION: u16 = super::kh1rows::STANCE_MOTION;
/// KH2's blend into the stance (Steam KH2 `0x140405630`: `0x1403C8310 (motion, 6, 8.0, ...)`,
/// then the idle queued with 0; Checked (decomp)).
const STANCE_BLEND: f32 = 8.0;
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
    /// Its KH2 model (`kh2_data::sora::FORMS`: its moveset's files), also the slot cache's key for
    /// its baked moves (`kh2slots::slot_key`).
    pub fn model(self) -> &'static str {
        match self {
            Kind::Valor => "P_EX100_BTLF",
            Kind::Wisdom => "P_EX100_MAGF",
            Kind::Final => "P_EX100_ULTF",
        }
    }
    /// Runs on KH1's own states where it can (`native_ok`): its moves baked into KH1 slots
    /// (`kh2slots::FORM_SLOTS`), its numbers on KH1's states. The others keep KH2's controller.
    pub fn native_capable(self) -> bool {
        matches!(self, Kind::Valor | Kind::Wisdom)
    }
    /// Its growth ability (KH2's form growth: Valor High Jump, Wisdom Quick Run, Final Glide).
    pub fn growth(self) -> &'static str {
        match self {
            Kind::Valor => "High Jump",
            Kind::Wisdom => "Quick Run",
            Kind::Final => "Glide",
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
            log!(Forms, "Final Form: the secret code entered");
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
/// Sora's line as a Drive starts (t745; the user: "hes also missing his drive transformation lines"):
/// with Battle Quotes KH1, KH1's "Give me strength!" (clip 37 at priority 3, `kh1voice`); with
/// Battle Quotes KH2, none of ours, so KH2's own line is heard: the Drive motion 52's voice 8 at
/// frame 0 (VTBL: clips 25, 32, 33 at priority 2, Checked (files)), baked as a `KH2_VOICE` key and
/// played on KH1's motion pass (seen firing in the user's t743 log). KH1's line, started a frame
/// before at a higher priority, kept KH2's out by KH1's voice rule (**Assumption**: heard as
/// missing, not logged).
unsafe fn drive_voice(actor: usize) {
    if !super::kh2sound::kh2_voice() {
        super::kh1voice::give_me_strength(actor);
    }
}

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
        log!(Forms, "Drive: HP {now} -> {} of {max}", rd::<i32>(stats + 0x3C));
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
    if busy() {
        return false;
    }
    // (Valor on KH1's own states, `native_now`: KH1's states have him, KH1's rule for its revert,
    // `native_can_change`. Its change: `native_accept`.)
    if native_now() {
        return unsafe { native_can_change(super::takeover::sora_actor()) };
    }
    // (Out of a form the change is `native_accept`'s: KH1's states have him, t875.)
    false
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
    pose::kh2().is_some_and(|k| kind.data(k).is_some()) && super::shared::active() && sora > 0x10000 && mp(sora) > COST_MP && own_needs(kind).is_ok()
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
    // (Valor on KH1's own states, `native_frame`: KH1 has him, KH1's rule for the change.)
    let native = native_ok(kind) && super::shared::active();
    // (t873) On Steam only KH1's own states run a form: one that can't (Final, or Valor / Wisdom
    // whose slots didn't build) is refused, not handed to the old controller.
    if !native {
        super::shared::note_cut_off(&format!("the Drive to {}", kind.name()));
        return Err(format!("{} can't run on KH1's own states here (not ported yet, or its slots didn't build)", kind.name()));
    }
    if native && !native_can_change(sora) {
        return Err("KH1's move has him and he can't change now (no queue)".into());
    }
    let have = mp(sora);
    if have <= COST_MP {
        return Err(format!("MP {have}: it costs {COST_MP} and needs some left to run on"));
    }
    WANT.store(kind.code(), Ordering::Relaxed);
    NATIVE.store(native, Ordering::Relaxed);
    REQUEST.store(1, Ordering::Relaxed);
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
        // (Each form's growth ability at the form's own level, KH2's fmlv: `form_growth`.)
        high_jump: 0,
        quick_run: 0,
        aerial_dodge: 0,
        glide: 0,
        dodge_roll: 0,
        ..sora.clone()
    };
    form_growth(&mut a, kind);
    a.equipped = form.moveset.combo_rows.iter().map(|r| r.ability).filter(|&x| x != 0).collect();
    // (Sora's own moves added to the form, `pose::port_moves`: only while he has them equipped.)
    if kind == Kind::Valor {
        a.equipped.retain(|&x| x != UPPER_SLASH || sora.equipped.contains(&UPPER_SLASH));
    }
    a
}

/// The form's growth ability at its form level (KH2: fmlv's growth level for the form's level,
/// Lv1/3/5/7 giving LV1/2/3/MAX; Valor High Jump, Wisdom Quick Run, Final Glide;
/// `formexp::growth`). It replaces t688's rules (Valor and Final at MAX, Wisdom one above Sora's
/// own): the tester's answer 6 (b), what-is-still-missing after m14.
fn form_growth(a: &mut Abilities, kind: Kind) {
    let g = super::formexp::growth(kind);
    match kind {
        Kind::Valor => a.high_jump = g,
        Kind::Wisdom => a.quick_run = g,
        Kind::Final => a.glide = g,
    }
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
    log!(Forms, "Revert ({why}){}", if ch.airborne() { ", falling" } else { "" });
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
            form_growth(&mut own, f.kind);
            f.own = own.clone();
            ch.abilities = own;
        }
        // (A form level gained in the form: its growth ability at once, `formexp`; looked at only
        // when a level was gained.)
        static SEEN: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let gen = super::formexp::generation();
        if SEEN.swap(gen, Ordering::Relaxed) != gen {
            let mut own = f.own.clone();
            form_growth(&mut own, f.kind);
            if own != f.own {
                f.own = own.clone();
                ch.abilities = own;
            }
        }
    }
}


/// `after_step`'s work (KH2's controller's step, or `native_frame` for a form on KH1's states).
unsafe fn drain_step(d: &mut Drive, actor: usize, battle: bool) {
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
                log!(Forms, "{}: a blue bar broken into yellow: MP {} -> {} (yellow {} -> {})", f.kind.name(), before.0, mp(actor), before.1, yellow(actor));
            }
        }
    }
    if !super::reflect::has_mp(actor, 1) && !d.revert {
        d.revert = true;
        log!(Forms, "{}: out of MP", f.kind.name());
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

// -------------------------------------------------------------------------------------------
// Valor Form on KH1's own states (the tester, what-is-still-missing after m11: "I want you to
// transfer drive forms as well", "go ahead"; `docs/kh1-drive-forms-native.md` §7).
//
// The form's whole life runs on KH1's own states, as base Sora's moves do: no KH2 controller.
// - The change: KH1's request of ours for Sora's Drive motion 52 (`kh1rows` "Drive", a baked slot,
//   `kh2slots`) with KH1's action flag, KH1's idle (or fall) queued after; KH2's clip's no-gravity
//   and invulnerable ranges (all through) hold him in the air and keep hits off him through
//   `kh2slots::no_gravity` / `invulnerable` as for any KH2 move on KH1's states; its body hit (ATKP
//   125, 18..28) is a KH1 attack object of ours (`bodyhits`). After `DRIVE_FRAMES` 60 Hz frames:
// - the swap: Sora's actor keeps its object; his requests point at Valor's own slots instead of
//   base Sora's (`kh2slots::redirect`: idle, walk, run, jump, fall, landing, reactions, recoils,
//   item use, the ground and air combo on KH1's requests and our own combo rows), KH2's combo pick
//   in KH1's picker reads Valor's PTYA (KH2's controller's data, only read, `takeover::kh2_rows`), and
//   KH2's physics numbers on KH1's states are Valor's (`takeover::refresh_kh2_numbers`: run 12,
//   High Jump at its form level, `formexp`); then Valor's arrival 53 on its request of ours.
// - Revert: everything back to base Sora's, what was playing cut into KH1's idle (or fall).
// The MP cost, drain, out-of-MP revert, KO revert, look and the second Keyblade's stats are the
// same as on KH2's controller (`drain_step`). Wisdom and Final keep KH2's controller for now.

/// The form arriving or playing runs on KH1's own states (Valor, `native_ok`).
static NATIVE: AtomicBool = AtomicBool::new(false);
/// The form's arrival (motion 53) was asked in the air on KH1's own states (`native_arrive`): KH1's
/// air step holds him through it (`arrival_held`).
static ARRIVE_AIR: AtomicBool = AtomicBool::new(false);

/// KH2's arrival in the air holds him where he is (Steam KH2: the new form's first think
/// `0x140405630`, with `+0xE10` 2 (the change made in the air: the form swap `0x14040D000` sets 1 or 2
/// by its flag bit 0), plays motion 53 in ACTION_FLY (`0x140421CE0 (actor, 0 vector,
/// 1.0)`: velocity 0, keep 1.0; ACTION_FLY's step `0x140421C80` has no vertical step at all) and
/// queues the fall motion 4 with an 8-frame blend whose start calls the action's slot +0x20
/// (`0x140409B00 (actor, 0)`: `0x1403CBC30`'s hop of 0, a fall from rest, ACTION_JUMP);
/// Checked (decomp)). Arrival 53 has no no-gravity range of its own (`P_EX100_BTLF.mset`, Checked
/// (files)), so KH1's air step fell through its frames before. Held now while Valor's arrival
/// plays on KH1's air state, then the fall starts from rest (`shared::physics::air_step`'s end of
/// a hold, KH2's `0x1403B4780`, at 0 down).
pub unsafe fn arrival_held(actor: usize) -> bool {
    if !ARRIVE_AIR.load(Ordering::Relaxed) {
        return false;
    }
    if !native_now() || super::kh2slots::playing_motion(actor) != Some(ARRIVE_MOTION) {
        // (Asked, not yet applied: KH1 applies the request on its next motion pass; held then too.)
        if !(native_now() && rd::<u32>(actor + 0x164) & 0xFFFF == super::kh1rows::by_motion(ARRIVE_MOTION).map_or(u32::MAX, |n| super::kh1rows::request(n) & 0xFFFF)) {
            ARRIVE_AIR.store(false, Ordering::Relaxed);
            return false;
        }
    }
    true
}

/// KH1's motion request (fresh queue) and queue, and its movement state setter (Epic RVAs,
/// mapped for Steam: `0x14029F980`, `0x14029FA30`, `0x1402B1960`).
const SET_MOTION_RVA: super::addr::Rva = super::addr::Rva(0x29D7F0);
const QUEUE_MOTION_RVA: super::addr::Rva = super::addr::Rva(0x29D8A0);
const SET_STATE_RVA: super::addr::Rva = super::addr::Rva(0x2AF7D0);
/// KH1's can-act test (Steam `0x1402956E0` mapped; the action flag, a reaction, a script hold).
const CAN_ACT_RVA: super::addr::Rva = super::addr::Rva(0x2956E0);
/// KH1's motion request's action flag (bit 24: KH1's `+0` bit 4, he can't act until it ends) and
/// a queued request's "action over" bit (`0x40000000`).
const ACTION: u32 = 0x0100_0000;
const QUEUED: u32 = 0x4000_0000;

/// Valor Form can run on KH1's own states in this game (Steam, shared states, the Drive on, our
/// slots built): its change, its moves and its revert native.
pub fn native_enabled() -> bool {
    let s = super::settings::get();
    s.drive && s.kh1_slots.is_some() && super::addr::Build::current() == super::addr::Build::Steam
}

/// Form `kind` runs on KH1's own states (`Kind::native_capable`, its slots built:
/// `kh2slots::form_built`).
pub fn native_ok(kind: Kind) -> bool {
    kind.native_capable() && native_enabled() && super::kh2slots::form_built(kind)
}

/// In a form on KH1's own states now.
pub fn native_now() -> bool {
    NATIVE.load(Ordering::Relaxed) && in_form()
}

/// The form he is in on KH1's own states now (its moveset's slots on his requests, `kh2slots`).
pub fn native_form() -> Option<Kind> {
    current().filter(|_| native_now())
}

/// Something of a form on KH1's own states to run each frame (`takeover::form_native_frame`):
/// its change asked for or under way, or the form itself.
pub fn native_running() -> bool {
    NATIVE.load(Ordering::Relaxed) && (in_form() || CHANGING.load(Ordering::Relaxed) || REQUEST.load(Ordering::Relaxed) != 0)
}


/// The Drive list's form `k` (or Revert in a form) goes to KH1's own states (`combat::command`):
/// Revert in a native form, else Valor where it can run natively.
pub fn native_choice(k: usize) -> bool {
    if in_form() {
        return native_now();
    }
    Kind::ALL.get(k).is_some_and(|&kind| native_ok(kind)) && super::shared::active()
}

/// The Drive list's form `k` is taken now (`combat::run_entry`, as `accept_now` for the others):
/// out of a form, the form run on KH1's own states (`native_choice`), KH1 letting him change.
pub fn native_accept(k: usize) -> bool {
    !busy() && !in_form() && native_choice(k) && unsafe { native_can_change(super::takeover::sora_actor()) }
}

/// KH1 lets him change now (KH2's `Character::can_change_form`: free, jumping, falling, landing,
/// or in a move's combo or finisher window): KH1's can-act test, or KH1's combo window (`+0x1CC`
/// bit 22, which our slots carry from KH2's combo and Finishing Plus windows, `kh2slots`).
/// **Assumption**: KH1's can-act covers KH2's free states (idle, walk, run, jump, fall, a landing
/// past its lock) as KH2's rule does.
unsafe fn native_can_change(actor: usize) -> bool {
    if actor < 0x10000 {
        return false;
    }
    let base = super::takeover::game_base();
    let can: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(base + CAN_ACT_RVA);
    can(actor) != 0 || rd::<u32>(actor + 0x1CC) & super::combomods::KH1_COMBO_WINDOW != 0
}

/// One of our own requests (`kh1rows`) for `motion` on Sora: asked with KH1's action flag, KH1's
/// idle (on the ground) or fall (in the air) queued after it, as KH1's own commands queue them.
unsafe fn ask_own(actor: usize, motion: u16, air: bool) -> bool {
    ask_own_then(actor, motion, None, air)
}

/// Our request for `motion` when it has a slot bound on `actor` now (`kh1rows`, `kh2slots`).
unsafe fn own_request(actor: usize, motion: u16) -> Option<u32> {
    let request = super::kh1rows::request(super::kh1rows::by_motion(motion)?);
    (super::kh1rows::installed() && super::kh2slots::has(request as usize, motion) && super::kh2slots::request_bound(actor, request as usize)).then_some(request)
}

/// `ask_own`, with `then` (a motion of ours with no action flag: the form's stance) queued between
/// it and KH1's idle on the ground, as KH2's arrival queues motion 6 then the idle
/// (`0x140405630`, Checked (decomp)). `then` is left out when it has no slot bound now. Returns
/// whether `motion` was asked (and whether `then` was, in the log).
unsafe fn ask_own_then(actor: usize, motion: u16, then: Option<u16>, air: bool) -> bool {
    let Some(request) = own_request(actor, motion) else { return false };
    let base = super::takeover::game_base();
    let set: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + SET_MOTION_RVA);
    let queue: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + QUEUE_MOTION_RVA);
    set(actor, request | ACTION, 0.0);
    match then.filter(|_| !air).and_then(|m| own_request(actor, m)) {
        Some(next) => {
            // (The action flag cleared as `motion` ends (bit 30, `0x14029F6E0`): the stance is free
            // all through, KH2's range id 2, so the stick, a jump or an Attack leave it, as in
            // KH2; its slot's "may be left" runs from frame 0, `kh2slots::kh1_tracks`. KH1's idle
            // after it with no blend, as KH2's.)
            queue(actor, QUEUED | next, STANCE_BLEND);
            queue(actor, 0, 0.0);
        }
        None => {
            queue(actor, QUEUED | if air { 6 } else { 0 }, 0.0);
        }
    }
    true
}

/// The form's frame on KH1's own states (`takeover::form_native_frame`, each physics frame of
/// Sora's while KH1's states have him: `steps` 60 Hz frames of it, `battle` KH1's battle flag).
pub unsafe fn native_frame(d: &mut Drive, ch: &mut Character, kh2: &pose::Kh2, actor: usize, battle: bool, steps: u32) {
    if !NATIVE.load(Ordering::Relaxed) {
        return;
    }
    let knocked = REQUEST.load(Ordering::Relaxed) == 3 && REQUEST.swap(0, Ordering::Relaxed) == 3;
    if (d.form.is_some() || d.change.is_some()) && (knocked || hp(actor) <= 0) {
        native_revert(d, ch, kh2, actor, "knocked out");
        return;
    }
    match REQUEST.swap(0, Ordering::Relaxed) {
        1 if d.form.is_none() && d.change.is_none() => native_start(d, actor),
        2 if d.form.is_some() => {
            native_revert(d, ch, kh2, actor, "the Revert command");
            return;
        }
        _ => {}
    }
    if let Some(n) = d.change {
        let n = n + steps.max(1);
        d.change = Some(n);
        if n >= DRIVE_FRAMES {
            d.change = None;
            native_arrive(d, ch, kh2, actor);
        }
        return;
    }
    if d.form.is_none() {
        // (Nothing under way: the change was refused.)
        NATIVE.store(false, Ordering::Relaxed);
        CHANGING.store(false, Ordering::Relaxed);
        return;
    }
    // The MP the form uses up, its look, the second Keyblade's stats: one KH2 step each 60 Hz
    // frame, as KH2's controller ran them.
    for _ in 0..steps.max(1) {
        drain_step(d, actor, battle);
    }
    if d.revert && native_can_change(actor) {
        d.revert = false;
        native_revert(d, ch, kh2, actor, "out of MP");
    }
}

/// The change asked for: Sora's Drive motion on KH1's state he is in (the ground, or held in the
/// air by its no-gravity range), MP paid, HP refilled, KH1's "Give me strength!".
unsafe fn native_start(d: &mut Drive, actor: usize) {
    let Some(kind) = Kind::from_code(WANT.load(Ordering::Relaxed)) else { return };
    let state = rd::<i32>(actor + 0x70);
    if !matches!(state, 0..=2) || mp(actor) <= COST_MP {
        log!(Forms, "Drive ({}, KH1's states): not now (KH1 state {state}, MP {})", kind.name(), mp(actor));
        return;
    }
    let base = super::takeover::game_base();
    let air = state != 0;
    if state == 1 {
        // (KH1's takeoff: into its air state, where the hold runs.)
        let set: unsafe extern "system" fn(usize, i32) -> u32 = std::mem::transmute(base + SET_STATE_RVA);
        set(actor, 2);
    }
    // KH2 holds him where he is (`Character::play_held`): no speed of his own through it.
    wr(actor + 0x2C, 0.0f32);
    if air {
        wr(actor + 0x3C0, [0.0f32; 3]);
    }
    if !ask_own(actor, DRIVE_MOTION, air) {
        log!(Forms, "Drive ({}, KH1's states): Sora's Drive motion has no slot of ours", kind.name());
        return;
    }
    let before = mp(actor);
    super::reflect::pay_mp(actor, COST);
    heal_full(actor);
    drive_voice(actor);
    d.change = Some(0);
    d.changing_to = Some(kind);
    CHANGING.store(true, Ordering::Relaxed);
    log!(Forms, "Drive: {} on KH1's own states (MP {before} -> {}), Sora's Drive motion on KH1's request of ours{}", kind.name(), mp(actor), if air { ", held in the air" } else { "" });
}

/// The swap: Valor's slots on his requests, its data for KH2's pick, its numbers, its arrival.
unsafe fn native_arrive(d: &mut Drive, ch: &mut Character, kh2: &pose::Kh2, actor: usize) {
    CHANGING.store(false, Ordering::Relaxed);
    let kind = d.changing_to.take().unwrap_or(Kind::Valor);
    let Some(f) = kind.data(kh2) else { return };
    let sora = ch.abilities.clone();
    let own = form_abilities(&sora, &f.character, kind);
    let growth = growth_name(&own, kind);
    // (Valor's Square moves on KH1's states are KH2's own Square rows as the exe reaches them,
    // `combomods::valor_square`: the controller's `square_finishers`, which took 12 / 38 for
    // finishers, isn't used here.)
    native_data(ch, &f.character, own.clone());
    super::kh2fx::owner_gone();
    d.form = Some(InForm { kind, left: 0, sora, own, logged_s: 0 });
    CURRENT.store(kind.code(), Ordering::Relaxed);
    IN_FORM.store(true, Ordering::Relaxed);
    if kind.two_blades() {
        super::valor::started();
    }
    super::formexp::entered(kind);
    let rec = super::formexp::record(kind);
    // (Valor's slots on his requests from now: before its arrival is asked.)
    super::kh2slots::refresh();
    let air = matches!(rd::<i32>(actor + 0x70), 1 | 2);
    // (On the ground the form's stance follows its arrival, then the idle: KH2's `0x140405630`
    // with `+0xE10` 1, every form (`+0xDE0` 1..6), Checked (decomp).)
    let arrival = ask_own_then(actor, ARRIVE_MOTION, Some(STANCE_MOTION), air);
    let stance = arrival && !air && own_request(actor, STANCE_MOTION).is_some();
    // (In the air: held through it with no speed, KH2's ACTION_FLY with velocity 0, `arrival_held`.)
    if arrival && air {
        wr(actor + 0x2C, 0.0f32);
        wr(actor + 0x3C0, [0.0f32; 3]);
        ARRIVE_AIR.store(true, Ordering::Relaxed);
    }
    log!(Forms, "{} (LV{}, {} EXP) on KH1's own states: its moves on his requests, KH2's numbers its own ({}), {} {}{}", kind.name(), rec.level, rec.exp, kind.model(), kind.growth(), growth, if stance { "; its arrival, then its stance (motion 6)" } else if arrival { "; its arrival" } else { "" });
}

/// The form's growth ability's level, as a name.
fn growth_name(a: &Abilities, kind: Kind) -> &'static str {
    let level = match kind {
        Kind::Valor => a.high_jump,
        Kind::Wisdom => a.quick_run,
        Kind::Final => a.glide,
    };
    ["none", "LV1", "LV2", "LV3", "MAX"][(level as usize).min(4)]
}

/// KH2's controller's data (only read on KH1's states: KH2's pick, its rows' checks, its growth)
/// for the moveset `data`, his facing, lock-on and glide button kept.
fn native_data(ch: &mut Character, data: &Character, abilities: Abilities) {
    let (facing, lock, glide) = (ch.facing(), ch.lock(), ch.glide_button);
    let mut c = data.clone();
    c.abilities = abilities;
    c.glide_button = glide;
    c.air_magic_falls = ch.air_magic_falls;
    c.air_magic_gravity = ch.air_magic_gravity;
    c.set_facing(facing);
    c.set_lock(lock);
    *ch = c;
}

/// Back to Sora on KH1's own states: base Sora's slots and data again, what played cut into KH1's
/// idle (or fall), the revert's light and sound.
unsafe fn native_revert(d: &mut Drive, ch: &mut Character, kh2: &pose::Kh2, actor: usize, why: &str) {
    CHANGING.store(false, Ordering::Relaxed);
    d.change = None;
    d.revert = false;
    d.changing_to = None;
    let Some(f) = d.form.take() else {
        NATIVE.store(false, Ordering::Relaxed);
        return;
    };
    IN_FORM.store(false, Ordering::Relaxed);
    CURRENT.store(0, Ordering::Relaxed);
    NATIVE.store(false, Ordering::Relaxed);
    if f.kind.two_blades() {
        super::valor::ended();
    }
    native_data(ch, &kh2.character, f.sora);
    super::kh2slots::refresh();
    super::formmagic::end_all();
    super::kh2fx::owner_gone();
    for el in REVERT_ELEMENTS {
        super::kh2fx::event(REVERT_FX, EventKind::Effect(el));
        super::kh2sound::event(REVERT_FX, EventKind::Effect(el));
    }
    // (KH2: what was playing is cut, he is free with his movement kept: KH1's idle on the ground,
    // its fall in the air, his action flag off.)
    let state = rd::<i32>(actor + 0x70);
    let air = matches!(state, 1 | 2);
    if matches!(state, 0..=2) {
        let base = super::takeover::game_base();
        let set: unsafe extern "system" fn(usize, u32, f32) -> usize = std::mem::transmute(base + SET_MOTION_RVA);
        set(actor, if air { 6 } else { 0 }, 8.0);
        wr(actor, rd::<u32>(actor) & !4);
    }
    // (In the air: KH2's arc hands him back, his fall from rest if a hold had him, t855.)
    if air {
        super::shared::physics::form_left_in_air(actor);
    }
    super::tint::refresh(actor, None);
    log!(Forms, "Revert ({why}) on KH1's own states{}", if air { ", falling" } else { "" });
}

unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

/// Sora's Drive motion: its effects and sounds play whatever the ini's lists (`kh2fx`,
/// `kh2sound`).
pub fn own_motion(motion: u16) -> bool {
    matches!(motion, DRIVE_MOTION | REVERT_FX)
}


/// A frame trigger 1 of the form's motion: Wisdom's shots, Valor's own elements.
pub fn effect(motion: u16, n: i16) {
    if in_wisdom() {
        super::formmagic::effect(n);
    } else if in_valor() || in_final() {
        super::kh2fx::form_event(motion, n);
    }
}
