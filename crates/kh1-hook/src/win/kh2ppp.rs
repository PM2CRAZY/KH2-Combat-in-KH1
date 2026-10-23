//! KH2's own particle programs in KH1's program machine (the effects port's steps 3 and 4;
//! `docs/kh2-effects.md` §17).
//!
//! KH1 and KH2 run the same particle program machine ("ppp"), and programs 1..235 are the same
//! in both but for a few that KH2 rewrote; 236..242 are KH2's only. KH1's program table (Steam
//! `0x14054FF00`, 0x50 bytes an entry: name, calc `+0x08`, draw `+0x18`, construct `+0x38`, reset
//! `+0x40`, destruct `+0x48`) can't grow in place, and KH1's relocation (`0x14031C2F0`) turns each
//! program record's type into a handle of its entry. So a KH2 program goes in as an entry of ours
//! in KH1's format: once KH1 has relocated our DPX, the records (and type-list words) that are
//! KH2's own type are pointed at our entry (a handle from KH1's `0x14038AD90`). KH1's machine then
//! calls it with KH1's effect instance, as it calls its own. No hook, nothing per frame beside
//! the program itself.
//!
//! 201 `pppRyjMegaBirth`, KH2's version (Steam KH2: calc `0x1404BC3B0`, step
//! `0x1404BBBE0`, birth `0x1404BAC40`, update `0x1404BB8B0`, draw `0x1404BBDB0`, particle matrix
//! `0x1404BC710`), written against KH1's instance and globals and drawing each particle with
//! KH1's own shape draw (`0x14031E210`, as KH1's own MegaBirth `0x140320DF0` does). What KH2
//! changed from KH1's (`0x1403212F0` and the functions it calls): spin as a 16-bit angle and
//! the particle drawn through the sine table, grow-in / shrink-out sizes (key `0xD3` / `0xD4`),
//! random sprite mirroring (`0xD2`), a random drift speed (`0xD8`), matrix mode 3 (the base and
//! instance matrices at birth), direction `w` 0, and the particle record laid out anew (same
//! 0x60 bytes). Construct and destruct are KH1's own (`0x140321520` / `0x1403215A0`, the same as
//! KH2's `0x1404BC5F0` / `0x1404BC670`). KH1's math (matrix product `0x1400E65E0`, point
//! transform `0x140386A40`, normalise `0x140386C40`, rotation from angles `0x1400E6730`, random
//! `0x14031BA30`) is KH2's same library (same sizes and code as KH2's `0x14013F850`,
//! `0x14013F900`, `0x140142080`, `0x140140560`, `0x1404DC710`), so ours calls KH1's.
//!
//! 241 `pppEiDrawShipoly` (KH2 only): the ribbon, recorded, splined and drawn as KH2 does,
//! through KH1's renderer (below; `docs/kh2-effects.md` §17.3).
//!
//! 43 `pppColor`, KH2's version (Steam KH2 `0x1404C0F60`): KH1's (`0x14032B940`) writes the
//! colour bytes only under its not-held / not-paused / not-repeat test, KH2's writes them on every
//! call (only the key deltas wait for that test) and clamps each channel at 32767 before the
//! scale. Only the words of our DPX point at it, so KH1's own effects keep KH1's pppColor.
//! Construct and reset are KH1's (`0x14032BAE0`: the colour words zeroed, as `kh2-fx` does).
//!
//! Steam only (the addresses are Steam's); elsewhere KH1's own programs stay.

use super::log;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// KH1 code (Steam RVAs).
const ALLOC_STEAM: usize = 0x1EC330;
const TO_HANDLE_STEAM: usize = 0x38AD90;
const RAND01_STEAM: usize = 0x31BA30;
const MUL_STEAM: usize = 0xE65E0;
const XFORM_STEAM: usize = 0x386A40;
const NORMALIZE_STEAM: usize = 0x386C40;
const EULER_STEAM: usize = 0xE6730;
const SHAPE_DRAW_STEAM: usize = 0x31E210;
const MB_CONSTRUCT_STEAM: usize = 0x321520;
const MB_DESTRUCT_STEAM: usize = 0x3215A0;
/// KH1's pppColor: calc (checked before plugging) and construct / reset.
const KH1_COLOR_CALC_STEAM: usize = 0x32B940;
const COLOR_CONSTRUCT_STEAM: usize = 0x32BAE0;
/// KH1's program table and the entry of its MegaBirth's calc (checked before plugging).
const TABLE_STEAM: usize = 0x54FF00;
const KH1_MB_CALC_STEAM: usize = 0x3212F0;
// KH1's effect globals (Steam RVAs), KH2's in brackets.
/// Effects held (`0x142B0EFC0`).
const HELD: usize = 0x2E9D7E0;
/// The effect being stepped (`0x142B0EC00`): `+0xD5` paused (KH2 `+0xC1`), `+0xD8` the repeat
/// frame of a tick (KH2 `+0xC4`).
const EFFECT: usize = 0x2E9D420;
/// The effect's DPD resources (`0x142B0EC08`): the allocator first, models `+0x20` (0x38 each,
/// vertices `+0x18`), shapes `+0x28` (0x10 each: animation, texture), vertex sets `+0x30`.
const CTX: usize = 0x2E9D428;
/// The instance being stepped (`0x142B0EC10`): `+0x10` its matrix, `+0x90` vertex override.
const OBJ: usize = 0x2E9D430;
/// The effect's root (`0x142B0EC20`), view (`0x142B0EC60`), projection (`0x142B0EDE0`), root
/// position (`0x142B0EC50`, KH1 keeps a copy) and the root's axis lengths (`0x142B0F010`).
const ROOT: usize = 0x2E9D440;
const VIEW: usize = 0x2E9D480;
const PROJ: usize = 0x2E9D600;
const ROOT_T: usize = 0x2E9D810;
const RS: usize = 0x2E9D830;
/// The draw packets' buffer cursor (`0x142B0EFD0`).
const CHAIN: usize = 0x2E9D7F0;
/// Set while an instance loop fast-forwards (KH2 skips MegaBirth then: its list `0x140779990`;
/// KH1's list `0x14054FEE0` holds KH1's MegaBirth, not ours).
const FAST_FORWARD: usize = 0x2E9D804;
/// KH1's sine table (4096 entries, the same values as KH2's `0x14077E640`).
const SINE: usize = 0x54BED0;

/// KH2's program types ours stand in for.
pub const MEGA_BIRTH: u32 = 201;
pub const SHIPOLY: u32 = 241;
pub const COLOR: u32 = 43;

static BASE: AtomicUsize = AtomicUsize::new(0);
/// KH2's MegaBirth / ribbon are plugged into the DPX (their words point at our entries).
static PLUGGED_MB: AtomicBool = AtomicBool::new(false);
static PLUGGED_SHIPOLY: AtomicBool = AtomicBool::new(false);
static PLUGGED_COLOR: AtomicBool = AtomicBool::new(false);

/// Whether KH2's own program `ty` is in (an effect that runs it may then play through KH1).
pub fn plugged(ty: u32) -> bool {
    match ty {
        MEGA_BIRTH => PLUGGED_MB.load(Ordering::Relaxed),
        SHIPOLY => PLUGGED_SHIPOLY.load(Ordering::Relaxed),
        COLOR => PLUGGED_COLOR.load(Ordering::Relaxed),
        _ => false,
    }
}

#[repr(C)]
struct Entry {
    name: *const u8,
    slots: [usize; 9],
}

/// Our program entries (KH1's 0x50-byte format). Written once on the game thread before any
/// record points at them; never moved.
static mut TABLE: [Entry; 3] = [const { Entry { name: std::ptr::null(), slots: [0; 9] } }; 3];
static NAME_MB: &[u8] = b"pppRyjMegaBirth (KH2)\0";
static NAME_SHIPOLY: &[u8] = b"pppEiDrawShipoly (KH2)\0";
static NAME_COLOR: &[u8] = b"pppColor (KH2)\0";

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}
fn base() -> usize {
    BASE.load(Ordering::Relaxed)
}
unsafe fn ptr(h: u32) -> usize {
    super::pose::ptr(h)
}
unsafe fn to_handle(p: usize) -> u32 {
    let f: unsafe extern "system" fn(usize) -> u32 = std::mem::transmute(base() + TO_HANDLE_STEAM);
    f(p)
}
unsafe fn rand01() -> f32 {
    let f: unsafe extern "system" fn() -> f32 = std::mem::transmute(base() + RAND01_STEAM);
    f()
}
/// `out = a ∘ b` (KH1's product: each row of `b` transformed by `a`).
unsafe fn mul(out: usize, a: usize, b: usize) {
    let f: unsafe extern "system" fn(usize, usize, usize) = std::mem::transmute(base() + MUL_STEAM);
    f(out, a, b)
}
/// `out = v · m` (a 4-vector through a matrix).
unsafe fn xform(out: usize, m: usize, v: usize) {
    let f: unsafe extern "system" fn(usize, usize, usize) = std::mem::transmute(base() + XFORM_STEAM);
    f(out, m, v)
}
/// `out = normalise(v.xyz)`, `out.w = 0`.
unsafe fn normalize(out: usize, v: usize) {
    let f: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base() + NORMALIZE_STEAM);
    f(out, v)
}
/// `out = m` turned by the angles (radians) at `a`.
unsafe fn euler(out: usize, m: usize, a: usize) {
    let f: unsafe extern "system" fn(usize, usize, usize) = std::mem::transmute(base() + EULER_STEAM);
    f(out, m, a)
}
unsafe fn alloc(size: u32) -> u32 {
    let f: unsafe extern "system" fn(usize, u32) -> usize = std::mem::transmute(base() + ALLOC_STEAM);
    let p = f(rd::<usize>(base() + CTX), size);
    let h = to_handle(p);
    let q = ptr(h);
    if h != 0 && q != 0 {
        std::ptr::write_bytes(q as *mut u8, 0, size as usize);
    }
    h
}
unsafe fn identity(m: usize) {
    for i in 0..16 {
        wr::<f32>(m + i * 4, if i % 5 == 0 { 1.0 } else { 0.0 });
    }
}
unsafe fn copy(dst: usize, src: usize, n: usize) {
    std::ptr::copy(src as *const u8, dst as *mut u8, n);
}
/// `v.xyz *= s` (w kept).
unsafe fn scale3(out: usize, v: usize, s: f32) {
    for k in 0..3 {
        wr::<f32>(out + k * 4, s * rd::<f32>(v + k * 4));
    }
    wr::<f32>(out + 12, rd::<f32>(v + 12));
}
/// `out = a + b` (4 floats).
unsafe fn add4(out: usize, a: usize, b: usize) {
    for k in 0..4 {
        wr::<f32>(out + k * 4, rd::<f32>(b + k * 4) + rd::<f32>(a + k * 4));
    }
}

