//! The debug menu (the user, t354: "you would press a button combination and a menu would appear,
//! it doesn't pause the game state like the pause menu but gets overlayed like the command menu").
//!
//! - **Turned on** by the Config menu's Debug Menu row (`config.rs`; the ini's `debug_menu`, off by
//!   default). Then **L1 + R1 + Triangle** opens it, and the same buttons close it again. Closed
//!   that way it opens again on the page it was left on; Exit on the main page (or Circle there)
//!   closes it and the next open starts on the main page.
//! - **Drawn** over everything KH1 drew that frame (the HUD too), from a hook on KH1's end of the
//!   frame's drawing (Steam `0x1400FBBE0 (renderer)`, Epic `0x1400FBD80` by the table's neighbours:
//!   the field loop calls it after the HUD's tasks, and it closes the frame's command list,
//!   `0x1400F7300`), once a frame, with `fx12::draw_overlay`. (t356: t355 drew from KH1's frame end,
//!   `0x140109CE0` / Steam `0x140109B40`, where no command list is open any more: KH1's list getter
//!   read a null command buffer and the game crashed as the menu opened. The frame end keeps the
//!   menu's input and the stat locks.) It is a grey box with some opacity, white text, the row
//!   under the cursor yellow. The text is a monospace font (DejaVu Sans Mono Bold, Bitstream Vera
//!   licence) made into a 16 x 6 grid of 14 x 28 cells (`debugfont.rs`, coverage levels,
//!   ASCII 32..126), scaled with the frame buffer's height (1080 = 1).
//! - **The game runs on** under it; while it is open KH1 and KH2 see no buttons and centred
//!   sticks (`pad`, from `probe::pad` after KH1's records are merged), so Sora stands still. The
//!   menu reads the pad as pressed: D-pad up / down move, left / right change a value, KH1's
//!   confirm button picks and its cancel button goes back (t357: the game's own setting, as its
//!   menus use it: `0x140546390` confirm / `+4` cancel, Steam `0x1405456A0`, set by
//!   `0x1402F6700` to Circle `0x2000` or Cross `0x4000`; the hints name them). Every button is kept from the game after it closes until all are
//!   let go (KH1 would take a held button as a press). While L1 + R1 are held with the menu on,
//!   KH1 doesn't see Triangle.
//! - **Frame Advance** (t371, Steam only): holds the game still; R2 or the period key
//!   plays one frame, held for a second plays at full speed (`frameadv`). Turning it on turns the
//!   Trace Recorder's Game Frames row on (the seconds count game frames, `trace::game_frames`).
//!   Not kept in the ini: it starts off each launch. (The tester, after t463: "make frame advance
//!   a submenu and throw it plus frame advance option in there".) Its rows are on their own page,
//!   Frame Advance (main page): the Frame Advance row, Held Sound, Camera (Free: the camera
//!   moves while held, `frameadv::camera_live`; Held), Game Frames (the same row as the Trace
//!   Recorder's) and the Held Tasks page.
//! - **KH1 Spells** (main page): Gravity, Stop and Aero each "KH1 cast" (KH1's whole cast, its
//!   own motion, clock and effects) or "KH2 cast" (KH1's spell on KH2's cast motion, the default)
//!   at the Magic styles KH2 (`formmagic::kh1_cast`); kept in the ini (`kh1_cast_*`).
//! - **Pages**: Trace Recorder (the trace's mode, seconds, the sound log and its categories,
//!   `trace::ITEMS` / `GROUPS`: each group's row turns all of it on or off with left / right,
//!   Cross opens it to set each item; every change is written to the ini) and Stats (Sora's Level,
//!   HP, max HP, MP, max MP, Strength, Defense and AP: left / right change it, Cross locks it).
//! - **Stats** live in KH1's three copies (t358: the user, "touch saves as well", after the Status
//!   page was found to read Level from the save, Steam `0x14031272D` `movzx eax, [r13+4]`): Sora's
//!   save record (`*0x1428695A0` = the save block `0x142DE9D60`, Steam `0x142DE9360`, `0x16C00`
//!   bytes written to the save file; record 0: `+4` Level, `+5` HP, `+6` max HP, `+7` MP, `+8` max
//!   MP, `+9` AP, `+0xA` Strength, `+0xB` Defense, bytes). The record's max HP, max MP, AP,
//!   Strength and Defense are KH1's base numbers, before equipment: the stats function adds the
//!   weapon's and accessories' and caps Strength, Defense and AP at 100. So a stat set to `v` puts
//!   `v` less that bonus in the record (the bonus read from KH1's own stats function, unhooked,
//!   `menu::kh1_stats`: its result less the record's number), and KH1's next rebuild gives `v`
//!   back. HP and MP are the record's own numbers; Level is only in the record. Then Sora's stats block in the world (the
//!   actor's `+0x6C` handle: `+0x3C` HP, `+0x40` max HP, `+0x44` MP, `+0x48` max MP, `+0x4C`
//!   Strength, `+0x50` Defense, `+0xE4` AP, as KH1's stats function `0x1402A2FA0` fills it) and the
//!   Status menu's copy (`0x142E94BC0 + member * 0x100`, Steam `0x142E941E0`, filled by the same
//!   function for each member). A change is written to all of them once; KH1 may change it after
//!   (damage, MP spent, a rebuild at the pause menu's exit or an area). A locked stat is written
//!   to all of them every frame and again whenever KH1's stats function fills a block for Sora
//!   (`menu::stats` -> `lock_stats`), so nothing moves it. HP and MP stay within their maximums;
//!   a locked HP above 0 also clears the block's dying flag (bit 4 of `+0`, as KH1's HP change
//!   `0x1402A2790` does). Not kept in the ini: the locks last the session.
//! - **Weapons** (t359, under Stats): every Sora weapon (item kind 2, KH1's item table) with how
//!   many he owns, KH1's inventory count `save + 0x499 + id` (`equip`): left / right add or take
//!   one (0..99, never below the copies equipped), Cross gives one or takes every free one.
//!   (t360) Cross equips it instead (KH1's own weapon change, `0x1402845A0`, which loads the model;
//!   one is given first when he owns none): the way off a Dream weapon or the wooden sword, whose
//!   hold KH1's Equipment page refuses to change.
//! - **Abilities** (t359, under Stats): Sora's own abilities (KH1's 1, 2, 5..28, 53..65; ours from
//!   `growth::MENU`, but not the glides, which the Glide style decides) with how many copies are in
//!   his lists (`save + 0x44`, 48 bytes, bit 7 set = off; the shared `+0x599`, 10, for High Jump and
//!   Mermaid Kick) and how many are on: right adds a copy (off in his list, as KH1's own learning
//!   does), left takes one (an off copy first), Cross adds one or takes them all. The mod's own
//!   upkeep (`menu::Growth::update`: KH2's abilities given by level, boss or namesake, Aerial Dodge
//!   held back until Genie Jafar, no Zantetsuken, ours taken out at `abilities = test / all`)
//!   leaves alone what was added or taken here this session (`ability_forced`).

