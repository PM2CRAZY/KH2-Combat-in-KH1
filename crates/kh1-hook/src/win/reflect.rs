//! KH2's Reflect in KH1's Magic menu (the user: "lets add reflect into the magic menu, i want the
//! sound and VFX"). `docs/kh1-reflect-spell.md` has KH1's side and what is ours.
//!
//! - **Menu.** KH1's command records are 16 bytes each, 256 of them (`0x14028D7D0 (id)` =
//!   `*0x142D37750 + id * 16`, filled from `btltbl.bin`); 0x7D.. are empty. Command 0x7D is
//!   written as a spell of its own (type 2, spell byte 7, no target submenu) each time the battle
//!   menus are filled (`0x140292330`), and put into the magic list (`0x140529638`: `+4` / `+8`
//!   disabled bits, `+0x10` count, `+0x14` command ids) after KH1's spells, before Summon, once
//!   Sora has Aero (`learned`). Greyed as KH1 greys a spell (magic sealed, MP short by KH1's own
//!   check `0x1402A2A80` with a cost of 1 MP, `COST`). Its name: KH1 draws a command's name from
//!   its record's word index (`+4`) through `0x14028D820`, called once in its name drawer
//!   `0x1402787B0`; that call is pointed at `word`, which answers our index with the level's
//!   name (Reflect / Reflera / Reflega).
//! - **Levels.** Reflect's level is KH1's Aero level (`level`): Aero / Aerora / Aeroga give
//!   Reflect / Reflera / Reflega, each from its own KH2 files (`magic/REFLECT_<n>.mag`: script
//!   and effect package; MAGC row of that level; ATKP and `magi` at sub `level × 2 − 2`).
//! - **Shortcuts.** `shortcuts` puts it in KH1's shortcuts (Customize and battle); a shortcut
//!   gives KH1's command step the same command as the menu, so it comes here the same way.
//! - **Rows.** KH1's magic list draws its rows with one of 7 row panels each (layout
//!   `0x142859E60`: `+0x10` count, `+0x28` the panels, 0xC0 bytes each, built from one template
//!   by `0x140278A00`); 7 spells + Reflect + Summon need 8. The layout is pointed at our copy of
//!   them with an eighth (whenever KH1 builds it again).
//! - **Cast.** Choosing it gives KH1's command step `0x1402A5380` command type 2, spell 7, which
//!   KH1 has no spell for: `combat::command` hands it here (`chosen`) and KH1 never runs it. KH2
//!   (`takeover`) casts it as soon as Sora can cast (`before_step`): KH2's cast motion from the
//!   MAGC row (every level: ground 71, combo finisher 72, air 73), KH1's 1 MP paid
//!   (`0x1402A39D0`), and the level's own script (`magic/REFLECT_<n>.mag`, run by `kh2-magic` as
//!   the playground does) from trigger 20, released at the motion's magic-cast event (frame 0).
//! - **Barrier.** While the script's reflect-kind attack (ATKP 573) lives, enemy hits on Sora are
//!   stopped (`hurt`) and counted (`attack_get_reflect_count`); the script then fires its counter
//!   (ATKP 574, then the 667 burst), landed on KH1's enemies as KH2's swings are
//!   (`combat::Strikes::spell_strike`).
//! - **Effects and sounds.** Each effect the script starts plays from the spell file's own
//!   package (`kh2fx::spell_start`, bound to Sora as his own effects are) with its element's sound
//!   (`kh2sound::spell_element`).

use super::addr::Rva;
use super::{combat, hook, log, pose, takeover};
use character_core::{CastMotions, Character, EventKind, Output, StateName};
use glam::{Mat4, Quat, Vec3};
use kh2_formats::battle::{Atkp, Battle};
use kh2_magic::{Event, Magic, ObjId, SpellFile, World};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

/// Our command record (an empty one of KH1's 256) and its spell byte (KH1's are 0..6).
pub const COMMAND: u8 = 0x7D;
pub const SPELL: u8 = 7;
/// The record's word (name) index: answered by `word`, never looked up in KH1's table.
const WORD: u16 = 0x1FF;
/// Reflect's MAGC element.
const ELEMENT: u8 = 5;
/// KH2's levels: Reflect, Reflera, Reflega.
const LEVELS: u8 = 3;
const NAMES: [&str; 3] = ["Reflect", "Reflera", "Reflega"];
/// The Customize spell list's help line for it (a text id of ours, `config`'s text lookup).
pub const HELP_ID: u32 = 0x7F00_0006;
const HELP: &str = "Surround yourself with a barrier that blocks attacks,\nthen strikes back at every foe nearby.";

const RECORD_RVA: Rva = Rva(0x28D7D0);
const WORD_RVA: Rva = Rva(0x28D820);
const NAME_CALL_RVA: Rva = Rva(0x2787FB);
const MAGIC_MENU_RVA: Rva = Rva(0x529638);
const MAGIC_LAYOUT_RVA: Rva = Rva(0x2859E60);
/// KH1's spells: 0x38 bytes each, `+0` the level loaded (0: none; `0x14028D0D0 (spell)`),
/// `+0x18` the spell's data (`+0x30` its MP cost).
const SPELLS_RVA: Rva = Rva(0x2D23580);
const MP_CHECK_RVA: Rva = Rva(0x2A2A80);
const MP_PAY_RVA: Rva = Rva(0x2A39D0);
const SAVE_PTR_RVA: Rva = Rva(0x28695A0);
/// Bit 4: no magic now (`0x14028CE40`'s first test).
const MAGIC_SEAL_RVA: Rva = Rva(0x2D54438);
const SOUND_RVA: Rva = Rva(0x177F60);
/// Reflect's cost at every level: what KH1 calls 1 MP. KH1's MP is whole points (Sora's stats
/// `+0x44`, of the maximum `+0x48`: what the HUD's MP bar shows); a spell cost of 100 or more is in
/// hundredths of a point: `0x1402A2A80` checks `cost / 100 <= MP`, `0x1402A39D0` takes
/// `cost / 100` points (`0x1402A2870 (actor, -n)`). (Costs under 100 are charge-gauge units,
/// `+0xBE`, 30 a point.)
const COST: i32 = 100;
const ROW_BYTES: usize = 0xC0;
const ROWS: usize = 8;
/// How long a chosen Reflect waits for KH2's move to allow a cast (as KH1's queued commands).
const WAIT_S: f32 = 5.0;

