//! KH2's own Drive orb art on the Drive orb prizes (the user: "keep the drive orbs running through
//! KH1s engine, but if possible, let's try to get the texture from KH2"). `docs/kh1-prizes.md`,
//! "KH2's prize art" and "The Drive orbs' look".
//!
//! **The art**: KH2 draws its prizes with the same effect program as KH1 (`pppRyjMegaBirthPrize`),
//! from `obj/PRIZE.mdlx` (`kh2_formats::prize`): its types 7 and 8, the small and big Drive orb,
//! are one sprite animation (a gold orb under a turning ring) at two scales, on a 128 × 64 texture
//! page whose remastered image (`-0.dds`, 256 × 128) the package carries. Read on a thread at
//! start from the user's KH2 install; nothing of it is kept on disk or in the mod.
//!
//! **Drawn by KH1**: KH1's prize program picks each type's sprite as an index into its effect's
//! shape table (`*0x142E9DE08 + 0x28`, 16 bytes a shape: the animation, a packet size). Our
//! types' entries name a slot of ours instead (the index is signed and 64-bit scaled, so a slot
//! near the game is reachable), holding a copy of KH2's animation whose frames' texture register
//! names a page and palette past the end of the GS's memory, which no KH1 texture has. KH1's
//! texture map lookup (`0x14010D860`, hooked) answers that register with a texture entry of ours:
//! a copy of the entry of KH1's own prize texture with our GPU texture (KH2's image, made by KH1's
//! creator, `tint::make_texture`) and its sizes. Everything else is KH1's: the stepping of the
//! animation, the scale, the colour and blink, the sorting and the draw.
//!
//! **Sizes**: KH2's scales for its small and big orbs, times how KH1 sizes its MP orb against
//! KH2's (their first frames' reach × their entries' scales).
//!
//! If the art can't be read or set up, `entries` says so (the caller keeps KH1's MP orbs tinted)
//! and the log says why.

use super::addr::Rva;
use super::{hook, log};
use kh2_formats::prize::{PrizeArt, DRIVE_BIG, DRIVE_SMALL, FILE, MP_BIG, MP_SMALL};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

/// The effect being run's DPD (its shape table at `+0x28`).
const DPD_RVA: Rva = Rva(0x2E9DE08);
/// KH1's texture map: `(&TEX0) -> entry`.
const LOOKUP_RVA: Rva = Rva(0x10D860);
/// `push rdi; sub rsp, 0x30; mov qword [rsp + 0x20], -2`.
const LOOKUP_PROLOGUE: [u8; 15] = [0x40, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE, 0xFF, 0xFF, 0xFF];
/// `Axa::EntryTex`: its magic, the GPU texture, the HD size, the PS2 size.
const TEX_MAGIC: u32 = 0x02F0_2EB5;
const TEX_GPU: usize = 0x70;
const TEX_HD_W: usize = 0x7C;
const TEX_PS2_W: usize = 0x94;
/// How much of KH1's entry is copied into ours.
const ENTRY_LEN: usize = 0xC0;
/// The page and palette block address our frames' texture register names: the GS has 0x4000
/// blocks, so nothing of KH1's is a whole texture there.
const MARK: u64 = 0x3FFF;
const TBP_MASK: u64 = 0x3FFF;
const CBP_SHIFT: u32 = 37;
/// Our memory near the game: the two shape slots, our texture entry, the animations.
const AREA: usize = 0x10000;
const SLOTS_AT: usize = 0;
const ENTRY_AT: usize = 0x40;
const ANIM_AT: usize = 0x200;

