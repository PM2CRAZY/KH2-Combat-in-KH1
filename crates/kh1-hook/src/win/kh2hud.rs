//! KH2's Drive gauge art for KH1's HUD, built once into the mod folder (the owner, the main-branch
//! port's round 3: "go ahead", answer 1 (a): KH2's art only, no gauge and a log line without it).
//! `crate::hudart` does the work (pure); here: where the files are, a thread at start, the art kept
//! for the gauge's draw (a later version). Notes: `docs/kh1-drive-forms-native.md` §14.
//!
//! At start, on a thread (nothing on the game's): KH2's install (`kh2_data::Install`, the Mods
//! Manager's backup when there is one, as the sound cache does), `field2d/<lang>/zz0field.2dd`'s PKG
//! entry as the key (`stamp_any`); the cached file is used when it matches, else the BAR and its HD
//! pictures are read from the PKG, the `gaug` sequence and its picture (the second) built into
//! `kh1-kh2-cache\kh2-hud-gauge.bin`. Only with Drive on (ini `drive`).
//!
//! (t938) **The texture**, made by KH1 the way it makes its own HUD and effect textures (Steam,
//! Checked (decomp)), once, on the game's thread (`texture_now`, from Sora's frames):
//! - the picture as a DDS (`hudart::dds_header`, B8G8R8A8) in an HD entry of a small file of ours
//!   entered in KH1's loaded-file list (`kh2dpx::new_file` / `add_hd`, as KH2's common effect
//!   textures are), then KH1's texture lookup `0x14010C050 (textures, pixels, 0, 0, w, h)` makes
//!   the texture from it, with the PS2 picture's size (512 × 256: what the sequence's texture
//!   rectangles are in), as KH1's HUD loader `0x140274060` makes its own with the TIM2's;
//! - kept for good as `0x140274060` keeps a HUD texture (`+0xB4` = -1, `+0xB0` / `+0xB8` = 0);
//! - mapped at a GS address of its own with KH1's `0x14010DC50 (tbp, 0, texture, psm, cbp)`, as
//!   KH1 maps its HUD image at 0x3600: a TEX0 of ours (`tex0`), PSMCT32, that KH1's texture map
//!   lookup `0x14010D720 (&tex0)` (what the HUD's draws bind through) returns it for. The address
//!   is the first of `TBPS` KH1 has nothing at (asked with the lookup first). KH1 has no VRAM on PC:
//!   the map is keyed by TBP, PSM and CBP only (Checked (decomp)).
//! - Once a second (`texture_now`'s check): if KH1's map has lost it (Assumption: a room change may
//!   rebuild the map; not seen yet), it is mapped again, logged.
//!
//! Nothing is drawn yet (t939).

use super::log;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

/// The art, once built or loaded (the sequence and the PS2 picture's size).
static ART: OnceLock<Art> = OnceLock::new();
/// The picture (B8G8R8A8, `w` × `h`) until it is handed to KH1 for the texture.
static PIXELS: Mutex<Option<(Vec<u8>, u32, u32)>> = Mutex::new(None);

pub struct Art {
    /// The `gaug` SEQD.
    pub seqd: Vec<u8>,
    /// The PS2 picture's size (the sequence's texture rectangles are in it).
    pub ps2: (u16, u16),
}

/// The gauge's art, if it is ready.
pub fn art() -> Option<&'static Art> {
    ART.get()
}

fn cache_dir() -> std::path::PathBuf {
    std::path::Path::new(&super::dll_dir()).join("kh1-kh2-cache")
}

/// The languages KH2's field HUD comes in (the folder names), the player's first.
const LANGS: [&str; 5] = ["us", "fr", "gr", "it", "sp"];

