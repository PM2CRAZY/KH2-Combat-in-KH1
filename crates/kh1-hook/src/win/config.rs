//! Our Config menu settings (the user: switch between KH1's and KH2's camera in the game's Config
//! menu; then "toggle what sora uses for his ground and aerial moves"): Camera Style, Physics
//! (the user: "BUG: In the config there's not a setting to switch between KH1 and KH2 physics"),
//! Base Combo, Aerial Base Combo, Magic, Aerial Magic..., each KH1 / KH2.
//! `docs/kh1-config-menu.md` has KH1's side (§6 for this layout).
//!
//! The Config menu rework (the tester: "package all of kh1 in kh2's settings ... into one
//! clickable menu in the config menu on the first page"): page 1 is KH1's rows plus a "KH1 in
//! KH2" button above "Default settings" (answer 4b), the list drawn at 92% with KH1's own window
//! scale (answer 3a) so the ninth row fits. The button opens KH1 windows of our own, the way the
//! Config rows 0..2 open their pad diagrams (KH1's window manager, sounds, cursor):
//!
//! - KH1 in KH2: Camera, Combat, Movement, Misc;
//! - Camera: Camera Style, L2 Camera Hold, L2 Stick Menu; Combat: Physics, Base Combo, Aerial Base Combo, Hit Reactions, Knock-back Numbers, Medium Hits, Criticals, Recoil, and Magic, its own list
//!   (answer 1): Fire, Blizzard, Thunder, Cure, Air Physics and Animations, its own list (Blizzard, Thunder, Cure,
//!   Gravity, Stop, Aero: m19 of kh2-effects-in-kh1s-own-effect-system); Movement: Jump, Glide, Dodge Roll, Misc Animations (Guard Repel is part of Combat's Recoil); Misc:
//!   Battle Quotes, Gummi Routes, Debug Menu (answer 5a).
//!
//! Only the settings that were in the Config menu (answer 2a); "Default settings" leaves ours alone
//! (answer 6b). Backing out of a list re-creates its parent with the cursor on the row it came
//! from: KH1's Config title tab (`0x1402FADB0`) does that for the Config list when it regains the
//! focus, and its hook does it for ours.
//!
//! The choices are kept in `kh1-kh2.ini` (`camera`, `physics`, `ground_combo`, `air_combo`,
//! `magic_fire`, `anim_gravity`... = kh1 / kh2; `debug_menu` off / on, t354), not the save.
//!
//! Steam only (the windows' calls are Steam addresses); the text lookup is in for both builds.

use super::addr::{Build, Rva};
use super::styles::Kind;
use super::{camera, hook, log, styles};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};

const TEXT_RVA: Rva = Rva(0x2CE7C0);
/// Its first instruction (`mov rax, [rip -> TEXT_TABLE_RVA]`; Epic: 48 8B 05 89 CE B4 02).
fn text_prologue() -> [u8; 7] {
    super::addr::mov_rax_rip(TEXT_RVA, TEXT_TABLE_RVA)
}
const TEXT_TABLE_RVA: Rva = Rva(0x2E1B650);
const TEXT_BASE_RVA: Rva = Rva(0x2E1B648);
const UPDATE_RVA: Rva = Rva(0x2F7DD0);
const UPDATE_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18];
const HELP_RVA: Rva = Rva(0x2F7D60);
const HELP_PROLOGUE: [u8; 10] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x57, 0x48, 0x83, 0xEC, 0x40];
/// The Config window definition's row count and visible rows (u16).
const ROWS_RVA: Rva = Rva(0x5470B6);
const VISIBLE_RVA: Rva = Rva(0x5470B8);
/// Per-row help ids (8 u32); the ninth reads the next table's first entry (row 0's max value,
/// read only by the update's left / right, never during the help call).
const HELP_IDS_RVA: Rva = Rva(0x5470C0);
/// Value panel styles by value: selected row, other rows (left, right).
const STYLES_SELECTED_RVA: Rva = Rva(0x547130);
const STYLES_OTHER_RVA: Rva = Rva(0x547140);
/// The Config list's row under the cursor (the help window reads it; the title tab re-creates the
/// list on it).
const ROW_RVA: Rva = Rva(0x2E998F8);
/// Row height and gap (shared by every menu).
const ROW_H_RVA: Rva = Rva(0x546050);
const ROW_GAP_RVA: Rva = Rva(0x546058);
/// Menu pad: `+8` pressed this frame, `+0x10` pressed with repeat.
const PAD_PTR_RVA: Rva = Rva(0x2E96320);
/// Confirm mask; the cancel mask is the u32 after it (Steam `0x1405456A4`).
const CONFIRM_MASK_RVA: Rva = Rva(0x546390);
const LEFT: u32 = 0x8000_0080;
const RIGHT: u32 = 0x2000_0020;
/// Up (the d-pad's and the stick's bits, as left / right).
const UP: u32 = 0x1000_0010;
/// Nonzero when the window has the focus (`0x1402E5220`).
const FOCUS_RVA: Rva = Rva(0x2E2E40);
/// Menu sounds (`0x14017A040`): 1 move / toggle, 2 open, 3 back, 4 buzzer.
const SOUND_RVA: Rva = Rva(0x177F60);
const PANEL_RVA: Rva = Rva(0x2D5590);
/// `push rbx; sub rsp, 0x50`.
const PANEL_PROLOGUE: [u8; 6] = [0x40, 0x53, 0x48, 0x83, 0xEC, 0x50];
/// The list cursor's row (`(win+0xFE + 8 + (win+0xFC & 0xF)) >> 4`).
const CURSOR_ROW_RVA: Rva = Rva(0x2ED030);
const DECOR_RVA: Rva = Rva(0x2F0740);

