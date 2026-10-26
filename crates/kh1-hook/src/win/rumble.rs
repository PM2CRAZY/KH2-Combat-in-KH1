//! KH2's pad vibration through KH1's own vibration player (the tester, what-is-still-missing
//! answer 9 (a): "if rumble is on it should work then"; `docs/kh1-anim-baking.md` §13.6).
//!
//! - **KH2.** A motion's frame trigger 31 (Steam KH2 `0x14040DD70` case 0x1F) looks its parameter
//!   up in `00battle.bin`'s `vbrt` (`0x1403EEFC0`) and starts that pattern on the pad
//!   (`0x14019AE60`): two motor channels of keys. Checked (decomp + exe + files). Sora's hit
//!   reactions 10..15 (pattern 3), 16 / 18 (4), recoils 41 / 43 (5), the guard repel 42 (1), the
//!   Drive 52 (1, then 9 at frame 20), the casts, Explosion, Finishing Leap, Counterguard, Magnet
//!   Burst, and the forms' own (`kh2slots`'s baked slots carry every one, `kh2_trigger_keys`).
//! - **KH1.** The trigger is a sound event of the baked motion (`ks::KH2_RUMBLE`), fired by KH1's
//!   motion pass; `kh2slots::sound_event` sends it here, and KH2's pattern, written once in KH1's
//!   own pattern layout for the frame scale it fires at (`kh1_data::vibe`), is started on pad 0 by
//!   KH1's own player (Steam `0x1401E1400 (pattern, 0)`, as KH1's hits start theirs from
//!   `btlvibe.dat`, `0x14028F8D0`; Checked (decomp)). KH1's player steps it (task 20001) and KH1's
//!   pad read sends it only with KH1's Vibration option on, and clears it each frame
//!   (`0x14028B0B0`, `0x1402FB030`: the config's `+0x10` 0 = on; Checked (decomp)). Nothing of ours
//!   runs per frame.
//! - Steam only (KH1's player's address is Steam's; elsewhere the trigger does nothing).

use super::{log, pose};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

/// KH1's vibration start (Steam `0x1401E1400 (pattern, pad)`): its first bytes, `cmp byte
/// [rcx], 0x14; mov r8, rcx` (Checked (exe)), checked before it is called.
const PLAY_STEAM: usize = 0x1E1400;
const PLAY_BYTES: [u8; 6] = [0x80, 0x39, 0x14, 0x4C, 0x8B, 0xC1];
/// KH1's frame scale (Steam `0x14233FBDC`: 1.0 at 60 fps, 2.0 at 30).
const FRAME_SCALE_STEAM: usize = 0x233FBDC;

/// KH1 patterns made so far, by (KH2 pattern id, frame scale): KH1's player keeps a pointer to the
/// one playing, so they live for the game's run (nine patterns × the scales seen, a few hundred
/// bytes each).
static MADE: Mutex<Option<HashMap<(i16, u32), &'static [u8]>>> = Mutex::new(None);

/// KH2's frame trigger 31 with pattern `id` on KH2 motion `motion`, fired by KH1's motion pass on
/// Sora (`kh2slots::sound_event`).
pub unsafe fn play(motion: u16, id: i16) {
    static N: AtomicU32 = AtomicU32::new(0);
    let said = super::log_on(super::LogCat::Animation) && N.fetch_add(1, Ordering::Relaxed) < 40;
    if super::addr::Build::current() != super::addr::Build::Steam {
        return;
    }
    let base = super::takeover::game_base();
    let f = base + PLAY_STEAM;
    if !super::crash::readable(f + PLAY_BYTES.len() - 1) || std::slice::from_raw_parts(f as *const u8, PLAY_BYTES.len()) != PLAY_BYTES {
        if said {
            log!(Animation, "KH2 rumble: KH1's vibration start isn't the one expected; KH2 motion {motion}'s pattern {id} not played");
        }
        return;
    }
    let scale = std::ptr::read_unaligned((base + FRAME_SCALE_STEAM) as *const f32);
    let step = if scale.is_finite() { scale.round().clamp(1.0, 6.0) as u32 } else { 1 };
    let Some(p) = pattern(id, step) else {
        if said {
            log!(Animation, "KH2 rumble: KH2 motion {motion}'s pattern {id} isn't in KH2's vbrt");
        }
        return;
    };
    let start: unsafe extern "system" fn(*const u8, i32) = std::mem::transmute(f);
    start(p.as_ptr(), 0);
    if said {
        log!(Animation, "KH2 rumble: KH2 motion {motion}'s pattern {id} on KH1's vibration player (frame scale {step}, {} steps)", (p.len() - 0x18) / 4);
    }
}

/// KH2's pattern `id` in KH1's layout for frame scale `step`, made once.
fn pattern(id: i16, step: u32) -> Option<&'static [u8]> {
    let mut g = MADE.lock().ok()?;
    let map = g.get_or_insert_with(HashMap::new);
    if let Some(p) = map.get(&(id, step)) {
        return Some(p);
    }
    let v = pose::kh2()?.battle.vibration(id)?;
    let p: &'static [u8] = Box::leak(kh1_data::vibe::kh1_pattern(&v.channels, step).into_boxed_slice());
    map.insert((id, step), p);
    Some(p)
}
