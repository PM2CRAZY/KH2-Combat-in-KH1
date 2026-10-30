//! KH2's Drive orbs as KH1 prizes (the owner, the main-branch port's round 4, row 3, "go ahead":
//! answers 1 (c), 2 (a), 3 (a); main s318: "For the drive orbs, I want them to run through KH1's
//! engine, not an overlay"). Notes: `docs/kh1-drive-forms-native.md` §15.
//!
//! Steam KH1, Checked (decomp) and (exe):
//! - **Types** (t944): KH1's prizes are types 0..6 (munny 1 / 5 / 20, HP 1 / 5, MP 2 / 15). Two
//!   more, 7 and 8, the small and big Drive orbs (KH2's own kinds 7 / 8). KH1's per-type row table
//!   `0x140528BE0` (7 rows of 0x14: height, glow, sound, a fourth word, munny shown) is followed by
//!   a word in use (`0x140528C6C`), so it is copied with rows 7 / 8 (MP's small and big rows: the
//!   same height, glow and sound) and KH1's six reads of it (`0x1402AC0F3`, `0x1402AC3A7`,
//!   `0x1402AC474`, `0x1402AC49B` in the pickup, `0x1402AC50D` and `0x1402AC86D`, the prize steps)
//!   are pointed at the copy, each read's displacement checked first. KH1's own code then runs them.
//! - **Their look** (t944, answer 2 (a) until KH2's art): KH1's prize program `pppRyjMegaBirthPrize`'s
//!   draw `0x140340D10 (program, data)` draws each prize by its type's entry in the program data
//!   (`+4 + type × 12`: shape, colour r g b a, scale; shape 0xFFFF draws nothing; 10 entries, KH1 uses
//!   0..6). Entries 7 / 8 are set (each draw, before KH1's) to MP's small and big orbs' shape and scale
//!   in KH2's Drive orb gold (main's tint: red full, green 0.66, blue 0.12 of the colour's brightest).
//! - **Their drop** (t945): KH1's enemy drop `0x1402ABC40 (counts[7], position, mode)` (only from the AI
//!   drop ops `0x1402BC640` / `0x1402BC6B0`, Checked (decomp)) runs as ever; after it, unless KH1 has
//!   prizes off (`0x142D60FAC`), a roll of KH2's normal enemies' rows (`crate::drivegauge::DROPS`)
//!   gives the Drive orbs, each count × KH1's own count factor (`0x142D60FA4`, Jackpot) as KH1's drop
//!   does, made by KH1's `0x1402AB2A0 (type, position, mode)`.
//! - **Their pickup** (t946): KH1's pickup `0x1402AC080 (prize, position) -> member` gives types 0..6
//!   their reward and plays the row's sound; for 7 / 8 nothing. After it, a Drive orb taken gives KH2's
//!   value (1 / 3, `crate::drivegauge::SMALL_ORB` / `BIG_ORB`): in a form on the Drive gauge 12 ticks
//!   a point to the form gauge (`form::orb`), else the points to the bars (`drivegauge::add`; KH2's
//!   world factor 1.0 and no MP Charge in KH1). Assumption: whichever party member takes it, it's
//!   Sora's gauge (KH2 gives it to the player's; KH1 lets the party pick up).
//! - **Their art** (t948, answer 3 (a): KH2's art): KH2's Drive orb shape and texture are added to
//!   KH1's prize file once into the mod folder (`crate::prizeart`, from the player's own KH1
//!   `prize.dpx` and KH2 `obj/PRIZE.mdlx` and their HD images). KH1's prize system set-up
//!   `0x1402AB7D0` makes its prize effect with `0x1402AE760`, whose one call `0x1402AE77B` hands
//!   KH1's prize file (`0x142DAB4B0`) to KH1's effect make `0x1401E0100 (dpx, instance, 0, 20000)`;
//!   that call is pointed at ours, which hands our file instead. Ours is entered in KH1's loaded
//!   files with its HD images (as KH1 enters a file it loads, `kh2dpx::new_file` / `add_hd`) and
//!   relocated by KH1's own `0x1401E0080`, the first time KH1 makes the prize effect with it ready
//!   (built on a thread at start). KH1 then draws entries 7 / 8 with KH2's shape (slot 4), colour and
//!   scale. Until it is ready, or if it can't be made, the gold MP orbs above stay.
//! - Only with Drive on (the ini's `drive`) and on Steam.