/// Steam-only KH1 menu calls (Checked (decomp) and (exe)), used only when `Build::current()` is
/// Steam.
/// The list cursor's move (up / down, wrapping on a fresh press) and the glove's target.
const CURSOR_MOVE_STEAM: usize = 0x2EF670;
const GLOVE_STEAM: usize = 0x2D9920;
/// The glow's target set from a point: `(frames, x, y)` (`0x1402D9860`, Checked (decomp)): the
/// glint `0x1402D9B40` draws (30 x 30 around the point) slides there over `frames`. The window
/// version above (`0x1402D9920`) puts it at the selected label's top-right corner: x = window x +
/// 11 + 142, y = window y + 71 + row x the window's row pitch (`+0xF0`, from the def's `+0x22`).
/// KH1's Config list def has a pitch of 0 (Checked (exe): `0x1405463A0 +0x22` = 0), so in KH1
/// itself the glint stays on the top row; ours give it the rows' pitch.
const GLOW_AT_STEAM: usize = 0x2D9860;
/// A window def's row pitch (window `+0xF0` = it x 16).
const DEF_PITCH: usize = 0x22;
/// Window framework: create `(def, open type) -> window` (0 when the 24-window pool is full; it
/// becomes the focus), close it and the windows after it (`+0x20`), the last window after it,
/// link `a +0x20 = b` (next) / `a +0x18 = b` (parent), bring to the front.
const CREATE_STEAM: usize = 0x2E5A40;
const CLOSE_STEAM: usize = 0x2E3D10;
const CHAIN_END_STEAM: usize = 0x2E5510;
const LINK_NEXT_STEAM: usize = 0x2E2DE0;
const LINK_PARENT_STEAM: usize = 0x2E2E40;
const FRONT_STEAM: usize = 0x2E5E40;
/// The Config title tab's update (def `0x140546190`): with the focus it creates the Config list
/// (cursor on `ROW_RVA`) and its help line; then it draws the tab. `push rdi; sub rsp, 0x40`.
const TITLE_STEAM: usize = 0x2FADB0;
const TITLE_PROLOGUE: [u8; 6] = [0x40, 0x57, 0x48, 0x83, 0xEC, 0x40];
/// Window definitions (0x30 bytes): the Config list (copied for ours) and its help line.
const LIST_DEF_STEAM: usize = 0x5463A0;
const HELP_DEF_STEAM: usize = 0x546160;
/// The Config labels (8 u32); row 7's is "Default settings".
const LABELS_STEAM: usize = 0x546410;
/// The finger: `(win, x, y, 0, 0, frame, 0, 0x80808080, 0x80808080, 0)`; its size from the
/// object at `FINGER_INFO_STEAM` (`+0x150` width, `+0x152` height, x16).
const FINGER_STEAM: usize = 0x2EEEE0;
const FINGER_INFO_STEAM: usize = 0x2E91610;
/// The window pool: 24 windows of 0x118 bytes, `+0xB8` the state (0 free).
const POOL_STEAM: usize = 0x2E92780;
const POOL_LEN: usize = 24;
const WIN_SIZE: usize = 0x118;

/// Window fields (Checked (decomp)).
const WIN_PARENT: usize = 0x18;
const WIN_CLOSING: usize = 0x68;
const WIN_DEF: usize = 0x80;
const WIN_STATE: usize = 0xB8;
const WIN_ROW16: usize = 0xFE;
/// The open / close type KH1's Config windows are created with (a slide in x).
const OPEN_TYPE: i32 = 6;

/// KH1's own window scale (Steam, Checked (decomp)): with bit 0 of a window's `+0x6C` set, every
/// primitive the window draws (box `0x1402F2BE0`, text `0x1402EBCD0`, arrows `0x1402F29B0`,
/// finger `0x1402EEF50`) is scaled by `+0xA0` (x) / `+0xA4` (y) around its anchor (`+0xD8`:
/// 0 the centre, 1 the top edge at the centre; `0x1402E4F40` / `0x1402E4FB0`). KH1's open / close
/// zooms (`0x1402E66D0`) use it; the Config list's slide (type 6) doesn't touch these fields.
/// The tester (answer 3a): the whole Config list about 8% smaller, text and boxes together, so a
/// ninth row fits above the help box (row 8's bottom: 0.92 x 321 = 295 < 300).
const LIST_SCALE: f32 = 0.92;
const WIN_FLAGS2: usize = 0x6C;
const WIN_SCALE_X: usize = 0xA0;
const WIN_SCALE_Y: usize = 0xA4;
const WIN_ANCHOR: usize = 0xD8;
const ANCHOR_TOP: u32 = 1;

/// The Config list drawn at `LIST_SCALE` from the top of the window (the screen's top, so the first
/// row moves up 6 and the help box stays clear). Set every update, before it draws.
unsafe fn scale_list(win: usize) {
    if Build::current() != Build::Steam {
        return;
    }
    wr(win + WIN_SCALE_X, LIST_SCALE);
    wr(win + WIN_SCALE_Y, LIST_SCALE);
    wr(win + WIN_ANCHOR, ANCHOR_TOP);
    wr(win + WIN_FLAGS2, rd::<u32>(win + WIN_FLAGS2) | 1);
}

/// Page 1: KH1's rows 0..6, our button (row 7), KH1's "Default settings" (row 8; KH1's row 7).
const BUTTON_ROW: i32 = 7;
const DEFAULT_ROW: i32 = 8;
const PAGE_ROWS: u16 = 9;
const KH1_DEFAULT_ROW: i32 = 7;

/// What one setting switches.
#[derive(Clone, Copy)]
enum Row {
    Camera,
    /// L2 Camera Hold (Off | On, `camera::L2_HOLD`).
    L2Hold,
    /// L2 Stick Menu (Off | On, `camera::L2_DPAD`): L2 + the right stick is the d-pad.
    L2Dpad,
    /// The physics setting (`shared::physics::rule`): KH1's or KH2's numbers for every move.
    Physics,
    Style(Kind),
    /// Gummi Routes (Vanilla | Skip).
    Gummi,
    /// The debug menu (Off | On, `debugmenu`).
    Debug,
    /// Medium Hits (Blow | Flinch, `hitreact::medium_flinch`).
    MediumHits,
    /// Criticals (Off | On, `kh2damage::criticals`).
    Criticals,
}

/// A row of one of our lists: a setting (label, left value, right value, help, what it switches),
/// or a button that opens another list (label, help).
#[derive(Clone, Copy)]
enum Item {
    Set([u32; 4], Row),
    Open(u32, u32, List),
    /// A setting with three values (label, the three values left to right, help).
    Pick([u32; 5], Pick),
}

/// What a three-value row switches.
#[derive(Clone, Copy)]
enum Pick {
    /// Knock-back Numbers (KH1 | Scaled | KH2, `hitreact::Numbers`).
    KnockNumbers,
}

fn pick_value(what: Pick) -> usize {
    match what {
        Pick::KnockNumbers => super::hitreact::numbers() as usize,
    }
}

fn pick_set(what: Pick, v: usize) {
    match what {
        Pick::KnockNumbers => super::hitreact::set_numbers(super::hitreact::Numbers::ALL[v.min(2)]),
    }
}

/// Our lists (each its own KH1 window definition).
#[derive(Clone, Copy, PartialEq, Eq)]
enum List {
    Top,
    Camera,
    Combat,
    Magic,
    Movement,
    Misc,
    /// Magic > Animations: each KH1 spell's cast animation (the tester, m19).
    MagicAnim,
}
const LISTS: [List; 7] = [List::Top, List::Camera, List::Combat, List::Magic, List::Movement, List::Misc, List::MagicAnim];

impl List {
    fn index(self) -> usize {
        self as usize
    }
    /// The list backing out returns to (None: KH1's Config list, on the button).
    fn parent(self) -> Option<List> {
        match self {
            List::Top => None,
            List::Magic => Some(List::Combat),
            List::MagicAnim => Some(List::Magic),
            _ => Some(List::Top),
        }
    }
    fn items(self) -> &'static [Item] {
        match self {
            List::Top => &TOP,
            List::Camera => &CAMERA,
            List::Combat => &COMBAT,
            List::Magic => &MAGIC,
            List::Movement => &MOVEMENT,
            List::Misc => &MISC,
            List::MagicAnim => &MAGIC_ANIM,
        }
    }
}

