//! KH2's Drive orbs as KH1 prizes (the user: "For the drive orbs, I want them to run through KH1's
//! engine, not an overlay"). They are KH1's own prize objects, two new prize types next to KH1's
//! seven, so KH1's prize code does everything for them: the scatter and bounce, the pull toward
//! a party member, the lifetime and blink-out, the pickup on touch and its sound, and the draw
//! (`docs/kh1-prizes.md`).
//!
//! KH1's prize system (Checked, Epic exe):
//! - A prize is a task of the prize list (`0x142D619B0`, 256 entries of 0x40 bytes at
//!   `0x142D619D0`; the step `0x1402AA940`) with a draw record (0x20 bytes each at `0x142432B40`,
//!   `+0x18` its type, `+0x19` live / shown bits) that the effect program `pppRyjMegaBirthPrize`
//!   (`0x14033E8D0`, from `prize.dpx`) draws: per type, a sprite, a colour and a scale from the
//!   program's own data (10 entries).
//! - Its type (`+0x1C`): 0..2 munny 1 / 5 / 20, 3..4 HP 1 / 5, 5..6 MP 2 / 15. Per type KH1 keeps a
//!   row of 5 dwords at `0x1405298E0` (height, pull range, pickup sound, a second id, munny shown).
//! - Made by `0x1402A9110 (type, position, mode)`, or in a loop by the drop functions: an enemy's
//!   AI drop op (`0x1402BA490` / `0x1402BA500` -> `0x1402A9AB0 (counts[7], position, mode)`), an
//!   event's (`0x1402A97B0`, a `map_prize.bin` row) and an object's (`0x1402A9A20`).
//! - Taken by `0x1402A9EF0 (prize, position)`: the first party member near enough gets its reward
//!   by type (`0x1402E59E0` munny, `0x1402A2790` HP, `0x1402A2950` MP; a type above 6 gets none),
//!   then its row's sound plays; it returns that member.
//!
//! What this adds:
//! - **Two types**, 7 a small Drive orb and 8 a big one. KH1's row table is copied to a table of
//!   ours with rows 7 and 8 (MP's small and big rows, so the same height, pull and sound), and the
//!   six places that read it (`TABLE_SITES`) point at the copy.
//! - **Their look**: the effect program's entries 7 and 8 become KH2's own Drive orb, its sprite
//!   animation and texture read from the user's KH2 install (`prizeart`), drawn by KH1's program;
//!   until that is ready, or if it can't be, MP's small and big orbs' sprites and sizes in KH2's
//!   drive-orb yellow-orange (`ppp_draw`, each time it runs).
//! - **Their drop**: after an enemy's own drop (`ai_drop`), KH2's normal enemies' Drive orb rows
//!   (`drivegauge::DROPS`) roll, and KH1 makes the orbs (`0x1402A9110`, the same mode as its own).
//!   The same hook rolls the Drive Recovery items' drop (`items::enemy_drop`), with or without
//!   the orbs.
//! - **Their pickup**: KH1's pickup (`pickup`) runs as ever; one of ours picked up adds its points
//!   to the Drive gauge (`gauge::orb`: small 1, big 3; in a form its form gauge, × 12 frames).
//! - **Drive Converter** (KH2FM's, equipped on base Sora): an enemy's munny is partly Drive orbs
//!   (`converted_drop`, `crate::support::convert_munny`).

use super::addr::Rva;
use super::{hook, log};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};