/// Sora in the spell scripts' world; enemies are `FOE0 + i`.
const SORA: ObjId = 1;
const FOE0: ObjId = 100;
/// Spell attacks' keys in the hit records (apart from KH2's swing ids).
const SPELL_KEY: u64 = 1 << 60;

static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG_FILL: AtomicUsize = AtomicUsize::new(0);
/// Each level's name (KH1 text codes) and the help line.
static NAME: OnceLock<[usize; 3]> = OnceLock::new();
static HELP_TEXT: OnceLock<usize> = OnceLock::new();
static ROWS_BUF: OnceLock<usize> = OnceLock::new();

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

fn base() -> usize {
    BASE.load(Ordering::Relaxed)
}

fn note(s: String) {
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 300 {
        log(&s);
    }
}

/// When Sora has Reflect: once he has KH1's Aero (the user's rule; one place to change it).
/// `save` is KH1's save block (`*0x1428695A0`); party member 0's spells are the bits of
/// `+0x74` (`0x1402830C0 (member, spell)`), Aero spell 6.
pub unsafe fn learned(save: usize) -> bool {
    save > 0x10000 && rd::<u8>(save + 0x74) & (1 << 6) != 0
}

/// Reflect's level, 1..3: KH1's Aero level (the user: Aero / Aerora / Aeroga give Reflect /
/// Reflera / Reflega; one place to change it). The level KH1 has loaded for Aero (its spell
/// table `0x142D23580 + 6 * 0x38`, `+0`: what KH1's own menus name and cast, Aero / Aerora /
/// Aeroga), else the save's (`save + 0x492 + spell`, which `0x14028D3D0` keeps with it), else 1.
pub unsafe fn level(save: usize) -> u8 {
    let loaded = rd::<i32>(base() + SPELLS_RVA + 6 * 0x38);
    if (1..=LEVELS as i32).contains(&loaded) {
        return loaded as u8;
    }
    let saved = if save > 0x10000 { rd::<u8>(save + 0x492 + 6) } else { 0 };
    if (1..=LEVELS).contains(&saved) {
        saved
    } else {
        1
    }
}

/// The level cast now: Aero's, or the highest level below it whose files loaded.
unsafe fn cast_level(save: usize) -> Option<u8> {
    (1..=level(save)).rev().find(|&l| parsed(l).is_some())
}

/// The name for the level KH1 shows now (KH1 text), for our command and the Customize lists.
pub unsafe fn name_now() -> Option<usize> {
    let save = rd::<u64>(base() + SAVE_PTR_RVA) as usize;
    let n = NAME.get()?;
    (super::settings::get().reflect && learned(save)).then(|| n[(level(save) - 1) as usize])
}

/// Our text ids (`config`'s menu text lookup): the Customize help line.
pub fn text(id: u32) -> Option<usize> {
    (id == HELP_ID).then(|| HELP_TEXT.get().copied()).flatten()
}

/// It can be offered in KH1's spell lists now (Customize's): on, named, learned, its files in.
pub unsafe fn offered() -> bool {
    let save = rd::<u64>(base() + SAVE_PTR_RVA) as usize;
    base() != 0 && super::settings::get().reflect && NAME.get().is_some() && learned(save) && !PARSED.get().is_some_and(|p| p.iter().all(|s| s.is_none()))
}

// ---------------------------------------------------------------- the spell's data

/// A level's MAGC row (`00battle`): its file, cast motions and voices.
fn magc(level: u8) -> Option<&'static kh2_formats::battle::Magc> {
    pose::kh2()?.battle.magc.iter().find(|m| m.id == ELEMENT && m.level == level && m.world == 0)
}

/// The cast motions (MAGC; 71 / 72 / 73 if the row is missing).
fn motions(level: u8) -> CastMotions {
    let m = |v: i16, d: u16| if v >= 0 { v as u16 } else { d };
    match magc(level) {
        Some(r) => CastMotions { ground: m(r.ground_motion, 71), air: m(r.air_motion, 73), finisher: (r.finish_motion >= 0).then_some(r.finish_motion as u16) },
        None => CastMotions { ground: 71, air: 73, finisher: Some(72) },
    }
}

/// A level's spell file from the user's KH2 install (read once, the first caller waits; after
/// KH2 Sora has loaded).
pub fn spell_file(level: u8) -> Option<Arc<Vec<u8>>> {
    static FILES: [OnceLock<Option<Arc<Vec<u8>>>>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let slot = FILES.get((level as usize).wrapping_sub(1))?;
    slot.get_or_init(|| {
        let name = magc(level).map_or(format!("magic/REFLECT_{level}.mag"), |m| m.filename.clone());
        let exe = format!("{}\\KINGDOM HEARTS II FINAL MIX.exe", super::exe_dir());
        let r = (|| -> Result<Vec<u8>, String> {
            let install = kh2_data::Install::locate(&exe).map_err(|e| e.to_string())?;
            let src = if install.has_backup() { kh2_data::Source::Backup } else { kh2_data::Source::Live };
            install.read_any(src, &name).map_err(|e| e.to_string())
        })();
        match r {
            Ok(b) => {
                log(&format!("Reflect: {name} read ({} bytes)", b.len()));
                Some(Arc::new(b))
            }
            Err(e) => {
                log(&format!("Reflect: {name} not read ({e}): no Reflect at level {level}"));
                None
            }
        }
    })
    .clone()
}

/// Each level's script and effect package, parsed (set by the loader thread; `None` for a level
/// whose file is missing or unusable).
static PARSED: OnceLock<Vec<Option<SpellFile>>> = OnceLock::new();

fn parsed(level: u8) -> Option<&'static SpellFile> {
    PARSED.get()?.get((level as usize).wrapping_sub(1))?.as_ref()
}

/// Every level's effect elements and their sounds (`kh2sound`): (level, element, sound).
pub fn element_sounds() -> Vec<(u8, u16, i32)> {
    let mut out = Vec::new();
    for level in 1..=LEVELS {
        let Some(b) = spell_file(level) else { continue };
        let Some(pax) = SpellFile::bar_entry(&b, 18).and_then(|p| kh2_formats::pax::Pax::parse(&p).ok()) else { continue };
        out.extend(pax.elements.iter().map(|e| (level, e.id, e.sound)));
    }
    out
}