/// Points our entries at their functions (once); entry 0 MegaBirth, 1 the ribbon, 2 the colour.
unsafe fn table(i: usize) -> usize {
    let t = std::ptr::addr_of_mut!(TABLE) as *mut Entry;
    if (*t).name.is_null() {
        let b = base();
        (*t).slots = [mb_calc as *const () as usize, 0, mb_draw as *const () as usize, 0, 0, 0, b + MB_CONSTRUCT_STEAM, 0, b + MB_DESTRUCT_STEAM];
        (*t).name = NAME_MB.as_ptr();
        let r = t.add(1);
        (*r).slots = [
            sp_calc as *const () as usize,
            0,
            sp_draw as *const () as usize,
            0,
            0,
            0,
            sp_construct as *const () as usize,
            sp_reset as *const () as usize,
            sp_destruct as *const () as usize,
        ];
        (*r).name = NAME_SHIPOLY.as_ptr();
        let c = t.add(2);
        (*c).slots = [col_calc as *const () as usize, 0, 0, 0, 0, 0, b + COLOR_CONSTRUCT_STEAM, b + COLOR_CONSTRUCT_STEAM, 0];
        (*c).name = NAME_COLOR.as_ptr();
    }
    t.add(i) as usize
}

/// After KH1 relocated the DPX at `dpx`: each KH2-own program word (`programs`: offset in the
/// DPX, KH2 type) that KH1 pointed at its own entry is pointed at ours. Steam only.
pub fn plug(dpx: usize, programs: &[(usize, u32)]) {
    if super::addr::Build::current() != super::addr::Build::Steam || programs.is_empty() {
        return;
    }
    let b = unsafe { super::sys::GetModuleHandleA(std::ptr::null()) } as usize;
    BASE.store(b, Ordering::Relaxed);
    unsafe {
        // KH1's MegaBirth entry, as its relocation hands it out.
        let kh1_mb = b + TABLE_STEAM + MEGA_BIRTH as usize * 0x50;
        if rd::<usize>(kh1_mb + 8) != b + KH1_MB_CALC_STEAM {
            log("KH2 effects (native): KH1's program table isn't as expected; KH2's MegaBirth not plugged in");
            return;
        }
        // The words of KH2-only programs were relocated to KH1's dummy entry (1).
        let kh1_dummy = b + TABLE_STEAM + 0x50;
        // KH1's pppColor entry: only our DPX's words of it are pointed at KH2's.
        let kh1_col = b + TABLE_STEAM + COLOR as usize * 0x50;
        let col_ok = rd::<usize>(kh1_col + 8) == b + KH1_COLOR_CALC_STEAM;
        if !col_ok {
            log("KH2 effects (native): KH1's pppColor entry isn't as expected; KH2's pppColor not plugged in (KH1's runs)");
        }
        let (ours_mb, ours_sp, ours_col) = (to_handle(table(0)), to_handle(table(1)), to_handle(table(2)));
        if ours_mb == 0 || ours_sp == 0 || ours_col == 0 {
            log("KH2 effects (native): no handle for our program table; KH2's programs not plugged in");
            return;
        }
        let (mut mb, mut sp, mut col, mut other) = (0, 0, 0, 0);
        for &(at, ty) in programs {
            let w = dpx + at;
            let to = ptr(rd::<u32>(w));
            if ty == MEGA_BIRTH && to == kh1_mb {
                wr::<u32>(w, ours_mb);
                mb += 1;
            } else if ty == SHIPOLY && to == kh1_dummy {
                wr::<u32>(w, ours_sp);
                sp += 1;
            } else if ty == COLOR && col_ok && to == kh1_col {
                wr::<u32>(w, ours_col);
                col += 1;
            } else {
                other += 1;
            }
        }
        PLUGGED_MB.store(mb > 0, Ordering::Relaxed);
        PLUGGED_SHIPOLY.store(sp > 0, Ordering::Relaxed);
        PLUGGED_COLOR.store(col > 0, Ordering::Relaxed);
        log(&format!(
            "KH2 effects (native): KH2's MegaBirth plugged into {mb} program words, its ribbon (EiDrawShipoly) into {sp}, its pppColor into {col} ({other} left as they were)"
        ));
    }
}

/// The colour bytes' scale (0x80 = 1.0): KH1's `0x1403B30B8` (1/128, Checked (exe)); the KH2
/// decomp drops the operand of its scale call (`0x14013FCC0`), taken as KH1's (and `kh2-fx`'s).
const COLOR_SCALE: f32 = 1.0 / 128.0;

/// `cvttss2si` (what `(int)` of a float is in both games: out of range or NaN gives `i32::MIN`).
fn cvtt(x: f32) -> i32 {
    if x.is_nan() || x >= 2147483648.0 || x < -2147483648.0 {
        i32::MIN
    } else {
        x as i32
    }
}

/// KH2's pppColor calc (`0x1404C0F60`) on KH1's instance and globals. Work offset 0: four u16
/// channels (`+0`), the colour bytes the draws read (`+8`). The key's deltas (`+4`) are added on
/// its frame (not held, not paused, bit 0 of the frame word `+0xC` clear, as KH1's), but the
/// bytes are written on every call: each channel times the effect's colour (KH1's instance
/// `+0x30`, KH2's same), clamped at 32767, times 1/128; in monochrome mode RGB become the grey.
/// KH1's writes the bytes only under the key test's first three conditions (`docs/kh2-effects.md`
/// §17.6).
unsafe extern "system" fn col_calc(obj: usize, key: usize, prog: usize) {
    let b = base();
    let w = work(obj, prog, 0);
    let eff = rd::<usize>(b + EFFECT);
    if rd::<u32>(b + HELD) == 0 && rd::<u8>(eff + 0xD5) == 0 && rd::<u32>(obj + 0xC) & 1 == 0 && rd::<u32>(key) == rd::<u32>(obj + 0xC) {
        for c in 0..4 {
            wr::<u16>(w + c * 2, rd::<u16>(w + c * 2).wrapping_add(rd::<u16>(key + 4 + c * 2)));
        }
    }
    let mut v = [0i32; 4];
    for (c, out) in v.iter_mut().enumerate() {
        let mut x = rd::<u16>(w + c * 2) as f32 * rd::<f32>(eff + 0x30 + c * 4);
        if 32767.0 < x {
            x = 32767.0;
        }
        *out = cvtt(x * COLOR_SCALE);
        wr::<u8>(w + 8 + c, *out as u8);
    }
    if rd::<u32>(b + MONO) != 0 {
        let g = cvtt((v[0] & 0xFF) as f32 * 0.3 + (v[1] & 0xFF) as f32 * 0.59 + (v[2] & 0xFF) as f32 * 0.11) as u8;
        for c in 0..3 {
            wr::<u8>(w + 8 + c, g);
        }
    }
}

/// The program's work offsets (`prog + 0xC` relative to the record after KH1's relocation).
unsafe fn work(obj: usize, prog: usize, i: usize) -> usize {
    let list = (prog as isize + rd::<i32>(prog + 0xC) as isize) as usize;
    (obj as isize + 0xA0 + rd::<i32>(list + i * 4) as isize) as usize
}

/// KH2's MegaBirth calc (`0x1404BC3B0`): allocate on the first run, refresh the base matrix every
/// run, then step (`step`).
unsafe extern "system" fn mb_calc(obj: usize, key: usize, prog: usize) {
    let b = base();
    if rd::<u32>(b + FAST_FORWARD) != 0 {
        return;
    }
    let colour = work(obj, prog, 1);
    let st = work(obj, prog, 2);
    let mode = rd::<u8>(key + 0xCC);
    if rd::<u32>(st + 0x50) == 0 {
        let n = rd::<u16>(key + 0x1C) as u32;
        wr::<u32>(st + 0x5C, n);
        wr::<u32>(st + 0x50, alloc(n * 0x60));
        if mode.wrapping_sub(1) < 3 {
            wr::<u32>(st + 0x54, alloc(n << 6));
        }
        if rd::<u8>(key + 0xC9) != 0 {
            wr::<u32>(st + 0x58, alloc(n << 5));
        }
        copy(st + 0x40, key + 0x98, 12);
        wr::<f32>(st + 0x4C, 0.0);
        normalize(st + 0x40, st + 0x40);
    }
    if ptr(rd::<u32>(st + 0x50)) == 0 {
        return;
    }
    if mode.wrapping_sub(1) < 3 && ptr(rd::<u32>(st + 0x54)) == 0 {
        return;
    }
    if rd::<u8>(key + 0xC9) != 0 && ptr(rd::<u32>(st + 0x58)) == 0 {
        return;
    }
    if matches!(rd::<u8>(key + 0x26), 1 | 3 | 5 | 7 | 9) {
        // Position only: the root's axis lengths and place, not its turn.
        identity(st);
        for k in 0..3 {
            scale3(st + k * 0x10, st + k * 0x10, rd::<f32>(b + RS + k * 4));
        }
        copy(st + 0x30, b + ROOT_T, 16);
        wr::<f32>(st + 0x3C, 1.0);
    } else {
        copy(st, b + ROOT, 64);
    }
    step(st, key, colour);
}

