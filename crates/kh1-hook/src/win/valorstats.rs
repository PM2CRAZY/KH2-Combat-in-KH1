//! Valor Form's second Keyblade's stats (the user: "if a secondary keyblade is active ... sora
//! should see stat changes from that keyblade. For example, the increased strength and MP, or
//! in the case of certain keyblades, reduced MP").
//!
//! KH1 gives a member one weapon: its weapon-table row adds Strength (`+0x30`, signed) and max
//! MP (`+0x38`, signed) in KH1's stats function (`0x1402A2FA0 (member, stats)`), which then caps
//! Strength at 100 (`docs/kh1-equipment.md` §1). In Valor the slot's Keyblade adds its own the
//! same way, on top of Sora's:
//!
//! - **Sora's live stats** (`actor + 0x6C`): each KH2 step (`form::after_step`) his Strength and
//!   max MP are set to KH1's own figures for his equipment (the stats function run into a block
//!   of ours, `menu::kh1_stats`) plus the Valor Keyblade's, Strength at most 100, max MP at
//!   least 0; his MP and its gauge (`+0x44`, `+0xBE`) kept within the new max as KH1 keeps them.
//!   Nothing is remembered from frame to frame, so KH1 rebuilding his stats (the pause menu's
//!   exit, an area) or a level-up raising them in place (`0x1402B34C0`) can't make the bonus
//!   count twice or stay behind.
//! - Out of Valor (its end, `valor::ended`, and the next steps): once, his Strength and max MP put
//!   back to KH1's own figures, his MP kept within the max.
//! - **The menus**: KH1's stats function for Sora (the pause menu's copy the Status and
//!   Equipment pages show, and the party refresh) adds the same while he is Valor
//!   (`menu::stats` calls `add`). While Valor's Keyblade is being chosen, KH1's compare box shows
//!   Valor's figures (his own Keyblade and the slot's -> his own and the hovered one,
//!   `equipmenu::box_update`, `with_second`), and every other call counts his own Keyblade in
//!   place of the one KH1's preview put in his weapon byte.
//!
//! KH2's hits land through KH1's own contact code, which reads Sora's live Strength, so they
//! deal the boosted damage too.

use super::log;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

/// Sora's stats block fields (`0x1402A2FA0`): MP, max MP, Strength, the MP gauge (u16, at most
/// max MP × 30).
const MP: usize = 0x44;
const MAX_MP: usize = 0x48;
const STRENGTH: usize = 0x4C;
const GAUGE: usize = 0xBE;
/// KH1's caps on Strength (and Defense, AP).
const STRENGTH_CAP: i32 = 100;
/// The stats block's size (its last field read here is `+0xE4`).
const STATS_SIZE: usize = 0x200;
/// Sora's weapon (his save record's `+0x36`).
const WEAPON: usize = 0x36;

/// What Sora's stats get now on top of KH1's own: the second Keyblade's (Valor, Master, Final: its
/// item, Strength, max MP).
/// (The 2 extra MP bars a form once added, s269, are gone with the Drive gauge: a form runs on its
/// form gauge now, `gauge`, not on MP.)
unsafe fn deltas() -> Option<(u8, i32, i32)> {
    bonus()
}

/// Sora's live stats hold a form's bonus (they're put back once out of the form).
static DIRTY: AtomicBool = AtomicBool::new(false);
/// The last (Keyblade, Strength, max MP) logged.
static LAST: Mutex<Option<(u8, i32, i32)>> = Mutex::new(None);
static LOGGED: AtomicU32 = AtomicU32::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