use super::addr::Rva;
use super::{fx12, hook, log, trace};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// KH1's frame end (`0x140109CE0 (renderer)`): its first instructions, `mov rax, rsp; push rdi;
/// sub rsp, 0x90` (the same bytes in both builds).
const FRAME_END_RVA: Rva = Rva(0x109CE0);
const FRAME_END_PROLOGUE: [u8; 11] = [0x48, 0x8B, 0xC4, 0x57, 0x48, 0x81, 0xEC, 0x90, 0x00, 0x00, 0x00];
/// The end of the frame's drawing (Steam `0x1400FBBE0 (renderer)`: `push rsi; push rdi; push r14;
/// sub rsp, 0x40`): the frame's command list is still open at its start.
const DRAW_END_RVA: Rva = Rva(0xFBD80);
const DRAW_END_PROLOGUE: [u8; 9] = [0x40, 0x56, 0x57, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x40];
/// The Status menu's copy of each member's stats (0x100 apart, Sora's first).
const PARTY_STATS_RVA: Rva = Rva(0x2E94BC0);
/// KH1's menu buttons as the player set them: confirm (u32 mask in the pad's order), cancel at `+4`.
const CONFIRM_MASK_RVA: Rva = Rva(0x546390);
/// KH1's weapon change (`0x1402845A0 (item, done)`, Steam `0x140286720`): writes the member's weapon
/// byte, frees the weapon object and loads and makes the new one (what the Equipment page's preview
/// uses); `done` is called once the object is made, and must not be null.
const EQUIP_WEAPON_RVA: Rva = Rva(0x2845A0);
/// The pause menu's refresh of the party's stats copy (`0x1402E5A80`, as after an equipment change).
const STATS_REFRESH_RVA: Rva = Rva(0x2E5A80);
/// KH1's menu sounds (`0x140177F60 (n)`, as the Config rows use it: 0 the cursor, 1 a change).
const SOUND_RVA: Rva = Rva(0x177F60);

// The pad's buttons (PS2 order, `probe::pad`).
const UP: u16 = 1 << 4;
const RIGHT: u16 = 1 << 5;
const DOWN: u16 = 1 << 6;
const LEFT: u16 = 1 << 7;
const L1: u16 = 1 << 10;
const R1: u16 = 1 << 11;
const TRIANGLE: u16 = 1 << 12;
const CIRCLE: u16 = 1 << 13;
const CROSS: u16 = 1 << 14;
const DIRS: u16 = UP | DOWN | LEFT | RIGHT;

/// A held direction repeats after this long, then every `REPEAT_MS`.
const REPEAT_AFTER_MS: u128 = 350;
const REPEAT_MS: u128 = 60;

static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG_FRAME_END: AtomicUsize = AtomicUsize::new(0);
static ORIG_DRAW_END: AtomicUsize = AtomicUsize::new(0);
/// The menu was drawn this frame (KH1's frame end calls the drawing's end again).
static DRAWN: AtomicBool = AtomicBool::new(false);
/// The Config menu's Debug Menu row (the ini's `debug_menu`).
static ENABLED: AtomicBool = AtomicBool::new(false);
static OPEN: AtomicBool = AtomicBool::new(false);
/// After it closes: the game sees no buttons until all are let go.
static SWALLOW: AtomicBool = AtomicBool::new(false);
/// The open / close buttons are held (one toggle per press).
static COMBO_HELD: AtomicBool = AtomicBool::new(false);
/// The pad as pressed (KH1's records merged), the menu's input.
static PAD: AtomicU32 = AtomicU32::new(0);

/// What the game is kept from seeing this pad read (`pad`).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Hide {
    Nothing,
    /// Triangle (L1 + R1 held with the menu on: the open buttons).
    Triangle,
    /// Every button, and the sticks centred (the menu is open, or was just closed).
    All,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Page {
    Main,
    Trace,
    Categories,
    Group(usize),
    Stats,
    Weapons,
    Abilities,
    /// The Frame Advance rows (after t463: the tester's submenu).
    FrameAdvance,
    /// (t399) The tasks Frame Advance holds, each one switchable (`frameadv::tasks`).
    HeldTasks,
    /// Gravity, Stop and Aero: KH1's whole cast or KH2's cast motion at the Magic styles KH2
    /// (`formmagic::kh1_cast`).
    Spells,
}

impl Page {
    fn parent(self) -> Option<Page> {
        match self {
            Page::Main => None,
            Page::Trace | Page::Stats | Page::FrameAdvance => Some(Page::Main),
            Page::Categories => Some(Page::Trace),
            Page::Group(_) => Some(Page::Categories),
            Page::Weapons | Page::Abilities => Some(Page::Stats),
            Page::HeldTasks => Some(Page::FrameAdvance),
            Page::Spells => Some(Page::Main),
        }
    }

    /// Its cursor's place in `Menu::cursor`.
    fn slot(self) -> usize {
        match self {
            Page::Main => 0,
            Page::Trace => 1,
            Page::Categories => 2,
            Page::Stats => 3,
            Page::Weapons => 4,
            Page::Abilities => 5,
            Page::Group(i) => 6 + i.min(trace::GROUPS.len() - 1),
            Page::HeldTasks => 6 + trace::GROUPS.len(),
            Page::FrameAdvance => 7 + trace::GROUPS.len(),
            Page::Spells => 8 + trace::GROUPS.len(),
        }
    }

    fn title(self) -> String {
        match self {
            Page::Main => "DEBUG MENU".into(),
            Page::Trace => "DEBUG MENU > TRACE RECORDER".into(),
            Page::Categories => "TRACE RECORDER > CATEGORIES".into(),
            Page::Group(i) => format!("CATEGORIES > {}", trace::GROUPS[i].name.to_ascii_uppercase()),
            Page::Stats => "DEBUG MENU > STATS".into(),
            Page::Weapons => "STATS > WEAPONS".into(),
            Page::Abilities => "STATS > ABILITIES".into(),
            Page::FrameAdvance => "DEBUG MENU > FRAME ADVANCE".into(),
            Page::HeldTasks => "FRAME ADVANCE > HELD TASKS".into(),
            Page::Spells => "DEBUG MENU > KH1 SPELLS".into(),
        }
    }