/// One tick of the slots (`0x1404BBBE0`; not on a tick's repeat frame): births and updates.
unsafe fn step(st: usize, key: usize, colour: usize) {
    let b = base();
    let eff = rd::<usize>(b + EFFECT);
    if rd::<u32>(b + HELD) != 0 || rd::<u8>(eff + 0xD5) != 0 || rd::<u8>(eff + 0xD8) != 0 || rd::<i32>(key + 4) == 0xFFFF {
        return;
    }
    wr::<u16>(st + 0x60, rd::<u16>(st + 0x60).wrapping_add(1));
    let n = rd::<u32>(st + 0x5C) as i32;
    if n <= 0 {
        return;
    }
    let recs = ptr(rd::<u32>(st + 0x50));
    let mut mats = ptr(rd::<u32>(st + 0x54));
    let mut cols = ptr(rd::<u32>(st + 0x58));
    let mut born = 0i32;
    for i in 0..n as usize {
        let rec = recs + i * 0x60;
        if rd::<u16>(rec + 0x26) == 0 {
            if rd::<u16>(key + 0x20) <= rd::<u16>(st + 0x60) && born < rd::<u16>(key + 0x1E) as i32 {
                birth(st, key, colour, rec, mats, cols);
                born += 1;
            }
        } else {
            update(st, key, rec, colour, cols);
            // The shape's animation (the step drawn is the one before this tick's advance).
            let shapes = rd::<usize>(rd::<usize>(b + CTX) + 0x28);
            let anim = rd::<usize>(shapes + rd::<i32>(key + 4) as isize as usize * 0x10);
            let step = rd::<u16>(rec + 0x22);
            let at = anim + rd::<u16>(rec + 0x24) as usize * 8;
            wr::<u16>(rec + 0x24, step);
            let t = (rd::<i16>(key + 8) as u16).wrapping_add(rd::<u16>(rec + 0x20));
            wr::<u16>(rec + 0x20, t);
            let wait = rd::<i16>(at + 0x12);
            if wait as i32 <= t as i32 {
                wr::<u16>(rec + 0x20, t.wrapping_sub(wait as u16));
                wr::<u16>(rec + 0x22, step.wrapping_add(1));
                if rd::<i16>(anim + 6) as i32 <= step.wrapping_add(1) as i32 {
                    wr::<u16>(rec + 0x20, 0);
                    wr::<u16>(rec + 0x22, if rd::<i8>(at + 0x14) < 0 { 0 } else { step });
                }
            }
        }
        if mats != 0 {
            mats += 0x40;
        }
        if cols != 0 {
            cols += 0x20;
        }
    }
    if born > 0 {
        wr::<u16>(st + 0x60, 0);
    }
}

/// `r` by the key's distribution (`0xC8`; KH2's order of draws).
unsafe fn spread(mode: u8, r: f32) -> f32 {
    match mode {
        1 => {
            rand01();
            rand01() * r
        }
        2 => {
            let a = rand01();
            let c = rand01();
            a * r * c
        }
        3 => {
            let a = rand01();
            let c = rand01();
            r - r * a * c * 0.7
        }
        4 => {
            let a = rand01();
            let c = rand01();
            let d = rand01();
            let e = rand01();
            a * r * c * d * e
        }
        5 => {
            let a = rand01();
            let c = rand01();
            let d = rand01();
            r - r * a * c * d * 0.5
        }
        _ => r,
    }
}

/// A particle's birth (`0x1404BAC40`).
unsafe fn birth(st: usize, key: usize, colour: usize, rec: usize, mat: usize, col: usize) {
    let b = base();
    let cone = rd::<u8>(key + 0x27) as f32;
    std::ptr::write_bytes(rec as *mut u8, 0, 0x60);
    if mat != 0 {
        std::ptr::write_bytes(mat as *mut u8, 0, 0x40);
    }
    if col != 0 {
        std::ptr::write_bytes(col as *mut u8, 0, 0x20);
    }
    let typ = rd::<u8>(key + 0x26);
    let mode = rd::<u8>(key + 0xC8);
    let radius = rd::<f32>(key + 0xB8);
    if typ < 8 {
        // Direction: the base direction turned by three random angles within the cone.
        let mut v = [0f32; 4];
        copy(v.as_mut_ptr() as usize, key + 0x8C, 12);
        let mut a = [0f32; 4];
        for x in a.iter_mut().take(3) {
            *x = ((rand01() * (cone + cone) - cone) * std::f32::consts::PI) / 180.0;
        }
        if typ.wrapping_sub(2) < 2 {
            a[0] = 0.0;
            a[1] = 0.0;
        }
        let mut m = [0f32; 16];
        let mp = m.as_mut_ptr() as usize;
        identity(mp);
        euler(mp, mp, a.as_ptr() as usize);
        let dir = rec + 0x10;
        xform(dir, mp, v.as_ptr() as usize);
        for k in 0..3 {
            wr::<f32>(dir + k * 4, rd::<f32>(key + 0xBC + k * 4) * rd::<f32>(dir + k * 4));
        }
        normalize(dir, dir);
        wr::<f32>(dir + 12, 0.0);
    }
    if typ < 4 || typ > 9 {
        if radius != 0.0 {
            let r = spread(mode, radius);
            scale3(rec, rec + 0x10, r);
        }
    } else if typ < 6 {
        if radius != 0.0 {
            let half = radius * 0.5;
            if mode == 1 {
                rand01();
            }
            for k in 0..3 {
                let v = match mode {
                    2..=5 => spread(mode, radius),
                    _ => rand01() * radius,
                };
                wr::<f32>(rec + k * 4, v - half);
            }
            for k in 0..3 {
                wr::<f32>(rec + k * 4, rd::<f32>(key + 0xBC + k * 4) * rd::<f32>(rec + k * 4));
            }
        }
    } else {
        // A vertex of the vertex set's model (or of the instance's vertex override).
        let ctx = rd::<usize>(b + CTX);
        let mut verts = ptr(rd::<u32>(rd::<usize>(b + OBJ) + 0x90));
        let vs = rd::<i16>(key + 0xD0);
        if vs >= 0 {
            let set = rd::<usize>(ctx + 0x30) + vs as usize * 0x10;
            if verts == 0 {
                verts = rd::<usize>(rd::<usize>(ctx + 0x20) + rd::<i16>(set) as isize as usize * 0x38 + 0x18);
            }
            let count = rd::<i16>(set + 2);
            let u = match mode {
                1 => {
                    rand01();
                    Some(rand01())
                }
                2 => {
                    let x = rand01();
                    let y = rand01();
                    let z = rand01();
                    Some(z * y * x)
                }
                3 => {
                    let x = rand01();
                    let y = rand01();
                    let z = rand01();
                    Some(1.0 - z * y * x)
                }
                4 => {
                    let x = rand01();
                    let y = rand01();
                    let z = rand01();
                    let w = rand01();
                    Some(w * y * x * z)
                }
                5 => {
                    let x = rand01();
                    let y = rand01();
                    let z = rand01();
                    let w = rand01();
                    let q = rand01();
                    Some(1.0 - q * y * x * z * w)
                }
                _ => None,
            };
            let cursor = st + 0x62;
            if count as i32 <= rd::<u16>(cursor) as i32 {
                wr::<u16>(cursor, 0);
            }
            let idx = match u {
                Some(u) => (count as f32 * u) as i32 as isize,
                None => {
                    let c = rd::<u16>(cursor);
                    wr::<u16>(cursor, c.wrapping_add(1));
                    c as isize
                }
            };
            let vi = rd::<u16>((rd::<usize>(set + 8) as isize + idx * 2) as usize) as usize;
            let p = verts + vi * 6;
            for k in 0..3 {
                wr::<f32>(rec + k * 4, rd::<i16>(p + k * 2) as f32 * rd::<f32>(key + 0xBC + k * 4));
            }
            if typ.wrapping_sub(8) < 2 {
                normalize(rec + 0x10, rec);
                wr::<f32>(rec + 0x1C, 0.0);
            }
        }
    }
    // Colour offsets (as KH2 computes them: asymmetric, wrapping).
    for k in 0..4 {
        let r = rd::<u8>(key + 0x48 + k);
        wr::<i8>(rec + 0x28 + k, (rand01() * ((r as u32 * 2) as f32) - (r >> 1) as f32) as i32 as i8);
    }
    if rd::<u8>(key + 0x24) != 0 {
        wr::<f32>(rec + 0x54, rd::<u8>(colour + 0xB) as f32);
        wr::<u8>(rec + 0x5C, rd::<u8>(key + 0x24));
    }
    if rd::<u8>(key + 0x25) != 0 {
        wr::<u8>(rec + 0x5D, rd::<u8>(key + 0x25));
    }
    wr::<f32>(rec + 0x58, 1.0);
    if rd::<u8>(key + 0xD3) != 0 {
        wr::<f32>(rec + 0x58, 0.0);
        wr::<u8>(rec + 0x5E, rd::<u8>(key + 0xD3));
    }
    if rd::<u8>(key + 0xD4) != 0 {
        wr::<u8>(rec + 0x5F, rd::<u8>(key + 0xD4));
    }
    // Spin, degrees to 16-bit angles.
    let deg = |d: f32| ((d * 32768.0) / 180.0) as i32 as i16;
    wr::<i16>(rec + 0x2E, deg(rd::<f32>(key + 0x7C)));
    wr::<i16>(rec + 0x30, deg(rd::<f32>(key + 0x80)));
    let cb = rd::<u8>(key + 0xCB);
    if cb != 0 {
        let s = ((rand01() * rd::<f32>(key + 0x88) * 32768.0) / 180.0) as i32 as i16;
        wr::<i16>(rec + 0x32, s);
        if cb & 3 == 3 {
            if 0.5 < rand01() {
                wr::<i16>(rec + 0x32, rd::<i16>(rec + 0x32).wrapping_neg());
            }
        } else if cb & 2 != 0 {
            wr::<i16>(rec + 0x32, s.wrapping_neg());
        }
    }
    if cb & 4 != 0 {
        wr::<i16>(rec + 0x2E, rd::<i16>(rec + 0x2E).wrapping_add(rd::<i16>(rec + 0x32)));
    }
    if cb & 8 != 0 {
        wr::<i16>(rec + 0x30, rd::<i16>(rec + 0x30).wrapping_add(rd::<i16>(rec + 0x32)));
    }
    // Scale and its velocity, their random parts.
    wr::<f32>(rec + 0x34, rd::<f32>(key + 0x4C));
    wr::<f32>(rec + 0x38, rd::<f32>(key + 0x50));
    wr::<f32>(rec + 0x3C, rd::<f32>(key + 0x58));
    wr::<f32>(rec + 0x40, rd::<f32>(key + 0x5C));
    let ca = rd::<u8>(key + 0xCA);
    if ca != 0 {
        let rx = rd::<f32>(key + 0x70) * rand01();
        wr::<f32>(rec + 0x44, rx);
        if ca & 0x20 == 0 {
            let ry = rand01() * rd::<f32>(key + 0x74);
            wr::<f32>(rec + 0x48, ry);
            if ca & 3 == 3 {
                if 0.5 < rand01() {
                    wr::<f32>(rec + 0x44, rd::<f32>(rec + 0x44) * -1.0);
                }
                if 0.5 < rand01() {
                    wr::<f32>(rec + 0x48, rd::<f32>(rec + 0x48) * -1.0);
                }
            } else if ca & 2 != 0 {
                wr::<f32>(rec + 0x48, ry * -1.0);
                wr::<f32>(rec + 0x44, rd::<f32>(rec + 0x44) * -1.0);
            }
        } else {
            wr::<f32>(rec + 0x48, rx);
            if ca & 3 == 3 {
                if 0.5 < rand01() {
                    wr::<f32>(rec + 0x48, rd::<f32>(rec + 0x48) * -1.0);
                    wr::<f32>(rec + 0x44, rd::<f32>(rec + 0x44) * -1.0);
                }
            } else if ca & 2 != 0 {
                wr::<f32>(rec + 0x44, rx * -1.0);
                wr::<f32>(rec + 0x48, rx * -1.0);
            }
        }
    }
    if ca & 4 != 0 {
        wr::<f32>(rec + 0x34, rd::<f32>(rec + 0x44) + rd::<f32>(rec + 0x34));
        wr::<f32>(rec + 0x38, rd::<f32>(rec + 0x48) + rd::<f32>(rec + 0x38));
    }
    if ca & 8 != 0 {
        wr::<f32>(rec + 0x3C, rd::<f32>(rec + 0x44) + rd::<f32>(rec + 0x3C));
        wr::<f32>(rec + 0x40, rd::<f32>(rec + 0x48) + rd::<f32>(rec + 0x40));
    }
    // Random mirroring of the sprite.
    let mirror = rd::<u8>(key + 0xD2);
    if mirror == 1 {
        if 0.5 < rand01() {
            wr::<f32>(rec + 0x34, rd::<f32>(rec + 0x34) * -1.0);
        }
    } else if mirror == 2 || mirror == 3 {
        if mirror == 3 && 0.5 < rand01() {
            wr::<f32>(rec + 0x34, rd::<f32>(rec + 0x34) * -1.0);
        }
        if 0.5 < rand01() {
            wr::<f32>(rec + 0x38, rd::<f32>(rec + 0x38) * -1.0);
        }
    }
    // Speed and drift, each with its random range.
    wr::<f32>(rec + 0x4C, rd::<f32>(key + 0xA4));
    wr::<f32>(rec + 0x50, rd::<f32>(key + 0xB0));
    let r = rd::<f32>(key + 0xAC);
    if r != 0.0 {
        let u = rand01();
        wr::<f32>(rec + 0x4C, ((r + r) * u - r) + rd::<f32>(rec + 0x4C));
    }
    let r = rd::<f32>(key + 0xD8);
    if r != 0.0 {
        let u = rand01();
        wr::<f32>(rec + 0x50, ((r + r) * u - r) + rd::<f32>(rec + 0x50));
    }
    let life = rd::<i16>(key + 0x22);
    wr::<i16>(rec + 0x26, if life == 0 { -1 } else { life });
    wr::<u8>(rec + 0x2C, 0);
    // The matrix kept from birth: the base (1), the instance's (2) or both (3).
    if mat != 0 {
        let m = rd::<usize>(b + OBJ) + 0x10;
        match rd::<u8>(key + 0xCC) {
            1 => copy(mat, st, 64),
            2 => copy(mat, m, 64),
            3 => mul(mat, st, m),
            _ => {}
        }
    }
    if col != 0 {
        copy(col + 0x10, key + 0x28, 16);
    }
}

