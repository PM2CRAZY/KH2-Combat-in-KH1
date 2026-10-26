//! KH2's pad vibration patterns as KH1 vibration patterns (the tester, what-is-still-missing
//! answer 9 (a): "if rumble is on it should work then"; `docs/kh1-anim-baking.md` §13.6).
//!
//! **KH2's player** (Steam KH2 `0x14019AE60` starts a pattern, `0x14019B1D0` / `0x14019B000` step
//! it; Checked (decomp + exe)): its clock starts at 0 on the first step and adds the raw frame delta
//! × 0.5 each step after (`0x140717484`: 1.0 a 60 Hz frame, so 30 Hz ticks). The step its clock
//! reaches the pattern's last key time it plays the time `max − 1` and ends. On each step, each of
//! the two channels whose first key is at or before the clock writes its motor byte (`pad + 0x48 +
//! channel`): the first key `k + 1` at or after the clock, `level(k + 1) × (t − t(k)) / (t(k + 1) −
//! t(k))` (1.0 when the two times are equal), truncated; past its last key, nothing.
//!
//! **KH1's player** (Steam `0x1401E1400 (pattern, pad)` starts one, the task 20001 `0x1401E1900` →
//! `0x1401E1500` steps both pads; Checked (decomp)): a pattern is `0x14` at `+0`, `+4` bit 0 loop,
//! `+8` / `+0xC` the two channels' entry offsets, `+0x10` / `+0x14` their counts; an entry is `u8`
//! frames, `u8` level. The first entry's level is written as it is; each later one goes from the
//! entry before's level to its own over its frames, its clock adding the frame scale
//! (`0x14233FBDC`, 1.0 at 60 fps, 2.0 at 30) after the write, the next entry taken (once a step)
//! when the clock reaches its frames. When both channels are through, the motor bytes and the
//! pattern are cleared. The pad read (`0x14028B0B0`) sends the bytes and clears them each frame,
//! only with KH1's Vibration option on (`0x1402FB030`: the config's `+0x10` = 0).
//!
//! So a step `n` of KH1's player shows entry `n − 1`'s level (step 0 the first entry's): the
//! pattern here has one entry per step of the frame scale it is made for, each the level KH2
//! writes on that step. Steps 1.. match KH2's exactly; step 0 shows step 1's level (KH1's player
//! shows its first level twice).

/// KH2's level on its player's step `n` with clock increments of `step / 2` (`step`: 60 Hz frames
/// a game frame), for both channels; `None` past the pattern's end.
pub fn kh2_levels(channels: &[Vec<(i32, i32)>; 2], step: u32, n: u32) -> Option<[u8; 2]> {
    let max = channels.iter().flat_map(|c| c.iter().map(|k| k.0)).max()?;
    let mut t = n as f32 * step as f32 * 0.5;
    if n > 0 && (n - 1) as f32 * step as f32 * 0.5 >= max as f32 {
        return None;
    }
    if max as f32 <= t {
        t = (max - 1) as f32;
    }
    let mut out = [0u8; 2];
    for (ch, keys) in channels.iter().enumerate() {
        out[ch] = level(keys, t);
    }
    Some(out)
}

/// One channel's level at clock `t` (0 when it writes nothing: the pad read clears the bytes).
fn level(keys: &[(i32, i32)], t: f32) -> u8 {
    let Some(first) = keys.first() else { return 0 };
    if first.0 as f32 > t {
        return 0;
    }
    for w in keys.windows(2) {
        let (a, b) = (w[0], w[1]);
        if b.0 as f32 >= t {
            let span = (b.0 - a.0) as f32;
            let frac = if span == 0.0 { 1.0 } else { (t - a.0 as f32) / span };
            return ((b.1 as f32 * frac) as i32) as u8;
        }
    }
    0
}