    /// The page's button hint, with KH1's confirm and cancel buttons' marks (`ok`, `no`).
    fn hint(self, ok: &str, no: &str) -> String {
        match self {
            Page::Main => format!("Up/Down: move   Left/Right: change   {ok}: select   {no}: close"),
            Page::Trace => format!("Left/Right: change   {ok}: select   {no}: back"),
            Page::Categories => format!("Left/Right: all on/off   {ok}: open   {no}: back"),
            Page::Group(_) => format!("Left/Right/{ok}: on/off   {no}: back"),
            Page::Stats => format!("Left/Right: change   {ok}: lock   {no}: back"),
            Page::Weapons => format!("Left/Right: count   {ok}: equip   {no}: back"),
            Page::Abilities => format!("Left/Right: copies   {ok}: add/remove   {no}: back"),
            Page::FrameAdvance => format!("Left/Right: change   {ok}: select   {no}: back"),
            Page::HeldTasks => format!("Left/Right/{ok}: held / runs   {no}: back"),
            Page::Spells => format!("Left/Right/{ok}: KH1 / KH2 cast   {no}: back"),
        }
    }
}

/// What a row does.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Enter(Page),
    Back,
    Exit,
    TraceMode,
    Seconds,
    SoundLog,
    GameFrames,
    FrameAdvance,
    HeldSound,
    FreeCamera,
    HeldTask(usize),
    /// Gravity 4, Stop 5, Aero 6.
    Kh1Cast(u8),
    GroupRow(usize),
    Item(usize),
    Stat(usize),
    Weapon(u8),
    Ability(u8),
}

struct Menu {
    page: Page,
    cursor: [usize; 9 + trace::GROUPS.len()],
    /// The pad last frame, the directions held and since when, the last repeat.
    last: u16,
    held: u16,
    held_at: Option<Instant>,
    repeat_at: Option<Instant>,
}

static MENU: Mutex<Menu> = Mutex::new(Menu { page: Page::Main, cursor: [0; 9 + trace::GROUPS.len()], last: 0, held: 0, held_at: None, repeat_at: None });

/// One of Sora's stats: its name, its place in a stats block (`None`: only in the save), its byte
/// in his save record, whether that byte is KH1's base number (equipment added on top), its
/// range.
struct Stat {
    name: &'static str,
    off: Option<usize>,
    save: usize,
    base: bool,
    min: i32,
    max: i32,
}

const LEVEL: usize = 0;
const HP: usize = 1;
const MAX_HP: usize = 2;
const MP: usize = 3;
const MAX_MP: usize = 4;
const N_STATS: usize = 8;
const STATS: [Stat; N_STATS] = [
    Stat { name: "Level", off: None, save: 0x04, base: false, min: 1, max: 100 },
    Stat { name: "HP", off: Some(0x3C), save: 0x05, base: false, min: 0, max: 255 },
    Stat { name: "Max HP", off: Some(0x40), save: 0x06, base: true, min: 1, max: 255 },
    Stat { name: "MP", off: Some(0x44), save: 0x07, base: false, min: 0, max: 255 },
    Stat { name: "Max MP", off: Some(0x48), save: 0x08, base: true, min: 0, max: 255 },
    Stat { name: "Strength", off: Some(0x4C), save: 0x0A, base: true, min: 0, max: 255 },
    Stat { name: "Defense", off: Some(0x50), save: 0x0B, base: true, min: 0, max: 255 },
    Stat { name: "AP", off: Some(0xE4), save: 0x09, base: true, min: 0, max: 255 },
];
/// The order locks are written in (the maximums before HP and MP, which stay within them).
const LOCK_ORDER: [usize; N_STATS] = [MAX_HP, MAX_MP, HP, MP, 5, 6, 7, LEVEL];

/// The locked stats' values (`None`: not locked).
static LOCKS: Mutex<[Option<i32>; N_STATS]> = Mutex::new([None; N_STATS]);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    ENABLED.store(super::settings::get().debug_menu, Ordering::Relaxed);
    hook::install(base + FRAME_END_RVA, &FRAME_END_PROLOGUE, frame_end as *const () as usize, &ORIG_FRAME_END, "frame end (debug menu input, stat locks)");
    hook::install(base + DRAW_END_RVA, &DRAW_END_PROLOGUE, draw_end as *const () as usize, &ORIG_DRAW_END, "end of the frame's drawing (debug menu)");
}

/// The Config menu's Debug Menu row.
pub fn enabled() -> bool {
    ENABLED.load(Ordering::Relaxed)
}

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
    if !on && OPEN.swap(false, Ordering::Relaxed) {
        SWALLOW.store(true, Ordering::Relaxed);
    }
    // (t371) Frame Advance is a debug menu row: off with the menu, so the game isn't left held.
    if !on {
        super::frameadv::set_on(false);
    }
    super::settings::save("debug_menu", if on { "on" } else { "off" });
    log(&format!("Config menu: debug menu {}", if on { "on (L1 + R1 + Triangle)" } else { "off" }));
}

/// Each pad read (`probe::pad`), with KH1's records merged: opens or closes the menu on L1 + R1 +
/// Triangle, keeps the pad for the menu, and says what the game must not see.
pub fn pad(buttons: u16) -> Hide {
    PAD.store(buttons as u32, Ordering::Relaxed);
    let enabled = ENABLED.load(Ordering::Relaxed);
    let mods = buttons & (L1 | R1) == L1 | R1;
    let combo = mods && buttons & TRIANGLE != 0;
    if combo && !COMBO_HELD.swap(true, Ordering::Relaxed) {
        if OPEN.load(Ordering::Relaxed) {
            close(false);
        } else if enabled {
            open();
        }
    } else if !combo {
        COMBO_HELD.store(false, Ordering::Relaxed);
    }
    if SWALLOW.load(Ordering::Relaxed) && !OPEN.load(Ordering::Relaxed) && buttons == 0 {
        SWALLOW.store(false, Ordering::Relaxed);
    }
    if OPEN.load(Ordering::Relaxed) || SWALLOW.load(Ordering::Relaxed) {
        Hide::All
    } else if enabled && mods {
        Hide::Triangle
    } else {
        Hide::Nothing
    }
}

