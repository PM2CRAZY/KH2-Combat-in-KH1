//! Third-party add-ons in the game (tm44; `crate::addons` has the data, the save layout and the C
//! API, `docs/addons.md` the author's guide). Everything here goes through KH1's own systems the
//! KH2 list already uses (`kh2list`): KH1's ability table (`menu::ensure_table`: each add-on
//! ability an entry of its own at its runtime id), KH1's Abilities page (`kh2list`'s view: one row
//! of KH1's member picker per add-on, after "KH2", member ids 11.. whose list KH1's page reads at
//! `save + member * 0x74 + 0x44`), KH1's AP (`kh2list`'s AP hooks: equipped with Sora's AP) and
//! KH1's prompt box (`menu`'s "New Ability" boxes). No hook, no address and nothing per frame of
//! its own: the files are read once at start (`load`), the lists are looked at by the menu's
//! once-a-second step (`grant`), and the page's view puts them in for each of its calls.

use super::log;
use crate::addons::{self as a, AreaState, LIST_LEN, MAX_ROWS};
use std::collections::VecDeque;
use std::sync::OnceLock;

/// The add-ons' folder's name (in KH1's folder, and in the folder this DLL is in).
pub const FOLDER: &str = "kh1-kh2-addons";
/// Our first add-on row's member id (the KH2 row's is `kh2list::MEMBER`, 10).
pub const FIRST_MEMBER: u8 = super::kh2list::MEMBER + 1;
/// The last save byte an add-on row's list reaches (member 13's, `+0x628..+0x657`).
const SAVE_END: usize = list_off(FIRST_MEMBER + MAX_ROWS as u8 - 1) + LIST_LEN;

/// Where KH1's page reads member `m`'s list (`save + 4 + m * 0x74 + 0x40`, `0x140311850` /
/// `0x140311C20` / `0x140310650`, Checked (decomp, Steam)).
pub const fn list_off(m: u8) -> usize {
    4 + m as usize * 0x74 + 0x40
}

/// Reads every add-on (`<KH1's folder>\kh1-kh2-addons\<name>\addon.toml`, then the same folder
/// beside this DLL when Panacea loaded it from its own dll folder; each folder in name order) and
/// sets them for the game and the API. At start (`DllMain`), before the menus go in.
pub fn load() {
    let mut roots = vec![format!("{}\\{FOLDER}", super::exe_dir())];
    let beside = format!("{}\\{FOLDER}", super::dll_dir());
    if !roots.iter().any(|r| r.eq_ignore_ascii_case(&beside)) {
        roots.push(beside);
    }
    let mut found: Vec<(String, String)> = Vec::new();
    for root in &roots {
        let Ok(dir) = std::fs::read_dir(root) else { continue };
        let mut dirs: Vec<std::path::PathBuf> = dir.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_dir()).collect();
        dirs.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_ascii_lowercase()));
        for d in dirs {
            let file = d.join("addon.toml");
            match std::fs::read_to_string(&file) {
                Ok(t) => found.push((d.to_string_lossy().into_owned(), t)),
                Err(_) if !file.exists() => {}
                Err(e) => log!(Menus, "add-ons: {} couldn't be read ({e})", file.display()),
            }
        }
    }
    a::set_log_sink(|s| log!(General, "{s}"));
    if found.is_empty() {
        log!(Menus, "add-ons: none ({} looked in)", roots.join(", "));
        a::set_load_done();
        return;
    }
    let read = |folder: &str, rel: &str| std::fs::read(std::path::Path::new(folder).join(rel.replace('/', "\\"))).ok();
    let (l, lines) = a::load(&found, &read);
    for line in lines {
        log!(Menus, "add-ons: {line}");
    }
    let finishers = l.addons.iter().flat_map(|x| x.abilities.iter()).filter(|x| x.finisher.is_some()).count();
    if finishers > 0 {
        // (Not built yet: an add-on finisher needs a KH1 attack row, request and motion slot made
        // at load from its data, and `kh1rows::OURS` is a fixed table: `docs/addons.md` section 7.)
        log!(Menus, "add-ons: {finishers} finisher(s) read and checked; they don't play yet in this build (abilities only)");
    }
    a::set_loaded(l);
    a::set_load_done();
}

/// The add-ons' area in KH1's config block, if the block is there.
pub unsafe fn area(base: usize) -> Option<&'static mut [u8]> {
    let list = super::kh2list::list_ptr(base)?;
    let config = list - crate::growth::KH2_LIST_OFF;
    Some(std::slice::from_raw_parts_mut((config + a::SAVE_OFF) as *mut u8, a::AREA_LEN))
}