use super::log;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};

const TABLE_RVA: usize = 0x528BE0;
const ROW: usize = 0x14;
const KH1_TYPES: usize = 7;
pub const SMALL: u16 = 7;
pub const BIG: u16 = 8;
const TYPES: usize = 9;
/// The reads of the row table: (site, kind, the field): kind 0 `movss xmm, [r14 + r*4 + disp32]`
/// (disp at +6, the base-relative field), 1 `mov r32, [r14 + r*4 + disp32]` (disp at +4), 2 `lea r64,
/// [rip + disp32]` (disp at +3, 7 long). Checked (exe).
const SITES: [(usize, u8, usize); 6] = [(0x2AC0F3, 0, 0x0), (0x2AC3A7, 0, 0x4), (0x2AC474, 1, 0x10), (0x2AC49B, 1, 0x8), (0x2AC50D, 2, 0), (0x2AC86D, 2, 0)];

const DRAW_RVA: usize = 0x340D10;
const DRAW_BYTES: [u8; 12] = [0x48, 0x89, 0x5C, 0x24, 0x20, 0x55, 0x56, 0x57, 0x41, 0x55, 0x41, 0x56];
const DROP_RVA: usize = 0x2ABC40;
const DROP_BYTES: [u8; 9] = [0x48, 0x8B, 0xC4, 0x48, 0x89, 0x48, 0x08, 0x41, 0x54];
const PICKUP_RVA: usize = 0x2AC080;
const PICKUP_BYTES: [u8; 10] = [0x48, 0x8B, 0xC4, 0x48, 0x89, 0x58, 0x18, 0x55, 0x56, 0x57];
const MAKE_RVA: usize = 0x2AB2A0;
const MAKE_BYTES: [u8; 10] = [0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18];
/// KH1's prizes off and its count factor (read by its drop, `0x1402ABC40`).
const PRIZES_OFF_RVA: usize = 0x2D60FAC;
const COUNT_FACTOR_RVA: usize = 0x2D60FA4;

/// KH2's Drive orb gold over MP's orb colour's brightest channel (main s318).
const GOLD: [f32; 3] = [1.0, 0.66, 0.12];
/// KH1's prize file's buffer, the call in `0x1402AE760` that makes the prize effect from it and the
/// effect make it calls (Checked (decomp) and (exe)).
const PRIZE_DPX_RVA: usize = 0x2DAB4B0;
const CREATE_CALL_RVA: usize = 0x2AE77B;
const EFFECT_MAKE_RVA: usize = 0x1E0100;
/// KH1's DPX relocation (`0x1401E0080`, what `0x1402AE790` runs on its prize file).
const RELOCATE_RVA: usize = 0x1E0080;
const RELOCATE_BYTES: [u8; 11] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x74, 0x24, 0x10, 0x57];

/// The art, once built or loaded (on its thread).
static ART: std::sync::OnceLock<crate::prizeart::Art> = std::sync::OnceLock::new();
/// Our prize file in KH1: 0 not yet, 1 couldn't be entered, else where it is.
static OURS: AtomicUsize = AtomicUsize::new(0);

static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG_DRAW: AtomicUsize = AtomicUsize::new(0);
static ORIG_DROP: AtomicUsize = AtomicUsize::new(0);
static ORIG_PICKUP: AtomicUsize = AtomicUsize::new(0);
static SEED: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);
static LOGGED: AtomicU32 = AtomicU32::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

fn roll() -> u32 {
    let mut x = SEED.load(Ordering::Relaxed);
    if x == 0x9E37_79B9_7F4A_7C15 {
        x ^= std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
    }
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    SEED.store(x, Ordering::Relaxed);
    (x >> 32) as u32
}

fn note(line: impl FnOnce() -> String) {
    if LOGGED.fetch_add(1, Ordering::Relaxed) < 40 {
        log!(Forms, "Drive orbs: {}", line());
    }
}