/// KH1's prize row table (7 rows of 5 dwords).
const TABLE_RVA: Rva = Rva(0x5298E0);
const ROW: usize = 0x14;
const KH1_TYPES: usize = 7;
/// Our types.
pub const SMALL: u16 = 7;
pub const BIG: u16 = 8;
const TYPES: usize = 9;
/// The six reads of the row table (`movss / mov [r14 + i*4 + table + field]`, r14 the image base,
/// and two `lea reg, [rip + table]`): (site, kind, the field's offset in the row).
/// Kind 0: `f3 41/42 0f 10 b4 xx` + disp32 (6 bytes in); 1: `41/43 8b xx xx` + disp32 (4 bytes in);
/// 2: `48 8d xx` + rip disp32 (3 bytes in, the instruction 7 long).
const TABLE_SITES: [(Rva, u8, u32); 6] = [
    (Rva(0x2A9F63), 0, 0x0),
    (Rva(0x2AA217), 0, 0x4),
    (Rva(0x2AA2E4), 1, 0x10),
    (Rva(0x2AA30B), 1, 0x8),
    (Rva(0x2AA37D), 2, 0x0),
    (Rva(0x2AA6DD), 2, 0x0),
];
/// Makes one prize `(type, position, mode)`.
const SPAWN_RVA: Rva = Rva(0x2A9110);
/// The pickup `(prize, position) -> member or 0`.
const PICKUP_RVA: Rva = Rva(0x2A9EF0);
/// `mov rax, rsp; mov [rax + 0x18], rbx; push rbp; push rsi; push rdi`.
const PICKUP_PROLOGUE: [u8; 10] = [0x48, 0x8B, 0xC4, 0x48, 0x89, 0x58, 0x18, 0x55, 0x56, 0x57];
/// The AI drop `(counts, position, mode)`.
const AI_DROP_RVA: Rva = Rva(0x2A9AB0);
/// `mov rax, rsp; mov [rax + 8], rcx; push r12`.
const AI_DROP_PROLOGUE: [u8; 9] = [0x48, 0x8B, 0xC4, 0x48, 0x89, 0x48, 0x08, 0x41, 0x54];
/// `pppRyjMegaBirthPrize`'s draw `(program, data)`.
const PPP_DRAW_RVA: Rva = Rva(0x33E8D0);
/// `mov [rsp + 0x20], rbx; push rbp; push rsi; push rdi`.
const PPP_DRAW_PROLOGUE: [u8; 8] = [0x48, 0x89, 0x5C, 0x24, 0x20, 0x55, 0x56, 0x57];
/// Prizes off (KH1's drop functions return at once while it is set).
const PRIZES_OFF_RVA: Rva = Rva(0x2D619AC);
/// KH1's prize count factor (1 + 0.5 per Jackpot equipped in the party, set where KH1 reads the
/// party's abilities, beside Lucky Strike's `0x142D619A8`): the drop functions make
/// `(int)(count × factor)` of each type.
const COUNT_FACTOR_RVA: Rva = Rva(0x2D619A4);
/// The prizes' draw records (0x20 bytes each: `+0x18` type, `+0x19` bits, bit 0 live, `+0x1C` the
/// animation timer, `+0x1E` its step).
const RECORDS_RVA: Rva = Rva(0x2432B40);
const RECORDS: usize = 256;

/// KH2's drive orbs' yellow-orange (KH2's orb is a warm gold; red full, green about two thirds,
/// a little blue), as fractions of MP's orb's brightest channel.
const DRIVE_TINT: [f32; 3] = [1.0, 0.66, 0.12];

static ON: AtomicBool = AtomicBool::new(false);
static ORIG_PICKUP: AtomicUsize = AtomicUsize::new(0);
static ORIG_AI_DROP: AtomicUsize = AtomicUsize::new(0);
static ORIG_PPP: AtomicUsize = AtomicUsize::new(0);
static SEED: AtomicU64 = AtomicU64::new(0x9E37_79B9_7F4A_7C15);
static DROPS_LOGGED: AtomicU32 = AtomicU32::new(0);
static PICKUPS_LOGGED: AtomicU32 = AtomicU32::new(0);
static CONVERTED_LOGGED: AtomicU32 = AtomicU32::new(0);

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}
fn base() -> usize {
    unsafe { super::sys::GetModuleHandleA(std::ptr::null()) as usize }
}

/// A small xorshift (the drop roll).
fn roll() -> u32 {
    let mut x = SEED.load(Ordering::Relaxed);
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    SEED.store(x, Ordering::Relaxed);
    (x >> 32) as u32
}

/// Drive orbs are in KH1's prize system (the table and its readers patched, the hooks in).
pub fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

