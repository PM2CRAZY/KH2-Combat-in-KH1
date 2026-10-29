//! Drive Form levels on KH1 (the tester, what-is-still-missing after m14, answer 6 (b): "build
//! form levels into KH1"). KH2's rule (`crate::formlevel`, Steam KH2 `0x1403E8380` and its
//! callers, Checked (decomp)) on KH1's own hit and defeat paths; notes in
//! `docs/kh1-drive-forms-native.md` §9.
//!
//! - **Kept in Sora's save**: `save + 0x882` Valor, `+0x884` Wisdom, `+0x886` Final, one u16 each
//!   (`formlevel::Record::pack`: level in the top 3 bits, EXP below; 0 = never entered, read as
//!   Lv1). These are the seven bytes after the bool at `+0x880` that ends KH1's save record's
//!   first block (its field list, the descriptor `0x1402CF620` and `0x1402CF7E0` pass ending "1d 10c" at `+0x888`): no
//!   instruction in the Steam exe reads or writes `+0x881..+0x887` by displacement, nor
//!   `0x142DE9BE1..BE7` by address, Checked (exe); the whole 0x16C00-byte block is what KH1 saves
//!   (copied whole by `0x1402CF7E0`), so they travel with the save. Assumption: nothing reaches them
//!   through a computed pointer (a static scan can't see that).
//! - **Valor, +1 per hit landed on an enemy** (KH2: the enemy's damage callbacks `0x1403DB730` /
//!   `0x1403DBB90` -> `0x1403E86B0`, the attack's owner the player, his form 1): KH1's attack
//!   contact `0x1402BF330` (hooked since stage 9, `probe::hit`) making a hit record for an attack
//!   Sora owns on an actor of kind 3 (an enemy), while he is in Valor. Both Keyblades' hits are
//!   Sora's attacks (`lefthits`), as both of KH2's are.
//! - **Wisdom, +1 per enemy defeated by Sora** (KH2: the enemy's death callbacks `0x1403DB680` /
//!   `0x1403DBAF0` -> `0x1403E8780`: its last attacker `+0xD38` the player, in form 2, the enemy a
//!   Heartless): KH1 has no last-attacker field, so the last actor whose attack made a record on
//!   each enemy is noted at the contact (`noted`), and the defeat is KH1's enemy scripts' EXP
//!   command (`0x1402BC9A0` / `0x1402BCA20 (args)`: `args[0]` the enemy, `args[1]` the EXP; they
//!   show the number at it and add it to the party, `0x140284990`), hooked at their entry so EXP
//!   Zero doesn't stop it. Assumption: every KH1 enemy gives its EXP through one of the two once
//!   when defeated, and counts as a Heartless (KH1's ids aren't KH2's).
//! - **Final, +1 per Nobody defeated**: KH1 has none, so Final gains nothing (asked).
//! - Limit (its limits' finishers) and Master (Drive orbs) aren't on the testing branch.
//! - Base Sora's side (the tester, after m15, answer 6: "(c)"): his growth level from each form's
//!   level (`base_growth`) and the fmlv rows' abilities (`rewards_due` / `rewarded`), given in
//!   KH1's own ability lists by `menu::Growth::update`.
//! - The cap is 2 + the forms Sora owns (KH2), here Valor and Wisdom (the Drive list's, always
//!   his) and Final once he has entered it (its record set).

use super::{form::Kind, log, pose};
use crate::formlevel::{self, Record, Table};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

/// The forms' records in Sora's save (three u16, Valor, Wisdom, Final).
pub const FORM_LEVELS_OFF: usize = 0x882;

/// KH1's enemy scripts' EXP commands (Steam), their first whole instructions (`push rbx; sub
/// rsp, 0x20; mov rbx, rcx` / `push rbx; sub rsp, 0x50`).
const EXP_SHOWN: usize = 0x2BC9A0;
const EXP_SHOWN_PROLOGUE: [u8; 9] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x20, 0x48, 0x8B, 0xD9];
const EXP_AT: usize = 0x2BCA20;
const EXP_AT_PROLOGUE: [u8; 6] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x50];
static ORIG_SHOWN: AtomicUsize = AtomicUsize::new(0);
static ORIG_AT: AtomicUsize = AtomicUsize::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

fn kh2_form(kind: Kind) -> u8 {
    match kind {
        Kind::Valor => formlevel::form::VALOR,
        Kind::Wisdom => formlevel::form::WISDOM,
        Kind::Final => formlevel::form::FINAL,
    }
}

