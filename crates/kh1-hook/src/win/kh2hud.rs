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

use super::log;
use std::sync::OnceLock;

/// The art, once built or loaded.
static ART: OnceLock<crate::hudart::Art> = OnceLock::new();

/// The gauge's art, if it is ready.
pub fn art() -> Option<&'static crate::hudart::Art> {
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
            Ok((art, how)) => {
                log!(Loading, "Drive gauge art: {how} in {:.0} ms ({}x{}, its sequence {} bytes)", t.elapsed().as_secs_f32() * 1000.0, art.w, art.h, art.seqd.len());
                let _ = ART.set(art);
            }
            Err(e) => log!(Loading, "Drive gauge art: not made ({e}); no Drive gauge on the HUD"),
        }
    });
}

fn load() -> Result<(crate::hudart::Art, String), String> {
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
        Ok(art) => return Ok((art, format!("{} from the cache", dir.join(crate::hudart::FILE_NAME).display()))),
        Err(why) => why,
    };
    let bar = install.read_any(src, &name).map_err(|e| format!("{name}: {e}"))?;
    let (_, image) = crate::hudart::gauge_sequence(&bar)?;
    let assets = install.read_remastered_any(src, &name).map_err(|e| format!("{name}'s HD pictures: {e}"))?;
    let png = &assets.get(image).ok_or(format!("{name}: {} HD pictures, its gauge's is number {}", assets.len(), image + 1))?.data;
    let (seqd, pixels, w, h) = crate::hudart::build(&bar, png)?;
    match crate::hudart::store(&dir, &d, &seqd, &pixels, w, h) {
        Ok(p) => log!(Loading, "Drive gauge art: written ({})", p.display()),
        Err(e) => log!(Loading, "Drive gauge art: not written ({e}); made again next start"),
    }
    let art = crate::hudart::Art { seqd: crate::slotcache::Block::new(seqd), pixels: crate::slotcache::Block::new(pixels), w, h };
    Ok((art, format!("made from {name} ({why})")))
}
