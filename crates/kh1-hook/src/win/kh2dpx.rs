//! KH2's effects played by KH1's own effect system (t368, the effects port's step 1; project notes
//! `kh2-effects-in-kh1.md`). KH1 and KH2 share the effect engine: the same DPX (`0x82`) and DPD
//! (`0x96`) files, the same particle programs (1..235) and the same 30 Hz effect clock. So a KH2
//! effect is made into a KH1 DPX once (`kh2_formats::kh1dpx::build`: its textures moved into
//! KH1's own-texture window, the KH2 common textures it draws with copied in, programs KH1 lacks
//! set to its dummy program) and started with KH1's own calls, on Sora's KH1 bones. KH1 then
//! steps, binds, textures, draws, pauses and ends it like any of its own effects; nothing of
//! ours runs per frame.
//!
//! Step 1: Explosion's wrist flashes (Sora's package `obj/P_EX100.a.us`, elements 46 and 47:
//! effect 33 on KH2 bones 170 and 78), which use only programs KH1 has. KH2's bones are put on
//! KH1's by the bind poses (option A, as the base attacks): KH2 170 -> KH1 93, KH2 78 -> KH1 198.
//! Not yet (step 2): the elements' delay, fade-out, end-with-motion and rotation offsets
//! (both have a half turn about z, which KH1's bone binding doesn't take).
//!
//! KH1 calls (Steam): the DPX relocation `0x1401E0080` (each effect's DPD offset to a handle,
//! each particle data's programs to KH1's functions; once per DPX), and the start on a bone
//! `0x1402ADCF0 (dpx, effect number, actor, bone)` (an element from KH1's list, its effect
//! instance from KH1's pool, bound to the bone each frame; bone `0x2000 | n` is bone `n`).
//!
//! (t369) Textures. KH1's PC port never draws a DPD's PS2 texture data: for each texture record
//! it asks for the HD texture of the loaded file the record lies in (`0x14010C050`: the file
//! from KH1's loaded-file list whose data holds the pointer, `0x1400E2B20`; then that file's HD
//! entry whose `+0x54` is the record's pixel offset in the file, `+0x20` of the record; KH1 runs
//! with the offset keying on, set at start in `axaAppMain`). So the DPX is entered in that list
//! the way KH1 enters a file it loads (`0x1400E1680` / `0x1400E2F60`): a resource entry for our
//! buffer with an HD entry per texture record, holding the KH2 HD image (DDS) KH2 itself draws
//! for that texture (the remastered assets of `obj/P_EX100.a.us` and `effect/texcommon.dpd`,
//! matched by the offset each asset's header gives at `+0x24`; t370: t369 took `+0x20`, which is
//! the asset's own place in the stream). KH1's own loader then makes the textures.
//! The vtables are copied from KH1's own entries; memory comes from the game's CRT (KH1 frees
//! HD data with `_aligned_free` if a file is ever dropped; ours never is).
//!
//! Ini: `kh2_fx_native` (on by default): off = these elements are played by `kh2fx` as before.
//! Elements fall back to `kh2fx` too while the DPX isn't built (or couldn't be).

use super::addr::Rva;
use super::log;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

const RELOCATE_RVA: Rva = Rva(0x1DDF20);
const START_ON_BONE_RVA: Rva = Rva(0x2ABB60);
/// KH1's "which loaded file holds this pointer" (`0x1400E2B20`); its first instruction loads
/// the loaded-file list (`mov r11, [rip + disp]`).
const FILE_OF_RVA: Rva = Rva(0xE2B10);
/// Bone parameter bit: the low 12 bits are the bone's index in the actor's skeleton.
const RAW_BONE: u32 = 0x2000;
/// Explosion's effects in Sora's package (elements 44..48 -> 31, 32, 33, 33, 34).
const EXPLOSION: [u32; 4] = [31, 32, 33, 34];
/// The elements KH1 plays: (KH2 element, effect number, KH1 bone).
const NATIVE: [(i16, u32, u32); 2] = [(46, 33, 93), (47, 33, 198)];
/// KH1's handles cover 32 MB regions; the DPX stays inside one.
const REGION: usize = 0x200_0000;
/// The name our DPX has in KH1's loaded-file list.
const FILE_NAME: &[u8] = b"kh2fx_explosion_dpx";