fn slot(kind: Kind) -> usize {
    match kind {
        Kind::Valor => 0,
        Kind::Wisdom => 1,
        Kind::Final => 2,
    }
}

/// KH2's fmlv rows from the player's own `00battle.bin` (forms 1..5).
fn table() -> Option<&'static Table> {
    static T: OnceLock<Table> = OnceLock::new();
    if let Some(t) = T.get() {
        return Some(t);
    }
    let kh2 = pose::kh2()?;
    let t = Table::new(kh2.battle.fmlv.iter().filter(|r| (1..=5).contains(&r.form)).map(|r| formlevel::Row { form: r.form, level: r.level, growth: r.ability_level, reward: r.ability, exp: r.exp }));
    if t.is_empty() {
        return None;
    }
    Some(T.get_or_init(|| t))
}

unsafe fn save() -> Option<usize> {
    let base = super::takeover::game_base();
    if base == 0 {
        return None;
    }
    super::menu::save_block(base).filter(|s| super::crash::readable(s + FORM_LEVELS_OFF + 6))
}

unsafe fn raw(save: usize, kind: Kind) -> u16 {
    rd::<u16>(save + FORM_LEVELS_OFF + slot(kind) * 2)
}

/// The form's level and EXP.
pub unsafe fn record(kind: Kind) -> Record {
    save().map_or_else(Record::default, |s| Record::unpack(raw(s, kind)))
}

/// The form's growth ability level at its level (KH2's fmlv; LV1 without the table).
pub fn growth(kind: Kind) -> u8 {
    let level = unsafe { record(kind) }.level;
    table().map_or(1, |t| t.growth(kh2_form(kind), level).max(1))
}

/// Base Sora's own growth levels from the forms' levels (KH2's `0x1403E8380`: one under the
/// form's growth level, `formlevel::sora_growth`), for `growth::Kh1Sora::form_growth`: Valor's
/// High Jump, Wisdom's Quick Run, Final's Glide. All 0 without the table or the save.
pub fn base_growth() -> crate::growth::FormGrowth {
    let Some(t) = table() else { return Default::default() };
    let g = |kind: Kind| formlevel::sora_growth(t, kh2_form(kind), unsafe { record(kind) }.level);
    crate::growth::FormGrowth { high_jump: g(Kind::Valor), quick_run: g(Kind::Wisdom), glide: g(Kind::Final) }
}

/// The fmlv abilities the forms' levels have reached and Sora hasn't been given yet (KH2 gives
/// each with its level, `0x1403C3F00`): (form, level, KH2's ability id), in order per form.
pub fn rewards_due() -> Vec<(Kind, u8, u16)> {
    let Some(t) = table() else { return Vec::new() };
    let mut out = Vec::new();
    for kind in [Kind::Valor, Kind::Wisdom, Kind::Final] {
        let rec = unsafe { record(kind) };
        out.extend(formlevel::rewards_due(t, kh2_form(kind), &rec).into_iter().map(|(l, id)| (kind, l, id)));
    }
    out
}

/// `kind`'s abilities up to `level` are given (or have no KH1 counterpart yet): the save's
/// rewarded level.
pub unsafe fn rewarded(kind: Kind, level: u8) {
    let Some(s) = save() else { return };
    let v = raw(s, kind);
    if v == 0 {
        return;
    }
    let mut rec = Record::unpack(v);
    if level > rec.rewarded {
        rec.rewarded = level.min(rec.level);
        std::ptr::write_unaligned((s + FORM_LEVELS_OFF + slot(kind) * 2) as *mut u16, rec.pack());
    }
}

/// The form is Sora's from now (its record set at Lv1 the first time he enters it: KH2's records
/// start there; what counts toward the cap).
pub unsafe fn entered(kind: Kind) {
    let Some(s) = save() else { return };
    if raw(s, kind) == 0 {
        std::ptr::write_unaligned((s + FORM_LEVELS_OFF + slot(kind) * 2) as *mut u16, Record::default().pack());
    }
}

/// The level cap now: 2 + the forms he owns (Valor and Wisdom, and Final once entered).
unsafe fn cap(s: usize) -> u8 {
    formlevel::cap(2 + u8::from(raw(s, Kind::Final) != 0))
}