const KH: [u32; 2] = [0x7F00_0002, 0x7F00_0003];
const fn kh(label: u32, help: u32) -> [u32; 4] {
    [label, KH[0], KH[1], help]
}
const TOP: [Item; 4] = [
    Item::Open(0x7F00_0203, 0x7F00_0204, List::Camera),
    Item::Open(0x7F00_0205, 0x7F00_0206, List::Combat),
    Item::Open(0x7F00_0209, 0x7F00_020A, List::Movement),
    Item::Open(0x7F00_020B, 0x7F00_020C, List::Misc),
];
const CAMERA: [Item; 3] = [
    Item::Set(kh(0x7F00_0001, 0x7F00_0004), Row::Camera),
    Item::Set([0x7F00_0121, 0x7F00_011D, 0x7F00_011E, 0x7F00_0122], Row::L2Hold),
    Item::Set([0x7F00_0123, 0x7F00_011D, 0x7F00_011E, 0x7F00_0124], Row::L2Dpad),
];
const COMBAT: [Item; 9] = [
    Item::Set(kh(0x7F00_0119, 0x7F00_011A), Row::Physics),
    Item::Set(kh(0x7F00_0101, 0x7F00_0102), Row::Style(Kind::GroundCombo)),
    Item::Set(kh(0x7F00_0103, 0x7F00_0104), Row::Style(Kind::AirCombo)),
    Item::Set(kh(0x7F00_0140, 0x7F00_0141), Row::Style(Kind::HitReactions)),
    Item::Pick([0x7F00_0142, 0x7F00_0002, 0x7F00_0144, 0x7F00_0003, 0x7F00_0143], Pick::KnockNumbers),
    Item::Set([0x7F00_0145, 0x7F00_0147, 0x7F00_0148, 0x7F00_0146], Row::MediumHits),
    Item::Set([0x7F00_0149, 0x7F00_011D, 0x7F00_011E, 0x7F00_014A], Row::Criticals),
    Item::Set(kh(0x7F00_0172, 0x7F00_0173), Row::Style(Kind::Recoil)),
    Item::Open(0x7F00_0207, 0x7F00_0208, List::Magic),
];
// (The tester, thread kh2-effects-in-kh1s-own-effect-system m19: a row per spell for what is
// cast, "we'll remove ground magic"; each KH1 spell's animation in its own list, so both lists
// keep KH1's full-size rows on screen.)
const MAGIC: [Item; 6] = [
    Item::Set(kh(0x7F00_0180, 0x7F00_0181), Row::Style(Kind::Fire)),
    Item::Set(kh(0x7F00_0182, 0x7F00_0183), Row::Style(Kind::Blizzard)),
    Item::Set(kh(0x7F00_0184, 0x7F00_0185), Row::Style(Kind::Thunder)),
    Item::Set(kh(0x7F00_011F, 0x7F00_0120), Row::Style(Kind::Cure)),
    Item::Set(kh(0x7F00_018F, 0x7F00_0190), Row::Style(Kind::AirPhysics)),
    Item::Open(0x7F00_020D, 0x7F00_020E, List::MagicAnim),
];
const MAGIC_ANIM: [Item; 6] = [
    Item::Set(kh(0x7F00_0182, 0x7F00_0186), Row::Style(Kind::BlizzardAnim)),
    Item::Set(kh(0x7F00_0184, 0x7F00_0187), Row::Style(Kind::ThunderAnim)),
    Item::Set(kh(0x7F00_011F, 0x7F00_0188), Row::Style(Kind::CureAnim)),
    Item::Set(kh(0x7F00_0189, 0x7F00_018A), Row::Style(Kind::GravityAnim)),
    Item::Set(kh(0x7F00_018B, 0x7F00_018C), Row::Style(Kind::StopAnim)),
    Item::Set(kh(0x7F00_018D, 0x7F00_018E), Row::Style(Kind::AeroAnim)),
];
const MOVEMENT: [Item; 4] = [
    Item::Set(kh(0x7F00_0109, 0x7F00_010A), Row::Style(Kind::Jump)),
    Item::Set(kh(0x7F00_010B, 0x7F00_010C), Row::Style(Kind::Glide)),
    Item::Set(kh(0x7F00_010D, 0x7F00_010E), Row::Style(Kind::DodgeRoll)),
    // (Guard Repel is part of the Recoil row now, Combat: the tester, thread
    // kh2-recoil-and-parry-system, "we need to also combine guard repel and recoil into the same
    // config".)
    Item::Set(kh(0x7F00_0115, 0x7F00_0116), Row::Style(Kind::MiscAnims)),
    // (Player Model, off for now, s259, the user: "comment it out for now":)
    // Item::Set(kh(0x7F00_0117, 0x7F00_0118), Row::Style(Kind::PlayerModel)),
];
const MISC: [Item; 3] = [
    Item::Set(kh(0x7F00_010F, 0x7F00_0110), Row::Style(Kind::BattleQuotes)),
    Item::Set([0x7F00_0111, 0x7F00_0113, 0x7F00_0114, 0x7F00_0112], Row::Gummi),
    Item::Set([0x7F00_011B, 0x7F00_011D, 0x7F00_011E, 0x7F00_011C], Row::Debug),
];