/// A live particle's tick (`0x1404BB8B0`).
unsafe fn update(st: usize, key: usize, rec: usize, colour: usize, col: usize) {
    let mut alpha = rd::<u8>(colour + 0xB) as u32;
    if col != 0 {
        for k in 0..4 {
            wr::<f32>(col + k * 4, rd::<f32>(col + k * 4) + rd::<f32>(col + 0x10 + k * 4));
        }
        for k in 0..4 {
            wr::<f32>(col + 0x10 + k * 4, rd::<f32>(key + 0x38 + k * 4) + rd::<f32>(col + 0x10 + k * 4));
        }
        alpha = (rd::<f32>(col + 12) as i32).wrapping_add(rd::<u8>(colour + 0xB) as i32) as u32;
        if 0xFF < alpha as i32 {
            alpha = 0xFF;
        }
    }
    wr::<i16>(rec + 0x2E, rd::<i16>(rec + 0x2E).wrapping_add(rd::<i16>(rec + 0x30)));
    let sa = if rd::<u8>(key + 0xCB) & 0x10 == 0 { rd::<f32>(key + 0x84) } else { rd::<i16>(rec + 0x32) as f32 + rd::<f32>(key + 0x84) };
    wr::<i16>(rec + 0x30, (rd::<i16>(rec + 0x30) as f32 + sa) as i32 as i16);
    let v = rd::<f32>(rec + 0x3C);
    wr::<f32>(rec + 0x34, v + rd::<f32>(rec + 0x34));
    wr::<f32>(rec + 0x38, rd::<f32>(rec + 0x40) + rd::<f32>(rec + 0x38));
    let add = if rd::<u8>(key + 0xCA) & 0x10 == 0 {
        wr::<f32>(rec + 0x3C, v + rd::<f32>(key + 0x64));
        rd::<f32>(key + 0x68)
    } else {
        wr::<f32>(rec + 0x3C, rd::<f32>(rec + 0x44) + rd::<f32>(key + 0x64) + v);
        rd::<f32>(rec + 0x48) + rd::<f32>(key + 0x68)
    };
    wr::<f32>(rec + 0x40, rd::<f32>(rec + 0x40) + add);
    let mut speed = rd::<f32>(rec + 0x4C) + rd::<f32>(key + 0xA8);
    wr::<f32>(rec + 0x4C, speed);
    if rd::<u8>(key + 0xCE) == 0 {
        // The speed stops at 0 rather than turn round.
        let (s0, acc) = (rd::<f32>(key + 0xA4), rd::<f32>(key + 0xA8));
        let keep = if s0 > 0.0 && acc < 0.0 {
            Some(0.0 < speed)
        } else if s0 < 0.0 && acc > 0.0 {
            Some(speed < 0.0)
        } else {
            None
        };
        if let Some(keep) = keep {
            if !keep && speed != 0.0 {
                speed = 0.0;
                wr::<f32>(rec + 0x4C, 0.0);
            }
        }
    }
    wr::<f32>(rec + 0x50, rd::<f32>(key + 0xB4) + rd::<f32>(rec + 0x50));
    let mut t = [0f32; 4];
    let tp = t.as_mut_ptr() as usize;
    scale3(tp, rec + 0x10, speed);
    add4(rec, rec, tp);
    scale3(tp, st + 0x40, rd::<f32>(rec + 0x50));
    add4(rec, rec, tp);
    if rd::<i16>(key + 0x22) != 0 {
        wr::<u16>(rec + 0x26, rd::<u16>(rec + 0x26).wrapping_sub(1));
    }
    let fade_in = rd::<u8>(rec + 0x5C);
    let age = rd::<u8>(rec + 0x2C).wrapping_add(1);
    wr::<u8>(rec + 0x2C, age);
    let life = rd::<u16>(rec + 0x26);
    if fade_in != 0 && age <= fade_in {
        wr::<f32>(rec + 0x54, rd::<f32>(rec + 0x54) - alpha as i32 as f32 / fade_in as f32);
    }
    let fade_out = rd::<u8>(rec + 0x5D);
    if fade_out != 0 && life <= fade_out as u16 {
        wr::<f32>(rec + 0x54, alpha as i32 as f32 / rd::<u8>(key + 0x25) as f32 + rd::<f32>(rec + 0x54));
    }
    let grow = rd::<u8>(rec + 0x5E);
    if grow != 0 && age <= grow {
        wr::<f32>(rec + 0x58, age as f32 / grow as f32);
    }
    let shrink = rd::<u8>(rec + 0x5F);
    if shrink != 0 && life <= shrink as u16 {
        wr::<f32>(rec + 0x58, life as f32 / shrink as f32);
    }
}