pub fn start() {
    if !super::settings::get().drive {
        return;
    }
    std::thread::spawn(|| {
        let t = std::time::Instant::now();
        match load() {
            Ok((b, how)) => {
                log!(Loading, "Drive gauge art: {how} in {:.0} ms ({}x{}, PS2 {}x{}, its sequence {} bytes)", t.elapsed().as_secs_f32() * 1000.0, b.w, b.h, b.ps2.0, b.ps2.1, b.seqd.len());
                if let Ok(mut g) = PIXELS.lock() {
                    *g = Some((b.pixels, b.w, b.h));
                }
                let _ = ART.set(Art { seqd: b.seqd, ps2: b.ps2 });
            }
            Err(e) => log!(Loading, "Drive gauge art: not made ({e}); no Drive gauge on the HUD"),
        }
    });
}

fn load() -> Result<(crate::hudart::Built, String), String> {
    let exe = format!("{}\\KINGDOM HEARTS II FINAL MIX.exe", super::exe_dir());
    let install = kh2_data::Install::locate(&exe).map_err(|e| format!("KH2's install: {e}"))?;
    let src = if install.has_backup() { kh2_data::Source::Backup } else { kh2_data::Source::Live };
    let (name, stamp) = LANGS
        .iter()
        .map(|l| format!("field2d/{l}/zz0field.2dd"))
        .map(|n| {
            let s = install.stamp_any(src, &n);
            (n, s)
        })
        .find(|(_, s)| s != "missing")
        .ok_or("no field2d/<lang>/zz0field.2dd")?;
    let d = crate::hudart::Desc { build: format!("{:?}", super::addr::Build::current()), source: stamp };
    let dir = cache_dir();
    let why = match crate::hudart::load(&dir, &d) {
        // (Copied out of the file's one buffer, so the picture's part is freed once KH1 has it.)
        Ok(a) => return Ok((crate::hudart::Built { seqd: a.seqd.to_vec(), pixels: a.pixels.to_vec(), w: a.w, h: a.h, ps2: a.ps2 }, format!("{} from the cache", dir.join(crate::hudart::FILE_NAME).display()))),
        Err(why) => why,
    };
    let bar = install.read_any(src, &name).map_err(|e| format!("{name}: {e}"))?;
    let (_, image) = crate::hudart::gauge_sequence(&bar)?;
    let assets = install.read_remastered_any(src, &name).map_err(|e| format!("{name}'s HD pictures: {e}"))?;
    let png = &assets.get(image).ok_or(format!("{name}: {} HD pictures, its gauge's is number {}", assets.len(), image + 1))?.data;
    let b = crate::hudart::build(&bar, png)?;
    match crate::hudart::store(&dir, &d, &b) {
        Ok(p) => log!(Loading, "Drive gauge art: written ({})", p.display()),
        Err(e) => log!(Loading, "Drive gauge art: not written ({e}); made again next start"),
    }
    Ok((b, format!("made from {name} ({why})")))
}

// ------------------------------------------------------------------------- the texture (t938)

/// KH1's map of a texture at a GS address (Steam `0x14010DC50 (tbp, 0, texture, psm, cbp)`).
const MAP_RVA: usize = 0x10DC50;
const MAP_BYTES: [u8; 11] = [0x48, 0x8B, 0xC4, 0x56, 0x57, 0x41, 0x56, 0x48, 0x83, 0xEC, 0x70];
/// KH1's texture map lookup (Steam `0x14010D720 (&tex0)`).
const LOOKUP_RVA: usize = 0x10D720;
const LOOKUP_BYTES: [u8; 15] = [0x40, 0x57, 0x48, 0x83, 0xEC, 0x30, 0x48, 0xC7, 0x44, 0x24, 0x20, 0xFE, 0xFF, 0xFF, 0xFF];
/// KH1's texture lookup (Steam `0x14010C050`, `kh2dpx::TEXTURE_OF_STEAM`).
const TEXTURE_OF_BYTES: [u8; 13] = [0x40, 0x53, 0x41, 0x54, 0x41, 0x55, 0x41, 0x56, 0x41, 0x57, 0x48, 0x81, 0xEC];
/// The GS addresses tried for ours, the first free one taken (PSMCT32 512 × 256 is 0x800 blocks:
/// each fits under 0x4000; KH1's HUD is at 0x3600 / 0x3A00 as PSMT4, other keys).
const TBPS: [u64; 4] = [0x3800, 0x3000, 0x2800, 0x2000];
/// The key's offset in our file's bytes (the HD entry's `+0x54`).
const KEY: usize = 0x20;
const FILE_LEN: usize = 0x40;