/// The page-1 button: label, help (the help carries the mod's version on its second line: the
/// user, a version shown in the Config menu, to tell which build a PC runs).
const BUTTON_LABEL: u32 = 0x7F00_0201;
const BUTTON_HELP: u32 = 0x7F00_0202;
const STRINGS: [(u32, &str); 80] = [
    (0x7F00_0201, "KH1 in KH2"),
    (0x7F00_0202, "Choose KH1's or KH2's camera, combat, movement and more."),
    (0x7F00_0203, "Camera"),
    (0x7F00_0204, "Choose KH1's or KH2's camera."),
    (0x7F00_0205, "Combat"),
    (0x7F00_0206, "Choose KH1's or KH2's physics, combos and magic for Sora."),
    (0x7F00_0207, "Magic"),
    (0x7F00_0208, "Choose KH1's or KH2's version of each spell for Sora."),
    (0x7F00_020D, "Animations"),
    (0x7F00_020E, "Choose KH1's or KH2's cast animation for each KH1 spell."),
    (0x7F00_0180, "Fire"),
    (0x7F00_0181, "Cast KH1's or KH2's Fire, on the ground and in the air.\nIts animation is its own. Drive Forms keep their own."),
    (0x7F00_0182, "Blizzard"),
    (0x7F00_0183, "Cast KH1's or KH2's Blizzard, on the ground and in the air.\nDrive Forms keep their own."),
    (0x7F00_0184, "Thunder"),
    (0x7F00_0185, "Cast KH1's or KH2's Thunder, on the ground and in the air.\nDrive Forms keep their own."),
    (0x7F00_0186, "KH1's Blizzard on KH1's or KH2's cast animation.\nKH2's Blizzard always plays KH2's."),
    (0x7F00_0187, "KH1's Thunder on KH1's or KH2's cast animation.\nKH2's Thunder always plays KH2's."),
    (0x7F00_0188, "KH1's Cure on KH1's or KH2's cast animation.\nKH2's Cure always plays KH2's."),
    (0x7F00_018F, "Air Physics"),
    (0x7F00_0190, "KH1's or KH2's physics while Sora casts in the air,\nfor every spell. Drive Forms keep their own."),
    (0x7F00_0189, "Gravity"),
    (0x7F00_018A, "KH1's Gravity on KH1's or KH2's cast animation\n(KH2's Thunder cast)."),
    (0x7F00_018B, "Stop"),
    (0x7F00_018C, "KH1's Stop on KH1's or KH2's cast animation\n(KH2's Cure cast)."),
    (0x7F00_018D, "Aero"),
    (0x7F00_018E, "KH1's Aero on KH1's or KH2's cast animation\n(KH2's Cure cast)."),
    (0x7F00_0209, "Movement"),
    (0x7F00_020A, "Choose KH1's or KH2's jump, glide, dodge roll and animations for Sora."),
    (0x7F00_020B, "Misc"),
    (0x7F00_020C, "Battle quotes, Gummi routes and the debug menu."),
    (0x7F00_0001, "Camera Style"),
    (0x7F00_0002, "KH1"),
    (0x7F00_0003, "KH2"),
    (0x7F00_0004, "Choose KH1's camera or KH2's field camera."),
    (0x7F00_0121, "L2 Camera Hold"),
    (0x7F00_0122, "While L2 is held, the right stick doesn't move KH2's camera\nand R3 doesn't reset it, as in KH2."),
    (0x7F00_0123, "L2 Stick Menu"),
    (0x7F00_0124, "While L2 is held, the right stick moves the command menu\nlike the d-pad, with either camera, as in KH2."),
    (0x7F00_0119, "Physics"),
    (0x7F00_011A, "Choose KH1's or KH2's physics for Sora's moves. Air casts:\nCombat > Magic > Air Physics. Drive Forms keep their own."),
    (0x7F00_0101, "Base Combo"),
    (0x7F00_0102, "Choose KH1's or KH2's ground combo for Sora.\nDrive Forms keep their own."),
    (0x7F00_0103, "Aerial Base Combo"),
    (0x7F00_0104, "Choose KH1's or KH2's aerial combo for Sora.\nDrive Forms keep their own."),
    (0x7F00_0140, "Hit Reactions"),
    (0x7F00_0141, "Choose KH1's or KH2's animations when Sora is hit.\nDrive Forms keep their own."),
    (0x7F00_0142, "Knock-back Numbers"),
    (0x7F00_0143, "How far KH2's hit reactions push Sora: KH1's numbers,\nKH2's scaled by the damage, or KH2's own."),
    (0x7F00_0144, "Scaled"),
    (0x7F00_0145, "Medium Hits"),
    (0x7F00_0146, "With KH2's hit reactions, a hit of 20-29% of Sora's HP\nknocks him away (Blow) or only flinches him."),
    (0x7F00_0147, "Blow"),
    (0x7F00_0148, "Flinch"),
    (0x7F00_0149, "Criticals"),
    (0x7F00_014A, "KH1's critical hits on Sora's attacks (On), or none, as in KH2.\nCritical Plus only works with them On."),
    (0x7F00_0172, "Recoil"),
    (0x7F00_0173, "KH1's or KH2's recoil, parry and guard repel when Sora's\nattack or guard meets an enemy's, for every move."),
    (0x7F00_011F, "Cure"),
    (0x7F00_0120, "Cast KH1's or KH2's Cure, on the ground and in the air.\nDrive Forms keep their own."),
    (0x7F00_0109, "Jump"),
    (0x7F00_010A, "Choose KH1's or KH2's jump for Sora.\nDrive Forms keep their own."),
    (0x7F00_010B, "Glide"),
    (0x7F00_010C, "Choose KH1's or KH2's glide for Sora.\nDrive Forms keep their own."),
    (0x7F00_010D, "Dodge Roll"),
    (0x7F00_010E, "Choose KH1's or KH2's dodge roll for Sora.\nDrive Forms keep their own."),
    (0x7F00_0115, "Misc Animations"),
    (0x7F00_0116, "Choose KH1's or KH2's animations for standing, walking,\nrunning and items."),
    (0x7F00_0117, "Player Model"),
    (0x7F00_0118, "Draw Sora with KH1's model or KH2's.\nDrive Forms and Keyblades follow."),
    (0x7F00_010F, "Battle Quotes"),
    (0x7F00_0110, "Choose KH1's or KH2's battle quotes for Sora."),
    (0x7F00_0111, "Gummi Routes"),
    (0x7F00_0112, "Fly the Gummi Ship between worlds, or skip the flights."),
    (0x7F00_0113, "Vanilla"),
    (0x7F00_0114, "Skip"),
    (0x7F00_011B, "Debug Menu"),
    (0x7F00_011C, "Turn on the debug menu: L1 + R1 + Triangle opens and closes it."),
    (0x7F00_011D, "Off"),
    (0x7F00_011E, "On"),
];

/// A setting's value: right (KH2, true) or left (KH1).
fn value(what: Row) -> bool {
    use super::shared::physics;
    match what {
        Row::Camera => camera::KH2.load(Ordering::Relaxed),
        Row::L2Hold => camera::L2_HOLD.load(Ordering::Relaxed),
        Row::L2Dpad => camera::L2_DPAD.load(Ordering::Relaxed),
        Row::Physics => physics::rule() == physics::Rule::Kh2,
        Row::Style(k) => styles::setting(k),
        Row::Gummi => super::gummi::SKIP.load(Ordering::Relaxed),
        Row::Debug => super::debugmenu::enabled(),
        Row::MediumHits => super::hitreact::medium_flinch(),
        Row::Criticals => super::kh2damage::criticals(),
    }
}

fn set(what: Row, v: bool) {
    use super::shared::physics;
    match what {
        Row::Camera => {
            camera::KH2.store(v, Ordering::Relaxed);
            super::settings::save("camera", if v { "kh2" } else { "kh1" });
            log(&format!("Config menu: camera style {}", if v { "KH2" } else { "KH1" }));
        }
        Row::L2Hold => camera::set_l2_hold(v),
        Row::L2Dpad => camera::set_l2_dpad(v),
        // (Live: the next physics step reads it; saved and logged by `set_rule`.)
        Row::Physics => physics::set_rule(if v { physics::Rule::Kh2 } else { physics::Rule::Kh1 }),
        Row::Style(k) => {
            styles::set(k, v);
            // (The shared list follows the Glide style at once, `menu::glide_style_now`.)
            if k == Kind::Glide {
                unsafe { super::menu::glide_style_now(BASE.load(Ordering::Relaxed)) };
            }
        }
        Row::Gummi => super::gummi::set(v),
        Row::Debug => super::debugmenu::set_enabled(v),
        Row::MediumHits => super::hitreact::set_medium_flinch(v),
        Row::Criticals => super::kh2damage::set_criticals(v),
    }
}