/// KH2's MegaBirth draw (`0x1404BBDB0`): every live particle a camera-facing copy of its shape
/// step, through KH1's shape draw.
unsafe extern "system" fn mb_draw(obj: usize, key: usize, prog: usize) {
    let b = base();
    let colour = work(obj, prog, 1);
    let st = work(obj, prog, 2);
    let recs = ptr(rd::<u32>(st + 0x50));
    let mut mats = ptr(rd::<u32>(st + 0x54));
    let mut cols = ptr(rd::<u32>(st + 0x58));
    let n = rd::<u32>(st + 0x5C) as i32;
    let mode = rd::<u8>(key + 0xCC);
    if recs == 0 || (mode.wrapping_sub(1) < 3 && mats == 0) || (rd::<u8>(key + 0xC9) != 0 && cols == 0) || rd::<i32>(key + 4) == 0xFFFF {
        return;
    }
    let m_now = rd::<usize>(b + OBJ) + 0x10;
    // View · base · instance, for matrix mode 0.
    let mut vbm = [0f32; 16];
    let vbm_p = vbm.as_mut_ptr() as usize;
    if mode == 0 {
        mul(vbm_p, st, m_now);
        mul(vbm_p, b + VIEW, vbm_p);
    }
    let ctx = rd::<usize>(b + CTX);
    let shape = rd::<usize>(ctx + 0x28) + rd::<i32>(key + 4) as isize as usize * 0x10;
    let anim = rd::<usize>(shape);
    let tex = rd::<i32>(shape + 8);
    let mut flags: u16 = if rd::<u8>(key + 0xF) == 0 { 0x80 } else { 0 };
    if rd::<u8>(key + 0xCD) == 1 {
        flags |= 0x10;
    }
    if rd::<u8>(key + 0xC) != 0 {
        flags |= 0x800;
    }
    if rd::<u8>(key + 0xD) != 0 {
        flags |= 0x400;
    }
    let use_z = rd::<u8>(key + 0xE) != 0;
    let offset = [rd::<f32>(key + 0x10), rd::<f32>(key + 0x14), rd::<f32>(key + 0x18), 0.0];
    let inst = [rd::<u8>(colour + 8), rd::<u8>(colour + 9), rd::<u8>(colour + 0xA), rd::<u8>(colour + 0xB)];
    let mut m = [0f32; 16];
    let mut z = [0f32; 16];
    let (mp, zp) = (m.as_mut_ptr() as usize, z.as_mut_ptr() as usize);
    // KH1's shape-draw packet (as KH1's own MegaBirth and DrawShapeX fill it).
    let mut packet = [0u8; 0x30];
    let pk = packet.as_mut_ptr() as usize;
    for i in 0..n.max(0) as usize {
        let rec = recs + i * 0x60;
        if rd::<u16>(rec + 0x26) != 0 {
            matrix(rec, key, st, m_now, vbm_p, mats, mp, zp, use_z);
            // The projected matrix's z row cleared (sprites are flat).
            std::ptr::write_bytes((mp + 0x20) as *mut u8, 0, 16);
            if use_z {
                add4(zp + 0x30, zp + 0x30, offset.as_ptr() as usize);
                mul(zp, b + PROJ, zp);
            }
            let mut c = [
                rd::<i8>(rec + 0x28) as i32 + inst[0] as i32,
                rd::<i8>(rec + 0x29) as i32 + inst[1] as i32,
                rd::<i8>(rec + 0x2A) as i32 + inst[2] as i32,
                ((rd::<i8>(rec + 0x2B) as i32 as f32 + inst[3] as i32 as f32) - rd::<f32>(rec + 0x54)) as i32,
            ];
            if cols != 0 {
                for (k, v) in c.iter_mut().enumerate() {
                    *v = v.wrapping_add(rd::<f32>(cols + k * 4) as i32);
                }
            }
            wr::<u16>(pk + 2, flags);
            wr::<u8>(pk + 4, c[0].clamp(0, 0xFF) as u8);
            wr::<u8>(pk + 5, c[1].clamp(0, 0xFF) as u8);
            wr::<u8>(pk + 6, c[2].clamp(0, 0xFF) as u8);
            wr::<u8>(pk + 7, c[3].clamp(0, 0x7F) as u8);
            wr::<u64>(pk + 8, 0);
            wr::<usize>(pk + 0x10, mp);
            wr::<usize>(pk + 0x18, rd::<usize>(b + CHAIN));
            wr::<u8>(pk + 0x20, 0);
            wr::<usize>(pk + 0x28, if use_z { zp } else { 0 });
            let frame = (anim as isize + rd::<i16>(anim + 0x10 + rd::<u16>(rec + 0x24) as usize * 8) as isize) as usize;
            let draw: unsafe extern "system" fn(usize, usize, i32, usize) -> usize = std::mem::transmute(b + SHAPE_DRAW_STEAM);
            let next = draw(frame, pk, tex, 0);
            wr::<usize>(b + CHAIN, next);
        }
        if mats != 0 {
            mats += 0x40;
        }
        if cols != 0 {
            cols += 0x20;
        }
    }
}

/// A particle's matrix (`0x1404BC710`): the sprite plane turned by its spin, then scaled by the
/// root's axis lengths x its scale x its size, at its place by the key's matrix mode, projected.
/// `z` gets the unprojected matrix when the key asks for the Z correction.
#[allow(clippy::too_many_arguments)]
unsafe fn matrix(rec: usize, key: usize, st: usize, m_now: usize, vbm: usize, mat: usize, out: usize, z: usize, use_z: bool) {
    let b = base();
    std::ptr::write_bytes(out as *mut u8, 0, 64);
    let mut sx = rd::<f32>(b + RS) * rd::<f32>(rec + 0x34);
    let mut sy = rd::<f32>(b + RS + 4) * rd::<f32>(rec + 0x38);
    let size = rd::<f32>(rec + 0x58);
    if size < 1.0 {
        sx *= size;
        sy *= size;
    }
    wr::<f32>(out + 0x3C, 1.0);
    wr::<f32>(out, sx);
    wr::<f32>(out + 0x28, sx);
    wr::<f32>(out + 0x14, sy);
    let angle = rd::<i16>(rec + 0x2E);
    if angle != 0 {
        let s = rd::<f32>(b + SINE + (angle as u16 >> 4) as usize * 4);
        let c = rd::<f32>(b + SINE + ((((angle as i64) + 0x4000) >> 4) as usize & 0xFFF) * 4);
        let r = [c, s, 0.0, 0.0, -s, c, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        mul(out, out, r.as_ptr() as usize);
    }
    add4(out + 0x30, out + 0x30, rec);
    let mut tmp = [0f32; 16];
    let tp = tmp.as_mut_ptr() as usize;
    match rd::<u8>(key + 0xCC) {
        0 => {
            copy(tp, out, 64);
            xform(tp + 0x30, vbm, out + 0x30);
            if use_z {
                copy(z, tp, 64);
            }
            mul(out, b + PROJ, tp);
        }
        mode @ 1..=3 => {
            if mode == 2 {
                mul(tp, st, mat);
            } else {
                mul(tp, mat, m_now);
            }
            mul(tp, b + VIEW, tp);
            xform(out + 0x30, tp, out + 0x30);
            if use_z {
                copy(z, out, 64);
            }
            mul(out, b + PROJ, out);
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------------------------
// 241 `pppEiDrawShipoly` (KH2 only): a ribbon through the instance's recent places
// (`docs/kh2-effects.md` §17.3). Steam KH2: calc `0x1404B9C00`, draw `0x1404B9260`, the strip
// `0x1404B8380`, spline `0x1404B8F90` / `0x1404BAA90`, construct `0x1404BA790`, reset
// `0x1404BA8C0`, destruct `0x1404BAA40`. Drawn with KH1's own renderer calls, the same library
// KH2's PC port draws its ribbon with (each matched function by function in the two decomps).

// KH1's renderer (Steam RVAs), KH2's in brackets.
/// GIF tag with PRIM (`0x140472DA0`).
const GS_PRIM_STEAM: usize = 0x386330;
/// TEST (`0x1404730A0`), ZBUF (`0x1404733D0`), TEX0 (`0x1404730D0`), ALPHA (`0x140472AD0`),
/// CLAMP (`0x140472C80`): each (register value, context 0).
const GS_TEST_STEAM: usize = 0x386630;
const GS_ZBUF_STEAM: usize = 0x386960;
const GS_TEX0_STEAM: usize = 0x386660;
const GS_ALPHA_STEAM: usize = 0x386060;
const GS_CLAMP_STEAM: usize = 0x386210;
/// Shader (`0x140127030`), shader constant (`0x140125F00`).
const SHADER_STEAM: usize = 0x108700;
const SHADER_CONST_STEAM: usize = 0x1075D0;
/// The camera matrices to the renderer, inverse of projection ∘ view out (`0x1404AEB30`).
const CAMERA_STEAM: usize = 0x31EAA0;
/// A rigid matrix's inverse (`0x14013F3A0`).
const INVERSE_STEAM: usize = 0xE5570;
/// Vertices from the renderer (`0x140111970`: count, stride) and their draw (`0x14011A2F0`:
/// count, primitive, flags).
const VERTICES_STEAM: usize = 0xF3040;
const DRAW_VERTICES_STEAM: usize = 0xFB9C0;
/// The effect allocator's free (`0x1404AF410`).
const FREE_STEAM: usize = 0x31F2F0;
/// Monochrome mode (`0x140B60240`) and the depth buffer's base (`0x140B60258`).
const MONO: usize = 0x2538200;
const ZBP: usize = 0x25381D4;
/// View ∘ root (`0x142B0ED60`).
const VIEW_ROOT: usize = 0x2E9D580;
/// The ribbon's vertex colour scale (the KH2 decomp drops the operand; as `kh2-fx` read it).
const RIBBON_COLOUR_SCALE: f32 = 1.0 / 225.0;
/// The scroll-draw variant (key `+8` set, `0x1404B7530`) isn't ported: logged once.
static LOGGED_VARIANT: AtomicBool = AtomicBool::new(false);

type V4 = [f32; 4];

fn sub4(a: V4, b: V4) -> V4 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2], a[3] - b[3]]
}
fn add4v(a: V4, b: V4) -> V4 {
    [b[0] + a[0], b[1] + a[1], b[2] + a[2], b[3] + a[3]]
}
fn scale4(a: V4, s: f32) -> V4 {
    [s * a[0], s * a[1], s * a[2], s * a[3]]
}
fn len3(a: V4) -> f32 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}
/// Normalise `xyz`, `w` 0 (KH2's `0x140140490`); returns the length.
fn norm3(a: &mut V4) -> f32 {
    a[3] = 0.0;
    let l = len3(*a);
    if l != 0.0 {
        let r = 1.0 / l;
        a[0] *= r;
        a[1] *= r;
        a[2] *= r;
    }
    l
}
/// `b·(1−t) + a·t` (KH2's `0x140141AE0`).
fn lerp4(a: V4, b: V4, t: f32) -> V4 {
    let u = 1.0 - t;
    [b[0] * u + a[0] * t, b[1] * u + a[1] * t, u * b[2] + a[2] * t, b[3] * u + a[3] * t]
}
unsafe fn v4(p: usize) -> V4 {
    rd::<V4>(p)
}
unsafe fn xform_v(m: usize, v: V4) -> V4 {
    let mut o = [0f32; 4];
    xform(o.as_mut_ptr() as usize, m, v.as_ptr() as usize);
    o
}

/// The ribbon's state (work offset 3; construct `0x1404BA790`).
mod st {
    pub const HEAD: usize = 0x00;
    pub const RING: usize = 0x04;
    pub const COUNT: usize = 0x06;
    pub const BUF: usize = 0x08;
    pub const EXPAND: usize = 0x0C;
    pub const EXPAND_VEL: usize = 0x10;
    pub const EXPAND_ACC: usize = 0x14;
    pub const SCROLL_U: usize = 0x24;
    pub const SCROLL_V: usize = 0x28;
    pub const M: usize = 0x30;
    pub const DRIFT: usize = 0x70;
    pub const DRIFT_VEL: usize = 0x80;
    pub const DRIFT_ACC: usize = 0x90;
    pub const ATTRACT: usize = 0xA0;
    pub const ATTRACT_VEL: usize = 0xB0;
    pub const ATTRACT_ACC: usize = 0xC0;
    pub const STRENGTH: usize = 0xD0;
    pub const STRENGTH_VEL: usize = 0xD4;
    pub const STRENGTH_ACC: usize = 0xD8;
    pub const KEY: usize = 0xDC;
}