fn open() {
    OPEN.store(true, Ordering::Relaxed);
    let page = MENU.lock().map(|mut m| {
        // (What's held now isn't a press on the menu.)
        m.last = PAD.load(Ordering::Relaxed) as u16;
        m.held = 0;
        m.held_at = None;
        m.page
    });
    log(&format!("debug menu: opened ({:?})", page.unwrap_or(Page::Main)));
}

/// Closes the menu; `to_main`: the next open starts on the main page (Exit), else on this one.
fn close(to_main: bool) {
    OPEN.store(false, Ordering::Relaxed);
    SWALLOW.store(true, Ordering::Relaxed);
    if to_main {
        if let Ok(mut m) = MENU.lock() {
            m.page = Page::Main;
        }
    }
    log(&format!("debug menu: closed{}", if to_main { "" } else { " (opens on the same page)" }));
}

fn sound(n: i32) {
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 {
        return;
    }
    unsafe {
        let f: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
        f(n);
    }
}

/// KH1's frame end: the menu's input and the stat locks, then KH1's own.
/// KH1's confirm and cancel buttons now (Cross and Circle unless the game says otherwise).
unsafe fn buttons(base: usize) -> (u16, u16) {
    let at = base + CONFIRM_MASK_RVA;
    let pick = |v: u32| match v & 0xFFFF {
        0x2000 => Some(CIRCLE),
        0x4000 => Some(CROSS),
        _ => None,
    };
    match (pick(rd::<u32>(at)), pick(rd::<u32>(at + 4))) {
        (Some(ok), Some(no)) if ok != no => (ok, no),
        (Some(ok), _) => (ok, if ok == CROSS { CIRCLE } else { CROSS }),
        _ => (CROSS, CIRCLE),
    }
}

/// A button's mark in the hints.
fn mark(b: u16) -> &'static str {
    if b == CIRCLE {
        "O"
    } else {
        "X"
    }
}

unsafe extern "system" fn frame_end(renderer: usize) {
    let base = BASE.load(Ordering::Relaxed);
    if base != 0 {
        apply_locks(base);
        if OPEN.load(Ordering::Relaxed) {
            step(base);
        }
        // (t371) Whether the next frame runs or is held (`frameadv`): after this frame's tasks.
        super::frameadv::frame_end(base);
    }
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_FRAME_END.load(Ordering::Acquire));
    f(renderer);
    DRAWN.store(false, Ordering::Relaxed);
}

/// The end of the frame's drawing: the menu drawn into the frame's list first (once a frame).
unsafe extern "system" fn draw_end(renderer: usize, a: usize, b: usize, c: usize) -> i32 {
    let base = BASE.load(Ordering::Relaxed);
    if base != 0 && OPEN.load(Ordering::Relaxed) && !DRAWN.swap(true, Ordering::Relaxed) {
        draw(base);
    }
    let f: unsafe extern "system" fn(usize, usize, usize, usize) -> i32 = std::mem::transmute(ORIG_DRAW_END.load(Ordering::Acquire));
    f(renderer, a, b, c)
}

// ---- the pages ----

/// A page's rows: text and action.
unsafe fn rows(base: usize, page: Page) -> Vec<(String, Action)> {
    let on_off = |v: bool| if v { "On" } else { "Off" };
    match page {
        Page::Main => vec![
            ("Trace Recorder  >".into(), Action::Enter(Page::Trace)),
            ("Stats  >".into(), Action::Enter(Page::Stats)),
            ("Frame Advance  >".into(), Action::Enter(Page::FrameAdvance)),
            ("KH1 Spells  >".into(), Action::Enter(Page::Spells)),
            ("Exit".into(), Action::Exit),
        ],
        // (After t463) Frame Advance's own rows.
        Page::FrameAdvance => vec![
            // (t371) `frameadv`: Steam only.
            (format!("{:<16}< {} >", "Frame Advance", if super::frameadv::supported() { on_off(super::frameadv::on()) } else { "Steam only" }), Action::FrameAdvance),
            // (KH1's pause's sound call while held, or not.)
            (format!("{:<16}< {} >", "Held Sound", if super::frameadv::hold_sound() { "Paused" } else { "Plays" }), Action::HeldSound),
            // (After t463) The camera moves while held (Free) or not (`frameadv::camera_live`).
            (format!("{:<16}< {} >", "Camera", if super::frameadv::free_camera() { "Free" } else { "Held" }), Action::FreeCamera),
            // (The Trace Recorder's row: Frame Advance turns it on.)
            (format!("{:<16}< {} >", "Game Frames", on_off(trace::game_frames())), Action::GameFrames),
            ("Held Tasks  >".into(), Action::Enter(Page::HeldTasks)),
            ("Back".into(), Action::Back),
        ],
        // (At the Magic styles KH1 every one is KH1's own anyway.)
        Page::Spells => {
            let mut v: Vec<(String, Action)> = ["Gravity", "Stop", "Aero"]
                .iter()
                .zip(4u8..)
                .map(|(name, sp)| (format!("{:<16}< {} >", name, if super::formmagic::kh1_cast(sp) { "KH1 cast" } else { "KH2 cast" }), Action::Kh1Cast(sp)))
                .collect();
            v.push(("Back".into(), Action::Back));
            v
        }
        Page::HeldTasks => {
            let mut v: Vec<(String, Action)> = super::frameadv::tasks()
                .iter()
                .enumerate()
                .map(|(i, t)| (if t.sub { format!("{:>6} {:<21}< {} >", t.priority, t.label, if t.runs { "Runs" } else { "Held" }) } else { format!("{:>6} {:#08x} {:<12}< {} >", t.priority, t.rva, t.label, if t.runs { "Runs" } else { "Held" }) }, Action::HeldTask(i)))
                .collect();
            if v.is_empty() {
                v.push(("(turn Frame Advance on first)".into(), Action::Back));
            }
            v.push(("Back".into(), Action::Back));
            v
        }
        Page::Trace => vec![
            (format!("{:<16}< {} >", "Trace", match trace::mode() { trace::Mode::Off => "Off", trace::Mode::On => "On", trace::Mode::Always => "Always" }), Action::TraceMode),
            (format!("{:<16}< {} >", "Seconds", trace::seconds()), Action::Seconds),
            // (t371) On: the seconds count game frames (`trace::game_frames`); Off: real time.
            (format!("{:<16}< {} >", "Game Frames", on_off(trace::game_frames())), Action::GameFrames),
            (format!("{:<16}< {} >", "Sound Log", on_off(trace::sound_log())), Action::SoundLog),
            ("Categories  >".into(), Action::Enter(Page::Categories)),
            ("Back".into(), Action::Back),
        ],
        Page::Categories => {
            let cats = trace::cats();
            let mut v: Vec<(String, Action)> = trace::GROUPS
                .iter()
                .enumerate()
                .map(|(i, g)| {
                    let n = g.items.iter().filter(|&&k| cats & trace::ITEMS[k].bit != 0).count();
                    let state = if n == g.items.len() { "All" } else if n == 0 { "None" } else { "Some" };
                    (format!("{:<16}< {:<4} >  >", g.name, state), Action::GroupRow(i))
                })
                .collect();
            v.push(("Back".into(), Action::Back));
            v
        }
        Page::Group(i) => {
            let cats = trace::cats();
            let mut v: Vec<(String, Action)> = trace::GROUPS[i].items.iter().map(|&k| (format!("{:<20}< {} >", trace::ITEMS[k].name, on_off(cats & trace::ITEMS[k].bit != 0)), Action::Item(k))).collect();
            v.push(("Back".into(), Action::Back));
            v
        }
        Page::Stats => {
            let locks = LOCKS.lock().map(|l| *l).unwrap_or([None; N_STATS]);
            let mut v: Vec<(String, Action)> = STATS
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let now = locks[i].or_else(|| stat_now(base, i));
                    let shown = now.map_or("--".to_string(), |n| n.to_string());
                    (format!("{:<12}< {:>3} >  {}", s.name, shown, if locks[i].is_some() { "[Locked]" } else { "" }), Action::Stat(i))
                })
                .collect();
            v.push(("Weapons  >".into(), Action::Enter(Page::Weapons)));
            v.push(("Abilities  >".into(), Action::Enter(Page::Abilities)));
            v.push(("Back".into(), Action::Back));
            v
        }
        Page::Weapons => {
            let held = super::equip::sora_weapon(base);
            let mut v: Vec<(String, Action)> = sora_weapons(base)
                .into_iter()
                .map(|id| {
                    let name: String = super::equip::item_name(base, id).unwrap_or_else(|| format!("Item {id:#04x}")).chars().take(20).collect();
                    (format!("{:<20}< {:>2} >  {}", name, super::equip::owned(base, id), if held == Some(id) { "[Equipped]" } else { "" }), Action::Weapon(id))
                })
                .collect();
            v.push(("Back".into(), Action::Back));
            v
        }
        Page::Abilities => {
            let mut v: Vec<(String, Action)> = Vec::new();
            if let Some(save) = super::menu::save_block(base) {
                for id in ability_ids() {
                    let (n, on) = copies(save, id);
                    let ours = crate::growth::MENU.iter().any(|a| a.id == id);
                    let name: String = format!("{}{}", crate::growth::name(id), if ours { " (KH2)" } else { "" }).chars().take(26).collect();
                    v.push((format!("{:<26}< {} >  {}", name, n, if on > 0 { format!("{on} on") } else { String::new() }), Action::Ability(id)));
                }
            }
            v.push(("Back".into(), Action::Back));
            v
        }
    }
}