/// 0 not made yet, 1 made, 2 can't be (logged once).
static STATE: AtomicU8 = AtomicU8::new(0);
/// Our TEX0 (0 until made) and KH1's texture.
static TEX0: AtomicU64 = AtomicU64::new(0);
static TEXTURE: AtomicU64 = AtomicU64::new(0);
static FRAMES: AtomicU8 = AtomicU8::new(0);

/// KH1's texture and our TEX0 once made (the Drive gauge's draw, `gaugehud`, t939).
pub fn texture() -> Option<(usize, u64)> {
    match (TEXTURE.load(Ordering::Relaxed), TEX0.load(Ordering::Relaxed)) {
        (0, _) | (_, 0) => None,
        (t, t0) => Some((t as usize, t0)),
    }
}

/// Our TEX0 once the texture is made (t939's draw binds through it).
pub fn tex0() -> Option<u64> {
    match TEX0.load(Ordering::Relaxed) {
        0 => None,
        t => Some(t),
    }
}

/// TEX0 for PSMCT32 `w` × `h` (powers of two) at `tbp`: TBP0, TBW (width / 64), PSM 0, TW, TH,
/// TCC 1 (the picture's alpha), TFX 0 (modulate), no CLUT.
fn tex0_of(tbp: u64, w: u32, h: u32) -> u64 {
    let log2 = |v: u32| 31 - v.max(1).leading_zeros() as u64;
    tbp | ((w as u64 / 64) & 0x3F) << 14 | log2(w) << 26 | log2(h) << 30 | 1 << 34
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

unsafe fn lookup(base: usize, tex0: u64) -> usize {
    let f: unsafe extern "system" fn(*const u64) -> usize = std::mem::transmute(base + LOOKUP_RVA);
    f(&tex0)
}

unsafe fn map(base: usize, tex0: u64, texture: usize) {
    let f: unsafe extern "system" fn(i32, u32, usize, u32, i32) = std::mem::transmute(base + MAP_RVA);
    f((tex0 & 0x3FFF) as i32, 0, texture, 0, 0);
}

fn fail(why: &str) {
    STATE.store(2, Ordering::Relaxed);
    log!(Loading, "Drive gauge texture: not made ({why}); no Drive gauge on the HUD");
}

/// From Sora's frames (`menu::Growth::update`, the game's thread): makes the texture once the art is
/// ready; then once a second, maps it again if KH1's map lost it. One load a frame otherwise.
pub unsafe fn texture_now(base: usize) {
    match STATE.load(Ordering::Relaxed) {
        2 => return,
        1 => {
            if FRAMES.fetch_add(1, Ordering::Relaxed) < 59 {
                return;
            }
            FRAMES.store(0, Ordering::Relaxed);
            let (t0, tex) = (TEX0.load(Ordering::Relaxed), TEXTURE.load(Ordering::Relaxed) as usize);
            let now = lookup(base, t0);
            if now != tex {
                map(base, t0, tex);
                log!(Loading, "Drive gauge texture: KH1's map had {now:#x} at TBP {:#x}, mapped {tex:#x} again", t0 & 0x3FFF);
            }
            return;
        }
        _ => {}
    }
    let Some(art) = ART.get() else { return };
    if base == 0 {
        return;
    }
    if super::addr::Build::current() != super::addr::Build::Steam {
        return fail("Steam only");
    }
    for (rva, bytes, what) in [(MAP_RVA, &MAP_BYTES[..], "the texture map"), (LOOKUP_RVA, &LOOKUP_BYTES[..], "the map lookup"), (super::kh2dpx::TEXTURE_OF_STEAM, &TEXTURE_OF_BYTES[..], "the texture lookup")] {
        if std::slice::from_raw_parts((base + rva) as *const u8, bytes.len()) != bytes {
            return fail(&format!("unexpected code at {what}, 0x{:X}", 0x1_4000_0000usize + rva));
        }
    }
    let textures = rd::<usize>(base + super::kh2dpx::TEXTURES_STEAM);
    if textures == 0 {
        // (KH1's texture manager isn't up yet: tried again next frame.)
        return;
    }
    let k = match super::kh2dpx::kh1_files(base) {
        Ok(k) => k,
        // (The loaded-file list may not be up yet either.)
        Err(_) => return,
    };
    let Some(&tbp) = TBPS.iter().find(|&&t| lookup(base, tex0_of(t, art.ps2.0 as u32, art.ps2.1 as u32)) == 0) else {
        return fail("KH1 has textures at every GS address tried");
    };
    let t0 = tex0_of(tbp, art.ps2.0 as u32, art.ps2.1 as u32);
    let Some((pixels, w, h)) = PIXELS.lock().ok().and_then(|mut g| g.take()) else { return fail("the picture is gone") };
    let mut dds = Vec::with_capacity(128 + pixels.len());
    dds.extend_from_slice(&crate::hudart::dds_header(w, h));
    dds.extend_from_slice(&pixels);
    drop(pixels);
    // (Our file's bytes: only their place matters, the HD entry is keyed by the offset in them.)
    let file: &'static mut [u8] = Box::leak(vec![0u8; FILE_LEN].into_boxed_slice());
    let at = file.as_ptr() as usize;
    let rec = match super::kh2dpx::new_file(&k, "kh2hud_gauge", at, FILE_LEN, kh2_formats::kh1dpx::md5(&dds[..128])) {
        Ok(r) => r,
        Err(e) => return fail(&format!("not entered in KH1's loaded files: {e}")),
    };
    if let Err(e) = super::kh2dpx::add_hd(&k, rec, KEY as u32, Some(&dds), 0) {
        return fail(&format!("no HD entry: {e}"));
    }
    let len = dds.len();
    drop(dds);
    let make: unsafe extern "system" fn(usize, usize, usize, usize, i32, i32) -> usize = std::mem::transmute(base + super::kh2dpx::TEXTURE_OF_STEAM);
    let tex = make(textures, at + KEY, 0, 0, art.ps2.0 as i32, art.ps2.1 as i32);
    if tex == 0 {
        return fail("KH1's texture lookup gave none");
    }
    // (Kept for good, as KH1's HUD loader `0x140274060` keeps its own.)
    std::ptr::write_unaligned((tex + 0xB8) as *mut u64, 0);
    std::ptr::write_unaligned((tex + 0xB0) as *mut u32, 0);
    std::ptr::write_unaligned((tex + 0xB4) as *mut i32, -1);
    map(base, t0, tex);
    let back = lookup(base, t0);
    TEXTURE.store(tex as u64, Ordering::Relaxed);
    TEX0.store(t0, Ordering::Relaxed);
    STATE.store(1, Ordering::Relaxed);
    log!(
        Loading,
        "Drive gauge texture: KH1 texture {tex:#x} made from the {w}x{h} picture ({len} bytes as DDS, file {rec:#x}), PS2 size {}x{}, mapped at TBP {tbp:#x} (TEX0 {t0:#x}); KH1's map lookup gives {back:#x}{}",
        art.ps2.0,
        art.ps2.1,
        if back == tex { "" } else { " (not ours!)" }
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn tex0_fields() {
        let t = super::tex0_of(0x3800, 512, 256);
        assert_eq!(t & 0x3FFF, 0x3800);
        assert_eq!((t >> 14) & 0x3F, 8);
        assert_eq!((t >> 20) & 0x3F, 0);
        assert_eq!(((t >> 26) & 0xF, (t >> 30) & 0xF, (t >> 34) & 1), (9, 8, 1));
    }
}