unsafe extern "system" fn sp_construct(obj: usize, prog: usize) {
    let s = work(obj, prog, 3);
    wr::<u32>(s + st::HEAD, 0);
    wr::<u16>(s + st::RING, 0x100);
    wr::<u32>(s + st::BUF, to_handle(0));
    for at in [st::DRIFT, st::DRIFT_VEL, st::DRIFT_ACC, st::ATTRACT, st::ATTRACT_VEL, st::ATTRACT_ACC] {
        wr::<[u8; 16]>(s + at, [0; 16]);
    }
    wr::<u16>(s + st::COUNT, 0);
    wr::<u64>(s + st::EXPAND_VEL, 0);
    wr::<u32>(s + st::EXPAND, 0);
    wr::<u64>(s + 0x1C, 0);
    wr::<u32>(s + 0x18, 0);
    wr::<u64>(s + st::STRENGTH_VEL, 0);
    wr::<u32>(s + st::STRENGTH, 0);
    wr::<u64>(s + st::SCROLL_U, 0);
    wr::<u32>(s + st::KEY, to_handle(0));
    // The effect isn't skipped off screen (KH2's effect `+0xC3`, KH1's `+0xD7`: a ribbon
    // reaches past the effect's bounds).
    let eff = rd::<usize>(base() + EFFECT);
    if eff != 0 {
        wr::<u8>(eff + 0xD7, 1);
    }
}

/// At an instance loop: the accumulators back to their first key's start.
unsafe extern "system" fn sp_reset(obj: usize, prog: usize) {
    let s = work(obj, prog, 3);
    let k = ptr(rd::<u32>(s + st::KEY));
    if k == 0 {
        return;
    }
    for i in 0..3 {
        wr::<f32>(s + st::ATTRACT + i * 4, rd::<f32>(s + st::ATTRACT + i * 4) - rd::<f32>(k + 0x5C + i * 4));
    }
    wr::<[u8; 16]>(s + st::ATTRACT_VEL, [0; 16]);
    wr::<[u8; 16]>(s + st::ATTRACT_ACC, [0; 16]);
    wr::<u64>(s + st::STRENGTH_VEL, 0);
    wr::<f32>(s + st::STRENGTH, rd::<f32>(s + st::STRENGTH) - rd::<f32>(k + 0x80));
    for i in 0..3 {
        wr::<f32>(s + st::DRIFT + i * 4, rd::<f32>(s + st::DRIFT + i * 4) - rd::<f32>(k + 0x38 + i * 4));
    }
    wr::<[u8; 16]>(s + st::DRIFT_VEL, [0; 16]);
    wr::<[u8; 16]>(s + st::DRIFT_ACC, [0; 16]);
    wr::<u64>(s + st::EXPAND_VEL, 0);
    wr::<f32>(s + st::EXPAND, rd::<f32>(s + st::EXPAND) - rd::<f32>(k + 0x2C));
    wr::<u64>(s + 0x1C, 0);
    wr::<f32>(s + 0x18, rd::<f32>(s + 0x18) - rd::<f32>(k + 0x20));
}

unsafe extern "system" fn sp_destruct(obj: usize, prog: usize) {
    let s = work(obj, prog, 3);
    let h = rd::<u32>(s + st::BUF);
    if h != 0 {
        let f: unsafe extern "system" fn(usize, usize) = std::mem::transmute(base() + FREE_STEAM);
        f(rd::<usize>(base() + CTX), ptr(h));
        wr::<u32>(s + st::BUF, to_handle(0));
    }
}

/// The ring slot `back` places behind the head (`head − back`, wrapped).
unsafe fn ring_at(s: usize, back: i32) -> usize {
    let mut i = rd::<i32>(s + st::HEAD) - back;
    if i < 0 {
        i += rd::<u8>(s + st::RING) as i32;
    }
    ptr(rd::<u32>(s + st::BUF)) + i as isize as usize * 16
}

/// A recorded point into the key's space (`+0x11`: 0 effect space, 1 world through the root;
/// 3 through KH1's `0x142E9D500` too; 2, an object KH2 sets, isn't ported).
unsafe fn record_space(key: usize, p: usize) {
    let b = base();
    match rd::<u8>(key + 0x11) {
        1 => xform(p, b + ROOT, p),
        3 => {
            xform(p, b + ROOT, p);
            xform(p, b + 0x2E9D500, p);
        }
        _ => {}
    }
}

/// The ribbon's calc (`0x1404B9C00`; once a tick, not while fast-forwarding): record, then
/// expand / drift / attract the history.
unsafe extern "system" fn sp_calc(obj: usize, key: usize, prog: usize) {
    let b = base();
    let eff = rd::<usize>(b + EFFECT);
    if rd::<u32>(b + FAST_FORWARD) != 0 || rd::<u32>(b + HELD) != 0 || rd::<u8>(eff + 0xD5) != 0 || rd::<u32>(obj + 0xC) & 1 != 0 {
        return;
    }
    let s = work(obj, prog, 3);
    let frozen = rd::<u8>(key + 10) != 0;
    let two = rd::<u8>(key + 9) == 0;
    if !frozen {
        copy(s + st::M, rd::<usize>(b + OBJ) + 0x10, 64);
    }
    if rd::<u32>(s + st::KEY) == 0 {
        wr::<u32>(s + st::KEY, to_handle(key));
    }
    if rd::<u32>(s + st::BUF) == 0 {
        let ring = rd::<u8>(key + 11).clamp(4, 0x80);
        wr::<u8>(s + st::RING, ring);
        let size = if two { (ring as u32) << 5 } else { (ring as u32) << 4 };
        let f: unsafe extern "system" fn(usize, u32) -> usize = std::mem::transmute(b + ALLOC_STEAM);
        wr::<u32>(s + st::BUF, to_handle(f(rd::<usize>(b + CTX), size)));
        match rd::<u8>(key + 0x9F) {
            1 => wr::<f32>(s + st::SCROLL_U, rand01()),
            2 => wr::<f32>(s + st::SCROLL_V, rand01()),
            3 => {
                wr::<f32>(s + st::SCROLL_U, rand01());
                wr::<f32>(s + st::SCROLL_V, rand01());
            }
            _ => {}
        }
    }
    if ptr(rd::<u32>(s + st::BUF)) == 0 {
        return;
    }
    let ring = rd::<u8>(s + st::RING) as u16;
    if !frozen {
        let head = rd::<i32>(s + st::HEAD) as isize as usize;
        let buf = ptr(rd::<u32>(s + st::BUF));
        if two {
            let (a, c) = (buf + head * 32, buf + head * 32 + 16);
            let w = rd::<f32>(key + 12);
            let up = [0.0, w * 0.5, 0.0, 1.0f32];
            xform(a, s + st::M, up.as_ptr() as usize);
            let down = [0.0, w * -0.5, 0.0, 1.0f32];
            xform(c, s + st::M, down.as_ptr() as usize);
            record_space(key, a);
            record_space(key, c);
        } else {
            let p = buf + head * 16;
            copy(p, s + st::M + 0x30, 16);
            record_space(key, p);
            if rd::<u8>(key + 0x1C) != 0 {
                wr::<f32>(p + 12, rd::<f32>(s + st::EXPAND));
            }
        }
        wr::<u16>(s + st::COUNT, (rd::<u16>(s + st::COUNT).wrapping_add(1)).min(ring));
    }
    let count = rd::<u16>(s + st::COUNT) as i32;
    let on_key = rd::<i32>(key) == rd::<i32>(obj + 0xC);
    // Expand: each point away from the history's centre at the speed it was recorded with.
    if rd::<u8>(key + 0x1C) != 0 && !two {
        let mut c = [0f32; 4];
        for i in 0..count {
            c = add4v(c, v4(ring_at(s, i)));
        }
        let r = 1.0 / count as u16 as f32;
        c = [c[0] * r, c[1] * r, c[2] * r, c[3]];
        for i in 1..count - 2 {
            let p = ring_at(s, i);
            let speed = rd::<f32>(p + 12);
            let mut d = sub4(v4(p), c);
            norm3(&mut d);
            let d = [d[0] * speed, d[1] * speed, d[2] * speed, d[3]];
            wr::<V4>(p, add4v(v4(p), d));
            wr::<f32>(p + 12, speed);
        }
        let acc = rd::<f32>(s + st::EXPAND_ACC);
        let vel = acc + rd::<f32>(s + st::EXPAND_VEL);
        wr::<f32>(s + st::EXPAND_VEL, vel);
        let e = vel + rd::<f32>(s + st::EXPAND);
        wr::<f32>(s + st::EXPAND, e);
        if on_key {
            wr::<f32>(s + st::EXPAND, e + rd::<f32>(key + 0x2C));
            wr::<f32>(s + st::EXPAND_VEL, vel + rd::<f32>(key + 0x30));
            wr::<f32>(s + st::EXPAND_ACC, acc + rd::<f32>(key + 0x34));
        }
    }
    // Drift: the history (not the newest point) moved by the drift.
    if rd::<u8>(key + 0x1D) != 0 && !two {
        for i in 1..count {
            let p = ring_at(s, i);
            wr::<V4>(p, add4v(v4(p), v4(s + st::DRIFT)));
        }
        wr::<V4>(s + st::DRIFT_VEL, add4v(v4(s + st::DRIFT_VEL), v4(s + st::DRIFT_ACC)));
        wr::<V4>(s + st::DRIFT, add4v(v4(s + st::DRIFT), v4(s + st::DRIFT_VEL)));
        if on_key {
            for k in 0..3 {
                wr::<f32>(s + st::DRIFT + k * 4, rd::<f32>(key + 0x38 + k * 4) + rd::<f32>(s + st::DRIFT + k * 4));
                wr::<f32>(s + st::DRIFT_VEL + k * 4, rd::<f32>(key + 0x44 + k * 4) + rd::<f32>(s + st::DRIFT_VEL + k * 4));
                wr::<f32>(s + st::DRIFT_ACC + k * 4, rd::<f32>(key + 0x50 + k * 4) + rd::<f32>(s + st::DRIFT_ACC + k * 4));
            }
        }
    }
    // Attract: the history pulled toward a point (2: as it is; else in the instance's frame),
    // a fixed step a tick, less with the distance over the key's range.
    let attract = rd::<u8>(key + 0x1E);
    if attract != 0 && !two {
        for i in 1..count {
            let p = ring_at(s, i);
            let target = if attract == 2 { v4(s + st::ATTRACT) } else { xform_v(s + st::M, v4(s + st::ATTRACT)) };
            let mut d = sub4(target, v4(p));
            let l = norm3(&mut d) / rd::<f32>(key + 0x8C);
            let f = if 0.0 <= l { if l <= 1.0 { l } else { 1.0 } } else { 0.0 };
            let k = (1.0 - f) * rd::<f32>(s + st::STRENGTH);
            let d = [d[0] * k, d[1] * k, d[2] * k, 0.0];
            wr::<V4>(p, add4v(v4(p), d));
        }
        wr::<V4>(s + st::ATTRACT_VEL, add4v(v4(s + st::ATTRACT_VEL), v4(s + st::ATTRACT_ACC)));
        wr::<V4>(s + st::ATTRACT, add4v(v4(s + st::ATTRACT), v4(s + st::ATTRACT_VEL)));
        let sv = rd::<f32>(s + st::STRENGTH_VEL) + rd::<f32>(s + st::STRENGTH_ACC);
        wr::<f32>(s + st::STRENGTH_VEL, sv);
        wr::<f32>(s + st::STRENGTH, sv + rd::<f32>(s + st::STRENGTH));
        if on_key {
            for k in 0..3 {
                wr::<f32>(s + st::ATTRACT + k * 4, rd::<f32>(key + 0x5C + k * 4) + rd::<f32>(s + st::ATTRACT + k * 4));
                wr::<f32>(s + st::ATTRACT_VEL + k * 4, rd::<f32>(key + 0x68 + k * 4) + rd::<f32>(s + st::ATTRACT_VEL + k * 4));
                wr::<f32>(s + st::ATTRACT_ACC + k * 4, rd::<f32>(key + 0x74 + k * 4) + rd::<f32>(s + st::ATTRACT_ACC + k * 4));
            }
            for (k, at) in [st::STRENGTH, st::STRENGTH_VEL, st::STRENGTH_ACC].into_iter().enumerate() {
                wr::<f32>(s + at, rd::<f32>(key + 0x80 + k * 4) + rd::<f32>(s + at));
            }
        }
    }
    if !frozen {
        let mut h = rd::<u32>(s + st::HEAD).wrapping_add(1);
        if rd::<u8>(s + st::RING) as u32 <= h {
            h = 0;
        }
        wr::<u32>(s + st::HEAD, h);
    }
}