/// Add-on rows the Abilities page shows now: every loaded add-on, unless the save's area holds
/// someone else's bytes (then none, logged once) or the save isn't readable that far.
pub unsafe fn rows_now(base: usize) -> usize {
    let Some(l) = a::loaded() else { return 0 };
    let Some(save) = super::menu::save_block(base) else { return 0 };
    if !super::crash::readable(save + SAVE_END) {
        return 0;
    }
    match area(base).map(|x| a::area_state(x)) {
        Some(AreaState::Ours) | Some(AreaState::Empty) => l.addons.len().min(MAX_ROWS),
        _ => {
            static SAID: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            if !SAID.swap(true, std::sync::atomic::Ordering::Relaxed) {
                log!(Menus, "add-ons: this save's config block +{:#x}..+{:#x} holds bytes that aren't ours: left alone, no add-on rows on it", a::SAVE_OFF, a::SAVE_OFF + a::AREA_LEN);
            }
            0
        }
    }
}

/// Puts the first `k` add-on rows' lists (runtime ids) where KH1's page reads members 11.. (for one
/// page call, `kh2list::enter`); the bytes that were there are returned for `leave_rows`.
pub unsafe fn enter_rows(base: usize, save: usize, k: usize) -> [[u8; LIST_LEN]; MAX_ROWS] {
    let mut old = [[0u8; LIST_LEN]; MAX_ROWS];
    let (Some(l), Some(ar)) = (a::loaded(), area(base)) else { return old };
    for (r, addon) in l.addons.iter().enumerate().take(k) {
        let at = save + list_off(FIRST_MEMBER + r as u8);
        for (i, b) in old[r].iter_mut().enumerate() {
            *b = std::ptr::read((at + i) as *const u8);
        }
        let shown = a::slot_for(ar, addon.hash).map_or([0u8; LIST_LEN], |s| addon.to_global(a::slot_list(ar, s)).0);
        for (i, b) in shown.iter().enumerate() {
            std::ptr::write((at + i) as *mut u8, *b);
        }
    }
    old
}

/// The page's lists back into the save's area (local ids, the bytes of abilities an add-on no
/// longer has kept after them), and the save's own bytes back where KH1's page read them.
pub unsafe fn leave_rows(base: usize, save: usize, k: usize, old: &[[u8; LIST_LEN]; MAX_ROWS]) {
    let (Some(l), Some(ar)) = (a::loaded(), area(base)) else { return };
    for (r, addon) in l.addons.iter().enumerate().take(k) {
        let at = save + list_off(FIRST_MEMBER + r as u8);
        let page: Vec<u8> = (0..LIST_LEN).map(|i| std::ptr::read((at + i) as *const u8)).collect();
        if let Some(s) = a::slot_for(ar, addon.hash) {
            let hidden = addon.to_global(a::slot_list(ar, s)).1;
            let back = addon.to_local(&page, &hidden);
            a::set_slot_list(ar, s, &back);
        }
        for (i, b) in old[r].iter().enumerate() {
            std::ptr::write((at + i) as *mut u8, *b);
        }
    }
}

/// AP of the add-ons' equipped abilities: from the page's view while one of its calls has it in
/// (`in_view`: its rows, `save`), else from the save's area.
pub unsafe fn ap_equipped(base: usize, in_view: Option<(usize, usize)>) -> i32 {
    let Some(l) = a::loaded() else { return 0 };
    match in_view {
        Some((save, k)) => (0..k.min(l.addons.len()))
            .map(|r| {
                let at = save + list_off(FIRST_MEMBER + r as u8);
                (0..LIST_LEN)
                    .map(|i| std::ptr::read((at + i) as *const u8))
                    .take_while(|b| *b != 0)
                    .filter(|b| b & 0x80 == 0)
                    .filter_map(|b| l.addons[r].by_global(b))
                    .map(|x| x.ap as i32)
                    .sum::<i32>()
            })
            .sum(),
        None => {
            let Some(ar) = area(base) else { return 0 };
            l.addons.iter().filter_map(|x| a::find_slot(ar, x.hash).map(|s| x.ap_on(a::slot_list(ar, s)))).sum()
        }
    }
}