/// What is read from KH2 (a thread at start).
struct Art {
    /// KH2's small and big Drive orbs: their shape's animation, scale, colour.
    orbs: [(Vec<u8>, f32, [u8; 4]); 2],
    shape: i32,
    steps: usize,
    /// KH2's MP orbs' reach × scales (small + big), to size ours as KH1 sizes its MP orbs.
    mp: f32,
    drive: [f32; 2],
    /// The image, BGRA, its size, and the PS2 page size the shapes' coordinates count in.
    bgra: Vec<u8>,
    w: u32,
    h: u32,
    ps2: (u32, u32),
    /// Where the image came from (for the log).
    image: String,
    /// KH2's shape animation speed (the program adds it to a prize's step timer each run).
    speed: i32,
}

static ART: OnceLock<Result<Art, String>> = OnceLock::new();
static ORIG_LOOKUP: AtomicUsize = AtomicUsize::new(0);
/// Our texture entry (0 until made): what the lookup answers our register with.
static ENTRY: AtomicUsize = AtomicUsize::new(0);
static FAILED: AtomicBool = AtomicBool::new(false);
static HOOKED: AtomicBool = AtomicBool::new(false);

/// Set up on the game thread at the first draw with the art read.
struct Ready {
    area: usize,
    /// Our copies of the small and big orbs' animations.
    anim: [usize; 2],
    /// The entries' scales, sized against KH1's MP orbs.
    scale: [f32; 2],
    rgba: [[u8; 4]; 2],
}

static READY: Mutex<Option<Ready>> = Mutex::new(None);

fn fail(why: &str) {
    if !FAILED.swap(true, Ordering::Relaxed) {
        log(&format!("Drive orbs: KH2's art not used ({why}); they are KH1's MP orbs in KH2's drive-orb yellow instead"));
    }
}

/// Reads KH2's prize art on a thread, and hooks KH1's texture lookup for it.
pub unsafe fn install(base: usize) {
    if hook::install(base + LOOKUP_RVA, &LOOKUP_PROLOGUE, lookup as *const () as usize, &ORIG_LOOKUP, "texture lookup (KH2's Drive orb art)").is_none() {
        fail("KH1's texture lookup wasn't as expected");
        return;
    }
    HOOKED.store(true, Ordering::Release);
    std::thread::spawn(|| {
        let r = load_art();
        if let Err(e) = &r {
            fail(e);
        }
        let _ = ART.set(r);
    });
}