/// KH2's add (`0x1403E8380`): `n` EXP to `kind`'s record; each level gained is logged and shown in
/// KH1's prompt box.
unsafe fn add(kind: Kind, n: u32, why: &str) {
    let (Some(s), Some(t)) = (save(), table()) else { return };
    let mut rec = Record::unpack(raw(s, kind));
    let cap = cap(s);
    let ups = formlevel::add(&mut rec, kh2_form(kind), n, cap, t);
    std::ptr::write_unaligned((s + FORM_LEVELS_OFF + slot(kind) * 2) as *mut u16, rec.pack());
    for u in &ups {
        let growth = u.form_growth.map(|g| format!(", {} {}", kind.growth(), growth_name(g))).unwrap_or_default();
        log!(Forms, "{} LV{} ({why}, cap {cap}){growth}; Sora's own: {}{} (given by KH1's menu, abilities = kh1)", kind.name(), u.level, u.sora_growth.map(|g| format!("{} {}, ", kind.growth(), growth_name(g))).unwrap_or_default(), if u.reward != 0 { format!("KH2 ability {}", u.reward) } else { "no ability".into() });
        super::menu::queue_prompt("Form Level Up", &format!("{} LV{}", kind.name(), u.level));
    }
    static LOGGED: AtomicUsize = AtomicUsize::new(0);
    if ups.is_empty() && LOGGED.fetch_add(1, Ordering::Relaxed) < 40 {
        log!(Forms, "{} EXP +{n} ({why}): LV{} with {} EXP (cap {cap})", kind.name(), rec.level, rec.exp);
    }
}

fn growth_name(g: u8) -> &'static str {
    ["none", "LV1", "LV2", "LV3", "MAX"][(g as usize).min(4)]
}

/// Each enemy's last attacker (KH2's `+0xD38`; KH1 has none): the actor whose attack last made a
/// hit record on it, at KH1's contact.
static LAST: Mutex<Vec<(usize, usize)>> = Mutex::new(Vec::new());

/// KH1's contact made hit record `rec` for `attack` of `owner` on `target` (`probe::hit`).
pub unsafe fn hit(owner: usize, target: usize, rec: usize) {
    if owner < 0x10000 || target < 0x10000 || rec < 0x10000 || rd::<u8>(target + 6) != 3 {
        return;
    }
    if let Ok(mut v) = LAST.lock() {
        match v.iter_mut().find(|e| e.0 == target) {
            Some(e) => e.1 = owner,
            None => {
                if v.len() >= 64 {
                    v.remove(0);
                }
                v.push((target, owner));
            }
        }
    }
    if owner == super::takeover::sora_actor() && super::form::current() == Some(Kind::Valor) {
        add(Kind::Valor, 1, "a hit landed");
    }
}

/// Enemy `enemy` was defeated (its script gives its EXP): Wisdom's EXP when Sora was its last
/// attacker (KH2's `0x1403E8780`).
unsafe fn defeated(enemy: usize) {
    let last = LAST.lock().ok().and_then(|mut v| v.iter().position(|e| e.0 == enemy).map(|i| v.remove(i).1));
    let sora = super::takeover::sora_actor();
    if sora == 0 || last != Some(sora) {
        return;
    }
    match super::form::current() {
        Some(Kind::Wisdom) => add(Kind::Wisdom, 1, "an enemy defeated"),
        // (Final counts Nobodies, which KH1 has none of.)
        _ => {}
    }
}

unsafe fn enemy_of(args: usize) -> usize {
    if args < 0x10000 {
        return 0;
    }
    pose::ptr(rd::<u32>(args))
}

unsafe extern "system" fn exp_shown(args: usize) {
    defeated(enemy_of(args));
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_SHOWN.load(Ordering::Acquire));
    f(args)
}

unsafe extern "system" fn exp_at(args: usize) {
    defeated(enemy_of(args));
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_AT.load(Ordering::Acquire));
    f(args)
}

/// The defeat hooks (Steam only: the addresses are the Steam build's).
pub unsafe fn install(base: usize) {
    if super::addr::Build::current() != super::addr::Build::Steam {
        log!(Forms, "form levels: defeats not hooked (not the Steam build)");
        return;
    }
    super::hook::install(base + EXP_SHOWN, &EXP_SHOWN_PROLOGUE, exp_shown as *const () as usize, &ORIG_SHOWN, "enemy EXP shown (form levels)");
    super::hook::install(base + EXP_AT, &EXP_AT_PROLOGUE, exp_at as *const () as usize, &ORIG_AT, "enemy EXP at a point (form levels)");
}