/// An HD texture for one texture record: the record's pixel offset in the DPX, the DDS.
struct Hd {
    offset: u32,
    dds: Vec<u8>,
}

struct Dpx {
    /// Owns the bytes; never freed or moved (KH1's effects point into it).
    _buf: Vec<u8>,
    at: usize,
    len: usize,
    md5: [u8; 16],
    hd: Vec<Hd>,
    relocated: bool,
}

static DPX: Mutex<Option<Dpx>> = Mutex::new(None);
static STARTED: AtomicU32 = AtomicU32::new(0);

/// Builds Explosion's KH1 DPX in the background (from the KH2 install, as `kh2fx`).
pub fn start() {
    if super::settings::get().kh2_effects.is_none() {
        return;
    }
    std::thread::spawn(|| {
        let t = std::time::Instant::now();
        match build() {
            Ok((r, hd, hd_report)) => {
                let dpx = &r.bytes;
                log(&format!("KH2 effects (native): Explosion's DPX built, {} bytes in {:.2} s; {}", dpx.len(), t.elapsed().as_secs_f32(), r.report));
                log(&format!("KH2 effects (native): HD textures: {hd_report}"));
                let mut buf = vec![0u8; dpx.len() * 2 + 16];
                let p = buf.as_ptr() as usize;
                let mut at = (p + 15) & !15;
                if (at & !(REGION - 1)) != ((at + dpx.len() - 1) & !(REGION - 1)) {
                    at = (at + dpx.len()) & !(REGION - 1);
                }
                let off = at - p;
                buf[off..off + dpx.len()].copy_from_slice(dpx);
                if let Ok(mut g) = DPX.lock() {
                    *g = Some(Dpx { _buf: buf, at, len: dpx.len(), md5: r.md5, hd, relocated: false });
                }
            }
            Err(e) => log(&format!("KH2 effects (native): Explosion's DPX not built ({e}); kh2fx plays it")),
        }
    });
}