fn load_art() -> Result<Art, String> {
    let exe = format!("{}\\KINGDOM HEARTS II FINAL MIX.exe", super::exe_dir());
    let install = kh2_data::Install::locate(&exe).map_err(|e| e.to_string())?;
    let src = if install.has_backup() { kh2_data::Source::Backup } else { kh2_data::Source::Live };
    let bytes = install.read_any(src, FILE).map_err(|e| format!("{FILE}: {e}"))?;
    let art = PrizeArt::parse(&bytes).map_err(|e| format!("{FILE}: {e}"))?;
    let e = art.entries;
    let (Some(small), Some(big)) = (e[DRIVE_SMALL].shape, e[DRIVE_BIG].shape) else {
        return Err(format!("{FILE}: no shape for prize types 7 and 8"));
    };
    let s = art.shape(small).map_err(|e| format!("{FILE}: {e}"))?;
    let b = if big == small { s.clone() } else { art.shape(big).map_err(|e| format!("{FILE}: {e}"))? };
    if b.tex0.tbp() != s.tex0.tbp() || b.tex0.tw() != s.tex0.tw() || b.tex0.th() != s.tex0.th() {
        return Err(format!("{FILE}: the small and big Drive orbs are on different pages"));
    }
    let page = art.page(s.tex0).ok_or(format!("{FILE}: no texture at the Drive orb's page {:#x}", s.tex0.tbp()))?;
    // KH2's MP orbs, for the sizes.
    let mp = match e[MP_SMALL].shape.map(|n| art.shape(n)) {
        Some(Ok(m)) => m.extent * (e[MP_SMALL].scale + e[MP_BIG].scale),
        _ => 0.0,
    };
    // The image: the remastered page, else the PS2 page.
    let mut image = format!("the PS2 page ({}x{}, its textures with their palettes)", page.width, page.height);
    let (mut rgba, mut w, mut h) = (page.rgba.clone(), page.width, page.height);
    let want = format!("-{}.", page.index);
    match install.read_remastered_any(src, FILE) {
        Ok(assets) => match assets.iter().find(|(n, _)| n.starts_with(&want)) {
            Some((name, dds)) => match kh2_formats::dds::decode(dds) {
                Ok(im) if im.width as u64 * page.height as u64 == im.height as u64 * page.width as u64 => {
                    image = format!("its remastered image {name} ({}x{} for the PS2 page's {}x{})", im.width, im.height, page.width, page.height);
                    (rgba, w, h) = (im.rgba, im.width, im.height);
                }
                Ok(im) => log(&format!("Drive orbs: {FILE}'s {name} is {}x{}, not the shape of its {}x{} page: the PS2 page is used", im.width, im.height, page.width, page.height)),
                Err(e) => log(&format!("Drive orbs: {FILE}'s {name}: {e}; the PS2 page is used")),
            },
            None => log(&format!("Drive orbs: {FILE} has no remastered image {want}dds; the PS2 page is used")),
        },
        Err(e) => log(&format!("Drive orbs: {FILE}'s remastered images: {e}; the PS2 page is used")),
    }
    let bgra = rgba.chunks_exact(4).flat_map(|p| [p[2], p[1], p[0], p[3]]).collect();
    log(&format!(
        "Drive orbs: KH2's art read from {FILE} ({}): prize types 7 and 8 are its shape {small}{} ({} steps, {} frames, texture page {:#x} palette {:#x}) at scales {} and {}; image {image}",
        if src == kh2_data::Source::Backup { "the backup archives" } else { "the live archives" },
        if big != small { format!(" and {big}") } else { String::new() },
        s.steps,
        s.frames.len(),
        s.tex0.tbp(),
        s.tex0.cbp(),
        e[DRIVE_SMALL].scale,
        e[DRIVE_BIG].scale
    ));
    let marked = |sh: &kh2_formats::prize::ShapeArt| -> Vec<u8> {
        let mut a = sh.animation.clone();
        for &f in &sh.frames {
            let at = f + 16;
            let t = u64::from_le_bytes(a[at..at + 8].try_into().unwrap());
            let t = (t & !TBP_MASK & !(TBP_MASK << CBP_SHIFT)) | MARK | (MARK << CBP_SHIFT);
            a[at..at + 8].copy_from_slice(&t.to_le_bytes());
        }
        a
    };
    Ok(Art {
        orbs: [(marked(&s), e[DRIVE_SMALL].scale, e[DRIVE_SMALL].rgba), (marked(&b), e[DRIVE_BIG].scale, e[DRIVE_BIG].rgba)],
        shape: small,
        steps: s.steps,
        mp,
        drive: [s.extent, b.extent],
        bgra,
        w,
        h,
        ps2: (1u32 << s.tex0.tw().min(11), 1u32 << s.tex0.th().min(11)),
        image,
        speed: art.speed,
    })
}

type LookupFn = unsafe extern "system" fn(*const u64) -> usize;

/// KH1's texture map lookup: our frames' register gets our entry, everything else KH1's.
unsafe extern "system" fn lookup(tex0: *const u64) -> usize {
    let ours = ENTRY.load(Ordering::Acquire);
    if ours != 0 && !tex0.is_null() {
        let t = std::ptr::read_unaligned(tex0);
        if t & TBP_MASK == MARK && (t >> CBP_SHIFT) & TBP_MASK == MARK {
            return ours;
        }
    }
    let f: LookupFn = std::mem::transmute(ORIG_LOOKUP.load(Ordering::Acquire));
    f(tex0)
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}
use super::crash::readable;