static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG_UPDATE: AtomicUsize = AtomicUsize::new(0);
static ORIG_HELP: AtomicUsize = AtomicUsize::new(0);
static ORIG_TEXT: AtomicUsize = AtomicUsize::new(0);
static ORIG_PANEL: AtomicUsize = AtomicUsize::new(0);
static ORIG_TITLE: AtomicUsize = AtomicUsize::new(0);
/// KH1's Config update runs: its "Default settings" label (drawn by us one row lower) is held back.
static HIDE_DEFAULT: AtomicBool = AtomicBool::new(false);
/// KH1's "Default settings" label id (its label table's row 7).
static DEFAULT_LABEL: AtomicU32 = AtomicU32::new(0);
static TEXT: std::sync::OnceLock<Vec<(u32, usize)>> = std::sync::OnceLock::new();

/// Our lists' window definitions (KH1's Config list's, with our update and row count).
#[repr(C, align(16))]
struct Defs([[u8; 0x30]; LISTS.len()]);
static mut DEFS: Defs = Defs([[0; 0x30]; LISTS.len()]);
/// Each list's row (the cursor's after its move, as KH1 keeps `ROW_RVA`), and the row to come back
/// to after a list opened from it closes.
static CUR_ROW: [AtomicI32; LISTS.len()] = [const { AtomicI32::new(0) }; LISTS.len()];
static SAVED_ROW: [AtomicI32; LISTS.len()] = [const { AtomicI32::new(0) }; LISTS.len()];
/// A list was backed out of: the title tab, when it has the focus again, re-creates this one
/// (index into `LISTS`) instead of KH1's Config list. -1: none.
static PENDING: AtomicI32 = AtomicI32::new(-1);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

fn def_of(list: List) -> usize {
    unsafe { std::ptr::addr_of!(DEFS.0[list.index()]) as usize }
}

/// The list a window shows (by its definition), if it's one of ours.
unsafe fn list_of(win: usize) -> Option<List> {
    let base = BASE.load(Ordering::Relaxed);
    let pool = base + POOL_STEAM;
    if win < pool || win >= pool + POOL_LEN * WIN_SIZE {
        return None;
    }
    let def = rd::<usize>(win + WIN_DEF);
    LISTS.iter().copied().find(|l| def_of(*l) == def)
}

pub unsafe fn install() {
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    TEXT.get_or_init(|| {
        STRINGS
            .iter()
            .map(|(id, s)| {
                let s = if *id == BUTTON_HELP { format!("{s}\nKH2 Combat version {}", crate::VERSION) } else { s.to_string() };
                (*id, Box::leak(super::menu::encode(&s).into_boxed_slice()).as_ptr() as usize)
            })
            .collect()
    });
    // The text lookup carries our strings for the shortcuts too (`shortcuts.rs`).
    if hook::install(base + TEXT_RVA, &text_prologue(), text as *const () as usize, &ORIG_TEXT, "menu text lookup").is_none() {
        return;
    }
    TEXT_HOOKED.store(true, Ordering::Release);
    if !super::settings::get().config_row {
        log("Config menu: no rows of ours (config_row = off)");
        return;
    }
    if Build::current() != Build::Steam {
        log("Config menu: the KH1 in KH2 menu is Steam only: no rows of ours (the ini switches)");
        return;
    }
    if rd::<u16>(base + ROWS_RVA) != 8 || rd::<u16>(base + VISIBLE_RVA) != 8 {
        log(&format!("Config menu: unexpected row counts {} / {}: no rows of ours", rd::<u16>(base + ROWS_RVA), rd::<u16>(base + VISIBLE_RVA)));
        return;
    }
    DEFAULT_LABEL.store(rd::<u32>(base + LABELS_STEAM + KH1_DEFAULT_ROW as usize * 4), Ordering::Relaxed);
    // Our definitions: KH1's Config list's (update `+0`, rows `+0x26`, visible rows `+0x28`).
    for l in LISTS {
        let d = &mut *std::ptr::addr_of_mut!(DEFS.0[l.index()]);
        std::ptr::copy_nonoverlapping((base + LIST_DEF_STEAM) as *const u8, d.as_mut_ptr(), 0x30);
        d[0..8].copy_from_slice(&(list_update as *const () as usize as u64).to_le_bytes());
        let n = l.items().len() as u16;
        d[0x26..0x28].copy_from_slice(&n.to_le_bytes());
        d[0x28..0x2A].copy_from_slice(&n.to_le_bytes());
        // The rows' pitch (KH1's own row step, as the update draws them), so the glint follows
        // the cursor (the tester: "the glowing cursor ... stays at the top").
        let pitch = (rd::<i32>(base + ROW_H_RVA) + rd::<i32>(base + ROW_GAP_RVA)) as i16;
        d[DEF_PITCH..DEF_PITCH + 2].copy_from_slice(&pitch.to_le_bytes());
    }
    if hook::install(base + HELP_RVA, &HELP_PROLOGUE, help as *const () as usize, &ORIG_HELP, "Config help line").is_none() {
        return;
    }
    // (Before the update's hook: without it KH1's Default label would show over our button.)
    if hook::install(base + PANEL_RVA, &PANEL_PROLOGUE, panel as *const () as usize, &ORIG_PANEL, "menu panel (Config page)").is_none() {
        return;
    }
    if hook::install(base + TITLE_STEAM, &TITLE_PROLOGUE, title as *const () as usize, &ORIG_TITLE, "Config title tab").is_none() {
        return;
    }
    if hook::install(base + UPDATE_RVA, &UPDATE_PROLOGUE, update as *const () as usize, &ORIG_UPDATE, "Config menu").is_none() {
        return;
    }
    let mut old = 0u32;
    super::sys::VirtualProtect((base + ROWS_RVA) as *mut _, 4, 0x04, &mut old);
    wr(base + ROWS_RVA, PAGE_ROWS);
    wr(base + VISIBLE_RVA, PAGE_ROWS);
    log("Config menu: the KH1 in KH2 button added (Camera, Combat, Movement, Misc)");
}

/// KH1's sysmsg lookup (its own instructions, re-done here: its first one is RIP-relative, so
/// the original can't be called), with our ids on top.
unsafe extern "system" fn text(id: u32) -> usize {
    if let Some(t) = TEXT.get().and_then(|t| t.iter().find(|(i, _)| *i == id)) {
        return t.1;
    }
    if id == super::shortcuts::ID_LABEL4 {
        return super::shortcuts::label4(|i| kh1_text(i));
    }
    if let Some(p) = super::reflect::text(id) {
        return p;
    }
    kh1_text(id)
}

