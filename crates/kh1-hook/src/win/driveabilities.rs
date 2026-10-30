//! KH2's Drive support abilities on KH1's own code (the owner, the main-branch port's round 6, row 5:
//! answers 1 (a), 2 (a) Drive Boost after the main build, 3 (a)). The rules: `crate::drivesupport`;
//! the abilities themselves: `growth::MENU` 89..92, in the KH2 list (t953). Notes:
//! `docs/kh1-drive-forms-native.md` §17.
//!
//! Steam KH1, Checked (decomp) and (exe):
//! - **A hit's HP** `0x1402BEEE0 (target, hit record)`: KH1's hit takes the record's damage (`+8`)
//!   off the target's HP through its HP add `0x1402A4920` (the stats at the target's `+0x6C` handle:
//!   HP `+0x3C`, max `+0x40`), unless the record is a no-damage kind (`+0xC & 0x1F` 6 / 7, `+0` bit
//!   5) or the target can't be hurt. Hooked (first bytes `48 89 5C 24 10 56 48 83 EC 20`), for Sora
//!   only (`takeover::sora_actor`): **Damage Control** (t957) lowers the record's damage before KH1's
//!   (and puts it back after, the record being KH1's); **Damage Drive** (t954) gives the HP he lost
//!   to the Drive gauge after it, out of a form, with the gauge the form cost.
//! - **Form Boost** (t955): the form gauge's time drain is ours (`form`'s step), × `form_boost`.
//! - **Drive Converter** (t956): KH1's enemy drop `0x1402ABC40 (drop, ..)` (hooked by `driveorbs`)
//!   is handed a copy of the drop whose munny counts keep KH2's share; the rest are made as Drive orbs
//!   after it, × KH1's count factor as KH1 makes its munny.
//! - Only with Drive on (the ini's `drive`) and on Steam. Copies are read from Sora's lists as KH1
//!   holds them (`menu::kh1_abilities_now`), at most twice a second.

use super::log;
use crate::drivesupport as rules;
use crate::growth::kh2;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;

const HIT_HP_RVA: usize = 0x2BEEE0;
const HIT_HP_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x10, 0x56, 0x48, 0x83, 0xEC, 0x20];
/// The actor's stats handle, and in the stats HP and max HP.
const ACTOR_STATS: usize = 0x6C;
const STATS_HP: usize = 0x3C;
const STATS_MAX: usize = 0x40;
/// The hit record's damage.
const RECORD_DAMAGE: usize = 8;
/// KH1's enemy drop's data: 7 prize counts, then 3 (chance, item) pairs.
const DROP_LEN: usize = 13;

static ORIG_HIT_HP: AtomicUsize = AtomicUsize::new(0);
static LOGGED: AtomicU32 = AtomicU32::new(0);
/// The copies of each (Damage Drive, Form Boost, Damage Control, Drive Converter) and when read.
static COPIES: Mutex<Option<(std::time::Instant, [u8; 4])>> = Mutex::new(None);
/// The drop handed to KH1 (a copy, its munny counts KH2's share).
static mut DROP: [i32; DROP_LEN] = [0; DROP_LEN];

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

fn note(line: impl FnOnce() -> String) {
    if LOGGED.fetch_add(1, Ordering::Relaxed) < 60 {
        log!(Forms, "Drive support: {}", line());
    }
}

fn on() -> bool {
    super::settings::get().drive && super::addr::Build::current() == super::addr::Build::Steam
}

pub unsafe fn install(base: usize) {
    if !on() {
        return;
    }
    super::hook::install(base + HIT_HP_RVA, &HIT_HP_BYTES, hit_hp as *const () as usize, &ORIG_HIT_HP, "KH1's hit HP (Damage Drive, Damage Control)");
}