/// The second weapon's (item, Strength, max MP) while Sora is Valor, Master or Final: the form's
/// slot's (`equip::VALOR_OFF`, kept by `valor::keyblade`, the Dream Sword when it holds none:
/// its weapon row's Strength and MP count as a Keyblade's; `equip::FINAL_OFF`, kept by
/// `valor::final_keyblade`, the Kingdom Key when it holds none).
pub unsafe fn bonus() -> Option<(u8, i32, i32)> {
    if !super::form::two_blades() {
        return None;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    super::menu::save_block(base)?;
    let item = if super::form::in_final() {
        super::equip::final_stored(base)
    } else if super::form::in_master() {
        // (Master's slot, kept by `valor::master_keyblade`, the Kingdom Key when it holds none.)
        super::equip::master_stored(base)
    } else {
        // (Valor's: a Keyblade or the Dream Sword, `equip::valor_stored`.)
        super::equip::valor_stored(base)
    };
    if item == 0 || !(super::equip::is_keyblade(base, item) || item == super::equip::VALOR_DEFAULT) {
        return None;
    }
    let s = super::equip::weapon_strength(base, item)? as i32;
    let m = super::equip::weapon_mp(base, item)? as i32;
    Some((item, s, m))
}

/// `menu::stats`: KH1's stats function filled `out` for `member`; Sora's gets the Valor
/// Keyblade's while he is Valor.
pub unsafe fn add(member: i32, out: usize) {
    if member != 0 || out < 0x10000 {
        return;
    }
    // Choosing Valor's Keyblade (the user: "in the main menu, when selecting a secondary keyblade
    // it doesnt accurately display its stats"). KH1's compare box shows Strength now -> with the
    // hovered weapon from a what-if (`0x1402E69A0`: the hovered one written in his weapon byte
    // for one call of this function), and MP now -> now + the hovered one's - the current one's.
    // s223 / s258 added his own Keyblade and the one under the cursor less the one in his byte to
    // every call, the byte read as the preview's (it is the what-if's there), and the figures
    // "now" were his without a second Keyblade out of Valor (the MP's subtracting Valor's). Now
    // (`equipmenu::box_update`): the box's "now" is Valor's with the slot's Keyblade, its what-if
    // Valor's with the hovered one; any other call during the pick (the menu's own copy, KH1's
    // refresh as the list closes) has his own Keyblade in place of the preview's, as he holds it.
    if let Some(own) = super::equipmenu::pick_own() {
        let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
        let Some(save) = super::menu::save_block(base) else { return };
        let byte = rd::<u8>(save + WEAPON);
        if byte != own {
            wr(save + WEAPON, own);
            super::menu::kh1_stats(0, out);
            wr(save + WEAPON, byte);
        }
        if super::equipmenu::comparing() {
            let before = rd::<i32>(out + STRENGTH);
            let (s, m) = (super::equip::weapon_strength(base, byte).unwrap_or(0) as i32, super::equip::weapon_mp(base, byte).unwrap_or(0) as i32);
            wr(out + STRENGTH, (before + s).clamp(0, STRENGTH_CAP));
            let max = (rd::<i32>(out + MAX_MP) + m).max(0);
            wr(out + MAX_MP, max);
            clamp_mp(out, max);
            static P: AtomicU32 = AtomicU32::new(0);
            if P.fetch_add(1, Ordering::Relaxed) < 30 {
                log(&format!("Second Keyblade pick: compare box with {byte:#x} in the left hand and his own {own:#x}: Strength {before} {s:+} = {}, max MP {max}", rd::<i32>(out + STRENGTH)));
            }
            return;
        }
    }
    let Some((_, s, m)) = deltas() else { return };
    wr(out + STRENGTH, (rd::<i32>(out + STRENGTH) + s).clamp(0, STRENGTH_CAP));
    let max = (rd::<i32>(out + MAX_MP) + m).max(0);
    wr(out + MAX_MP, max);
    clamp_mp(out, max);
}

/// Sora's Strength and max MP in Valor with `own` as his weapon and `second` in his left hand,
/// as `step` makes them: KH1's own figures with `own` in his weapon byte (its stats function,
/// unhooked, into a block of ours; the byte put back after), plus `second`'s (none: 0).
pub unsafe fn with_second(own: u8, second: u8) -> Option<(i32, i32)> {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    let save = super::menu::save_block(base)?;
    #[repr(C, align(16))]
    struct Block([u8; STATS_SIZE]);
    let mut b = Block([0; STATS_SIZE]);
    let at = b.0.as_mut_ptr() as usize;
    let byte = rd::<u8>(save + WEAPON);
    wr(save + WEAPON, own);
    let ok = super::menu::kh1_stats(0, at);
    wr(save + WEAPON, byte);
    if !ok {
        return None;
    }
    let s = super::equip::weapon_strength(base, second).unwrap_or(0) as i32;
    let m = super::equip::weapon_mp(base, second).unwrap_or(0) as i32;
    Some(((rd::<i32>(at + STRENGTH) + s).clamp(0, STRENGTH_CAP), (rd::<i32>(at + MAX_MP) + m).max(0)))
}

/// MP and its gauge within `max`, as KH1's stats function keeps them.
unsafe fn clamp_mp(stats: usize, max: i32) {
    if rd::<i32>(stats + MP) > max {
        wr(stats + MP, max);
    }
    let cap = (max * 30).clamp(0, u16::MAX as i32) as u16;
    if rd::<u16>(stats + GAUGE) > cap {
        wr(stats + GAUGE, cap);
    }
}

/// Each KH2 step (`form::after_step`), and as Valor ends (`valor::ended`): Sora's live Strength
/// and max MP are KH1's own figures plus the Valor Keyblade's while he is Valor, KH1's own once
/// after.
pub unsafe fn step(sora: usize) {
    let want = deltas();
    if want.is_none() && !DIRTY.load(Ordering::Relaxed) {
        return;
    }
    if sora < 0x10000 || !super::crash::readable(sora + 0x6C) {
        return;
    }
    let stats = super::pose::ptr(rd::<u32>(sora + 0x6C));
    if stats < 0x10000 || !super::crash::readable(stats) || !super::crash::readable(stats + STATS_SIZE - 1) {
        return;
    }
    // KH1's own figures for Sora's equipment (its stats function, unhooked, into our block).
    #[repr(C, align(16))]
    struct Block([u8; STATS_SIZE]);
    let mut own = Block([0; STATS_SIZE]);
    let at = own.0.as_mut_ptr() as usize;
    if !super::menu::kh1_stats(0, at) {
        return;
    }
    let (own_s, own_m) = (rd::<i32>(at + STRENGTH), rd::<i32>(at + MAX_MP));
    let (s, m) = match want {
        Some((_, ds, dm)) => ((own_s + ds).clamp(0, STRENGTH_CAP), (own_m + dm).max(0)),
        None => (own_s, own_m),
    };
    let (was_s, was_m) = (rd::<i32>(stats + STRENGTH), rd::<i32>(stats + MAX_MP));
    if was_s != s {
        wr(stats + STRENGTH, s);
    }
    if was_m != m {
        wr(stats + MAX_MP, m);
    }
    clamp_mp(stats, m);
    DIRTY.store(want.is_some(), Ordering::Relaxed);
    // Logged as it changes.
    let key = want.map(|(k, _, _)| k).unwrap_or(0);
    if let Ok(mut last) = LAST.try_lock() {
        if *last != Some((key, s, m)) {
            *last = Some((key, s, m));
            if LOGGED.fetch_add(1, Ordering::Relaxed) < 40 {
                let line = match want {
                    Some((k, ds, dm)) => {
                        let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
                        let name = super::equip::item_name(base, k).unwrap_or_else(|| format!("{k:#x}"));
                        format!("form stats: {name} {ds:+} Str, {dm:+} MP: Strength {own_s} -> {s}, max MP {own_m} -> {m} (was {was_s} / {was_m})")
                    }
                    None => format!("Valor Keyblade stats: none (out of Valor): Strength {was_s} -> {s}, max MP {was_m} -> {m}"),
                };
                log(&line);
            }
        }
    }
}
