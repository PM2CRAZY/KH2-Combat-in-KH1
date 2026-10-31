//! Valor's Keyblade slot in KH1's own menus (the owner, the main-branch port's round 11, row 20:
//! answer 2 "check with the decomp and how to implement it"). Notes: `docs/kh1-drive-forms-native.md`
//! §22.
//!
//! - **In use (t967).** KH2 (Steam KH2, Checked (decomp)): equipping a form's Keyblade puts the old
//!   one back in the stock (`0x1403C3F00`) and takes the new one out (`0x1403C49C0` → `0x1403C4A40`)
//!   around the form record's write (`0x1402FC7C0`, form records at save `+0x32F4`, 0x38 each), as
//!   for Sora's own weapon; its count (`0x1403C4720`) is the stock, so a slotted Keyblade is gone
//!   from Sora's list, the other forms' and (Assumption, the shop is an overlay not in the decomp)
//!   the sell list. KH1 (Steam, Checked (decomp) and (exe)): its stock counts every copy, equipped
//!   or not, and its free count `0x1402E8770 (item)` takes the members' equipped copies off when it
//!   reads (its callers: the Equipment lists, the shop's sell list). Native here: that count also
//!   takes off the copy Valor's slot holds (`equip::valor_holds`), so every KH1 reader sees it in
//!   use; not during a Valor pick, whose own list keeps the slot's Keyblade (`equipmenu`). KH1's
//!   stock itself is left as KH1 keeps it (its saves and shops count equipped copies).
//! - **The compare box (t968).** KH1's weapon compare box `0x1403050D0 (window)` (Checked
//!   (decomp)): "now" Strength / max MP from the window's `+0x4C` / `+0x50`, the item being replaced
//!   `0x142E9A07C`; "new" Strength from `0x1402E8DE0 (member, hovered)` (KH1's stats with the hovered
//!   weapon in his byte, which `valorstats` already makes Valor's: his own and the hovered one) and
//!   "new" max MP = now + the hovered one's − the replaced one's. During a Valor pick ours puts
//!   Valor's "now" figures in (his own Keyblade and the slot's, or his own twice with the slot
//!   empty, the replaced item then his own) for KH1's draw, and the replaced item back after.
//! - **Master's M: slot (t1025)** goes through the same two: the free count also takes off the copy
//!   Master's slot holds (`equip::master_holds`), and during an M: pick the compare box shows
//!   Master's "now" (his own Keyblade and the M: slot's), as Master adds its Keyblade as Valor does
//!   (t1024). During a pick only the other form's copy is taken off.
//! - **Final's F: slot (t1038)** the same way: the free count takes off the copy each of the three
//!   slots holds (`equip::SLOT_FORMS`, `equip::slot_holds`), during a pick the other two forms'
//!   only; an F: pick's compare box shows Final's "now" (his own Keyblade and the F: slot's, his own
//!   twice with it empty).
//! - Only with Drive on and on Steam. Both run when KH1 calls them (menus, shops), never each frame.

use super::log;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

const FREE_RVA: usize = 0x2E8770;
const FREE_BYTES: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x7C, 0x24, 0x10];
const COMPARE_RVA: usize = 0x3050D0;
const COMPARE_BYTES: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18];
/// The item being replaced (header, compare box).
const CURRENT_RVA: usize = 0x2E9A07C;
/// The compare window's "now" Strength and max MP.
const WIN_STRENGTH: usize = 0x4C;
const WIN_MAX_MP: usize = 0x50;
/// KH1's stats block: max MP, Strength (`valorstats`), and its size.
const MAX_MP: usize = 0x48;
const STRENGTH: usize = 0x4C;
const STATS_SIZE: usize = 0x200;
const STRENGTH_CAP: i32 = 100;

static ORIG_FREE: AtomicUsize = AtomicUsize::new(0);
static ORIG_COMPARE: AtomicUsize = AtomicUsize::new(0);
static LOGGED: AtomicU32 = AtomicU32::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

fn on() -> bool {
    super::settings::get().drive && super::addr::Build::current() == super::addr::Build::Steam
}

fn note(line: impl FnOnce() -> String) {
    if LOGGED.fetch_add(1, Ordering::Relaxed) < 40 {
        log!(Menus, "Valor slot: {}", line());
    }
}

pub unsafe fn install(base: usize) {
    if !on() {
        return;
    }
    super::hook::install(base + FREE_RVA, &FREE_BYTES, free_count as *const () as usize, &ORIG_FREE, "KH1's free count (Valor's slot in use)");
    super::hook::install(base + COMPARE_RVA, &COMPARE_BYTES, compare as *const () as usize, &ORIG_COMPARE, "KH1's weapon compare box (Valor's figures)");
}

/// KH1's free count, less the copies Valor's, Master's and Final's slots hold (Master's since
/// t1025, Final's t1038; during a form's pick, the other forms' only).
unsafe extern "system" fn free_count(item: u32) -> u64 {
    let f: unsafe extern "system" fn(u32) -> u64 = std::mem::transmute(ORIG_FREE.load(Ordering::Relaxed));
    let n = f(item);
    if n == 0 || !(1..=255).contains(&item) {
        return n;
    }
    let base = super::takeover::game_base();
    if base == 0 {
        return n;
    }
    let picking = super::equipmenu::slot_picking().map(|(kind, _)| kind);
    let mut left = n;
    for kind in super::equip::SLOT_FORMS {
        if picking != Some(kind) && left > 0 && super::equip::slot_holds(base, kind) == Some(item as u8) {
            left -= 1;
            note(|| format!("item {item:#04x}: {n} free, {left} with {}'s copy taken off", kind.name()));
        }
    }
    left
}

/// KH1's compare box, with Valor's "now" during a Valor pick.
unsafe extern "system" fn compare(window: usize) {
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_COMPARE.load(Ordering::Relaxed));
    let base = super::takeover::game_base();
    let (Some((kind, slot)), Some((own, in_byte, _))) = (super::equipmenu::slot_picking(), super::equipmenu::valor_preview()) else {
        return f(window);
    };
    if base == 0 || window < 0x10000 {
        return f(window);
    }
    // KH1's own figures with what his byte holds now, then Valor's: his own and the slot's (his own
    // again with the slot empty).
    #[repr(C, align(16))]
    struct Block([u8; STATS_SIZE]);
    let mut raw = Block([0; STATS_SIZE]);
    let at = raw.0.as_mut_ptr() as usize;
    if !super::menu::kh1_stats(0, at) {
        return f(window);
    }
    let second = if slot != 0 { slot } else { own };
    let st = |w: u8| super::equip::weapon_strength(base, w).unwrap_or(0) as i32;
    let mp = |w: u8| super::equip::weapon_mp(base, w).unwrap_or(0) as i32;
    let strength = (rd::<i32>(at + STRENGTH) - st(in_byte) + st(own) + st(second)).clamp(0, STRENGTH_CAP);
    let max_mp = (rd::<i32>(at + MAX_MP) - mp(in_byte) + mp(own) + mp(second)).max(0);
    wr(window + WIN_STRENGTH, strength);
    wr(window + WIN_MAX_MP, max_mp);
    let current = base + CURRENT_RVA;
    let was = rd::<i32>(current);
    if slot == 0 {
        wr(current, own as i32);
    }
    f(window);
    wr(current, was);
    note(|| format!("compare box: {} now Strength {strength}, max MP {max_mp} (his {own:#04x} and {second:#04x})", kind.name()));
}
