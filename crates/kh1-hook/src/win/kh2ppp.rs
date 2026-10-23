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

static BASE: AtomicUsize = AtomicUsize::new(0);
/// KH2's MegaBirth is plugged into the DPX (its words point at our entry).
static PLUGGED: AtomicBool = AtomicBool::new(false);

/// Whether KH2's own programs are in (an effect that runs one may then play through KH1).
pub fn plugged() -> bool {
    PLUGGED.load(Ordering::Relaxed)
}

#[repr(C)]
struct Entry {
    name: *const u8,
    slots: [usize; 9],
}

/// Our program entries (KH1's 0x50-byte format). Written once on the game thread before any
/// record points at them; never moved.
static mut TABLE: [Entry; 1] = [Entry { name: std::ptr::null(), slots: [0; 9] }];
static NAME_MB: &[u8] = b"pppRyjMegaBirth (KH2)\0";

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

/// Points our entries at their functions (once).
unsafe fn table() -> usize {
    let t = std::ptr::addr_of_mut!(TABLE) as *mut Entry;
    if (*t).name.is_null() {
        let b = base();
        (*t).slots = [mb_calc as *const () as usize, 0, mb_draw as *const () as usize, 0, 0, 0, b + MB_CONSTRUCT_STEAM, 0, b + MB_DESTRUCT_STEAM];
        (*t).name = NAME_MB.as_ptr();
    }
    t as usize
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
        let ours = to_handle(table());
        if ours == 0 {
            log("KH2 effects (native): no handle for our program table; KH2's MegaBirth not plugged in");
            return;
        }
        let (mut done, mut other) = (0, 0);
        for &(at, ty) in programs {
            let w = dpx + at;
            if ty == MEGA_BIRTH && ptr(rd::<u32>(w)) == kh1_mb {
                wr::<u32>(w, ours);
                done += 1;
            } else {
                other += 1;
            }
        }
        PLUGGED.store(done > 0, Ordering::Relaxed);
        log(&format!("KH2 effects (native): KH2's MegaBirth plugged into {done} program words ({other} other KH2-only words left to KH1's dummy)"));
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
