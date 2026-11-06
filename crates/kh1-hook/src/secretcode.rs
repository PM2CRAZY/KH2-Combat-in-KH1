//! The secret button code (`win::form`: it asks for Anti Form). Kept unreadable on purpose (the
//! user: it stays a secret). Pure, so it is tested here; the game side feeds it KH1's buttons
//! (PS2 order: Select, L3, R3, Start, Up, Right, Down, Left, L2, R2, L1, R1, Triangle, Circle,
//! Cross, Square).

/// The code, `code_bit`.
const CODE: [u8; 10] = [94, 94, 92, 92, 93, 95, 93, 95, 87, 84];
/// The code's length.
pub const LEN: usize = CODE.len();
/// The buttons the code watches (any other pressed breaks it), and the longest gap between two
/// presses.
pub const BUTTONS: u16 = (1 << 4) | (1 << 5) | (1 << 6) | (1 << 7) | (1 << 13) | (1 << 14) | (1 << 12) | (1 << 15);
pub const GAP_MS: u64 = 1500;

/// The pad bit of the code's `i`th button.
pub fn code_bit(i: usize) -> u16 {
    1 << (CODE[i] ^ 0x5A)
}

/// What a read of the buttons did to the code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    /// Nothing new (no fresh press, or one that kept the code going or started it).
    None,
    /// The whole code was entered.
    Entered,
    /// The code was `at` presses in and broke: too long a gap, or a press out of order.
    Broken { at: usize, gap: bool },
}

/// The code so far: the buttons held at the last read, the last presses, when the last came.
#[derive(Default)]
pub struct Matcher {
    held: u16,
    presses: Vec<u16>,
    at: usize,
    last_ms: Option<u64>,
}

impl Matcher {
    pub const fn new() -> Self {
        Matcher { held: 0, presses: Vec::new(), at: 0, last_ms: None }
    }

    /// How far in the code is now.
    pub fn at(&self) -> usize {
        self.at
    }

    /// One read of the buttons held (`now_ms` any steady clock). A fresh press of a watched button
    /// moves the code on; a wrong one falls back to the longest start of the code the last presses
    /// still make (so a third Up after two keeps two, as a player retrying expects).
    pub fn feed(&mut self, buttons: u16, now_ms: u64) -> Event {
        let pressed = buttons & !self.held & BUTTONS;
        self.held = buttons;
        if pressed == 0 {
            return Event::None;
        }
        let mut out = Event::None;
        if self.last_ms.is_some_and(|t| now_ms.saturating_sub(t) > GAP_MS) {
            if self.at >= 2 {
                out = Event::Broken { at: self.at, gap: true };
            }
            self.presses.clear();
            self.at = 0;
        }
        self.last_ms = Some(now_ms);
        for bit in (0..16).map(|i| 1u16 << i).filter(|b| pressed & b != 0) {
            let was = self.at;
            self.presses.push(bit);
            if self.presses.len() > LEN {
                self.presses.remove(0);
            }
            self.at = (0..=self.presses.len().min(LEN))
                .rev()
                .find(|&k| self.presses[self.presses.len() - k..].iter().enumerate().all(|(i, &b)| b == code_bit(i)))
                .unwrap_or(0);
            if self.at == LEN {
                self.at = 0;
                self.presses.clear();
                return Event::Entered;
            }
            if self.at < was && was >= 2 {
                out = Event::Broken { at: was, gap: false };
            }
        }
        out
    }
}

/// KH1's button remap (Steam, in its pad step `0x14028B0B0` after slot 0's held word is built;
/// Checked (decomp)): slot 0's held word in PS2 order (`raw`) to KH1's logical word. Each of the
/// remap table's 19 `(mask, bit)` pairs (`0x140528290`, read from the running game) gives `bit`
/// when every bit of `mask` is held; then Triangle `0x1000` gives `0x80000`, Square `0x8000`
/// `0x100000`, and the confirm button `0x210001`, the cancel button `6`: Cross confirms and Circle
/// cancels, swapped when KH1's settings block (`0x1404D8614`) `+0x1E` is not 0 (`0x1402F6AD0`).
pub fn kh1_logical(raw: u32, table: &[(u32, u32)], swap: bool) -> u32 {
    let mut v = table.iter().filter(|(m, _)| raw & m == *m).fold(0, |v, (_, b)| v | b);
    if raw & 0x1000 != 0 {
        v |= 0x8_0000;
    }
    if raw & 0x8000 != 0 {
        v |= 0x10_0000;
    }
    let (confirm, cancel) = if swap { (0x2000, 0x4000) } else { (0x4000, 0x2000) };
    if raw & confirm != 0 {
        v |= 0x21_0001;
    }
    if raw & cancel != 0 {
        v |= 6;
    }
    v
}