/// Whether our text lookup is in (our ids can be used).
pub static TEXT_HOOKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// KH1's own sysmsg text for `id`.
pub unsafe fn kh1_text(id: u32) -> usize {
    let base = BASE.load(Ordering::Relaxed);
    let table = rd::<u64>(base + TEXT_TABLE_RVA) as usize;
    let at = rd::<u64>(base + TEXT_BASE_RVA) as usize;
    let entry = table.wrapping_add(id.wrapping_sub(1) as usize * 2);
    // An id past the table (one of ours read from a table KH1 has only three of) reads as
    // empty instead of wild memory.
    if id == 0 || id >= 0x10000 || (id > 0x2000 && !super::crash::readable(entry)) {
        static EMPTY: [u8; 1] = [0];
        return EMPTY.as_ptr() as usize;
    }
    at.wrapping_add(rd::<u16>(entry) as usize)
}

/// The help line (KH1's, one per list): it reads its id from the Config table by the Config row;
/// under one of our lists the id of that list's row, on page 1 our button's or (row 8) KH1's
/// Default's. The slot holds it only for the call.
unsafe extern "system" fn help(win: usize) {
    let base = BASE.load(Ordering::Relaxed);
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_HELP.load(Ordering::Acquire));
    let row = rd::<i32>(base + ROW_RVA);
    let id = if let Some(list) = list_of(rd::<usize>(win + WIN_PARENT)) {
        match list.items().get(CUR_ROW[list.index()].load(Ordering::Relaxed) as usize) {
            Some(Item::Set(ids, _)) => Some(ids[3]),
            Some(Item::Pick(ids, _)) => Some(ids[4]),
            Some(Item::Open(_, help, _)) => Some(*help),
            None => None,
        }
    } else if row == BUTTON_ROW {
        Some(BUTTON_HELP)
    } else if row == DEFAULT_ROW {
        Some(rd::<u32>(base + HELP_IDS_RVA + KH1_DEFAULT_ROW as usize * 4))
    } else {
        None
    };
    let Some(id) = id else { return f(win) };
    if !(0..PAGE_ROWS as i32).contains(&row) {
        return f(win);
    }
    let at = base + HELP_IDS_RVA + row as usize * 4;
    let keep = rd::<u32>(at);
    wr(at, id);
    f(win);
    wr(at, keep);
}

/// KH1's panel draw (a label or a value of a menu row): KH1's "Default settings" label held back
/// while its update runs (we draw it on row 8).
unsafe extern "system" fn panel(win: usize, style: i32, a: i32, b: i32, x: i32, y: i32, w: i32, h: i32, id: u32) {
    if HIDE_DEFAULT.load(Ordering::Relaxed) && id == DEFAULT_LABEL.load(Ordering::Relaxed) {
        return;
    }
    let f: unsafe extern "system" fn(usize, i32, i32, i32, i32, i32, i32, i32, u32) = std::mem::transmute(ORIG_PANEL.load(Ordering::Acquire));
    f(win, style, a, b, x, y, w, h, id)
}

unsafe fn sound(id: i32) {
    let base = BASE.load(Ordering::Relaxed);
    let f: unsafe extern "system" fn(i32) = std::mem::transmute(base + SOUND_RVA);
    f(id)
}
unsafe fn focus(win: usize) -> bool {
    let f: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(BASE.load(Ordering::Relaxed) + FOCUS_RVA);
    f(win) != 0
}
unsafe fn cursor(win: usize) -> i32 {
    let f: unsafe extern "system" fn(usize) -> i32 = std::mem::transmute(BASE.load(Ordering::Relaxed) + CURSOR_ROW_RVA);
    f(win)
}
unsafe fn call1(rva: usize, a: usize) -> usize {
    let f: unsafe extern "system" fn(usize) -> usize = std::mem::transmute(BASE.load(Ordering::Relaxed) + rva);
    f(a)
}
unsafe fn call2(rva: usize, a: usize, b: usize) -> usize {
    let f: unsafe extern "system" fn(usize, usize) -> usize = std::mem::transmute(BASE.load(Ordering::Relaxed) + rva);
    f(a, b)
}

/// Free windows in KH1's pool.
unsafe fn free_windows() -> usize {
    let pool = BASE.load(Ordering::Relaxed) + POOL_STEAM;
    (0..POOL_LEN).filter(|i| rd::<u32>(pool + i * WIN_SIZE + WIN_STATE) == 0).count()
}

/// Creates `list` with its help line after `after` (KH1's title tab's and pad diagrams' order:
/// create, link next / parent, cursor row, the help line, front). 0 when the pool is full.
unsafe fn create_list(list: List, row: i32, after: usize) -> usize {
    let base = BASE.load(Ordering::Relaxed);
    let w = call2(CREATE_STEAM, def_of(list), OPEN_TYPE as usize);
    if w == 0 {
        log("Config menu: no free window for our list");
        return 0;
    }
    call2(LINK_NEXT_STEAM, after, w);
    call2(LINK_PARENT_STEAM, w, after);
    let row = row.clamp(0, list.items().len() as i32 - 1);
    wr::<i16>(w + WIN_ROW16, (row << 4) as i16);
    CUR_ROW[list.index()].store(row, Ordering::Relaxed);
    let h = call2(CREATE_STEAM, base + HELP_DEF_STEAM, OPEN_TYPE as usize);
    if h != 0 {
        call2(LINK_NEXT_STEAM, w, h);
        call2(LINK_PARENT_STEAM, h, w);
        call1(FRONT_STEAM, w);
    }
    w
}

/// Opens `list` from the window `win` (KH1's Config rows 1 / 2: the last window after it, `win`
/// closed, sound 2, the new list after it). False (buzzer) when the pool hasn't room for it.
unsafe fn open(win: usize, list: List) -> bool {
    if free_windows() < 2 {
        sound(4);
        return false;
    }
    let after = call1(CHAIN_END_STEAM, win);
    call1(CLOSE_STEAM, win);
    sound(2);
    create_list(list, 0, after);
    true
}

/// The Config title tab: with the focus back after one of our lists was backed out of, it
/// re-creates the parent list (cursor on its row) where KH1 would re-create the Config list. Then
/// KH1's (which, the focus gone to the new window, only draws the tab).
unsafe extern "system" fn title(win: usize) {
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_TITLE.load(Ordering::Acquire));
    if focus(win) {
        let p = PENDING.swap(-1, Ordering::Relaxed);
        // (Only on a tab that has been open: a new one, the pause menu re-opened, starts on KH1's
        // Config list.)
        if p >= 0 && rd::<u32>(win + WIN_STATE) == 3 && rd::<u32>(win + WIN_CLOSING) & 1 == 0 {
            let list = LISTS[p as usize];
            create_list(list, SAVED_ROW[list.index()].load(Ordering::Relaxed), win);
        }
    }
    f(win)
}