/// This frame's input on the open menu.
unsafe fn step(base: usize) {
    let Ok(mut m) = MENU.lock() else { return };
    let now = PAD.load(Ordering::Relaxed) as u16;
    let mut fire = now & !m.last;
    // Held directions repeat.
    let dirs = now & DIRS;
    let t = Instant::now();
    if dirs != 0 && dirs == m.held {
        let held_long = m.held_at.is_some_and(|a| t.duration_since(a).as_millis() >= REPEAT_AFTER_MS);
        let due = m.repeat_at.map_or(true, |a| t.duration_since(a).as_millis() >= REPEAT_MS);
        if held_long && due {
            fire |= dirs;
            m.repeat_at = Some(t);
        }
    } else {
        m.held = dirs;
        m.held_at = Some(t);
        m.repeat_at = None;
    }
    m.last = now;
    let (ok, no) = buttons(base);
    if fire & (DIRS | CROSS | CIRCLE) == 0 {
        return;
    }
    let page = m.page;
    let list = rows(base, page);
    let slot = page.slot();
    let n = list.len().max(1);
    let mut cur = m.cursor[slot].min(n - 1);
    if fire & UP != 0 {
        cur = (cur + n - 1) % n;
        sound(0);
    } else if fire & DOWN != 0 {
        cur = (cur + 1) % n;
        sound(0);
    }
    m.cursor[slot] = cur;
    if fire & no != 0 {
        match page.parent() {
            Some(p) => m.page = p,
            None => {
                drop(m);
                close(true);
            }
        }
        sound(1);
        return;
    }
    let (left, right, cross) = (fire & LEFT != 0, fire & RIGHT != 0, fire & ok != 0);
    if !(left || right || cross) {
        return;
    }
    let Some(&(_, action)) = list.get(cur) else { return };
    let delta = if right { 1 } else if left { -1 } else { 0 };
    match action {
        Action::Enter(p) if cross => {
            m.page = p;
            sound(1);
        }
        Action::Back if cross => {
            if let Some(p) = page.parent() {
                m.page = p;
            }
            sound(1);
        }
        Action::Exit if cross => {
            drop(m);
            close(true);
            sound(1);
        }
        Action::TraceMode => {
            use trace::Mode::*;
            let order = [Off, On, Always];
            let i = order.iter().position(|x| *x == trace::mode()).unwrap_or(1) as i32;
            let next = (i + if delta == 0 { 1 } else { delta }).rem_euclid(3) as usize;
            trace::set_mode(order[next]);
            sound(1);
        }
        Action::Seconds if delta != 0 => {
            let s = (trace::seconds() as i32 + delta).clamp(1, trace::MAX_SECONDS as i32) as u32;
            if s != trace::seconds() {
                trace::set_seconds(s);
                sound(1);
            }
        }
        Action::SoundLog => {
            trace::set_sound_log(!trace::sound_log());
            sound(1);
        }
        Action::GameFrames => {
            trace::set_game_frames(!trace::game_frames());
            sound(1);
        }
        Action::FrameAdvance => {
            if super::frameadv::supported() {
                super::frameadv::set_on(!super::frameadv::on());
                sound(1);
            }
        }
        Action::HeldSound => {
            super::frameadv::set_hold_sound(!super::frameadv::hold_sound());
            sound(1);
        }
        Action::FreeCamera => {
            super::frameadv::set_free_camera(!super::frameadv::free_camera());
            sound(1);
        }
        Action::HeldTask(i) => {
            super::frameadv::toggle_task(i);
            sound(1);
        }
        Action::Kh1Cast(sp) => {
            super::formmagic::set_kh1_cast(sp, !super::formmagic::kh1_cast(sp));
            sound(1);
        }
        Action::GroupRow(i) if cross => {
            m.page = Page::Group(i);
            sound(1);
        }
        Action::GroupRow(i) => {
            let bits = trace::GROUPS[i].items.iter().fold(0, |b, &k| b | trace::ITEMS[k].bit);
            let all = trace::cats() & bits == bits;
            trace::set_cats(bits, !all);
            sound(1);
        }
        Action::Item(k) => {
            let bit = trace::ITEMS[k].bit;
            trace::set_cats(bit, trace::cats() & bit == 0);
            sound(1);
        }
        Action::Stat(i) if cross => {
            toggle_lock(base, i);
            sound(1);
        }
        Action::Stat(i) if delta != 0 => {
            if change_stat(base, i, delta) {
                sound(1);
            }
        }
        Action::Weapon(id) if cross => {
            if equip_weapon(base, id) {
                sound(1);
            }
        }
        Action::Weapon(id) => {
            if change_weapon(base, id, delta) {
                sound(1);
            }
        }
        Action::Ability(id) => {
            if change_ability(base, id, delta) {
                sound(1);
            }
        }
        _ => {}
    }
}