/// Our row table and the six reads pointed at it. False (and nothing changed past the first
/// failure's site, which keeps KH1's own identical rows 0..6) if any site isn't as expected.
unsafe fn relocate_table(base: usize) -> bool {
    let Some(ours) = hook::alloc_near(base, 0x100) else {
        log("Drive orbs: no memory near the game for the prize table");
        return false;
    };
    let kh1 = base + TABLE_RVA;
    std::ptr::copy_nonoverlapping(kh1 as *const u8, ours as *mut u8, KH1_TYPES * ROW);
    // Small and big Drive orbs: MP's small (5) and big (6) rows.
    std::ptr::copy_nonoverlapping((kh1 + 5 * ROW) as *const u8, (ours + SMALL as usize * ROW) as *mut u8, ROW);
    std::ptr::copy_nonoverlapping((kh1 + 6 * ROW) as *const u8, (ours + BIG as usize * ROW) as *mut u8, ROW);
    let rows: Vec<String> = (0..TYPES).map(|t| format!("{:?}", rd::<[i32; 5]>(ours + t * ROW))).collect();
    log(&format!("Drive orbs: KH1's prize rows (+ ours 7, 8) {}", rows.join(" ")));
    // Check every site first, then write.
    let mut writes = Vec::new();
    for (site, kind, field) in TABLE_SITES {
        let at = base + site;
        let (off, len) = match kind {
            0 => (6, 10),
            1 => (4, 8),
            _ => (3, 7),
        };
        let disp = rd::<i32>(at + off);
        let (want, new) = match kind {
            0 | 1 => ((kh1 - base) as i64 + field as i64, ours as i64 - base as i64 + field as i64),
            _ => (kh1 as i64 - (at + len) as i64, ours as i64 - (at + len) as i64),
        };
        if disp as i64 != want || !(i32::MIN as i64..=i32::MAX as i64).contains(&new) {
            log(&format!("Drive orbs: the prize table's read at {at:#x} isn't as expected (disp {disp:#x}, want {want:#x}): no Drive orbs"));
            return false;
        }
        writes.push((at + off, disp, new as i32));
    }
    for (at, old, new) in writes {
        if !hook::patch(at, &old.to_le_bytes(), &new.to_le_bytes(), "prize table read (Drive orbs)") {
            return false;
        }
    }
    log(&format!("Drive orbs: KH1's prize table read from {ours:#x} (9 types) at its {} places", TABLE_SITES.len()));
    true
}

type PickupFn = unsafe extern "system" fn(usize, usize) -> usize;

/// KH1's pickup: as ever; a Drive orb taken adds to the Drive gauge.
unsafe extern "system" fn pickup(prize: usize, pos: usize) -> usize {
    let f: PickupFn = std::mem::transmute(ORIG_PICKUP.load(Ordering::Acquire));
    let ty = if prize > 0x10000 { rd::<u16>(prize + 0x1C) } else { 0 };
    let who = f(prize, pos);
    if who != 0 && (ty == SMALL || ty == BIG) {
        let value = if ty == BIG { crate::drivegauge::BIG_ORB } else { crate::drivegauge::SMALL_ORB };
        super::gauge::orb(value);
        if PICKUPS_LOGGED.fetch_add(1, Ordering::Relaxed) < 60 {
            log(&format!(
                "Drive orb ({}) picked up by {}: +{value}; Drive gauge {}",
                if ty == BIG { "big" } else { "small" },
                super::probe::object_id(who),
                super::gauge::describe()
            ));
        }
    }
    who
}

type AiDropFn = unsafe extern "system" fn(usize, usize, u32);

/// The drop's 13 dwords: KH1's seven counts, then three (chance, item) pairs.
const DROP_DWORDS: usize = 13;