/// Its cast voices (MAGC: the cast and the finisher cast, every level; `kh2sound`, with
/// `kh2_voice`).
pub fn cast_voices(kh2: &pose::Kh2) -> Vec<i16> {
    let mut v: Vec<i16> = kh2.battle.magc.iter().filter(|m| m.id == ELEMENT && m.world == 0).flat_map(|m| [m.voice, m.voice_finisher]).filter(|v| *v != 255).map(|v| v as i16).collect();
    v.sort_unstable();
    v.dedup();
    v
}

// ---------------------------------------------------------------- KH1's MP

/// Fire's own cost field (KH1's loaded spell data `+0x18 → +0x30`), for the log only.
unsafe fn fire_cost(base: usize) -> Option<i32> {
    let data = rd::<u64>(base + SPELLS_RVA + 0x18) as usize;
    (data > 0x10000 && super::crash::readable(data + 0x34)).then(|| rd::<i32>(data + 0x30))
}

/// Magic is allowed now and Sora has 1 MP, as KH1 judges its own spells (`0x14028CE40`).
unsafe fn affordable(base: usize, sora: usize) -> bool {
    let save = rd::<u64>(base + SAVE_PTR_RVA) as usize;
    if sora < 0x10000 || save < 0x10000 || rd::<u8>(base + MAGIC_SEAL_RVA) & 4 != 0 || rd::<u8>(save + 0x874) & 1 != 0 {
        return false;
    }
    let check: unsafe extern "system" fn(usize, i32) -> u32 = std::mem::transmute(base + MP_CHECK_RVA);
    check(sora, COST) & 0xFF != 0
}

/// Sora's MP now (stats `+0x44` of `+0x48`), for the log.
unsafe fn mp(sora: usize) -> (i32, i32) {
    let stats = pose::ptr(rd::<u32>(sora + 0x6C));
    if stats < 0x10000 {
        return (0, 0);
    }
    (rd::<i32>(stats + 0x44), rd::<i32>(stats + 0x48))
}

// ---------------------------------------------------------------- the menu

/// Our name for the command's word index; KH1's word otherwise (`0x14028D820`).
unsafe extern "system" fn word(index: i32) -> usize {
    if index == WORD as i32 {
        if let Some(n) = NAME.get() {
            let save = rd::<u64>(base() + SAVE_PTR_RVA) as usize;
            return n[(level(save) - 1) as usize];
        }
    }
    let f: unsafe extern "system" fn(i32) -> usize = std::mem::transmute(base() + WORD_RVA);
    f(index)
}

/// Our record (a self-cast spell, like Cure's and Aero's flags, without their target menu).
const RECORD: [u8; 16] = [0x02, 0x00, 0xFF, 0x00, WORD as u8, (WORD >> 8) as u8, 0x02, SPELL, 0x00, 0xFF, 0x01, 0x02, 0, 0, 0, 0];

unsafe fn ensure_record(base: usize) -> bool {
    let record: unsafe extern "system" fn(u32) -> usize = std::mem::transmute(base + RECORD_RVA);
    let r = record(COMMAND as u32);
    if r < 0x10000 {
        return false;
    }
    let have = rd::<[u8; 16]>(r);
    if have != RECORD {
        // Only over an empty one (KH1's 0x7D.. are all zero) or ours from before.
        if have != [0u8; 16] && have[4..8] != RECORD[4..8] {
            static ONCE: AtomicBool = AtomicBool::new(false);
            if !ONCE.swap(true, Ordering::Relaxed) {
                log(&format!("Reflect: command record {COMMAND:#x} is in use ({have:02x?}): no Reflect"));
            }
            return false;
        }
        wr(r, RECORD);
        static N: AtomicU32 = AtomicU32::new(0);
        if N.fetch_add(1, Ordering::Relaxed) < 5 {
            log(&format!("Reflect: command record {COMMAND:#x} written at {r:#x}"));
        }
    }
    true
}

/// The magic list's row panels, eight of them (KH1 builds seven): our copy, with the seventh
/// repeated, whenever the layout points elsewhere (KH1 built it again).
unsafe fn eight_rows(base: usize) -> bool {
    let layout = base + MAGIC_LAYOUT_RVA;
    let buf = *ROWS_BUF.get_or_init(|| Box::leak(vec![0u8; ROW_BYTES * ROWS + 16].into_boxed_slice()).as_ptr() as usize);
    let buf = (buf + 15) & !15;
    let rows = rd::<u64>(layout + 0x28) as usize;
    let n = rd::<i32>(layout + 0x10);
    if rows == buf {
        if n != ROWS as i32 {
            wr(layout + 0x10, ROWS as i32);
        }
        return true;
    }
    if rows < 0x10000 || n != 7 {
        static ONCE: AtomicBool = AtomicBool::new(false);
        if !ONCE.swap(true, Ordering::Relaxed) {
            log(&format!("Reflect: magic list layout not as expected (rows {rows:#x}, {n}): Reflect only while the list has room"));
        }
        return false;
    }
    std::ptr::copy_nonoverlapping(rows as *const u8, buf as *mut u8, ROW_BYTES * 7);
    std::ptr::copy_nonoverlapping((rows + ROW_BYTES * 6) as *const u8, (buf + ROW_BYTES * 7) as *mut u8, ROW_BYTES);
    wr(layout + 0x28, buf as u64);
    wr(layout + 0x10, ROWS as i32);
    static N: AtomicU32 = AtomicU32::new(0);
    if N.fetch_add(1, Ordering::Relaxed) < 5 {
        log(&format!("Reflect: magic list rows: 8 (KH1's 7 at {rows:#x} copied to {buf:#x})"));
    }
    true
}

/// Reflect can be in KH1's battle menus now (on, named, learned, a level's files in, our command
/// record in place). Logs its level when it changes.
unsafe fn in_menus(base: usize) -> bool {
    if base == 0 || !super::settings::get().reflect || NAME.get().is_none() {
        return false;
    }
    let save = rd::<u64>(base + SAVE_PTR_RVA) as usize;
    if !learned(save) || PARSED.get().is_some_and(|p| p.iter().all(|s| s.is_none())) || !ensure_record(base) {
        return false;
    }
    static LEVEL_LOGGED: AtomicU32 = AtomicU32::new(0);
    let l = level(save);
    if LEVEL_LOGGED.swap(l as u32, Ordering::Relaxed) != l as u32 {
        log(&format!(
            "Reflect: level {l}, {} (KH1's Aero level: loaded {}, saved {}){}",
            NAMES[(l - 1) as usize],
            rd::<i32>(base + SPELLS_RVA + 6 * 0x38),
            rd::<u8>(save + 0x492 + 6),
            match (PARSED.get().is_some(), cast_level(save)) {
                (false, _) => String::new(),
                (true, Some(c)) if c != l => format!("; its files didn't load, level {c} is cast"),
                (true, Some(_)) => String::new(),
                (true, None) => "; no level's files loaded".to_string(),
            }
        ));
    }
    true
}

