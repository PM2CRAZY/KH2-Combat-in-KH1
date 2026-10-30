//! KH2's dim during a Drive change (the owner, the main-branch port's round 10, row 21: answers
//! 1 (a) the fog too, 2 (a) KH2's second screen effect researched for its own version). Notes:
//! `docs/kh1-drive-forms-native.md` §21.
//!
//! - **KH2** (Steam KH2, Checked (decomp)): the change's step `0x14040D000`, forms 1..6 (not the
//!   revert): a scene colour entry (48, 48, 48) whose strength goes 0 → 1 over 12 frames
//!   (`0x1403F76E0`), a 30-frame wait, held until the change is done, then 1 → 0 over 15 frames
//!   (`0x1403F76A0`). Each frame the scene's light colour and fog colour are multiplied by
//!   colour / 128 (`0x1401626F0`).
//! - **KH1**: its light list and its fog fade (`win::drivedim`) step the same numbers.
//!
//! Pure Rust (no Windows calls), unit tested on any host.

/// KH2's colour, 48 of 128.
pub const DIM: f32 = 48.0 / 128.0;
/// Frames in, the least held, out (KH2's; KH1's ticks, Assumption: both 60 a second).
pub const FRAMES_IN: f32 = 12.0;
pub const FRAMES_HELD: u32 = 30;
pub const FRAMES_OUT: f32 = 15.0;

/// KH2's feedback blur (`0x1403F7850` / `0x1403F7880`, t965): on this many frames after the change
/// starts (`0x140150080(.., 30.0)` before it), the last frame drawn back at 0x70 of 0x80, offset
/// (2, 2) PS2 pixels, no spin; out over 20 frames once the change is done (`0x1403F7830`).
pub const BLUR_AFTER: u32 = 30;
pub const BLUR_ALPHA: i32 = 0x70;
pub const BLUR_OFFSET: i32 = 2;
pub const BLUR_OUT: u32 = 20;

/// The blur's alpha with `left` of the 20 fade-out frames to go (Assumption: linear, as KH2's
/// strength fade `0x1403B7810` is set up).
pub fn blur_alpha(left: u32) -> i32 {
    BLUR_ALPHA * left.min(BLUR_OUT) as i32 / BLUR_OUT as i32
}

/// A fog colour channel (KH1's 0..255 ints) dimmed as KH2 dims its fog (`c × 48 >> 7`).
pub fn fog(c: i32) -> i32 {
    c.clamp(0, 255) * 48 >> 7
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kh2s_numbers() {
        assert_eq!(DIM, 0.375);
        assert_eq!((fog(128), fog(255), fog(0), fog(-3)), (48, 95, 0, 0));
        assert!(FRAMES_IN < FRAMES_OUT && FRAMES_HELD == 30);
        assert_eq!((blur_alpha(20), blur_alpha(10), blur_alpha(0), blur_alpha(99)), (0x70, 0x38, 0, 0x70));
    }
}