/// Add-on row `r`'s title in KH1's text codes (its member name, `kh2list::name`).
pub fn title_text(r: usize) -> Option<usize> {
    static TEXT: OnceLock<Vec<usize>> = OnceLock::new();
    let t = TEXT.get_or_init(|| a::loaded().map_or(Vec::new(), |l| l.addons.iter().map(|x| Box::leak(super::menu::encode(&x.title).into_boxed_slice()).as_ptr() as usize).collect()));
    t.get(r).copied()
}

/// Each add-on ability's entry in our copy of KH1's ability table (`menu::ensure_table`): its
/// template's (`Icon::template`: kind and menu place), its AP, its own name and help.
pub unsafe fn table_entries(base: usize, cur: usize, ours: usize) {
    let Some(l) = a::loaded() else { return };
    static TEXT: OnceLock<Vec<(u32, u32)>> = OnceLock::new();
    let to_handle: unsafe extern "system" fn(usize) -> u32 = std::mem::transmute(base + super::menu::TO_HANDLE_RVA);
    let text = TEXT.get_or_init(|| {
        l.addons
            .iter()
            .flat_map(|x| x.abilities.iter())
            .map(|ab| {
                let name = Box::leak(super::menu::encode(&ab.name).into_boxed_slice()).as_ptr() as usize;
                let help = Box::leak(super::menu::encode(&ab.help).into_boxed_slice()).as_ptr() as usize;
                (to_handle(name), to_handle(help))
            })
            .collect()
    });
    for (k, ab) in l.addons.iter().flat_map(|x| x.abilities.iter()).enumerate() {
        if ab.global < a::FIRST_GLOBAL {
            continue;
        }
        let e = ours + (ab.global as usize - 1) * 12;
        let t = cur + (ab.icon.template() as usize - 1) * 12;
        std::ptr::copy_nonoverlapping(t as *const u8, e as *mut u8, 12);
        std::ptr::write(e as *mut u8, ab.ap);
        std::ptr::write_unaligned((e + 2) as *mut u16, std::ptr::read_unaligned((t + 2) as *const u16).wrapping_add(1));
        std::ptr::write_unaligned((e + 4) as *mut u32, text[k].0);
        std::ptr::write_unaligned((e + 8) as *mut u32, text[k].1);
    }
}

/// The menu's once-a-second step (`menu::Growth::update`, `abilities = kh1` with the KH2 list in
/// use): each add-on's abilities due by Sora's `level` and the story `progress` put in its row,
/// off until equipped, with KH1's "New Ability" box as the KH2 ones; then the API's copy of the
/// lists (`addons::set_snapshot`).
pub unsafe fn grant(base: usize, level: u8, progress: &[u8], prompts: &mut VecDeque<(String, String, String)>) {
    let Some(l) = a::loaded() else { return };
    if rows_now(base) == 0 {
        a::set_snapshot(None);
        return;
    }
    let Some(ar) = area(base) else { return };
    let mut snap = Vec::new();
    for addon in &l.addons {
        let Some(s) = a::slot_for(ar, addon.hash) else {
            static SAID: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            if !SAID.swap(true, std::sync::atomic::Ordering::Relaxed) {
                log!(Menus, "add-ons: {}: every one of the save's {} add-on slots is another add-on's (removed ones keep theirs): no row on this save", addon.key, a::SLOTS);
            }
            continue;
        };
        let mut list = [0u8; LIST_LEN];
        list.copy_from_slice(a::slot_list(ar, s));
        let mut learned: Vec<&str> = Vec::new();
        for id in addon.due(&list, level, progress) {
            let Some(ab) = addon.by_local(id) else { continue };
            if a::push(&mut list, id | 0x80) {
                log!(Menus, "add-ons: Sora is given {} ({}'s {}, id {id}, runtime id {}, {} AP, off, {})", ab.name, addon.key, ab.key, ab.global, ab.ap, ab.learned.describe());
                learned.push(&ab.name);
            } else {
                log!(Menus, "add-ons: no room for {} in {}'s row ({LIST_LEN})", ab.name, addon.key);
            }
        }
        if !learned.is_empty() {
            a::set_slot_list(ar, s, &list);
        }
        match learned.len() {
            0 => {}
            1 | 2 => prompts.extend(learned.iter().map(|n| ("New Ability".to_string(), n.to_string(), String::new()))),
            n => prompts.push_back(("New Abilities".to_string(), format!("{n} {} Abilities", addon.title), String::new())),
        }
        snap.push((addon.hash, list));
    }
    a::set_snapshot(Some(snap));
}