fn build() -> Result<(kh2_formats::kh1dpx::Kh1Dpx, Vec<Hd>, String), String> {
    let exe = format!("{}\\KINGDOM HEARTS II FINAL MIX.exe", super::exe_dir());
    let install = kh2_data::Install::locate(&exe).map_err(|e| e.to_string())?;
    let src = if install.has_backup() { kh2_data::Source::Backup } else { kh2_data::Source::Live };
    let pack = install.read_any(src, "obj/P_EX100.a.us").map_err(|e| e.to_string())?;
    let common = install.read_any(src, "effect/texcommon.dpd").map_err(|e| format!("texcommon.dpd: {e}"))?;
    let bar = kh2_formats::bar::Bar::parse(&pack).map_err(|e| e.to_string())?;
    let pax = bar
        .entries
        .iter()
        .find(|e| e.name == "p_ex" && e.data.starts_with(b"PAX_"))
        .or_else(|| bar.entries.iter().find(|e| e.data.starts_with(b"PAX_")))
        .ok_or("no PAX in P_EX100")?;
    let pax_at = pax.data.as_ptr() as usize - pack.as_ptr() as usize;
    let r = kh2_formats::kh1dpx::build(pax.data, &common, &EXPLOSION).map_err(|e| e.to_string())?;
    // KH2's HD textures for the two files, each keyed by the offset in its file of the texture
    // record's pixels it replaces.
    let own_hd = install.read_remastered_any(src, "obj/P_EX100.a.us").map_err(|e| format!("P_EX100 HD textures: {e}"))?;
    let common_hd = install.read_remastered_any(src, "effect/texcommon.dpd").map_err(|e| format!("texcommon HD textures: {e}"))?;
    let own_recs = kh2_formats::kh1dpx::texture_records(&pax.data[r.own_dpd_at..]).map_err(|e| e.to_string())?;
    let common_recs = kh2_formats::kh1dpx::texture_records(&common[r.common_dpd_at..]).map_err(|e| e.to_string())?;
    let mut hd = Vec::new();
    let mut report = Vec::new();
    for t in &r.textures {
        // A texture's HD image is the one for its TBP (KH1 and KH2 make one texture per TBP):
        // the asset whose offset is any of the source DPD's records with that TBP.
        let (assets, recs, base) = if t.own { (&own_hd, &own_recs, pax_at + r.own_dpd_at) } else { (&common_hd, &common_recs, r.common_dpd_at) };
        let exact = base + t.src_offset + 0x20;
        let found = assets.iter().find(|a| a.offset as usize & 0xFF_FFFF == exact).or_else(|| {
            assets.iter().find(|a| recs.iter().any(|&(o, tbp)| tbp == t.src_tbp && base + o + 0x20 == a.offset as usize & 0xFF_FFFF))
        });
        let what = if t.own { "own" } else { "common" };
        match found {
            Some(a) => {
                report.push(format!("{what} {:#x} (tbp {:#x}) <- {} ({} bytes{})", exact, t.src_tbp, a.name, a.data.len(), if a.offset as usize & 0xFF_FFFF == exact { "" } else { ", same TBP" }));
                hd.push(Hd { offset: (t.dpx_offset + 0x20) as u32, dds: a.data.clone() });
            }
            None => report.push(format!("{what} {:#x} (tbp {:#x}) <- none", exact, t.src_tbp)),
        }
    }
    let list = |v: &Vec<kh2_formats::egs::HdAsset>| v.iter().map(|a| format!("{} for {:#x} ({} bytes, at {:#x})", a.name, a.offset, a.data.len(), a.pkg_offset)).collect::<Vec<_>>().join(", ");
    report.push(format!("P_EX100 assets [{}], texcommon assets [{}]", list(&own_hd), list(&common_hd)));
    Ok((r, hd, report.join("; ")))
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

/// Enters the DPX in KH1's loaded-file list with its HD textures (see the module notes).
unsafe fn register(base: usize, d: &Dpx) -> Result<usize, String> {
    use super::sys::{GetModuleHandleA, GetProcAddress};
    let f = base + FILE_OF_RVA;
    if rd::<[u8; 3]>(f) != [0x4C, 0x8B, 0x1D] {
        return Err("unexpected code at the file lookup".into());
    }
    let list_at = (f as isize + 7 + rd::<i32>(f + 3) as isize) as usize;
    let list = rd::<usize>(list_at);
    if list == 0 {
        return Err("no loaded-file list".into());
    }
    // KH1's own entries: the vtables of a resource entry, its HD-entry list and (the commonest
    // one among them) an HD entry.
    let first = rd::<usize>(list + 8);
    if first == 0 {
        return Err("the loaded-file list is empty".into());
    }
    let (res_vt, list_vt) = (rd::<usize>(first), rd::<usize>(first + 0x668));
    let mut counts: Vec<(usize, u32)> = Vec::new();
    let mut r = first;
    for _ in 0..4096 {
        if r == 0 {
            break;
        }
        let mut e = rd::<usize>(r + 0x670);
        for _ in 0..4096 {
            if e == 0 {
                break;
            }
            let vt = rd::<usize>(e);
            match counts.iter_mut().find(|c| c.0 == vt) {
                Some(c) => c.1 += 1,
                None => counts.push((vt, 1)),
            }
            e = rd::<usize>(e + 8);
        }
        r = rd::<usize>(r + 8);
    }
    let Some(&(entry_vt, _)) = counts.iter().max_by_key(|c| c.1) else { return Err("no HD entry in KH1's files to copy".into()) };
    let crt = GetModuleHandleA(b"ucrtbase.dll\0".as_ptr());
    let malloc = GetProcAddress(crt, b"malloc\0".as_ptr());
    let aligned = GetProcAddress(crt, b"_aligned_malloc\0".as_ptr());
    if crt.is_null() || malloc.is_null() || aligned.is_null() {
        return Err("the game's CRT not found".into());
    }
    let malloc: unsafe extern "C" fn(usize) -> usize = std::mem::transmute(malloc);
    let aligned: unsafe extern "C" fn(usize, usize) -> usize = std::mem::transmute(aligned);
    // The resource entry (0x688 bytes, as `0x1400E2F60` makes it).
    let rec = malloc(0x688);
    if rec == 0 {
        return Err("out of memory".into());
    }
    std::ptr::write_bytes(rec as *mut u8, 0, 0x688);
    wr(rec, res_vt);
    wr(rec + 0x10, d.at);
    wr(rec + 0x18, d.len as u32);
    wr(rec + 0x1C, u32::MAX);
    for (i, &b) in FILE_NAME.iter().enumerate() {
        wr(rec + 0x20 + i, b);
        wr(rec + 0x1C4 + i, b);
        wr(rec + 0x620 + i, b);
    }
    wr(rec + 0x208, d.md5);
    wr(rec + 0x218, u32::MAX);
    wr(rec + 0x640, d.len as u32);
    wr(rec + 0x644, rd::<u32>(list + 0x20));
    wr(rec + 0x64C, u32::MAX);
    wr(rec + 0x658, d.at);
    wr(rec + 0x668, list_vt);
    // Its HD entries (0x78 bytes each, as `0x1400E1680` makes them).
    let mut tail = 0usize;
    for (i, h) in d.hd.iter().enumerate() {
        let e = malloc(0x78);
        let data = aligned(h.dds.len(), 16);
        if e == 0 || data == 0 {
            return Err("out of memory".into());
        }
        std::ptr::copy_nonoverlapping(h.dds.as_ptr(), data as *mut u8, h.dds.len());
        std::ptr::write_bytes(e as *mut u8, 0, 0x78);
        wr(e, entry_vt);
        let name = format!("-{:x}.dds\0", h.offset);
        for (k, &b) in name.as_bytes().iter().take(0x3F).enumerate() {
            wr(e + 0x10 + k, b);
        }
        wr(e + 0x50, h.dds.len() as u32);
        wr(e + 0x54, h.offset & 0xFF_FFFF);
        wr(e + 0x60, data);
        wr(e + 0x68, 999u32);
        if i < 0x100 {
            wr(rec + 0x21C + i * 4, h.offset);
        }
        if tail == 0 {
            wr(rec + 0x670, e);
        } else {
            wr(tail + 8, e);
        }
        tail = e;
        wr(rec + 0x678, e);
        wr(rec + 0x680, rd::<i32>(rec + 0x680) + 1);
    }
    // Appended to the list as `0x1400E2F60` does.
    if rd::<usize>(list + 8) == 0 {
        wr(list + 8, rec);
    }
    let last = rd::<usize>(list + 0x10);
    if last != 0 {
        wr(last + 8, rec);
    }
    wr(list + 0x10, rec);
    wr(list + 0x18, rd::<i32>(list + 0x18) + 1);
    Ok(rec)
}

/// A KH2 element of Sora's package (`kh2fx::event`, game thread): true if KH1 played it.
pub fn event(motion: u16, el: i16) -> bool {
    if !super::settings::get().kh2_fx_native {
        return false;
    }
    let Some(&(_, number, bone)) = NATIVE.iter().find(|n| n.0 == el) else { return false };
    let sora = super::takeover::sora_actor();
    if sora < 0x10000 {
        return false;
    }
    let Ok(mut g) = DPX.lock() else { return false };
    let Some(d) = g.as_mut() else { return false };
    let base = unsafe { super::sys::GetModuleHandleA(std::ptr::null()) } as usize;
    if !d.relocated {
        match unsafe { register(base, d) } {
            Ok(rec) => log(&format!("KH2 effects (native): Explosion's DPX entered in KH1's loaded files ({:#x}) with {} HD textures", rec, d.hd.len())),
            Err(e) => log(&format!("KH2 effects (native): Explosion's DPX not entered in KH1's loaded files ({e}); its textures won't draw")),
        }
        let f: unsafe extern "system" fn(usize) = unsafe { std::mem::transmute(base + RELOCATE_RVA) };
        unsafe { f(d.at) };
        d.relocated = true;
        log(&format!("KH2 effects (native): Explosion's DPX at {:#x} relocated by KH1", d.at));
    }
    let f: unsafe extern "system" fn(usize, u32, usize, u32) -> usize = unsafe { std::mem::transmute(base + START_ON_BONE_RVA) };
    let element = unsafe { f(d.at, number, sora, RAW_BONE | bone) };
    if STARTED.fetch_add(1, Ordering::Relaxed) < 20 {
        log(&format!("KH2 effect (native): element {el} of motion {motion} -> KH1 effect {number} on bone {bone}: {}", if element == 0 { "not started (no free element or effect slot)".to_string() } else { format!("element {element:#x}") }));
    }
    element != 0
}