/// A KH1 vibration pattern playing KH2's `channels` at the frame scale `step` (60 Hz frames a game
/// frame, 1..=255): see the module notes.
pub fn kh1_pattern(channels: &[Vec<(i32, i32)>; 2], step: u32) -> Vec<u8> {
    let step = step.clamp(1, 255);
    let mut levels: Vec<[u8; 2]> = (0..).map_while(|n| kh2_levels(channels, step, n)).collect();
    // (Entry j is shown on step j + 1; the step after KH2's last shows the cleared bytes: two
    // entries past it, the last taken as both channels end.)
    levels.remove(0);
    levels.push([0, 0]);
    levels.push([0, 0]);
    let n = levels.len();
    let mut p = vec![0u8; 0x18 + 4 * n];
    p[0] = 0x14;
    let (o0, o1) = (0x18u32, 0x18 + 2 * n as u32);
    p[8..12].copy_from_slice(&o0.to_le_bytes());
    p[12..16].copy_from_slice(&o1.to_le_bytes());
    p[16..20].copy_from_slice(&(n as u32).to_le_bytes());
    p[20..24].copy_from_slice(&(n as u32).to_le_bytes());
    for (j, l) in levels.iter().enumerate() {
        let dur = if j == 0 { 0 } else { step as u8 };
        for ch in 0..2 {
            let at = [o0, o1][ch] as usize + 2 * j;
            p[at] = dur;
            p[at + 1] = l[ch];
        }
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    /// KH1's player (`0x1401E1500`) stepped at `step` until done: each step's motor bytes.
    fn kh1_play(p: &[u8], step: f32) -> Vec<[u8; 2]> {
        let rd = |o: usize| u32::from_le_bytes(p[o..o + 4].try_into().unwrap()) as usize;
        let start = [rd(8), rd(0xC)];
        let mut at = start;
        let mut count = [rd(0x10) as i32, rd(0x14) as i32];
        let mut clock = [0f32; 2];
        let mut out = Vec::new();
        for _ in 0..1000 {
            let mut bytes = [0u8; 2];
            for ch in 0..2 {
                if count[ch] <= 0 {
                    continue;
                }
                let e = at[ch];
                let v = if e == start[ch] {
                    p[e + 1] as f32
                } else {
                    let v = (p[e + 1] as f32 - p[e - 1] as f32) / p[e] as f32 * clock[ch] + p[e - 1] as f32;
                    clock[ch] += step;
                    v
                };
                bytes[ch] = v.clamp(0.0, 255.0) as u8;
                if p[e] as f32 <= clock[ch] {
                    count[ch] -= 1;
                    at[ch] += 2;
                    clock[ch] -= p[e] as f32;
                }
            }
            if count[0] < 1 && count[1] < 1 {
                out.push([0, 0]);
                return out;
            }
            out.push(bytes);
        }
        panic!("never ended");
    }

    fn kh2_play(channels: &[Vec<(i32, i32)>; 2], step: u32) -> Vec<[u8; 2]> {
        (0..).map_while(|n| kh2_levels(channels, step, n)).collect()
    }

    /// KH2's patterns (`00battle.bin` vbrt, Checked (files)).
    fn patterns() -> Vec<[Vec<(i32, i32)>; 2]> {
        vec![
            [vec![(0, 255), (12, 255)], vec![]],
            [vec![(0, 255), (1, 255)], vec![]],
            [vec![], vec![(0, 128), (4, 160)]],
            [vec![(0, 255), (30, 255)], vec![(0, 255), (30, 160), (60, 0)]],
            [vec![(0, 255), (15, 255)], vec![(0, 255), (10, 255), (15, 0)]],
        ]
    }

    #[test]
    fn kh1_plays_kh2s_levels_at_60_and_30_fps() {
        for c in patterns() {
            for step in [1u32, 2] {
                let kh2 = kh2_play(&c, step);
                let kh1 = kh1_play(&kh1_pattern(&c, step), step as f32);
                // Same length (KH2's steps, then a cleared frame), the same levels from step 1 on.
                assert_eq!(kh1.len(), kh2.len() + 1, "{c:?} at {step}");
                assert_eq!(&kh1[1..kh2.len()], &kh2[1..], "{c:?} at {step}");
                assert_eq!(kh1[kh2.len()], [0, 0]);
                assert_eq!(kh1[0], kh2.get(1).copied().unwrap_or([0, 0]));
            }
        }
    }

    #[test]
    fn kh2s_ramp() {
        // Pattern 1 at 60 fps: 0, then 255 × t / 12 each half tick, the last step at t = 11.
        let c = &patterns()[0];
        let l = kh2_play(c, 1);
        assert_eq!(l.len(), 25);
        assert_eq!(l[0], [0, 0]);
        assert_eq!(l[1], [10, 0]);
        assert_eq!(l[23], [244, 0]);
        assert_eq!(l[24], [233, 0]);
    }
}