/// Usable now, as KH1's own spells are (its fill's flag 0x20: magic usable in this menu at all;
/// magic not sealed; 1 MP), and only while KH2 has Sora (KH1 can't cast it).
unsafe fn usable(base: usize, actor: usize, flags: usize) -> bool {
    let save = rd::<u64>(base + SAVE_PTR_RVA) as usize;
    let allowed = flags > 0x10000 && rd::<u8>(flags) & 0x20 != 0;
    allowed && affordable(base, actor) && takeover::kh2_has() && cast_level(save).is_some()
}

/// Why it's greyed (for the log).
unsafe fn greyed_why(actor: usize, flags: usize) -> String {
    let allowed = flags > 0x10000 && rd::<u8>(flags) & 0x20 != 0;
    format!("magic allowed {allowed}, MP {:?} of cost 1 ({COST}), KH2 has Sora {}, files loaded {}", mp(actor), takeover::kh2_has(), PARSED.get().is_some())
}

/// For a shortcut holding Reflect, after KH1's fill (`shortcuts`): `None` when it can't be offered
/// (the slot shows empty), else whether it's usable now.
pub unsafe fn shortcut_state(actor: usize, flags: usize) -> Option<bool> {
    let base = base();
    in_menus(base).then(|| usable(base, actor, flags))
}

/// After KH1 fills the battle menus (`0x140292330 (actor, flags)`): Reflect into the magic list.
pub unsafe fn magic_menu(actor: usize, flags: usize) {
    let base = base();
    if base == 0 {
        return;
    }
    let menu = base + MAGIC_MENU_RVA;
    let n = rd::<i32>(menu + 0x10);
    // (KH1 shows no list here: no spells, or magic off in this mode.)
    if !(1..=15).contains(&n) || !in_menus(base) {
        return;
    }
    let entries = menu + 0x14;
    let list: Vec<u8> = (0..n as usize).map(|k| rd::<u8>(entries + k)).collect();
    if list.contains(&COMMAND) {
        return;
    }
    let summon = matches!(list.last(), Some(6 | 0x4F));
    let at = if summon { n as usize - 1 } else { n as usize };
    // Rows: the list draws `count - 1` of its row panels (the last row has its own): 7 spells,
    // Reflect and Summon need an eighth.
    if n as usize >= 8 && !eight_rows(base) {
        return;
    }
    let usable = usable(base, actor, flags);
    for k in (at..n as usize).rev() {
        wr(entries + k + 1, rd::<u8>(entries + k));
    }
    wr(entries + at, COMMAND);
    for off in [4usize, 8] {
        let v = rd::<u32>(menu + off);
        let low = v & ((1u32 << at) - 1);
        let high = (v >> at) << (at + 1);
        wr(menu + off, low | high | ((!usable) as u32) << at);
    }
    wr(menu + 0x10, n + 1);
    static LOGGED: AtomicU32 = AtomicU32::new(u32::MAX);
    let key = (n as u32) << 8 | (at as u32) << 1 | usable as u32;
    if LOGGED.swap(key, Ordering::Relaxed) != key {
        log(&format!("Reflect in the Magic menu: entry {} of {} ({}){}", at + 1, n + 1, if usable { "usable" } else { "greyed" }, if usable { String::new() } else { format!(" ({})", greyed_why(actor, flags)) }));
    }
}

/// KH1's battle-menu fill when `shortcuts` hasn't hooked it.
unsafe extern "system" fn fill(a: usize, b: usize) {
    let f: unsafe extern "system" fn(usize, usize) = std::mem::transmute(ORIG_FILL.load(Ordering::Acquire));
    f(a, b);
    magic_menu(a, b);
}

pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    if !super::settings::get().reflect {
        log("Reflect: off (reflect = off)");
        return;
    }
    if !hook::redirect_call(base + NAME_CALL_RVA, base + WORD_RVA, word as *const () as usize, "command name word") {
        log("Reflect: the command name call wasn't as expected: no Reflect in the menu");
        return;
    }
    if !super::shortcuts::fill_hooked() && hook::install(base + super::shortcuts::FILL_RVA, &super::shortcuts::FILL_PROLOGUE, fill as *const () as usize, &ORIG_FILL, "battle menus fill (Reflect)").is_none() {
        return;
    }
    let _ = HELP_TEXT.set(Box::leak(super::menu::encode(HELP).into_boxed_slice()).as_ptr() as usize);
    let _ = NAME.set(NAMES.map(|n| Box::leak(super::menu::encode(n).into_boxed_slice()).as_ptr() as usize));
    log(&format!("Reflect: menu entry ready (command {COMMAND:#x}, learned with Aero, its level Aero's, 1 MP = KH1 cost {COST})"));
    // The spell files, once KH2 Sora is there (their MAGC rows name them).
    std::thread::spawn(|| {
        let t = Instant::now();
        while pose::kh2().is_none() {
            if t.elapsed().as_secs() > 300 {
                log("Reflect: KH2 Sora never loaded");
                let _ = PARSED.set(Vec::new());
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        let mut all = Vec::new();
        for level in 1..=LEVELS {
            let name = format!("REFLECT_{level}.mag");
            let parsed = spell_file(level).and_then(|b| match SpellFile::parse(&name, &b) {
                Ok(f) => Some(f),
                Err(e) => {
                    log(&format!("Reflect: {name} not usable ({e})"));
                    None
                }
            });
            if let Some(f) = &parsed {
                let m = motions(level);
                let sub = level as i32 * 2 - 2;
                let barrier = pose::kh2().and_then(|k| k.battle.attack_for_level(573, sub as u8, 0)).map(|a| a.kind);
                let counter = pose::kh2().and_then(|k| k.battle.attack_for_level(574, sub as u8, 0)).map(|a| (a.power, a.interval));
                log(&format!(
                    "Reflect level {level} ({}): script and {} effect elements ready; cast motions {} / finisher {:?} / air {}; sub {sub}: barrier ATKP 573 kind {barrier:?}, counter 574 (power, re-hit frames) {counter:?}",
                    NAMES[(level - 1) as usize],
                    f.pax.elements.len(),
                    m.ground,
                    m.finisher,
                    m.air
                ));
            }
            all.push(parsed);
        }
        let _ = PARSED.set(all);
    });
}

// ---------------------------------------------------------------- choosing it

/// A Reflect chosen and not cast yet (when).
static WAITING: Mutex<Option<Instant>> = Mutex::new(None);

unsafe fn refuse(base: usize, why: &str) {
    let sound: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
    sound(4);
    note(format!("Reflect refused: {why}"));
}

/// KH1's command step got our command (`combat::command`: from the Magic menu or a shortcut, the
/// same command): KH1 never runs it.
pub unsafe fn chosen(actor: usize) {
    let base = base();
    if !super::settings::get().reflect {
        return refuse(base, "reflect = off");
    }
    let save = rd::<u64>(base + SAVE_PTR_RVA) as usize;
    if cast_level(save).is_none() {
        return refuse(base, "its spell files aren't loaded");
    }
    if !takeover::kh2_has() {
        return refuse(base, "KH1 has Sora (KH2 casts Reflect)");
    }
    if !affordable(base, actor) {
        return refuse(base, &format!("MP {:?}, cost 1 ({COST})", mp(actor)));
    }
    if let Ok(mut w) = WAITING.lock() {
        *w = Some(Instant::now());
    }
    note(format!(
        "Reflect chosen (level {}, MP {:?}, cost 1; Fire's own cost field {:?}; KH2 can cast now: {:?})",
        level(save),
        mp(actor),
        fire_cost(base),
        takeover::with_kh2(|c, _| c.can_cast())
    ));
}

// ---------------------------------------------------------------- casting it

struct Runtime {
    magic: Magic,
    /// Each level's spell in `magic` (by level − 1), and each spell's level (by spell index).
    spells: [Option<usize>; 3],
    levels: Vec<u8>,
    /// The cast just started (motion, finisher, level): the script starts with this frame's world.
    fresh: Option<(u16, bool, u8)>,
    /// The cast motion waiting for its magic-cast event.
    casting: Option<u16>,
    /// Script effect id → our drawn effect, and whether its end was passed on.
    fx: HashMap<u64, (Option<u32>, bool)>,
    barrier: bool,
    hold: Hold,
    blocks: u32,
    counter_hits: u32,
    started: Option<Instant>,
    /// Hits blocked by a cast a new one took over, for the new barrier to count.
    carry: u32,
}

/// The barrier held while attacks keep hitting it (the user: "can we have reflect keep shielding
/// until attacks have stopped"). The scripts' barrier (ATKP 573) stands 27 frames at every level
/// (`kh2-magic` test `reflect_counters_what_it_blocked`); from its `HOLD_AT`th frame, while the
/// last blocked hit is at most `HOLD_FRAMES` frames old, the cast is held (`Magic::hold`: its
/// script stops where it is, its effects don't age, the barrier attack stays, so `hurt` keeps
/// stopping hits), at most `HOLD_MAX_S`. Let go, the script goes on: the barrier drops a few
/// frames later and the counter fires, its strength from the hits counted as before (test
/// `reflect_barrier_held`). Ours, not KH2's.
const HOLD_AT: u32 = 22;
const HOLD_FRAMES: u32 = 20;
const HOLD_MAX_S: f32 = 5.0;
/// The longest a Reflect's counter waits for the combo (`hold`).
const COMBO_WAIT_MAX_S: f32 = 6.0;

#[derive(Default)]
struct Hold {
    /// Frames the barrier has stood, not counting held ones (the script's own time).
    age: u32,
    /// Frames since the last blocked hit.
    since_hit: Option<u32>,
    /// Held since; the hold used up (the cap reached) for this cast.
    since: Option<Instant>,
    frames: u32,
    spent: bool,
    /// Held for the combo (`hold`): its counter waits for the combo's last Reflect.
    combo: bool,
}

static RT: Mutex<Option<Runtime>> = Mutex::new(None);
/// A Reflect barrier is up (for `hurt`), and the hits it stopped since KH2's last frame.
static BARRIER: AtomicBool = AtomicBool::new(false);
static BLOCKS: AtomicU32 = AtomicU32::new(0);

/// `hurt`: a barrier stands around Sora now.
/// Sora is in one of Reflect's cast motions (any level's ground, air or finisher; `hurt`: he
/// can't be hurt all through it, the user, not only while the barrier stands).
pub fn cast_motion(motion: u16) -> bool {
    (1..=3).any(|l| {
        let m = motions(l);
        motion == m.ground || motion == m.air || Some(motion) == m.finisher
    })
}

pub fn barrier_up() -> bool {
    BARRIER.load(Ordering::Relaxed)
}

/// `hurt`: the barrier stopped an enemy hit (counted for the counter).
pub fn blocked(from: usize) {
    let n = BLOCKS.fetch_add(1, Ordering::Relaxed) + 1;
    note(format!("Reflect blocked a hit from {from:#x} ({n} this frame)"));
}

/// The ATKP row a spell attack uses (its finisher sub falls back to the plain one when the data
/// has none: Reflect's rows are for subs 0 / 2 / 4; ours).
fn row(battle: &Battle, atk: u16, sub: u8, count: u8) -> Option<&Atkp> {
    battle.attack_for_level(atk, sub, count).or_else(|| battle.attack_for_level(atk, sub & !1, count))
}

/// Before KH2's step: a chosen Reflect starts its cast once KH2's move allows one.
pub unsafe fn before_step(actor: usize, ch: &mut Character) {
    let Ok(mut w) = WAITING.try_lock() else { return };
    let Some(t) = *w else { return };
    if t.elapsed().as_secs_f32() > WAIT_S {
        *w = None;
        note(format!("Reflect dropped: KH2's move couldn't take it within {WAIT_S} s"));
        return;
    }
    if !ch.can_cast() {
        return;
    }
    *w = None;
    let base = base();
    if !affordable(base, actor) {
        return refuse(base, &format!("MP {:?} (spent meanwhile)", mp(actor)));
    }
    let save = rd::<u64>(base + SAVE_PTR_RVA) as usize;
    let Some(level) = cast_level(save) else {
        return refuse(base, "its spell files aren't loaded");
    };
    let m = motions(level);
    let Some(motion) = ch.cast(m) else {
        note(format!("Reflect: KH2 couldn't start its cast motion ({m:?})"));
        return;
    };
    // KH1's own payment of a spell's cost: 1 MP at every level.
    let pay: unsafe extern "system" fn(usize, i32) = std::mem::transmute(base + MP_PAY_RVA);
    let before = mp(actor);
    pay(actor, COST);
    let finisher = Some(motion) == m.finisher && m.finisher != Some(m.ground);
    if let Ok(mut g) = RT.lock() {
        if let Some(rt) = runtime(&mut g) {
            rt.fresh = Some((motion, finisher, level));
            rt.started = Some(Instant::now());
        }
    }
    if let Some(r) = magc(level) {
        let v = if finisher { r.voice_finisher } else { r.voice };
        if v != 255 {
            super::kh2sound::voice(v as i16);
        }
    }
    note(format!(
        "{} cast (level {level}): motion {motion}{}, MP {before:?} -> {:?} (its effects loaded: {})",
        NAMES[(level - 1) as usize],
        if finisher { " (combo finisher)" } else if motion == m.air { " (air)" } else { "" },
        mp(actor),
        super::kh2fx::spell_ready(level)
    ));
}

fn runtime(g: &mut Option<Runtime>) -> Option<&mut Runtime> {
    if g.is_none() {
        let mut magic = Magic::new();
        let (mut spells, mut levels) = ([None; 3], Vec::new());
        for (i, f) in PARSED.get()?.iter().enumerate() {
            if let Some(f) = f {
                spells[i] = Some(magic.add_spell(f.clone()));
                levels.push(i as u8 + 1);
            }
        }
        if levels.is_empty() {
            return None;
        }
        *g = Some(Runtime { magic, spells, levels, fresh: None, casting: None, fx: HashMap::new(), barrier: false, hold: Hold::default(), blocks: 0, counter_hits: 0, started: None, carry: 0 });
    }
    g.as_mut()
}

/// The world as the spell script sees it this frame (y up).
struct Obj {
    id: ObjId,
    pos: Vec3,
    facing: f32,
    motion: u16,
    air: bool,
    target: Vec3,
    centre: Vec3,
}

struct KhWorld {
    objs: Vec<Obj>,
    target: Option<ObjId>,
    floor: f32,
    rng: u32,
}

impl KhWorld {
    fn get(&self, o: ObjId) -> Option<&Obj> {
        self.objs.iter().find(|x| x.id == o)
    }
}

/// y up <-> KH1's y down.
fn flip() -> Mat4 {
    Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0))
}