/// Drive Converter (KH2's rule, `crate::support::convert_munny`): with it equipped, the drop's
/// munny (KH1's types 0..2, as many as KH1 would make: its count factor applied) keeps `× 0.5`
/// a copy, truncated, and the rest come as Drive orbs (20 and 5 munny big ones, 1 munny small
/// ones). KH1's drop runs on a copy of the counts with no munny, then KH1's own spawn makes the
/// munny kept and the orbs (the same position and mode). False if it doesn't apply (none
/// equipped, no munny, no orbs): the drop is KH1's as it is.
unsafe fn converted_drop(f: AiDropFn, base: usize, counts: usize, pos: usize, mode: u32) -> bool {
    let copies = super::menu::support().drive_converter;
    if copies == 0 || !on() || counts < 0x10000 || pos < 0x10000 || rd::<u32>(base + PRIZES_OFF_RVA) != 0 || !super::crash::readable(counts + DROP_DWORDS * 4 - 1) {
        return false;
    }
    let mut local: [i32; DROP_DWORDS] = rd(counts);
    let factor = rd::<f32>(base + COUNT_FACTOR_RVA);
    let factor = if factor.is_finite() && factor > 0.0 { factor } else { 1.0 };
    let made = |n: i32| if n > 0 { (n as f32 * factor) as i32 } else { 0 };
    let munny = [made(local[0]), made(local[1]), made(local[2])];
    let (kept, small, big) = crate::support::convert_munny(munny, copies);
    if small + big == 0 {
        return false;
    }
    local[..3].fill(0);
    f(local.as_ptr() as usize, pos, mode);
    let spawn: unsafe extern "system" fn(u32, usize, u32) = std::mem::transmute(base + SPAWN_RVA);
    for (t, n) in kept.iter().enumerate() {
        for _ in 0..*n {
            spawn(t as u32, pos, mode);
        }
    }
    for _ in 0..small {
        spawn(SMALL as u32, pos, mode);
    }
    for _ in 0..big {
        spawn(BIG as u32, pos, mode);
    }
    let n = CONVERTED_LOGGED.fetch_add(1, Ordering::Relaxed);
    if n < 40 || n % 50 == 0 {
        let worth = |c: [i32; 3]| c[0] + c[1] * 5 + c[2] * 20;
        log(&format!(
            "Drive Converter x{copies}: munny drop (1 / 5 / 20) {munny:?} ({} munny) -> {kept:?} ({} munny) and {small} small, {big} big Drive orbs{}",
            worth(munny),
            worth(kept),
            if n >= 40 { format!(" (drop {})", n + 1) } else { String::new() }
        ));
    }
    true
}

/// An enemy's own drop (its AI's drop op): as ever (its munny partly Drive orbs with Drive
/// Converter, `converted_drop`), then KH2's Drive orbs for it.
unsafe extern "system" fn ai_drop(counts: usize, pos: usize, mode: u32) {
    let f: AiDropFn = std::mem::transmute(ORIG_AI_DROP.load(Ordering::Acquire));
    let base = base();
    if !converted_drop(f, base, counts, pos, mode) {
        f(counts, pos, mode);
    }
    if counts < 0x10000 || pos < 0x10000 || rd::<u32>(base + PRIZES_OFF_RVA) != 0 {
        return;
    }
    // (Only a drop with something in it: KH1's seven counts.)
    let kh1: [i32; 7] = rd(counts);
    if kh1.iter().all(|&n| n <= 0) {
        return;
    }
    // A Drive Recovery item, maybe (`items`: KH1's field item drop).
    super::items::enemy_drop(base, kh1, pos, mode);
    if !on() {
        return;
    }
    let (small, big) = crate::drivegauge::drop_for(roll());
    let spawn: unsafe extern "system" fn(u32, usize, u32) = std::mem::transmute(base + SPAWN_RVA);
    for _ in 0..small {
        spawn(SMALL as u32, pos, mode);
    }
    for _ in 0..big {
        spawn(BIG as u32, pos, mode);
    }
    if DROPS_LOGGED.fetch_add(1, Ordering::Relaxed) < 80 {
        let p: [f32; 3] = rd(pos);
        log(&format!(
            "Drive orbs: an enemy's drop (KH1's counts {kh1:?}, mode {mode}) at ({:.0}, {:.0}, {:.0}): {small} small, {big} big",
            p[0], p[1], p[2]
        ));
    }
}

type PppFn = unsafe extern "system" fn(usize, usize) -> u64;

/// The program's per-type entry (`+4 + type × 12`: sprite index, RGBA bytes, scale).
fn entry(data: usize, t: usize) -> usize {
    data + 4 + t * 12
}