/// A shape animation's first frame's reach (the largest corner coordinate, shape units).
unsafe fn reach(anim: usize) -> Option<f32> {
    if !readable(anim + 0x18) {
        return None;
    }
    let frame = anim.wrapping_add(rd::<i16>(anim + 0x10) as isize as usize);
    if !readable(frame + 0x20) {
        return None;
    }
    let (flag, n) = (rd::<i16>(frame), rd::<i16>(frame + 2));
    if !(0..=64).contains(&n) {
        return None;
    }
    let size = if flag & 8 != 0 { 24 } else { 16 };
    let corners = if flag & 8 != 0 { 8 } else { 4 };
    if !readable(frame + 0x20 + n as usize * size) {
        return None;
    }
    let mut r = 0.0f32;
    for i in 0..n as usize {
        for c in 0..corners {
            r = r.max((rd::<i16>(frame + 0x20 + i * size + c * 2) as f32 / 16.0).abs());
        }
    }
    Some(r)
}

/// The prize effect's entries for types 7 and 8 (12 bytes each: sprite index, colour, scale)
/// that draw KH2's Drive orb, or `None` (KH2's art not there, not read yet, or not usable: the
/// caller uses KH1's MP orbs). `data` is the program's data (KH1's entries at `+4`); called from
/// the program's draw, on the game thread, before KH1's.
pub unsafe fn entries(base: usize, data: usize) -> Option<[[u8; 12]; 2]> {
    if FAILED.load(Ordering::Relaxed) || !HOOKED.load(Ordering::Acquire) {
        return None;
    }
    let art = match ART.get()? {
        Ok(a) => a,
        Err(_) => return None,
    };
    // The shape table, and KH1's MP orb's shape in it.
    let dpd: usize = rd(base + DPD_RVA);
    if dpd < 0x10000 || !readable(dpd + 0x30) {
        return None;
    }
    let table: usize = rd(dpd + 0x28);
    let mp = data + 4 + MP_SMALL * 12;
    let mp_index: i32 = rd(mp);
    if table < 0x10000 || mp_index == 0xFFFF {
        return None;
    }
    let mp_slot = table.wrapping_add((mp_index as isize as usize).wrapping_mul(16));
    if !readable(mp_slot + 16) {
        return None;
    }
    let mut ready = READY.lock().unwrap_or_else(|e| e.into_inner());
    if ready.is_none() {
        *ready = Some(prepare(base, art, mp_slot, data)?);
    }
    let r = ready.as_ref()?;
    // Our slots, 16 bytes each at the table's alignment so their distance from it is a whole
    // index: our animation, then KH1's MP slot's packet size.
    let slots = (r.area + SLOTS_AT + 15) & !15;
    let mut out = [[0u8; 12]; 2];
    for (k, o) in out.iter_mut().enumerate() {
        let slot = slots + k * 16 + (table & 15);
        let d = slot as i64 - table as i64;
        let index = d / 16;
        if d % 16 != 0 || index < i32::MIN as i64 || index > i32::MAX as i64 || index as i32 as u32 == 0xFFFF {
            drop(ready);
            fail(&format!("the prize effect's shape table at {table:#x} is out of reach of our slot at {slot:#x}"));
            return None;
        }
        wr(slot, r.anim[k] as u64);
        wr(slot + 8, rd::<u64>(mp_slot + 8));
        o[..4].copy_from_slice(&(index as i32).to_le_bytes());
        o[4..8].copy_from_slice(&r.rgba[k]);
        o[8..].copy_from_slice(&r.scale[k].to_le_bytes());
    }
    Some(out)
}