pub unsafe fn install(base: usize) {
    if !super::settings::get().drive || super::addr::Build::current() != super::addr::Build::Steam {
        return;
    }
    BASE.store(base, Ordering::Relaxed);
    if std::slice::from_raw_parts((base + MAKE_RVA) as *const u8, MAKE_BYTES.len()) != MAKE_BYTES {
        log!(General, "Drive orbs: unexpected code at KH1's prize make 0x{:X}; no Drive orbs", 0x1_4000_0000usize + MAKE_RVA);
        return;
    }
    if !table(base) {
        return;
    }
    super::hook::install(base + DRAW_RVA, &DRAW_BYTES, draw as *const () as usize, &ORIG_DRAW, "KH1's prize draw (Drive orbs)");
    if super::hook::redirect_call(base + CREATE_CALL_RVA, base + EFFECT_MAKE_RVA, create as *const () as usize, "KH1's prize effect make (Drive orb art)") {
        start_art();
    }
    super::hook::install(base + DROP_RVA, &DROP_BYTES, drop as *const () as usize, &ORIG_DROP, "KH1's enemy prize drop (Drive orbs)");
    super::hook::install(base + PICKUP_RVA, &PICKUP_BYTES, pickup as *const () as usize, &ORIG_PICKUP, "KH1's prize pickup (Drive orbs)");
}

/// The row table copied with rows 7 / 8 and KH1's reads pointed at it (all checked before any is
/// written; none written if one isn't as expected).
unsafe fn table(base: usize) -> bool {
    let Some(ours) = super::hook::alloc_near(base, 0x100) else {
        log!(General, "Drive orbs: no memory near the game for the prize table; no Drive orbs");
        return false;
    };
    let kh1 = base + TABLE_RVA;
    std::ptr::copy_nonoverlapping(kh1 as *const u8, ours as *mut u8, KH1_TYPES * ROW);
    std::ptr::copy_nonoverlapping((kh1 + 5 * ROW) as *const u8, (ours + SMALL as usize * ROW) as *mut u8, ROW);
    std::ptr::copy_nonoverlapping((kh1 + 6 * ROW) as *const u8, (ours + BIG as usize * ROW) as *mut u8, ROW);
    let mut writes = Vec::new();
    for (site, kind, field) in SITES {
        let at = base + site;
        let (off, len) = match kind {
            0 => (6, 10),
            1 => (4, 8),
            _ => (3, 7),
        };
        let disp = rd::<i32>(at + off) as i64;
        let (want, new) = match kind {
            0 | 1 => ((kh1 - base + field) as i64, ours as i64 - base as i64 + field as i64),
            _ => (kh1 as i64 - (at + len) as i64, ours as i64 - (at + len) as i64),
        };
        if disp != want || !(i32::MIN as i64..=i32::MAX as i64).contains(&new) {
            log!(General, "Drive orbs: KH1's prize table read at {at:#x} isn't as expected (disp {disp:#x}, want {want:#x}); no Drive orbs");
            return false;
        }
        writes.push((at + off, disp as i32, new as i32));
    }
    for (at, old, new) in writes {
        if !super::hook::patch(at, &old.to_le_bytes(), &new.to_le_bytes(), "KH1's prize table read (Drive orbs)") {
            return false;
        }
    }
    let rows: Vec<[i32; 5]> = (0..TYPES).map(|t| rd::<[i32; 5]>(ours + t * ROW)).collect();
    log!(Forms, "Drive orbs: KH1's prize rows read from {ours:#x}, 9 types (7, 8 Drive orbs: MP's rows) {rows:?}");
    true
}

/// The prize program's draw: entries 7 / 8 set from MP's 5 / 6 in gold (KH1's own prize file:
/// ours has KH2's art there, t948), then KH1's draw.
unsafe extern "system" fn draw(program: usize, data: usize) {
    if data >= 0x10000 && super::crash::readable(data + 4 + 9 * 12 + 11) {
        for (ours, mp) in [(SMALL as usize, 5usize), (BIG as usize, 6usize)] {
            let (o, m) = (data + 4 + ours * 12, data + 4 + mp * 12);
            let shape = rd::<u32>(m);
            // (KH2's shape in our file: neither none nor MP's.)
            if rd::<u32>(o) != 0xFFFF && rd::<u32>(o) != shape {
                continue;
            }
            let [r, g, b, a] = rd::<[u8; 4]>(m + 4);
            let top = r.max(g).max(b) as f32;
            let gold = [(top * GOLD[0]) as u8, (top * GOLD[1]) as u8, (top * GOLD[2]) as u8, a];
            if rd::<u32>(o) != shape || rd::<[u8; 4]>(o + 4) != gold {
                wr(o, shape);
                wr(o + 4, gold);
                wr(o + 8, rd::<f32>(m + 8));
                note(|| format!("prize program entry {ours} set from MP's {mp}: shape {shape}, colour {gold:?} (MP's [{r}, {g}, {b}, {a}])"));
            }
        }
    }
    let f: unsafe extern "system" fn(usize, usize) = std::mem::transmute(ORIG_DRAW.load(Ordering::Relaxed));
    f(program, data);
}