/// `pppRyjMegaBirthPrize`'s draw: its entries 7 and 8 are KH2's Drive orb (`prizeart`), or until
/// that is ready (or if it can't be) MP's orbs recoloured (written each time they differ, so a
/// reloaded `prize.dpx` gets them too).
unsafe extern "system" fn ppp_draw(prog: usize, data: usize) -> u64 {
    if on() && data > 0x10000 && super::crash::readable(data) && super::crash::readable(entry(data, 9) + 11) {
        let kh2 = super::prizeart::entries(base(), data);
        for (k, (ours, mp)) in [(SMALL as usize, 5usize), (BIG as usize, 6)].into_iter().enumerate() {
            let want = match kh2 {
                Some(e) => e[k],
                None => {
                    let src: [u8; 12] = rd(entry(data, mp));
                    let mut want = src;
                    let m = src[4].max(src[5]).max(src[6]) as f32;
                    for c in 0..3 {
                        want[4 + c] = (m * DRIVE_TINT[c]).round().clamp(0.0, 255.0) as u8;
                    }
                    want
                }
            };
            let have: [u8; 12] = rd(entry(data, ours));
            if have != want {
                static LOGGED: AtomicU32 = AtomicU32::new(0);
                if LOGGED.fetch_add(1, Ordering::Relaxed) < 4 {
                    let all: Vec<String> = (0..10).map(|t| format!("{t}: {:02x?}", rd::<[u8; 12]>(entry(data, t)))).collect();
                    log(&format!("Drive orbs: the prize effect's entries at {data:#x} before: {}", all.join(" | ")));
                }
                // Another shape: our prizes' animation steps start over (a step past the new
                // shape's last would name no frame), as KH1 starts a new prize's.
                if have[..4] != want[..4] {
                    restart(ours as u8);
                }
                wr(entry(data, ours), want);
            }
        }
    }
    let f: PppFn = std::mem::transmute(ORIG_PPP.load(Ordering::Acquire));
    f(prog, data)
}

/// Every live prize of type `ty`: its animation step and timer back to 0.
unsafe fn restart(ty: u8) {
    let records = base() + RECORDS_RVA;
    for i in 0..RECORDS {
        let r = records + i * 0x20;
        if rd::<u8>(r + 0x18) == ty && rd::<u8>(r + 0x19) & 1 != 0 {
            wr::<u32>(r + 0x1C, 0);
        }
    }
}

/// The Drive orbs' parts: the row table, the draw and the pickup. False (none dropped) if any
/// isn't as expected.
unsafe fn install_orbs(base: usize) -> bool {
    if !relocate_table(base) {
        return false;
    }
    if hook::install(base + PPP_DRAW_RVA, &PPP_DRAW_PROLOGUE, ppp_draw as *const () as usize, &ORIG_PPP, "prize effect draw (Drive orbs)").is_none() {
        log("Drive orbs: the prize effect's draw wasn't as expected: none dropped (they would not show)");
        return false;
    }
    if hook::install(base + PICKUP_RVA, &PICKUP_PROLOGUE, pickup as *const () as usize, &ORIG_PICKUP, "prize pickup (Drive orbs)").is_none() {
        log("Drive orbs: KH1's pickup wasn't as expected: none dropped");
        return false;
    }
    true
}

pub unsafe fn install() {
    let base = base();
    if !super::settings::get().drive {
        return;
    }
    let orbs = install_orbs(base);
    // The enemy drop's hook also drops the Drive Recovery items (`items::enemy_drop`), so it goes
    // in even without the orbs.
    if hook::install(base + AI_DROP_RVA, &AI_DROP_PROLOGUE, ai_drop as *const () as usize, &ORIG_AI_DROP, "enemy prize drop (Drive orbs, Drive Recovery items)").is_none() {
        log("Drive orbs: KH1's enemy drop wasn't as expected: no Drive orbs or Drive Recovery items dropped");
        return;
    }
    if !orbs {
        return;
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64);
    SEED.store(0x9E37_79B9_7F4A_7C15 ^ now | 1, Ordering::Relaxed);
    ON.store(true, Ordering::Release);
    log("Drive orbs: KH1 prize types 7 (small) and 8 (big), dropped by enemies as KH2's are, picked up into the Drive gauge");
    super::prizeart::install(base);
}