/// KH1's Config list (page 1).
unsafe extern "system" fn update(win: usize) {
    let base = BASE.load(Ordering::Relaxed);
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(ORIG_UPDATE.load(Ordering::Acquire));
    let pad = rd::<u64>(base + PAD_PTR_RVA) as usize;
    scale_list(win);
    let mut kept: Option<(u32, u32)> = None;
    let mut faked = false;
    // The row before this frame's move.
    let row = rd::<i32>(base + ROW_RVA);
    // KH1's update moves the glint (`0x1402D9920`) when it has the focus and no confirm / cancel.
    let mut glint = false;
    if pad > 0x10000 && focus(win) {
        PENDING.store(-1, Ordering::Relaxed);
        let cancel = rd::<u32>(base + CONFIRM_MASK_RVA + 4);
        glint = rd::<u32>(pad + 8) & (rd::<u32>(base + CONFIRM_MASK_RVA) | cancel) == 0;
        let confirm = rd::<u32>(base + CONFIRM_MASK_RVA);
        let (pressed, repeat) = (rd::<u32>(pad + 8), rd::<u32>(pad + 0x10));
        let mut swallow = 0;
        if pressed & confirm != 0 {
            if row == BUTTON_ROW {
                // (Closed: KH1's update sees no focus and only draws; its row 7 would be Default.)
                open(win, List::Top);
                swallow = confirm;
            } else if row == DEFAULT_ROW {
                // KH1's Default settings is its row 7 (resets KH1's rows only: answer 6b).
                wr(base + ROW_RVA, KH1_DEFAULT_ROW);
                faked = true;
            }
        } else if row == BUTTON_ROW || row == DEFAULT_ROW || (row == 0 && pressed & UP != 0) {
            // KH1's left / right skip its rows 6 and 7 but not 8, where they would step the
            // difficulty: not seen when the cursor ends this frame on 7 or 8 (from 7, 8, or 0's
            // wrap).
            swallow = LEFT | RIGHT;
        }
        if swallow != 0 {
            kept = Some((pressed, repeat));
            wr(pad + 8, pressed & !swallow);
            wr(pad + 0x10, repeat & !swallow);
        }
    }
    HIDE_DEFAULT.store(true, Ordering::Relaxed);
    f(win);
    HIDE_DEFAULT.store(false, Ordering::Relaxed);
    if faked {
        wr(base + ROW_RVA, DEFAULT_ROW);
    }
    if let Some((p, r)) = kept {
        wr(pad + 8, p);
        wr(pad + 0x10, r);
    }
    // Our button and KH1's Default, drawn as KH1 draws its row 7 (style 1 selected, 0 not).
    let row = rd::<i32>(base + ROW_RVA);
    if glint {
        glint_to(win, row);
    }
    button(win, BUTTON_ROW, row == BUTTON_ROW, BUTTON_LABEL);
    button(win, DEFAULT_ROW, row == DEFAULT_ROW, DEFAULT_LABEL.load(Ordering::Relaxed));
}

/// One of our lists: KH1's Config update's input and draw, on our rows.
unsafe extern "system" fn list_update(win: usize) {
    let base = BASE.load(Ordering::Relaxed);
    let Some(list) = list_of(win) else { return };
    let items = list.items();
    let cur = &CUR_ROW[list.index()];
    let pad = rd::<u64>(base + PAD_PTR_RVA) as usize;
    if pad > 0x10000 && focus(win) {
        let confirm = rd::<u32>(base + CONFIRM_MASK_RVA);
        let cancel = rd::<u32>(base + CONFIRM_MASK_RVA + 4);
        let (pressed, repeat) = (rd::<u32>(pad + 8), rd::<u32>(pad + 0x10));
        if pressed & cancel != 0 {
            // Back: this list closes; the title tab re-creates its parent (ours, or KH1's Config
            // list on the button).
            call1(CLOSE_STEAM, win);
            PENDING.store(list.parent().map_or(-1, |p| p.index() as i32), Ordering::Relaxed);
            sound(3);
        } else if pressed & confirm != 0 {
            let row = cur.load(Ordering::Relaxed);
            match items.get(row as usize) {
                Some(Item::Set(_, what)) => {
                    set(*what, !value(*what));
                    sound(1);
                }
                Some(Item::Pick(_, what)) => {
                    pick_set(*what, (pick_value(*what) + 1) % 3);
                    sound(1);
                }
                Some(Item::Open(_, _, child)) => {
                    SAVED_ROW[list.index()].store(row, Ordering::Relaxed);
                    open(win, *child);
                }
                None => {}
            }
        } else {
            call1(CURSOR_MOVE_STEAM, win);
            call2(GLOVE_STEAM, win, 8);
            let row = cursor(win);
            cur.store(row, Ordering::Relaxed);
            // KH1's left / right on a two-value row: a step, or a wrap on a fresh press.
            if let Some(Item::Set(_, what)) = items.get(row as usize) {
                let v = value(*what);
                let new = if repeat & LEFT != 0 {
                    if v { Some(false) } else if pressed & LEFT != 0 { Some(true) } else { None }
                } else if repeat & RIGHT != 0 {
                    if !v { Some(true) } else if pressed & RIGHT != 0 { Some(false) } else { None }
                } else {
                    None
                };
                if let Some(n) = new {
                    set(*what, n);
                    sound(1);
                }
            }
            // (A three-value row: a step, or a wrap at either end on a fresh press.)
            if let Some(Item::Pick(_, what)) = items.get(row as usize) {
                let v = pick_value(*what);
                let new = if repeat & LEFT != 0 {
                    if v > 0 { Some(v - 1) } else if pressed & LEFT != 0 { Some(2) } else { None }
                } else if repeat & RIGHT != 0 {
                    if v < 2 { Some(v + 1) } else if pressed & RIGHT != 0 { Some(0) } else { None }
                } else {
                    None
                };
                if let Some(n) = new {
                    pick_set(*what, n);
                    sound(1);
                }
            }
        }
    }
    let row = cur.load(Ordering::Relaxed);
    for (i, item) in items.iter().enumerate() {
        match item {
            Item::Set(ids, what) => setting(win, i as i32, row == i as i32, ids, value(*what)),
            Item::Open(label, _, _) => button(win, i as i32, row == i as i32, *label),
            Item::Pick(ids, what) => picker(win, i as i32, row == i as i32, ids, pick_value(*what)),
        }
    }
}