impl World for KhWorld {
    fn exists(&self, o: ObjId) -> bool {
        self.get(o).is_some()
    }
    fn pos(&self, o: ObjId) -> Vec3 {
        self.get(o).map_or(Vec3::ZERO, |x| x.pos)
    }
    fn facing(&self, o: ObjId) -> f32 {
        self.get(o).map_or(0.0, |x| x.facing)
    }
    fn motion(&self, o: ObjId) -> u16 {
        self.get(o).map_or(0, |x| x.motion)
    }
    fn airborne(&self, o: ObjId) -> bool {
        self.get(o).is_some_and(|x| x.air)
    }
    fn jumping(&self, o: ObjId) -> bool {
        self.get(o).is_some_and(|x| x.air)
    }
    fn bone(&self, o: ObjId, code: u16) -> Option<Mat4> {
        let x = self.get(o)?;
        let frame = |at: Vec3| Mat4::from_rotation_translation(Quat::from_rotation_y(x.facing), at);
        Some(match code {
            0x4000 => frame(x.pos),
            0x4001 => frame(x.centre),
            0x4009 => frame(x.target),
            // Sora's model and bones: KH2's pose as drawn (model -> KH1's world), y flipped.
            0x4002 if o == SORA => pose::kh2_pose(|_, w| flip() * *w).unwrap_or_else(|| frame(x.pos)),
            0x4003..=0x4008 if o == SORA => pose::kh2_pose(|p, w| p.get(178).map(|h| flip() * *w * *h)).flatten().unwrap_or_else(|| frame(x.centre)),
            c if c < 0x4000 && o == SORA => pose::kh2_pose(|p, w| p.get(c as usize).map(|b| flip() * *w * *b)).flatten().unwrap_or_else(|| frame(x.centre)),
            _ => frame(x.centre),
        })
    }
    fn target_point(&self, o: ObjId) -> Vec3 {
        self.get(o).map_or(Vec3::ZERO, |x| x.target)
    }
    fn target_of(&self, o: ObjId) -> Option<ObjId> {
        if o == SORA {
            self.target
        } else {
            Some(SORA)
        }
    }
    fn party(&self, n: u32) -> Option<ObjId> {
        (n == 0).then_some(SORA)
    }
    fn has_ability(&self, _: ObjId, _: u32) -> bool {
        false
    }
    fn ground(&self, from: Vec3, depth: f32) -> Option<Vec3> {
        (from.y >= self.floor && from.y - depth <= self.floor).then(|| Vec3::new(from.x, self.floor, from.z))
    }
    fn bg_hit(&self, _: Vec3, _: Vec3) -> bool {
        false
    }
    fn hitback(&mut self, o: ObjId, _: Vec3, dist: f32) {
        note(format!("Reflect: script pushed object {o} by {dist:.0} (not applied)"));
    }
    fn random(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / (1u32 << 24) as f32
    }
}