/// Makes our texture and entry and copies the animations (once).
unsafe fn prepare(base: usize, art: &Art, mp_slot: usize, data: usize) -> Option<Ready> {
    let mp_anim: usize = rd(mp_slot);
    let frame = mp_anim.wrapping_add(rd::<i16>(mp_anim + 0x10) as isize as usize);
    if !readable(frame + 0x18) {
        fail("KH1's MP orb shape isn't readable");
        return None;
    }
    // KH1's prize texture's entry, the model for ours.
    let kh1_tex0: u64 = rd(frame + 0x10);
    let orig: LookupFn = std::mem::transmute(ORIG_LOOKUP.load(Ordering::Acquire));
    let kh1 = orig(&kh1_tex0);
    if kh1 < 0x10000 || !readable(kh1 + ENTRY_LEN) || rd::<u32>(kh1 + 0x10) != TEX_MAGIC {
        fail(&format!("no texture entry for KH1's MP orb's texture ({kh1_tex0:#x}: {kh1:#x})"));
        return None;
    }
    let Some(area) = hook::alloc_near(base, AREA) else {
        fail("no memory near the game");
        return None;
    };
    let kh1_speed: u16 = rd(data + 0x7C);
    let mut anim = [0usize; 2];
    let mut at = area + ANIM_AT;
    for (k, (a, _, _)) in art.orbs.iter().enumerate() {
        if at + a.len() > area + AREA {
            fail("KH2's Drive orb animation is too long");
            return None;
        }
        std::ptr::copy_nonoverlapping(a.as_ptr(), at as *mut u8, a.len());
        // KH1's program steps the timer by its own speed: the steps' waits scaled so each lasts
        // as many runs as in KH2.
        if kh1_speed > 0 && art.speed > 0 && kh1_speed as i32 != art.speed {
            let steps = rd::<i16>(at + 6).max(0) as usize;
            for i in 0..steps {
                let w = at + 16 + i * 8 + 2;
                wr::<u16>(w, (rd::<u16>(w) as i64 * kh1_speed as i64 / art.speed as i64).clamp(1, 0xFFFF) as u16);
            }
        }
        anim[k] = at;
        at = (at + a.len() + 15) & !15;
    }
    let Some(gpu) = super::tint::make_texture(art.w, art.h, &art.bgra, b"kh2combat_kh2_drive_orb\0") else {
        fail("KH1 made no texture for KH2's image");
        return None;
    };
    let entry = area + ENTRY_AT;
    std::ptr::copy_nonoverlapping(kh1 as *const u8, entry as *mut u8, ENTRY_LEN);
    wr(entry + TEX_GPU, gpu);
    wr(entry + TEX_HD_W, [art.w as u16, art.h as u16]);
    wr(entry + TEX_PS2_W, [art.ps2.0 as u16, art.ps2.1 as u16]);
    // Sizes: KH2's, times how KH1 sizes its MP orbs against KH2's.
    let kh1_mp = reach(mp_anim).map(|r| r * (rd::<f32>(data + 4 + MP_SMALL * 12 + 8) + rd::<f32>(data + 4 + MP_BIG * 12 + 8)));
    let k = match kh1_mp {
        Some(m) if art.mp > 0.0 && (m / art.mp).is_finite() && (0.2..=5.0).contains(&(m / art.mp)) => m / art.mp,
        other => {
            log(&format!("Drive orbs: KH1's MP orb size against KH2's not found ({other:?} against {}): KH2's scales as they are", art.mp));
            1.0
        }
    };
    let scale = [art.orbs[0].1 * k, art.orbs[1].1 * k];
    ENTRY.store(entry, Ordering::Release);
    log(&format!(
        "Drive orbs: drawn with KH2's art: shape {} ({} steps, reach {} / {}) in KH1's prize effect, its texture register answered with our entry {entry:#x} (a copy of KH1's prize texture's {kh1:#x}, texture {gpu:#x} from {FILE}'s {}, PS2 {}x{}); scales {:.3} and {:.3} (KH2's × {k:.3}, KH1's MP orb {:?} against KH2's {}); animation speed KH1 {kh1_speed}, KH2 {}",
        art.shape, art.steps, art.drive[0], art.drive[1], art.image, art.ps2.0, art.ps2.1, scale[0], scale[1], kh1_mp, art.mp, art.speed
    ));
    Some(Ready { area, anim, scale, rgba: [art.orbs[0].2, art.orbs[1].2] })
}