/// KH1's enemy drop, then the Drive orbs (KH2's normal enemies' rows).
unsafe extern "system" fn drop(counts: usize, pos: usize, mode: i32) {
    let f: unsafe extern "system" fn(usize, usize, i32) = std::mem::transmute(ORIG_DROP.load(Ordering::Relaxed));
    f(counts, pos, mode);
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 || pos < 0x10000 || rd::<i32>(base + PRIZES_OFF_RVA) != 0 {
        return;
    }
    let (small, big) = crate::drivegauge::drop_for(roll());
    if small == 0 && big == 0 {
        return;
    }
    let factor = rd::<f32>(base + COUNT_FACTOR_RVA);
    let make: unsafe extern "system" fn(u16, usize, i32) = std::mem::transmute(base + MAKE_RVA);
    let mut made = [0; 2];
    for (k, (ty, n)) in [(SMALL, small), (BIG, big)].into_iter().enumerate() {
        // (KH1's own count: `(int)(count × factor)`.)
        let n = (n as f32 * factor) as i32;
        for _ in 0..n.max(0) {
            make(ty, pos, mode);
        }
        made[k] = n.max(0);
    }
    note(|| format!("an enemy's drop: {} small, {} big (KH2's row {small} / {big}, KH1's count factor {factor})", made[0], made[1]));
}

/// KH1's pickup, then a Drive orb's value (KH2's `0x1403DCA50` / `0x1403A9CF0`).
unsafe extern "system" fn pickup(prize: usize, pos: usize) -> usize {
    let f: unsafe extern "system" fn(usize, usize) -> usize = std::mem::transmute(ORIG_PICKUP.load(Ordering::Relaxed));
    let ty = if prize >= 0x10000 { rd::<u16>(prize + 0x1C) } else { 0 };
    let who = f(prize, pos);
    if who != 0 && (ty == SMALL || ty == BIG) {
        let value = if ty == BIG { crate::drivegauge::BIG_ORB } else { crate::drivegauge::SMALL_ORB };
        if super::form::orb(value) {
            note(|| format!("{} orb taken in a form: +{} form gauge ticks", if ty == BIG { "big" } else { "small" }, value as f32 * crate::drivegauge::ORB_TICKS));
        } else if !super::form::in_form() {
            super::drivegauge::add(value as i32, "a Drive orb");
        }
    }
    who
}

fn cache_dir() -> std::path::PathBuf {
    std::path::Path::new(&super::dll_dir()).join("kh1-kh2-cache")
}

/// The art, on a thread (nothing on the game's): the cached file when it matches the sources,
/// else built from them and written.
fn start_art() {
    std::thread::spawn(|| {
        let t = std::time::Instant::now();
        match load_art() {
            Ok((art, how)) => {
                log!(Loading, "Drive orb art: {how} in {:.0} ms (DPX {} bytes, {} HD images)", t.elapsed().as_secs_f32() * 1000.0, art.dpx.len(), art.hd.len());
                let _ = ART.set(art);
            }
            Err(e) => log!(Loading, "Drive orb art: not made ({e}); the Drive orbs stay MP's orbs in gold"),
        }
    });
}

