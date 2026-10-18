//! Wisdom Form (the user: "lets add wisdom form", from the second command page's Drive, costing
//! MP, Sora recoloured blue). Chosen from `drive`'s list, it plays out as KH2's Drive does
//! (`kh2-bevy`'s `drive.rs`, from the exe):
//! - Sora's Drive motion (52, invulnerable) with base Sora's moveset; when it ends, the swap:
//!   the form's moveset, motions and combo rows (`P_EX100_MAGF`, moveset 3), his facing and
//!   lock kept, and its arrival (53). Its abilities: Sora's support abilities, not his action
//!   or growth ones, and the form's own growth ability, Quick Run, at MAX (the user);
//! - the form lasts `(level + 3) × 600` frames of battle (KH2's gauge, exe: it drains 1 a
//!   frame in battle, ½ out of it); then, or on the Revert command (the same Drive entry), back to Sora with
//!   the revert motion (4), on the ground. Knocked out: back at once;
//! - its shots and its spells: `formmagic`.
//! The cost is MP (`COST`, KH1's own payment), not a Drive gauge, which KH1 has none of.

use super::{log, pose};
use character_core::{Abilities, Character, EventKind};
use kh2_formats::battle::Battle;
use kh2_rig::RigAnim;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;

pub use super::formmagic::{effect, element_sounds, package_spell, shots_step, spell_file, LAST_PACKAGE, SPELL_NAMES};

pub const DRIVE_MOTION: u16 = 52;
pub const ARRIVE_MOTION: u16 = 53;
pub const REVERT_MOTION: u16 = 4;
/// The longest Sora's Drive motion runs before the swap (KH2: its 160 frames).
const DRIVE_FRAMES: u32 = 160;
/// MP to change (KH1's payment takes hundredths of a bar from 100 up).
pub const COST_MP: i32 = 3;
const COST: i32 = COST_MP * 100;
/// The form's level (KH1 has no form experience): Lv 1, 40 s of battle.
const LEVEL: u32 = 1;
/// KH2's magic sub offset in Wisdom Form.
pub const MAGIC_SUB: i32 = 10;

/// A request: 1 Wisdom, 2 revert (the menu), 3 knocked out (`hurt`).
static REQUEST: AtomicU8 = AtomicU8::new(0);
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

pub fn busy() -> bool {
    CHANGING.load(Ordering::Relaxed) || REQUEST.load(Ordering::Relaxed) != 0
}

/// KH2's magic sub offset now (`reflect`).
pub fn magic_sub() -> i32 {
    if in_form() {
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

unsafe fn hp(sora: usize) -> i32 {
    let stats = pose::ptr(rd::<u32>(sora + 0x6C));
    if stats < 0x10000 {
        return 1;
    }
    rd::<i32>(stats + 0x3C)
}

/// Whether the Drive entry can be chosen now (`drive`'s menu): reverting in the form, else MP
/// for it and KH2 having Sora, its data loaded.
pub unsafe fn usable(sora: usize) -> bool {
    if busy() {
        return false;
    }
    if in_form() {
        return true;
    }
    pose::kh2().is_some_and(|k| k.wisdom.is_some()) && super::takeover::kh2_has() && sora > 0x10000 && mp(sora) >= COST_MP
}

/// Chosen from the Drive list (`drive::chosen`). Returns why not, if refused.
pub unsafe fn choose(sora: usize) -> Result<&'static str, String> {
    if busy() {
        return Err("a change is under way".into());
    }
    if in_form() {
        REQUEST.store(2, Ordering::Relaxed);
        return Ok("Revert");
    }
    if !pose::kh2().is_some_and(|k| k.wisdom.is_some()) {
        return Err("Wisdom Form's files didn't load".into());
    }
    if !super::takeover::kh2_has() {
        return Err("KH1 has Sora".into());
    }
    let have = mp(sora);
    if have < COST_MP {
        return Err(format!("MP {have}, it costs {COST_MP}"));
    }
    REQUEST.store(1, Ordering::Relaxed);
    Ok("Wisdom Form")
}

/// The Driver's form state (`takeover`).
#[derive(Default)]
pub struct Drive {
    /// Changing: Sora's Drive motion playing, its frames.
    change: Option<u32>,
    /// In the form: battle frames left, Sora's own abilities (back on revert), the form's.
    form: Option<InForm>,
    /// A revert asked for, waiting for the ground.
    revert: bool,
}

struct InForm {
    /// In half frames (the gauge drains 1 a frame in battle, ½ out of it).
    left: u32,
    sora: Abilities,
    own: Abilities,
    logged_s: u32,
}

/// The form's controller data while in it, else Sora's.
pub fn character<'a>(d: &Drive, kh2: &'a pose::Kh2) -> &'a Character {
    match (&d.form, &kh2.wisdom) {
        (Some(_), Some(f)) => &f.character,
        _ => &kh2.character,
    }
}

pub fn battle<'a>(d: &Drive, kh2: &'a pose::Kh2) -> &'a Arc<Battle> {
    match (&d.form, &kh2.wisdom) {
        (Some(_), Some(f)) => &f.battle,
        _ => &kh2.battle,
    }
}