/// The logical bits only `button` gives with `raw` held (`kh1_logical` with and without it).
pub fn logical_only(button: u32, raw: u32, table: &[(u32, u32)], swap: bool) -> u32 {
    kh1_logical(raw | button, table, swap) & !kh1_logical(raw & !button, table, swap)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Presses `bits` one at a time, `ms` apart, released between; the last event that wasn't None.
    fn enter(m: &mut Matcher, bits: &[u16], start: u64, ms: u64) -> Vec<Event> {
        let mut t = start;
        let mut ev = Vec::new();
        for &b in bits {
            let e = m.feed(b, t);
            if e != Event::None {
                ev.push(e);
            }
            t += ms / 2;
            m.feed(0, t);
            t += ms / 2;
        }
        ev
    }

    fn code() -> Vec<u16> {
        (0..LEN).map(code_bit).collect()
    }

    #[test]
    fn the_code_is_watched() {
        assert!(code().iter().all(|b| b & BUTTONS == *b));
    }

    #[test]
    fn entered_at_any_pace_within_the_gap() {
        for ms in [34, 100, 400, 1400] {
            let mut m = Matcher::new();
            assert_eq!(enter(&mut m, &code(), 1000, ms), vec![Event::Entered], "{ms} ms apart");
        }
    }

    #[test]
    fn a_repeated_first_press_still_enters() {
        let mut m = Matcher::new();
        let mut seq = vec![code_bit(0)];
        seq.extend(code());
        assert_eq!(enter(&mut m, &seq, 0, 200).last(), Some(&Event::Entered));
    }

    #[test]
    fn a_retry_after_a_wrong_press_enters() {
        let mut m = Matcher::new();
        let mut seq = code()[..5].to_vec();
        seq.push(1 << 12);
        seq.extend(code());
        let ev = enter(&mut m, &seq, 0, 200);
        assert_eq!(ev, vec![Event::Broken { at: 5, gap: false }, Event::Entered]);
    }

    #[test]
    fn too_long_a_gap_breaks_it() {
        let mut m = Matcher::new();
        let c = code();
        enter(&mut m, &c[..4], 0, 200);
        let ev = enter(&mut m, &c[4..], 10_000, 200);
        assert_eq!(ev, vec![Event::Broken { at: 4, gap: true }]);
        assert_ne!(m.at(), LEN);
    }

    #[test]
    fn crosss_own_logical_bits() {
        // (A made-up table: Cross alone gives 0x40, L1 + Cross 0x800, Up 0x1000.)
        let t = [(0x4000, 0x40), (0x4400, 0x800), (0x10, 0x1000)];
        assert_eq!(logical_only(0x4000, 0x10, &t, false), 0x21_0041);
        assert_eq!(logical_only(0x4000, 0x400, &t, false), 0x21_0841);
        // (Swapped: Cross cancels, Circle confirms.)
        assert_eq!(logical_only(0x4000, 0, &t, true), 0x46);
        assert_eq!(kh1_logical(0x2000, &t, true), 0x21_0001);
        // (A bit Circle held gives too stays.)
        assert_eq!(logical_only(0x4000, 0x2000, &[(0x4000, 6)], false), 0x21_0001);
    }

    #[test]
    fn held_buttons_and_other_buttons_dont_count() {
        let mut m = Matcher::new();
        let c = code();
        // (Held through two reads: one press.)
        m.feed(c[0], 0);
        m.feed(c[0], 16);
        assert_eq!(m.at(), 1);
        // (L1, not watched, pressed between: no break.)
        m.feed(0, 32);
        m.feed(1 << 10, 48);
        m.feed(0, 64);
        assert_eq!(m.at(), 1);
    }
}