/// Sora's equipped copies: Damage Drive, Form Boost, Damage Control, Drive Converter.
fn copies() -> [u8; 4] {
    let now = std::time::Instant::now();
    if let Ok(g) = COPIES.try_lock() {
        if let Some((at, c)) = *g {
            if now.duration_since(at).as_millis() < 500 {
                return c;
            }
        }
    }
    let base = super::takeover::game_base();
    let c = if base == 0 || !on() {
        [0; 4]
    } else {
        unsafe { super::menu::kh1_abilities_now(base) }.map_or([0; 4], |a| {
            [kh2::DAMAGE_DRIVE, kh2::FORM_BOOST, kh2::DAMAGE_CONTROL, kh2::DRIVE_CONVERTER].map(|n| crate::growth::support_copies(&a, n))
        })
    };
    if let Ok(mut g) = COPIES.try_lock() {
        *g = Some((now, c));
    }
    c
}

/// Form Boost's factor on the form gauge's time drain now (t955).
pub fn form_boost() -> f32 {
    rules::form_boost(copies()[1])
}

/// Sora's HP and max HP.
unsafe fn hp(actor: usize) -> Option<(usize, i32, i32)> {
    let stats = super::pose::ptr(rd::<u32>(actor + ACTOR_STATS));
    (stats >= 0x10000 && super::crash::readable(stats + STATS_MAX + 3)).then(|| (stats, rd::<i32>(stats + STATS_HP), rd::<i32>(stats + STATS_MAX)))
}

/// KH1's hit HP, with Damage Control before it and Damage Drive after it (Sora's only).
unsafe extern "system" fn hit_hp(target: usize, record: usize) -> u32 {
    let f: unsafe extern "system" fn(usize, usize) -> u32 = std::mem::transmute(ORIG_HIT_HP.load(Ordering::Relaxed));
    let sora = super::takeover::sora_actor();
    if target == 0 || target != sora || record < 0x10000 {
        return f(target, record);
    }
    let Some((_, before, max)) = hp(target) else { return f(target, record) };
    let c = copies();
    // Damage Control (t957): the record's damage halved for KH1's call, then KH1's back.
    let damage = rd::<i32>(record + RECORD_DAMAGE);
    let lowered = rules::damage_control(damage, before, max, c[2]);
    if lowered != damage {
        wr(record + RECORD_DAMAGE, lowered);
    }
    let r = f(target, record);
    if lowered != damage {
        wr(record + RECORD_DAMAGE, damage);
        note(|| format!("Damage Control: {damage} -> {lowered} at {before} of {max} HP ({} copies)", c[2]));
    }
    // Damage Drive (t954): the HP lost, out of a form, on the gauge that pays for the forms.
    if c[0] > 0 && !super::form::in_form() && super::form::gauge_cost() {
        if let Some((_, after, max)) = hp(target) {
            let points = rules::damage_drive(before - after, max, c[0]);
            if points > 0 {
                super::drivegauge::add(points, "Damage Drive");
            }
        }
    }
    r
}

/// KH1's enemy drop with Drive Converter (t956): the drop KH1 is to be handed (a copy with KH2's
/// munny share, or KH1's own), and the (small, big) Drive orbs the rest become.
pub unsafe fn convert(drop: usize) -> (usize, i32, i32) {
    let n = copies()[3];
    if n == 0 || drop < 0x10000 || !super::form::gauge_cost() || !super::crash::readable(drop + DROP_LEN * 4 - 1) {
        return (drop, 0, 0);
    }
    let ours = std::ptr::addr_of_mut!(DROP) as *mut i32;
    for i in 0..DROP_LEN {
        *ours.add(i) = rd::<i32>(drop + i * 4);
    }
    let (mut small, mut big) = (0, 0);
    for (ty, to_big) in rules::MUNNY_TYPES {
        let (keep, orbs) = rules::drive_converter(*ours.add(ty), n);
        *ours.add(ty) = keep;
        if to_big {
            big += orbs;
        } else {
            small += orbs;
        }
    }
    if small == 0 && big == 0 {
        return (drop, 0, 0);
    }
    note(|| format!("Drive Converter ({n} copies): {small} small and {big} big orbs in place of munny"));
    (ours as usize, small, big)
}