/// After KH2's step (Sora moved and posed, his swings struck): the spell's frame.
pub unsafe fn after_step(actor: usize, out: &Output, airborne: bool, strikes: &mut combat::Strikes, battle: &Battle, lock: Option<u32>) {
    let Ok(mut g) = RT.lock() else { return };
    let Some(rt) = g.as_mut() else {
        BARRIER.store(false, Ordering::Relaxed);
        return;
    };
    if rt.fresh.is_none() && rt.magic.casts.is_empty() && rt.magic.effects.is_empty() && rt.fx.is_empty() {
        BARRIER.store(false, Ordering::Relaxed);
        BLOCKS.store(0, Ordering::Relaxed);
        return;
    }
    let p = rd::<[f32; 3]>(actor + 0x10);
    let pos = Vec3::new(p[0], -p[1], p[2]);
    // Sora's body centre: KH2's hips (bone 0) as drawn, else 90 up.
    let centre = pose::kh2_pose(|b, w| b.first().map(|h| (flip() * *w).transform_point3(h.w_axis.truncate()))).flatten().unwrap_or(pos + Vec3::Y * 90.0);
    let mut objs = vec![Obj { id: SORA, pos, facing: out.facing, motion: out.motion, air: airborne, target: centre, centre }];
    let targets = strikes.targets();
    for (i, (id, feet, aim)) in targets.iter().enumerate() {
        let _ = id;
        objs.push(Obj { id: FOE0 + i as u32, pos: Vec3::from(*feet), facing: 0.0, motion: 0, air: false, target: Vec3::from(*aim), centre: Vec3::from(*aim) });
    }
    let target = lock.and_then(|l| targets.iter().position(|t| t.0 == l)).map(|i| FOE0 + i as u32);
    let floor = -rd::<f32>(actor + 0x324);
    let mut world = KhWorld { objs, target, floor, rng: 0x1234_5678 ^ (rt.counter_hits.wrapping_mul(2_654_435_761)) };
    // A cast that started this frame (trigger 20), then the release (trigger 21) at the cast
    // motion's magic-cast event (Reflect's: frame 0).
    if let Some((motion, finisher, level)) = rt.fresh.take() {
        // KH2's sub for a spell level: level × 2 − 2, + 1 for the combo finisher's cast.
        let sub = level as i32 * 2 - 2 + finisher as i32;
        let magi = pose::kh2().and_then(|k| k.magi.get(sub).map(|r| r.to_vec())).unwrap_or_else(|| vec![0; kh2_formats::pref::MagiTable::RECORD]);
        let spell = rt.spells[(level - 1) as usize].unwrap_or(0);
        // Cast again while the last one is still in flight (its barrier, or its counter not yet
        // done): that one ends here without its counter, and the hits it blocked carry over to
        // this one, whose counter then fires for them all (the user: "reflect needs to not do
        // the burst damage until the player stops spamming it").
        let mut carry = 0;
        if rt.magic.busy(SORA) {
            carry = rt.magic.attacks.iter().filter(|a| !a.freed && row(battle, a.atk, a.sub, 0).is_some_and(|r| r.kind == 3)).map(|a| a.reflect_count as u32).max().unwrap_or(0).max(rt.carry);
            rt.magic.cancel(SORA);
            for h in rt.fx.values().filter_map(|v| v.0) {
                super::kh2fx::spell_kill(h);
            }
            rt.fx.clear();
            note(format!("Reflect cast again in flight: the last one ends without its counter, {carry} blocked hit(s) carried over"));
        }
        match rt.magic.cast(spell, SORA, ELEMENT, sub, &magi, &mut world) {
            Ok(_) => {
                rt.casting = Some(motion);
                rt.hold = Hold::default();
                rt.blocks = carry;
                rt.carry = carry;
                rt.counter_hits = 0;
            }
            Err(e) => note(format!("Reflect: the script didn't start ({e})")),
        }
    }
    if let Some(m) = rt.casting {
        let fired = out.events.iter().any(|e| matches!(e, EventKind::MagicCast(x) if *x as u16 == out.motion)) && out.motion == m;
        if fired {
            rt.casting = None;
            let ok = rt.magic.release(SORA, &mut world);
            note(format!("Reflect released at frame {} of motion {m} ({})", out.clip_frame, if ok { "barrier" } else { "the script didn't take it" }));
        } else if out.motion != m || out.state != StateName::Magic {
            rt.casting = None;
            note(format!("Reflect's cast motion {m} was cut off before its release (now {:?} motion {})", out.state, out.motion));
        }
    }
    // The hits the barrier stopped since last frame: its reflect count (the script's counter).
    let blocks = BLOCKS.swap(0, Ordering::Relaxed);
    // Hits carried over from a cast this one took over: its barrier counts them once it stands.
    if rt.carry > 0 {
        if let Some(a) = rt.magic.attacks.iter_mut().find(|a| !a.freed && row(battle, a.atk, a.sub, 0).is_some_and(|r| r.kind == 3)) {
            a.reflect_count = a.reflect_count.saturating_add(rt.carry.min(255) as u8);
            note(format!("Reflect: {} carried hit(s) counted on the new barrier (count {})", rt.carry, a.reflect_count));
            rt.carry = 0;
        }
    }
    if blocks > 0 {
        let barrier = rt.magic.attacks.iter_mut().find(|a| !a.freed && row(battle, a.atk, a.sub, 0).is_some_and(|r| r.kind == 3));
        match barrier {
            Some(a) => {
                a.reflect_count = a.reflect_count.saturating_add(blocks.min(255) as u8);
                rt.blocks += blocks;
                note(format!("Reflect: {} hit(s) counted (barrier ATKP {}, count {})", blocks, a.atk, a.reflect_count));
            }
            None => note(format!("Reflect: {blocks} hit(s) stopped with no barrier left to count them")),
        }
    }
    let held = hold(rt, blocks, out.combo_live);
    let combo_wait = rt.hold.combo;
    for e in rt.magic.update(&mut world).into_iter().chain(rt.magic.take_events()) {
        match e {
            Event::Missing(n) => note(format!("Reflect's script used {n} (not modelled)")),
            Event::Error(s) => note(format!("Reflect's script error: {s}")),
            Event::Strike { .. } => {}
        }
    }
    // Effects: each the script starts plays from the spell's package, with its sound; ends,
    // kills and fades follow the script's.
    let mut seen = Vec::new();
    for e in &rt.magic.effects {
        seen.push(e.id);
        match rt.fx.get(&e.id) {
            None => {
                let level = rt.levels.get(e.spell).copied().unwrap_or(1);
                let h = if e.alive() { super::kh2fx::spell_start(level, e.element.id) } else { None };
                if e.alive() {
                    super::kh2sound::spell_element(level, e.element.id);
                }
                note(format!("Reflect effect: level {level} element {} (sound {}){}", e.element.id, e.element.sound, if h.is_some() { "" } else { ", not drawn" }));
                rt.fx.insert(e.id, (h, false));
            }
            Some((Some(h), ended)) => {
                let (h, ended) = (*h, *ended);
                if e.killed {
                    super::kh2fx::spell_kill(h);
                } else if e.fade.is_some() {
                    super::kh2fx::spell_alpha(h, e.alpha());
                    if !e.alive() {
                        super::kh2fx::spell_kill(h);
                    }
                } else if e.ended_at.is_some() && !ended {
                    super::kh2fx::spell_end(h);
                    rt.fx.insert(e.id, (Some(h), true));
                } else if combo_wait && e.alive() {
                    // Waiting for the combo: the barrier is gone from sight (its counter is
                    // all that is left to come).
                    super::kh2fx::spell_kill(h);
                    rt.fx.insert(e.id, (None, false));
                } else if held && e.alive() && !ended && !super::kh2fx::spell_running(h) {
                    // Held: the barrier's drawn effect played out on its own; again, for as long
                    // as the hold lasts (no sound: it is the same barrier).
                    let level = rt.levels.get(e.spell).copied().unwrap_or(1);
                    if let Some(n) = super::kh2fx::spell_start(level, e.element.id) {
                        rt.fx.insert(e.id, (Some(n), false));
                        note(format!("Reflect: barrier effect (element {}) started again for the hold", e.element.id));
                    }
                }
            }
            Some((None, _)) => {}
        }
    }
    rt.fx.retain(|id, (h, _)| seen.contains(id) || h.is_some_and(super::kh2fx::spell_running));
    // The spell's attacks: the barrier (kind 3) stands; the rest hit KH1's enemies.
    let mut attacks = Vec::new();
    let mut barrier = false;
    for a in rt.magic.attacks.iter().filter(|a| !a.freed && a.owner == SORA) {
        let shapes = a.shapes();
        let Some(first) = row(battle, a.atk, a.sub, 0) else { continue };
        if shapes.is_empty() {
            continue;
        }
        if first.kind == 3 {
            barrier = true;
            continue;
        }
        attacks.push(combat::SpellAttack { key: a.id | SPELL_KEY, group: if first.combo_group != 0 { first.combo_group as i64 } else { a.group }, atk: a.atk, sub: a.sub, shapes });
    }
    if !attacks.is_empty() {
        let hits = strikes.spell_strike(actor, &attacks, &|atk, sub, count| row(battle, atk, sub, count).cloned());
        for (atk, foe, dealt) in hits {
            rt.counter_hits += 1;
            note(format!("Reflect counter: ATKP {atk} hit {foe:#x} -> damage {dealt:?}"));
        }
    }
    for a in rt.magic.freed().filter(|a| a.owns_id) {
        strikes.spell_forget(a.id | SPELL_KEY);
    }
    if barrier && !held {
        rt.hold.age += 1;
    }
    if barrier != rt.barrier {
        rt.barrier = barrier;
        note(format!("Reflect barrier {} ({:.2} s after the cast)", if barrier { "up" } else { "down" }, rt.started.map_or(0.0, |t| t.elapsed().as_secs_f32())));
    }
    // (Waiting for the combo, the barrier stops nothing: Sora is attacking.)
    BARRIER.store(barrier && !combo_wait, Ordering::Relaxed);
    if rt.magic.casts.is_empty() && rt.casting.is_none() && rt.fresh.is_none() && rt.started.is_some() {
        note(format!("Reflect done: {} hit(s) blocked, {} counter hit(s)", rt.blocks, rt.counter_hits));
        rt.started = None;
    }
}