/// A natural cubic spline through `p[..n]` (`0x1404B8F90`): `c` = the second derivatives / 6,
/// solved with KH2's elimination (diagonal 4, ends 0).
fn spline(p: &[V4], n: usize, c: &mut [V4]) {
    let unit = [0.0, 0.0, 0.0, 1.0];
    c[n - 1] = unit;
    c[0] = unit;
    let mut d = [[0f32; 4]; 130];
    for k in 0..n - 1 {
        d[k] = sub4(p[k + 1], p[k]);
    }
    let pivot0 = [4.0, 4.0, 4.0, 5.0];
    let mut m = [[0f32; 4]; 130];
    m[0] = pivot0;
    c[1] = sub4(sub4(d[1], d[0]), c[0]);
    if n > 3 {
        for k in 0..n - 3 {
            let inv = [1.0 / m[k][0], 1.0 / m[k][1], 1.0 / m[k][2], 1.0];
            let t = [inv[0] * c[k + 1][0], inv[1] * c[k + 1][1], inv[2] * c[k + 1][2], inv[3] * c[k + 1][3]];
            c[k + 2] = sub4(sub4(d[k + 2], d[k + 1]), t);
            m[k + 1] = sub4(pivot0, inv);
        }
    }
    c[n - 2] = sub4(c[n - 2], c[n - 1]);
    for i in (1..=n - 2).rev() {
        let v = sub4(c[i], c[i + 1]);
        c[i] = [v[0] / m[i - 1][0], v[1] / m[i - 1][1], v[2] / m[i - 1][2], v[3]];
    }
}

/// The spline at `t` (`0x1404BAA90`): `p[i] + f·(Δp − 2c[i] − c[i+1]) + 3c[i]·f² + (c[i+1] −
/// c[i])·f³`, `i = ⌊t⌋` (both arrays padded with 0 past the last point, as KH2 reads past it at
/// the end with f = 0).
fn spline_at(p: &[V4], c: &[V4], t: f32) -> V4 {
    let i = t as i32 as usize;
    let f = t - i as f32;
    let q = add4v(scale4(c[i], 2.0), c[i + 1]);
    let e = sub4(sub4(p[i + 1], p[i]), q);
    let three = scale4(c[i], 3.0);
    let a = add4v(scale4(sub4(c[i + 1], c[i]), f), three);
    let a = add4v(scale4(a, f), e);
    add4v(scale4(a, f), p[i])
}

/// The ribbon's draw (`0x1404B9260`): its two edges through the history, then the strip.
unsafe extern "system" fn sp_draw(obj: usize, key: usize, prog: usize) {
    let b = base();
    let scale = work(obj, prog, 1);
    let colour = work(obj, prog, 2);
    let s = work(obj, prog, 3);
    if rd::<u8>(key + 8) != 0 {
        if !LOGGED_VARIANT.swap(true, Ordering::Relaxed) {
            log("KH2 effects (native): a ribbon with KH2's scroll-draw variant (key +8) isn't drawn (not ported)");
        }
        return;
    }
    let space = rd::<u8>(key + 0x11);
    let to_view = match space {
        0 => b + VIEW_ROOT,
        1 => b + VIEW,
        _ => return,
    };
    let buf = ptr(rd::<u32>(s + st::BUF));
    let n = rd::<u16>(s + st::COUNT) as usize;
    let sub = rd::<u8>(key + 0x12) as usize;
    if buf == 0 || n <= 2 || n > 128 {
        return;
    }
    let ring = rd::<u8>(s + st::RING) as i32;
    let newest = rd::<i32>(s + st::HEAD) - 1;
    let idx = |i: i32| {
        let j = newest - i;
        if j < 0 {
            j + ring
        } else {
            j
        }
    };
    let mut pts = [[0f32; 4]; 130];
    let mut cs = [[0f32; 4]; 130];
    // The two edges, newest first (`sub + 1` samples each).
    let mut ea = [[0f32; 4]; 257];
    let mut eb = [[0f32; 4]; 257];
    if sub == 0 {
        // No segment to draw (the strip still steps its scroll).
    } else if rd::<u8>(key + 9) == 0 {
        for side in 0..2 {
            for i in 0..n {
                let mut v = v4(buf + (idx(i as i32) as usize * 2 + side) * 16);
                v[3] = 1.0;
                pts[i] = xform_v(to_view, v);
            }
            spline(&pts, n, &mut cs);
            let e = if side == 0 { &mut ea } else { &mut eb };
            for (j, out) in e.iter_mut().enumerate().take(sub + 1) {
                let mut v = spline_at(&pts, &cs, j as f32 / (sub as f32 / (n - 1) as i32 as f32));
                v[3] = 1.0;
                *out = v;
            }
        }
    } else {
        for i in 0..n {
            let mut v = v4(buf + idx(i as i32) as usize * 16);
            v[3] = 1.0;
            pts[i] = v;
        }
        spline(&pts, n, &mut cs);
        let mut mid = [[0f32; 4]; 257];
        for (j, out) in mid.iter_mut().enumerate().take(sub + 1) {
            let mut v = spline_at(&pts, &cs, (j as f32 / sub as f32) * (n - 1) as i32 as f32);
            v[3] = 1.0;
            *out = xform_v(to_view, v);
        }
        if 0.0 < rd::<f32>(key + 0x90) {
            for j in 0..sub {
                mid[j][3] = len3(sub4(mid[j], mid[j + 1]));
            }
            mid[sub][3] = 0.0;
        }
        // Half the width across the screen: key width x the instance's scale x x the root's
        // (the KH2 decomp drops this operand; as `kh2-fx` read it).
        let half = rd::<f32>(key + 12) * rd::<f32>(scale) * rd::<f32>(b + RS);
        let mut prev = [0f32; 4];
        let mut dir = [0f32; 4];
        for j in 0..=sub {
            let (a, c) = if j != sub { (mid[j], mid[j + 1]) } else { (mid[j - 1], mid[j]) };
            let perp = if rd::<u8>(key + 9) == 1 {
                let mut d = sub4(a, c);
                norm3(&mut d);
                let d = scale3v(d, half);
                [-d[1], d[0], d[2], d[3]]
            } else {
                // The direction summed with the last one (the last sample keeps the last's y).
                if j == sub {
                    dir[0] = 0.0;
                } else {
                    dir = sub4(a, c);
                }
                dir[2] = 0.0;
                dir[3] = 0.0;
                norm3(&mut dir);
                dir = scale3v(dir, half);
                let sum = add4v(prev, dir);
                prev = dir;
                [-sum[1], sum[0], sum[2], sum[3]]
            };
            let (p, q) = (add4v(mid[j], perp), sub4(mid[j], perp));
            ea[j] = [p[0], p[1], mid[j][2], mid[j][3]];
            eb[j] = [q[0], q[1], mid[j][2], mid[j][3]];
        }
    }
    // Taper: the ribbon narrows toward its middle line along its length.
    if rd::<u8>(key + 0x10) != 0 {
        let mut t = 1.0f32;
        for j in 0..=sub {
            // (Only x, y, z are written back.)
            let m = scale3v(add4v(ea[j], eb[j]), 0.5);
            let (a, c) = (lerp4(ea[j], m, t), lerp4(eb[j], m, t));
            ea[j] = [a[0], a[1], a[2], ea[j][3]];
            eb[j] = [c[0], c[1], c[2], eb[j][3]];
            let next = t - 1.0 / sub as i32 as f32;
            t = if 0.0 <= next { next } else { 0.0 };
        }
    }
    let model = if rd::<i32>(key + 4) == 0xFFFF { 0 } else { rd::<usize>(rd::<usize>(b + CTX) + 0x20) + rd::<i32>(key + 4) as isize as usize * 0x38 };
    strip(model, key, s, colour, &ea, &eb, n);
}