// ---- Sora's stats ----

/// Sora's stats block in the world (his actor's `+0x6C`), if he is there.
unsafe fn live_block() -> Option<usize> {
    let sora = super::takeover::sora_actor();
    if sora < 0x10000 || !super::crash::readable(sora + 0x70) {
        return None;
    }
    let b = super::pose::ptr(rd::<u32>(sora + 0x6C));
    (b > 0x10000 && super::crash::readable(b) && super::crash::readable(b + 0xE8)).then_some(b)
}

/// The Status menu's copy of Sora's stats, while a save is loaded.
unsafe fn menu_block(base: usize) -> Option<usize> {
    super::menu::save_block(base)?;
    let b = base + PARTY_STATS_RVA;
    super::crash::readable(b + 0xE8).then_some(b)
}

/// Sora's save record (record 0 of the save block), while a save is loaded.
unsafe fn save_record(base: usize) -> Option<usize> {
    let r = super::menu::save_block(base)?;
    super::crash::readable(r + 0x74).then_some(r)
}

/// KH1's own stats for Sora's save and equipment (its stats function, unhooked: no locks of ours,
/// no Valor Keyblade, no KH2 AP), for the equipment's bonus over the record's base numbers.
#[repr(C, align(16))]
struct Block([u8; 0x200]);
unsafe fn kh1_own() -> Option<Block> {
    let mut b = Block([0; 0x200]);
    super::menu::kh1_stats(0, b.0.as_mut_ptr() as usize).then_some(b)
}

/// The stat as KH1 has it now (Level: the save; else the world's block, else the Status menu's).
unsafe fn stat_now(base: usize, i: usize) -> Option<i32> {
    match STATS[i].off {
        None => save_record(base).map(|r| rd::<u8>(r + STATS[i].save) as i32),
        Some(off) => live_block().or_else(|| menu_block(base)).map(|b| rd::<i32>(b + off)),
    }
}

/// The stat's range now: HP and MP within their maximums (a locked maximum's, else KH1's).
unsafe fn range(base: usize, i: usize, locks: &[Option<i32>; N_STATS]) -> (i32, i32) {
    let s = &STATS[i];
    let cap = |m: usize| locks[m].or_else(|| stat_now(base, m)).unwrap_or(s.max).clamp(s.min, s.max);
    match i {
        HP => (s.min, cap(MAX_HP)),
        MP => (s.min, cap(MAX_MP)),
        _ => (s.min, s.max),
    }
}

/// Writes `v` as stat `i` into a stats block (HP and MP kept within the block's maximums; a
/// lowered maximum brings them down with it). Level has no place there.
unsafe fn put(block: usize, i: usize, v: i32) {
    let s = &STATS[i];
    let Some(off) = s.off else { return };
    let at = |k: usize| block + STATS[k].off.unwrap_or(0);
    let v = match i {
        HP => v.min(rd::<i32>(at(MAX_HP))),
        MP => v.min(rd::<i32>(at(MAX_MP))),
        _ => v,
    }
    .max(s.min);
    wr(block + off, v);
    match i {
        HP if v > 0 => wr(block, rd::<u32>(block) & !4),
        MAX_HP if rd::<i32>(at(HP)) > v => wr(at(HP), v),
        MAX_MP if rd::<i32>(at(MP)) > v => wr(at(MP), v),
        _ => {}
    }
}

/// Writes `v` as stat `i` into Sora's save record: a base number less the equipment's bonus
/// (`own`: KH1's own stats for the record as it is), so KH1's next rebuild gives `v`.
unsafe fn put_save(record: usize, i: usize, v: i32, own: Option<&Block>) {
    let s = &STATS[i];
    let at = record + s.save;
    let byte = if s.base {
        let (Some(own), Some(off)) = (own, s.off) else { return };
        let bonus = rd::<i32>(own.0.as_ptr() as usize + off) - rd::<u8>(at) as i32;
        v - bonus
    } else {
        v
    };
    wr(at, byte.clamp(0, 255) as u8);
}

/// Stat `i` set to `v` everywhere KH1 keeps it.
unsafe fn put_all(base: usize, i: usize, v: i32) {
    for b in [live_block(), menu_block(base)].into_iter().flatten() {
        put(b, i, v);
    }
    if let Some(r) = save_record(base) {
        let own = if STATS[i].base { kh1_own() } else { None };
        put_save(r, i, v, own.as_ref());
    }
}

/// Left / right on a stat: its lock's value if locked, else KH1's, changed by `delta`; written to
/// every copy now. Whether it changed.
unsafe fn change_stat(base: usize, i: usize, delta: i32) -> bool {
    let Ok(mut locks) = LOCKS.lock() else { return false };
    let Some(now) = locks[i].or_else(|| stat_now(base, i)) else { return false };
    let (lo, hi) = range(base, i, &locks);
    let v = (now + delta).clamp(lo, hi);
    if v == now {
        return false;
    }
    if locks[i].is_some() {
        locks[i] = Some(v);
    }
    put_all(base, i, v);
    true
}

/// Cross on a stat: locked at its value now, or unlocked.
unsafe fn toggle_lock(base: usize, i: usize) {
    let Ok(mut locks) = LOCKS.lock() else { return };
    locks[i] = match locks[i] {
        Some(_) => None,
        None => stat_now(base, i),
    };
    log(&format!("debug menu: {} {}", STATS[i].name, locks[i].map_or("unlocked".to_string(), |v| format!("locked at {v}"))));
}