/// Page 1's glint at the selected label's top-right corner (x 153, y 71 + row x the rows' pitch),
/// taken through the list's 92% scale as its boxes are (anchor: the window's top edge, x at its
/// centre), so it follows the cursor over the scaled rows. KH1's own call (`0x1402D9920`) has
/// already run in the update with the def's pitch of 0 (the top row); this one, the same frame,
/// replaces its target. Coordinates in KH1's 1/16 units as `0x1402D9920` makes them.
unsafe fn glint_to(win: usize, row: i32) {
    let base = BASE.load(Ordering::Relaxed);
    let pitch = rd::<i32>(base + ROW_H_RVA) + rd::<i32>(base + ROW_GAP_RVA);
    let (wx, wy) = (rd::<i32>(win + 0xDC), rd::<i32>(win + 0xE0));
    let ax = wx + rd::<i16>(win + 0xE4) as i32 / 2;
    let (px, py) = ((wx + 153 * 16) as f32, (wy + (0x47 + row * pitch) * 16) as f32);
    let x = ax as f32 + (px - ax as f32) * LIST_SCALE;
    let y = wy as f32 + (py - wy as f32) * LIST_SCALE;
    let f: unsafe extern "system" fn(i32, i32, i32) = std::mem::transmute(base + GLOW_AT_STEAM);
    f(8, (x as i32 - 0x7000) >> 4, (y as i32 - 0x7400) >> 4);
}

fn row_y(base: usize, row: i32) -> i32 {
    unsafe { 0x47 + row * (rd::<i32>(base + ROW_H_RVA) + rd::<i32>(base + ROW_GAP_RVA)) }
}

/// A button row, drawn as KH1 draws its Default settings row (label only, style 1 selected).
unsafe fn button(win: usize, row: i32, selected: bool, label: u32) {
    let base = BASE.load(Ordering::Relaxed);
    let panel: unsafe extern "system" fn(usize, i32, i32, i32, i32, i32, i32, i32, u32) = std::mem::transmute(base + PANEL_RVA);
    panel(win, if selected { 1 } else { 0 }, 2, 0, 0x0B, row_y(base, row), 0x8E, 0x1B, label);
}

/// A setting row, drawn as KH1 draws its rows 0..5: the label (style 0xB and the four arrows
/// when selected), the two value panels, and the finger at the value when selected.
unsafe fn setting(win: usize, row: i32, selected: bool, ids: &[u32; 4], v: bool) {
    let base = BASE.load(Ordering::Relaxed);
    let panel: unsafe extern "system" fn(usize, i32, i32, i32, i32, i32, i32, i32, u32) = std::mem::transmute(base + PANEL_RVA);
    let decor: unsafe extern "system" fn(usize, i32, i32, i32) = std::mem::transmute(base + DECOR_RVA);
    let h = rd::<i32>(base + ROW_H_RVA);
    let y = row_y(base, row);
    if selected {
        let y2 = h / 2 - 1 + y;
        decor(win, -0x40, y2, 0x4B);
        decor(win, 0x99, y2, 6);
        decor(win, 0x141, y2, 6);
        decor(win, 0x1E9, y2, 0x17);
    }
    panel(win, if selected { 0xB } else { 0 }, 2, 0, 0x0B, y, 0x8E, 0x1B, ids[0]);
    let styles = base + if selected { STYLES_SELECTED_RVA } else { STYLES_OTHER_RVA };
    let v = v as usize;
    let (left, right) = (rd::<u32>(styles + v * 8) as i32, rd::<u32>(styles + v * 8 + 4) as i32);
    panel(win, left, 2, 2, 0x9F, y, 0xA2, 0xF0, ids[1]);
    panel(win, right, 2, 2, 0x147, y, 0xA2, 0x198, ids[2]);
    if selected {
        let finger: unsafe extern "system" fn(usize, i32, i32, i32, i32, i32, i32, u32, u32, i32) = std::mem::transmute(base + FINGER_STEAM);
        let info = rd::<u64>(base + FINGER_INFO_STEAM) as usize;
        if info < 0x10000 {
            return;
        }
        let yc = y + (h - (rd::<i16>(info + 0x152) as i32 >> 4)) / 2;
        let (x, frame) = if v == 0 { (0x139 - (rd::<i16>(info + 0x150) as i32 >> 4), 0x10) } else { (0x14F, 0xF) };
        finger(win, x, yc, 0, 0, frame, 0, 0x8080_8080, 0x8080_8080, 0);
    }
}

/// A three-value row's panel width.
const PICK_W: i32 = 0x6A;

/// A three-value row, drawn as `setting` with three narrower value panels between the same two
/// ends (KH1's two panels span x 0x9F..0x1E9 with a gap of 6; three of 0x6A here), the chosen one
/// in KH1's chosen-value style, the finger at it when selected.
unsafe fn picker(win: usize, row: i32, selected: bool, ids: &[u32; 5], v: usize) {
    let base = BASE.load(Ordering::Relaxed);
    let panel: unsafe extern "system" fn(usize, i32, i32, i32, i32, i32, i32, i32, u32) = std::mem::transmute(base + PANEL_RVA);
    let decor: unsafe extern "system" fn(usize, i32, i32, i32) = std::mem::transmute(base + DECOR_RVA);
    let h = rd::<i32>(base + ROW_H_RVA);
    let y = row_y(base, row);
    if selected {
        let y2 = h / 2 - 1 + y;
        decor(win, -0x40, y2, 0x4B);
        // (KH1's arrow before each value panel, as at 0x99 / 0x141 for its two.)
        decor(win, 0x99, y2, 6);
        decor(win, 0x9F + PICK_W, y2, 6);
        decor(win, 0x9F + 2 * PICK_W + 6, y2, 6);
        decor(win, 0x1E9, y2, 0x17);
    }
    panel(win, if selected { 0xB } else { 0 }, 2, 0, 0x0B, y, 0x8E, 0x1B, ids[0]);
    let styles = base + if selected { STYLES_SELECTED_RVA } else { STYLES_OTHER_RVA };
    // (KH1's table: value 0's left panel is the chosen style, its right one the other.)
    let (chosen, other) = (rd::<u32>(styles) as i32, rd::<u32>(styles + 4) as i32);
    const W: i32 = PICK_W;
    let xs = [0x9F, 0x9F + W + 6, 0x9F + 2 * (W + 6)];
    for (i, &x) in xs.iter().enumerate() {
        panel(win, if i == v { chosen } else { other }, 2, 2, x, y, W, x + W / 2, ids[1 + i]);
    }
    if selected {
        let finger: unsafe extern "system" fn(usize, i32, i32, i32, i32, i32, i32, u32, u32, i32) = std::mem::transmute(base + FINGER_STEAM);
        let info = rd::<u64>(base + FINGER_INFO_STEAM) as usize;
        if info < 0x10000 {
            return;
        }
        let yc = y + (h - (rd::<i16>(info + 0x152) as i32 >> 4)) / 2;
        let x0 = xs[v.min(2)];
        let (x, frame) = if v == 0 { (x0 + W - 8 - (rd::<i16>(info + 0x150) as i32 >> 4), 0x10) } else { (x0 + 8, 0xF) };
        finger(win, x, yc, 0, 0, frame, 0, 0x8080_8080, 0x8080_8080, 0);
    }
}