/// Before the spell's frame: whether its cast is held this frame (the barrier stretched while
/// hits keep coming, `HOLD_AT`), with `blocks` hits stopped since the last frame.
///
/// Also held, with something blocked, while Sora's combo goes on (`combo`: steps left): the
/// user, "if sora does a combo with reflect, any attack done to every reflect should burst out of
/// the final reflect in the chain". The barrier is hidden and stops nothing then, and its counter
/// waits: a Reflect cast later in the combo takes its blocked hits over (`after_step`'s chain)
/// and waits or bursts in its turn; the combo over (its last step, a while with no swing, a
/// landing, a hit taken), the last one bursts for them all. At most `COMBO_WAIT_MAX_S`.
fn hold(rt: &mut Runtime, blocks: u32, combo: bool) -> bool {
    let h = &mut rt.hold;
    h.since_hit = if blocks > 0 { Some(0) } else { h.since_hit.map(|n| n.saturating_add(1)) };
    let was = h.since.is_some();
    let cap = if h.combo { COMBO_WAIT_MAX_S } else { HOLD_MAX_S };
    let over = h.since.is_some_and(|t| t.elapsed().as_secs_f32() >= cap);
    let hits = rt.barrier && !h.spent && !over && h.age >= HOLD_AT && h.since_hit.is_some_and(|n| n <= HOLD_FRAMES);
    let for_combo = rt.barrier && !h.spent && !over && h.age >= HOLD_AT && !hits && combo && rt.blocks > 0;
    if for_combo != h.combo {
        h.combo = for_combo;
        note(format!("Reflect {}: {} blocked", if for_combo { "waits for the combo (the chain's last Reflect bursts)" } else { "no longer waiting for the combo" }, rt.blocks));
    }
    let want = hits || for_combo;
    if want {
        if !was {
            h.since = Some(Instant::now());
            h.frames = 0;
            note(format!("Reflect barrier held: last hit {} frames ago ({} blocked so far)", h.since_hit.unwrap_or(0), rt.blocks));
        } else if blocks > 0 {
            note(format!("Reflect barrier held: last hit 0 frames ago ({} blocked so far, held {} frames)", rt.blocks, h.frames));
        }
        h.frames += 1;
    } else if was {
        h.spent = true;
        let held = h.since.take().map_or(0.0, |t| t.elapsed().as_secs_f32());
        note(format!(
            "Reflect barrier let go: last hit {} frames ago, held {} frames ({held:.2} s{}), {} blocked: the counter follows",
            h.since_hit.unwrap_or(0),
            h.frames,
            if over { ", the cap" } else { "" },
            rt.blocks
        ));
    }
    rt.magic.hold(SORA, want);
    want
}

/// KH1 has Sora this frame: a Reflect in flight ends with its effects (KH2 runs it).
pub fn kh1_frame() {
    BARRIER.store(false, Ordering::Relaxed);
    let Ok(mut g) = RT.try_lock() else { return };
    let Some(rt) = g.as_mut() else { return };
    if rt.magic.casts.is_empty() && rt.fx.is_empty() && rt.fresh.is_none() {
        return;
    }
    for h in rt.fx.values().filter_map(|v| v.0) {
        super::kh2fx::spell_kill(h);
    }
    let file = rt.magic.spells.clone();
    rt.magic = Magic::new();
    for f in file {
        rt.magic.add_spell(f);
    }
    rt.fx.clear();
    rt.fresh = None;
    rt.casting = None;
    rt.barrier = false;
    if rt.started.take().is_some() {
        note("Reflect ended: KH1 has Sora".to_string());
    }
}