/// The locked stats into every copy (every frame).
unsafe fn apply_locks(base: usize) {
    let Ok(locks) = LOCKS.try_lock() else { return };
    if locks.iter().all(|l| l.is_none()) {
        return;
    }
    for b in [live_block(), menu_block(base)].into_iter().flatten() {
        for i in LOCK_ORDER {
            if let Some(v) = locks[i] {
                put(b, i, v);
            }
        }
    }
    if let Some(r) = save_record(base) {
        // (KH1's own stats once for every base number this frame.)
        let own = if LOCK_ORDER.iter().any(|&i| STATS[i].base && locks[i].is_some()) { kh1_own() } else { None };
        for i in LOCK_ORDER {
            if let Some(v) = locks[i] {
                put_save(r, i, v, own.as_ref());
            }
        }
    }
}

/// KH1's stats function filled `out` for `member` (`menu::stats`): Sora's locked stats over it.
pub unsafe fn lock_stats(member: i32, out: usize) {
    if member != 0 || out < 0x10000 {
        return;
    }
    let Ok(locks) = LOCKS.try_lock() else { return };
    for i in LOCK_ORDER {
        if let Some(v) = locks[i] {
            put(out, i, v);
        }
    }
}

// ---- weapons and abilities ----

/// KH1's inventory: one count per item id from `save + 0x499` (`equip::owned`).
const STOCK_OFF: usize = 0x499;
/// Sora's weapon kind in KH1's item table.
const SORA_WEAPON: u8 = 2;

/// Every Sora weapon in KH1's item table (empty before it is loaded).
unsafe fn sora_weapons(base: usize) -> Vec<u8> {
    (1..=255u8).filter(|&id| super::equip::kind(base, id) == Some(SORA_WEAPON)).collect()
}

/// The `done` KH1's weapon change wants: nothing to do.
unsafe extern "system" fn equip_done() {}

/// Cross on a weapon: Sora holds it (one is given first when he owns none), whatever he held
/// before (KH1's own weapon row stays refused while he holds a Dream weapon or the wooden sword,
/// so this is also the way out of those). Whether his weapon changed.
unsafe fn equip_weapon(base: usize, id: u8) -> bool {
    let Some(save) = super::menu::save_block(base) else { return false };
    let was = super::equip::sora_weapon(base);
    if was == Some(id) {
        return false;
    }
    if super::equip::owned(base, id) == 0 {
        wr(save + STOCK_OFF + id as usize, 1u8);
    }
    // (The id as a whole register: KH1 indexes its item table with all of `ecx`, and a byte argument
    // leaves the rest of it undefined: t360 crashed reading the table at exe+286737.)
    let equip: unsafe extern "system" fn(u32, unsafe extern "system" fn()) = std::mem::transmute(base + EQUIP_WEAPON_RVA);
    equip(id as u32, equip_done);
    let refresh: unsafe extern "system" fn() = std::mem::transmute(base + STATS_REFRESH_RVA);
    refresh();
    let name = |i: Option<u8>| i.map_or("none".to_string(), |i| super::equip::item_name(base, i).unwrap_or_else(|| format!("item {i:#x}")));
    log(&format!("debug menu: Sora's weapon {} ({:#04x}) -> {} ({id:#04x})", name(was), was.unwrap_or(0), name(Some(id))));
    true
}

/// Left / right (`delta`) on a weapon: its count changed (0..99, never below the copies
/// equipped). Whether it changed.
unsafe fn change_weapon(base: usize, id: u8, delta: i32) -> bool {
    let Some(save) = super::menu::save_block(base) else { return false };
    let now = super::equip::owned(base, id) as i32;
    let least = super::equip::equipped_count(base, id).min(99) as i32;
    let want = if delta != 0 { now + delta } else if now > least { least } else { now + 1 };
    let v = want.clamp(least, 99);
    if v == now {
        return false;
    }
    wr(save + STOCK_OFF + id as usize, v as u8);
    log(&format!("debug menu: {} owned {now} -> {v}", super::equip::item_name(base, id).unwrap_or_else(|| format!("item {id:#x}"))));
    true
}

/// The abilities the page lists: Sora's KH1 ones (not Donald's, Goofy's or the guests' 29..52, not
/// the glides) and ours (not KH2's Glide, the Glide style's).
fn ability_ids() -> Vec<u8> {
    use crate::growth::{kh1, ours, MENU};
    let mut ids: Vec<u8> = (1..=kh1::LAST).filter(|id| !(29..=52).contains(id) && *id != kh1::GLIDE && *id != kh1::SUPERGLIDE).collect();
    ids.extend(MENU.iter().map(|a| a.id).filter(|id| *id != ours::GLIDE));
    ids
}

/// An ability's list: (offset in the save, length, whether it's Sora's own with AP, where bit 7
/// marks a copy that's off).
fn list_of(id: u8) -> (usize, usize, bool) {
    use crate::growth::{kh1, List, MENU};
    let shared = id == kh1::HIGH_JUMP || id == kh1::MERMAID_KICK || MENU.iter().any(|a| a.id == id && a.list == List::Shared);
    if shared {
        (0x599, 10, false)
    } else {
        (0x44, 48, true)
    }
}

/// The list's bytes up to its end (the first 0).
unsafe fn list_bytes(save: usize, id: u8) -> (usize, usize, bool, Vec<u8>) {
    let (off, len, sora) = list_of(id);
    let at = save + off;
    let bytes: Vec<u8> = (0..len).map(|i| rd::<u8>(at + i)).take_while(|b| *b != 0).collect();
    (at, len, sora, bytes)
}

/// Copies of `id` in Sora's lists, and how many are on.
unsafe fn copies(save: usize, id: u8) -> (usize, usize) {
    let (_, _, sora, bytes) = list_bytes(save, id);
    let mine: Vec<u8> = bytes.into_iter().filter(|b| b & 0x7F == id).collect();
    let on = mine.iter().filter(|b| !sora || *b & 0x80 == 0).count();
    (mine.len(), on)
}

/// What the debug menu added (true) or took out (false) this session, by ability id: the mod's
/// own ability upkeep leaves those as they are (`menu::Growth::update`).
static FORCED: Mutex<Vec<(u8, bool)>> = Mutex::new(Vec::new());

/// Whether the debug menu added (`Some(true)`) or took out (`Some(false)`) ability `id`.
pub fn ability_forced(id: u8) -> Option<bool> {
    let id = id & 0x7F;
    FORCED.try_lock().ok().and_then(|f| f.iter().find(|(i, _)| *i == id).map(|(_, v)| *v))
}