fn load_art() -> Result<(crate::prizeart::Art, String), String> {
    use crate::prizeart::{KH1_FILE, KH2_FILE};
    let kh1 = kh1_data::Kh1Install::locate(super::exe_dir()).map_err(|e| format!("KH1's install: {e}"))?;
    let exe = format!("{}\\KINGDOM HEARTS II FINAL MIX.exe", super::exe_dir());
    let kh2 = kh2_data::Install::locate(&exe).map_err(|e| format!("KH2's install: {e}"))?;
    let src = if kh2.has_backup() { kh2_data::Source::Backup } else { kh2_data::Source::Live };
    let d = crate::prizeart::Desc { build: format!("{:?}", super::addr::Build::current()), kh1: kh1.stamp(kh1_data::Source::Live, KH1_FILE), kh2: kh2.stamp_any(src, KH2_FILE) };
    if d.kh1 == "missing" || d.kh2 == "missing" {
        return Err(format!("{} not found", if d.kh1 == "missing" { KH1_FILE } else { KH2_FILE }));
    }
    let dir = cache_dir();
    let why = match crate::prizeart::load(&dir, &d) {
        Ok(art) => return Ok((art, format!("{} from the cache", dir.join(crate::prizeart::FILE_NAME).display()))),
        Err(why) => why,
    };
    let dpx = kh1.read(kh1_data::Source::Live, KH1_FILE).map_err(|e| format!("{KH1_FILE}: {e}"))?;
    let kh1_hd = |k: u32| kh1.read_remastered(kh1_data::Source::Live, KH1_FILE, &format!("-{k:x}.")).ok().map(|a| a.1);
    let mdlx = kh2.read_any(src, KH2_FILE).map_err(|e| format!("{KH2_FILE}: {e}"))?;
    let hd: Vec<(u32, Vec<u8>)> = kh2.read_remastered_any(src, KH2_FILE).map_err(|e| format!("{KH2_FILE}'s HD images: {e}"))?.into_iter().map(|a| (a.offset, a.data)).collect();
    let b = crate::prizeart::build(&dpx, kh1_hd, &mdlx, &hd)?;
    log!(Loading, "Drive orb art: {}", b.report);
    match crate::prizeart::store(&dir, &d, &b) {
        Ok(p) => log!(Loading, "Drive orb art: written ({})", p.display()),
        Err(e) => log!(Loading, "Drive orb art: not written ({e}); made again next start"),
    }
    Ok((crate::prizeart::art_of(b), format!("made ({why})")))
}

/// KH1's effect make from `0x1402AE760`: our prize file in place of KH1's once it is in KH1.
unsafe extern "system" fn create(dpx: usize, instance: usize, a: i32, b: u32) -> u64 {
    let base = BASE.load(Ordering::Relaxed);
    let mut file = dpx;
    if dpx == base + PRIZE_DPX_RVA {
        if let Some(ours) = ours(base) {
            file = ours;
        }
    }
    let f: unsafe extern "system" fn(usize, usize, i32, u32) -> u64 = std::mem::transmute(base + EFFECT_MAKE_RVA);
    f(file, instance, a, b)
}

/// Our prize file in KH1, entered the first time it is asked for with the art ready.
unsafe fn ours(base: usize) -> Option<usize> {
    match OURS.load(Ordering::Relaxed) {
        0 => {}
        1 => return None,
        at => return Some(at),
    }
    let Some(art) = ART.get() else {
        note(|| "the prize effect made with KH1's file (KH2's orb art not ready yet)".to_string());
        return None;
    };
    match enter(base, art) {
        Ok(at) => {
            OURS.store(at, Ordering::Relaxed);
            log!(Forms, "Drive orbs: KH2's orb art in KH1's prize file at {at:#x} ({} bytes, {} HD images), relocated by KH1", art.dpx.len(), art.hd.len());
            Some(at)
        }
        Err(e) => {
            OURS.store(1, Ordering::Relaxed);
            log!(Forms, "Drive orbs: KH2's orb art not entered ({e}); MP's orbs in gold");
            None
        }
    }
}

/// The art's DPX copied into our handle region, entered in KH1's loaded files with its HD images,
/// and relocated by KH1.
unsafe fn enter(base: usize, art: &crate::prizeart::Art) -> Result<usize, String> {
    if !super::hook::prologue_is(base + RELOCATE_RVA, &RELOCATE_BYTES) {
        return Err("unexpected code at KH1's DPX relocation".into());
    }
    let k = super::kh2dpx::kh1_files(base)?;
    let at = super::kh2dpx::keep(&art.dpx).ok_or("no memory for it")?;
    let rec = super::kh2dpx::new_file(&k, "kh2_prize_dpx", at, art.dpx.len(), kh2_formats::kh1dpx::md5(&art.dpx))?;
    for (key, dds) in &art.hd {
        super::kh2dpx::add_hd(&k, rec, *key, Some(&dds[..]), 0)?;
    }
    let f: unsafe extern "system" fn(usize) = std::mem::transmute(base + RELOCATE_RVA);
    f(at);
    Ok(at)
}