/// `xyz · s`, `w` kept.
fn scale3v(a: V4, s: f32) -> V4 {
    [s * a[0], s * a[1], s * a[2], a[3]]
}

/// The strip between the two edges (`0x1404B8380`), through KH1's renderer: a triangle strip
/// per segment, `2 x (key 0x13 + 1)` vertices across.
#[allow(clippy::too_many_arguments)]
unsafe fn strip(model: usize, key: usize, s: usize, colour: usize, ea: &[V4], eb: &[V4], n: usize) {
    let b = base();
    let sub = rd::<u8>(key + 0x12);
    if n < 3 {
        return;
    }
    // Texture region along (u) and across (v), and the direction u runs.
    let mut u1: f32 = match rd::<u8>(key + 0x9C) {
        1 => 0.5,
        2 => 0.25,
        _ => 1.0,
    };
    let mut vmax: f32 = match rd::<u8>(key + 0x9D) {
        1 => 0.5,
        2 => 0.25,
        _ => 1.0,
    };
    let mut u0: f32 = 0.5;
    match rd::<u8>(key + 0x9E) {
        1 => u1 += 0.25,
        2 => u1 += 0.5,
        3 => {
            u0 = 0.75;
            u1 += 0.75;
        }
        _ => u0 = 0.0,
    }
    u0 += 0.01;
    u1 -= 0.01;
    vmax -= 0.01;
    let fill = if rd::<u8>(key + 0xA1) != 0 { n as f32 / rd::<u8>(s + st::RING) as f32 } else { 1.0 };
    let span = if rd::<u8>(key + 0xA1) != 0 { fill * (u1 - u0) } else { u1 - u0 };
    let (mut u, du) = match rd::<u8>(key + 0xA0) {
        0 | 2 => (u0, span / sub as f32),
        1 | 3 => (u1, -(span / sub as f32)),
        _ => (0.0, 0.0),
    };
    // Alpha along the ribbon (key 0x17): up over the first part, down after.
    let mut f = 1.0f32;
    let (mut up, mut down) = (0.0f32, 0.0f32);
    if rd::<u8>(key + 0x17) != 0 {
        let rise = (sub as f32 * rd::<f32>(key + 0x18)) as i32 as f32;
        let fall = sub as f32 - rise;
        if 0.0 < rise {
            f = 0.0;
            up = 1.0 / rise;
        } else {
            f = 1.0;
        }
        if 0.0 < fall {
            down = 1.0 / fall;
        }
    }
    // GS state, as KH2 sets it (KH1's same calls).
    let gs = |at: usize, v: u64| {
        let f: unsafe extern "system" fn(usize, i32) = std::mem::transmute(b + at);
        f(&v as *const u64 as usize, 0);
    };
    let tag: u64 = 0x6000_0000_0000_8001;
    let prim: unsafe extern "system" fn(usize) = std::mem::transmute(b + GS_PRIM_STEAM);
    prim(&tag as *const u64 as usize);
    gs(GS_TEST_STEAM, if rd::<u8>(key + 0x14) != 0 { 0x3000D } else { 0x7000D });
    gs(GS_ZBUF_STEAM, (rd::<u32>(b + ZBP) & 0x1FF) as u64 | 0x1_0100_0000);
    let omd = if model != 0 { rd::<usize>(model + 0x28) } else { 0 };
    let tex0 = if omd != 0 {
        let g = (omd as isize + rd::<i32>(omd + 4) as isize) as usize;
        rd::<u32>(g + 8) as u64 | (rd::<u32>(g + 0xC) as u64) << 32
    } else {
        0
    };
    gs(GS_TEX0_STEAM, tex0);
    let alpha = match rd::<u8>(key + 0x15) {
        0 => Some(0x80_0000_0044u64),
        1 => Some(0x80_0000_0048),
        2 => Some(0x80_0000_0042),
        3 => Some(0x80_0000_0041),
        4 => Some(0x80_0000_0046),
        _ => None,
    };
    if let Some(a) = alpha {
        gs(GS_ALPHA_STEAM, a);
    }
    gs(GS_CLAMP_STEAM, 0);
    let shader: unsafe extern "system" fn(i32) = std::mem::transmute(b + SHADER_STEAM);
    shader(0x2B + (rd::<u32>(b + MONO) != 0) as i32);
    let k4 = [0.0f32, 1.0, 0.0, 1.0];
    let constant: unsafe extern "system" fn(i32, usize, i32) = std::mem::transmute(b + SHADER_CONST_STEAM);
    constant(4, k4.as_ptr() as usize, 0);
    let mut cam = [0f32; 16];
    let camera: unsafe extern "system" fn(usize) -> usize = std::mem::transmute(b + CAMERA_STEAM);
    camera(cam.as_mut_ptr() as usize);
    let cols = rd::<u8>(key + 0x13) as u32 + 1;
    let rgb = [rd::<u8>(colour + 8), rd::<u8>(colour + 9), rd::<u8>(colour + 0xA)];
    let eff = rd::<usize>(b + EFFECT);
    // Vertices go to the renderer in world space (the inverse view of the view-space edges),
    // or through the projection when the effect draws in screen space (KH2's `+0xC8`, KH1's
    // `+0xDC`).
    let screen = eff != 0 && rd::<i32>(eff + 0xDC) != 0;
    let mut inv = [0f32; 16];
    let inverse: unsafe extern "system" fn(usize, usize) -> usize = std::mem::transmute(b + INVERSE_STEAM);
    inverse(b + VIEW, inv.as_mut_ptr() as usize);
    let m = if screen { b + PROJ } else { inv.as_ptr() as usize };
    let mut falling = false;
    let (mut across, mut delta) = (0.0f32, 0.0f32);
    let verts: unsafe extern "system" fn(u32, u32, u32) -> usize = std::mem::transmute(b + VERTICES_STEAM);
    let draw: unsafe extern "system" fn(u32, u32, u32) = std::mem::transmute(b + DRAW_VERTICES_STEAM);
    let to_prim = 0x302E_4000_0000_8000u64 | (cols as u64 * 2);
    for seg in 0..sub as usize {
        let a_inst = rd::<u8>(colour + 0xB) as f32;
        let mut near = a_inst;
        let mut far = a_inst;
        if rd::<u8>(key + 0x17) != 0 {
            near = a_inst * f;
            let next = if !falling {
                if 1.0 <= f {
                    falling = true;
                    f - down
                } else {
                    f + up
                }
            } else {
                f - down
            };
            f = if 0.0 <= next { if next <= 1.0 { next } else { 1.0 } } else { 0.0 };
            far = a_inst * f;
        }
        let (mut un, next_u);
        if 0.0 < rd::<f32>(key + 0x90) {
            un = u + (ea[seg][3] / rd::<f32>(key + 0x90)) * (u1 - u0);
            next_u = if u1 < un { (un - u1) + u0 } else { un };
        } else {
            un = u + du;
            next_u = if un <= u1 { un } else { u1 };
        }
        un += rd::<f32>(s + st::SCROLL_U);
        let uc = u + rd::<f32>(s + st::SCROLL_U);
        if rd::<u8>(key + 0x16) != 0 {
            across = 0.0;
            delta = 2.0 / (cols as i32 - 1) as f32;
        }
        prim(&to_prim as *const u64 as usize);
        let count = cols * 2;
        let vb = verts(count, 0x3C, 0);
        if vb == 0 {
            return;
        }
        let last = (cols as i32 - 1) as f32;
        for c in 0..cols as usize {
            let t = c as i32 as f32 / last;
            let v = (vmax - 0.01) * t + 0.01 + rd::<f32>(s + st::SCROLL_V);
            let (mut c0, mut c1) = (near, far);
            if rd::<u8>(key + 0x16) != 0 {
                c0 = near * across;
                c1 = far * across;
                let nx = across + delta;
                if nx < 1.0 {
                    across = if 0.0 <= nx { nx } else { 0.0 };
                } else {
                    delta *= -1.0;
                    across = 1.0;
                }
            }
            let mut p0 = lerp4(eb[seg], ea[seg], t);
            p0[3] = 1.0;
            let mut p1 = lerp4(eb[seg + 1], ea[seg + 1], t);
            p1[3] = 1.0;
            let (w0, w1) = (xform_v(m, p0), xform_v(m, p1));
            let (st0, st1) = if 1 < rd::<u8>(key + 0xA0) { ([v, uc], [v, un]) } else { ([uc, v], [un, v]) };
            for (k, (pos, uv, al)) in [(w0, st0, c0), (w1, st1, c1)].into_iter().enumerate() {
                let i = c * 2 + k;
                let at = vb + i * 0x3C;
                wr::<V4>(at, pos);
                wr::<V4>(at + 0x10, pos);
                let a = (al as u32) as u8;
                wr::<V4>(
                    at + 0x20,
                    [rgb[0] as f32 * RIBBON_COLOUR_SCALE, rgb[1] as f32 * RIBBON_COLOUR_SCALE, rgb[2] as f32 * RIBBON_COLOUR_SCALE, a as f32 * RIBBON_COLOUR_SCALE],
                );
                wr::<f32>(at + 0x30, uv[0] * 4096.0);
                wr::<f32>(at + 0x34, uv[1] * 4096.0);
                wr::<u16>(at + 0x38, if i < 2 { 0x1000 } else { 0 });
            }
        }
        draw(count, 6, if seg > 0 { 4 } else { 0 });
        u = next_u;
    }
    // The texture scroll, wrapped to 0..1 (not on a tick's repeat frame).
    if rd::<u32>(b + HELD) == 0 && rd::<u8>(eff + 0xD5) == 0 && rd::<u8>(eff + 0xD8) == 0 {
        let wrap = |mut v: f32| {
            while 1.0 < v {
                v -= 1.0;
            }
            while v < 0.0 {
                v += 1.0;
            }
            v
        };
        wr::<f32>(s + st::SCROLL_V, wrap(rd::<f32>(key + 0x98) + rd::<f32>(s + st::SCROLL_V)));
        wr::<f32>(s + st::SCROLL_U, wrap(rd::<f32>(key + 0x94) + rd::<f32>(s + st::SCROLL_U)));
    }
}