/// The form's abilities (KH2's, exe): Sora's support abilities carry over, his action and
/// growth ones don't; the form's own growth ability at its level; every combo row of the form's
/// table unlocked.
fn form_abilities(sora: &Abilities, form: &Character) -> Abilities {
    let mut a = Abilities {
        guard: form.moveset.clips.contains_key(&character_core::motion::GUARD),
        aerial_recovery: false,
        combo_plus: sora.combo_plus,
        air_combo_plus: sora.air_combo_plus,
        finishing_plus: sora.finishing_plus,
        negative_combo: sora.negative_combo,
        combo_master: sora.combo_master,
        one_swing_on_miss: sora.one_swing_on_miss,
        high_jump: 0,
        // (The user: Wisdom with Quick Run MAX.)
        quick_run: 4,
        aerial_dodge: 0,
        glide: 0,
        dodge_roll: 0,
        ..sora.clone()
    };
    a.equipped = form.moveset.combo_rows.iter().map(|r| r.ability).filter(|&x| x != 0).collect();
    a
}

/// Replaces the controller (keeps facing, lock-on, the glide button).
fn swap(ch: &mut Character, anim: &mut RigAnim, data: &Character, rig: &Arc<kh2_data::sora::Rig>, abilities: Abilities) {
    let (facing, lock, glide) = (ch.facing(), ch.lock(), ch.glide_button);
    let mut c = data.clone();
    c.abilities = abilities;
    c.glide_button = glide;
    c.air_magic_falls = ch.air_magic_falls;
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
        revert(d, ch, anim, kh2, false, "knocked out");
        return;
    }
    match REQUEST.swap(0, Ordering::Relaxed) {
        1 if d.form.is_none() && d.change.is_none() => {
            if ch.airborne() || !ch.can_take_command() {
                // (KH2 takes the Drive command in the air too; here it waits for the ground.)
                log(&format!("Drive: Wisdom Form waits (airborne {}, busy {})", ch.airborne(), !ch.can_take_command()));
                REQUEST.store(1, Ordering::Relaxed);
                return;
            }
            if mp(actor) < COST_MP {
                log(&format!("Drive: MP {} short of {COST_MP} by now", mp(actor)));
                return;
            }
            if !ch.play(DRIVE_MOTION, true) {
                log("Drive: Sora has no Drive motion (52)");
                return;
            }
            let before = mp(actor);
            super::reflect::pay_mp(actor, COST);
            d.change = Some(0);
            CHANGING.store(true, Ordering::Relaxed);
            log(&format!("Drive: Wisdom Form (MP {before} -> {}), Sora's Drive motion", mp(actor)));
        }
        2 if d.form.is_some() => d.revert = true,
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
    if d.revert && !ch.airborne() && ch.can_take_command() {
        d.revert = false;
        revert(d, ch, anim, kh2, true, "the Revert command");
    }
}

unsafe fn arrive(d: &mut Drive, ch: &mut Character, anim: &mut RigAnim, kh2: &pose::Kh2) {
    CHANGING.store(false, Ordering::Relaxed);
    let Some(f) = &kh2.wisdom else { return };
    let sora = ch.abilities.clone();
    let own = form_abilities(&sora, &f.character);
    swap(ch, anim, &f.character, &f.rig, own.clone());
    ch.play(ARRIVE_MOTION, true);
    let left = (LEVEL + 3) * 600 * 2;
    d.form = Some(InForm { left, sora, own, logged_s: u32::MAX });
    IN_FORM.store(true, Ordering::Relaxed);
    log(&format!("Wisdom Form (Lv{LEVEL}): {:.0} s of battle (twice that out of it), Quick Run MAX", left as f32 / 120.0));
}

unsafe fn revert(d: &mut Drive, ch: &mut Character, anim: &mut RigAnim, kh2: &pose::Kh2, motion: bool, why: &str) {
    CHANGING.store(false, Ordering::Relaxed);
    d.change = None;
    d.revert = false;
    let Some(f) = d.form.take() else { return };
    IN_FORM.store(false, Ordering::Relaxed);
    swap(ch, anim, &kh2.character, &kh2.rig, f.sora);
    if motion {
        ch.play(REVERT_MOTION, true);
    }
    super::formmagic::end_all();
    log(&format!("Revert ({why})"));
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

/// After KH2's step: the look (`tint`), the gauge (battle frames), its end.
pub unsafe fn after_step(d: &mut Drive, actor: usize, battle: bool) {
    super::tint::refresh(actor, d.form.is_some());
    let Some(f) = d.form.as_mut() else { return };
    f.left = f.left.saturating_sub(if battle { 2 } else { 1 });
    let s = f.left.div_ceil(120);
    if s % 10 == 0 && s != f.logged_s {
        f.logged_s = s;
        log(&format!("Wisdom Form: {s} s left"));
    }
    if f.left == 0 && !d.revert {
        d.revert = true;
        log("Wisdom Form: the gauge is empty");
    }
}

/// Sora's Drive and revert motions: their effects and sounds play whatever the ini's lists
/// (`kh2fx`, `kh2sound`).
pub fn own_motion(motion: u16) -> bool {
    matches!(motion, DRIVE_MOTION | REVERT_MOTION)
}

/// Whether this frame's event goes to the form (its shots) instead of Sora's effects.
pub fn takes(d: &Drive, e: &EventKind) -> bool {
    d.form.is_some() && matches!(e, EventKind::Effect(_))
}