fn force(id: u8, added: bool) {
    if let Ok(mut f) = FORCED.lock() {
        f.retain(|(i, _)| *i != id);
        f.push((id, added));
    }
}

/// Right (`delta` 1) adds a copy, left (-1) takes one, Cross (0) adds one if he has none, else
/// takes them all. Whether anything changed.
unsafe fn change_ability(base: usize, id: u8, delta: i32) -> bool {
    let Some(save) = super::menu::save_block(base) else { return false };
    let (at, len, sora, mut bytes) = list_bytes(save, id);
    let have = bytes.iter().filter(|b| *b & 0x7F == id).count();
    let add = delta > 0 || (delta == 0 && have == 0);
    if add {
        if bytes.len() >= len {
            log(&format!("debug menu: no room for {} (the list is full)", crate::growth::name(id)));
            return false;
        }
        // (Off in Sora's list, as KH1's own learning puts it; the shared list has no off.)
        bytes.push(if sora { id | 0x80 } else { id });
        force(id, true);
    } else {
        if have == 0 {
            return false;
        }
        let take = if delta == 0 { have } else { 1 };
        for _ in 0..take {
            // An off copy first.
            let i = bytes.iter().rposition(|b| *b & 0x7F == id && (!sora || *b & 0x80 != 0)).or_else(|| bytes.iter().rposition(|b| *b & 0x7F == id));
            if let Some(i) = i {
                bytes.remove(i);
            }
        }
        if bytes.iter().all(|b| *b & 0x7F != id) {
            force(id, false);
        }
    }
    for k in 0..len {
        wr(at + k, bytes.get(k).copied().unwrap_or(0));
    }
    let (n, on) = copies(save, id);
    log(&format!("debug menu: {} copies {have} -> {n} ({on} on)", crate::growth::name(id)));
    true
}

// ---- drawing ----

/// The font (`debugfont`): a 16 x 6 grid of 14 x 28 cells, ASCII 32..126.
const FONT_COLS: usize = 16;
const CELL_W: usize = 14;
const CELL_H: usize = 28;
const FONT_W: usize = CELL_W * FONT_COLS;
const FONT_H: usize = CELL_H * 6;
static FONT_RGBA: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
/// The font's key in the renderer's font buffer ("DBGFONT1", `fx12::draw_overlay`).
const FONT_KEY: u64 = 0x4442_4746_4F4E_5431;

const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const YELLOW: [f32; 4] = [1.0, 0.86, 0.15, 1.0];
const DIM: [f32; 4] = [0.72, 0.72, 0.72, 1.0];
const BOX: [f32; 4] = [0.22, 0.22, 0.24, 0.72];
const RULE: [f32; 4] = [0.6, 0.6, 0.62, 0.9];

/// `text`'s glyphs at (x, y), `cw` x `ch` pixels a character.
fn text(out: &mut Vec<fx12::OverlayQuad>, x: f32, y: f32, cw: f32, ch: f32, s: &str, color: [f32; 4]) {
    for (n, c) in s.chars().enumerate() {
        if c == ' ' {
            continue;
        }
        let i = if (' '..='~').contains(&c) { c as usize - 32 } else { '?' as usize - 32 };
        let (col, row) = (i % FONT_COLS, i / FONT_COLS);
        let uv = [(col * CELL_W) as f32 / FONT_W as f32, (row * CELL_H) as f32 / FONT_H as f32, ((col + 1) * CELL_W) as f32 / FONT_W as f32, ((row + 1) * CELL_H) as f32 / FONT_H as f32];
        out.push(fx12::OverlayQuad { rect: [x + n as f32 * cw, y, cw, ch], uv, color, textured: true });
    }
}

unsafe fn draw(base: usize) {
    let Some(size) = fx12::overlay_size(base) else { return };
    let (page, cur) = match MENU.lock() {
        Ok(m) => (m.page, m.cursor[m.page.slot()]),
        Err(_) => return,
    };
    let all = rows(base, page);
    // Long pages scroll: `VISIBLE` rows around the cursor.
    const VISIBLE: usize = 16;
    let cur_all = cur.min(all.len().saturating_sub(1));
    let start = if all.len() > VISIBLE { cur_all.saturating_sub(VISIBLE / 2).min(all.len() - VISIBLE) } else { 0 };
    let list: Vec<(String, Action)> = all.iter().skip(start).take(VISIBLE).cloned().collect();
    let cur = cur_all - start;
    let title = if all.len() > VISIBLE { format!("{}  ({}/{})", page.title(), cur_all + 1, all.len()) } else { page.title() };
    let (ok, no) = buttons(base);
    let hint = page.hint(mark(ok), mark(no));
    // 1080 lines = the font's own size.
    let s = (size.1 / 1080.0).max(0.5);
    let (cw, ch) = ((CELL_W as f32 * s).round(), (CELL_H as f32 * s).round());
    let line = (ch * 1.15).round();
    let pad = (16.0 * s).round();
    let chars = list.iter().map(|(t, _)| t.len()).chain([title.len(), hint.len()]).max().unwrap_or(10) as f32;
    let (x0, y0) = ((48.0 * s).round(), (48.0 * s).round());
    let w = chars * cw + pad * 2.0;
    let h = pad * 2.0 + line * (list.len() as f32 + 2.0) + pad;
    let mut q: Vec<fx12::OverlayQuad> = Vec::with_capacity(512);
    q.push(fx12::OverlayQuad { rect: [x0, y0, w, h], uv: [0.0; 4], color: BOX, textured: false });
    let mut y = y0 + pad;
    text(&mut q, x0 + pad, y, cw, ch, &title, WHITE);
    y += line;
    q.push(fx12::OverlayQuad { rect: [x0 + pad, y - (line - ch) * 0.5, w - pad * 2.0, (2.0 * s).max(1.0).round()], uv: [0.0; 4], color: RULE, textured: false });
    y += pad * 0.5;
    for (i, (t, _)) in list.iter().enumerate() {
        text(&mut q, x0 + pad, y, cw, ch, t, if i == cur.min(list.len().saturating_sub(1)) { YELLOW } else { WHITE });
        y += line;
    }
    y += pad * 0.5;
    text(&mut q, x0 + pad, y, cw, ch, &hint, DIM);
    let rgba = FONT_RGBA.get_or_init(super::debugfont::decode);
    let font = fx12::MeshTexture { key: FONT_KEY, width: FONT_W as u32, height: FONT_H as u32, rgba };
    fx12::draw_overlay(base, &q, &font, size);
}
