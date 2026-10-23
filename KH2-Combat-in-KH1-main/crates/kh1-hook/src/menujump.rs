//! The jump press that backs out of a command submenu (`win::menujump`): KH1's jump and cancel are
//! one button, so a press made with Magic, Items, Summon, Drive (or a deeper list) open is the
//! menu's alone, and stays the menu's until it is let go. Kept here, apart from the game, so it
//! can be tested.

/// The press is held back from KH2 (until the button is let go).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JumpHold {
    held_back: bool,
}

impl JumpHold {
    pub const fn new() -> Self {
        JumpHold { held_back: false }
    }

    /// KH1's command step, before it runs: a jump / cancel press with a submenu open is the
    /// menu's (KH1 backs out a level and doesn't jump). `true`: this press is newly held back.
    pub fn menu_step(&mut self, submenu_open: bool, jump_pressed: bool) -> bool {
        if submenu_open && jump_pressed && !self.held_back {
            self.held_back = true;
            return true;
        }
        false
    }

    /// The jump button as KH2 is to see it: down only if it isn't the held-back press. Letting go
    /// ends the hold.
    pub fn filter(&mut self, jump_down: bool) -> bool {
        if !jump_down {
            self.held_back = false;
        }
        jump_down && !self.held_back
    }

    pub fn held_back(&self) -> bool {
        self.held_back
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One game frame: KH1's command step first (it closes the submenu on the press), then KH2's
    /// read (KH1 runs its command step at a lower task priority than the actors, so earlier).
    fn frame(h: &mut JumpHold, submenu_open: bool, pressed: bool, down: bool) -> bool {
        h.menu_step(submenu_open, pressed);
        h.filter(down)
    }

    #[test]
    fn the_press_that_closes_a_submenu_never_reaches_kh2() {
        let mut h = JumpHold::default();
        // The press frame: the submenu is open as KH1's step starts.
        assert!(!frame(&mut h, true, true, true));
        // Held on after the menu closed: still not a jump.
        for _ in 0..30 {
            assert!(!frame(&mut h, false, false, true));
        }
        // Let go, then a new press at the top level jumps.
        assert!(!frame(&mut h, false, false, false));
        assert!(frame(&mut h, false, true, true));
    }

    #[test]
    fn backing_out_two_levels_holds_both_presses() {
        let mut h = JumpHold::default();
        // A spell's target list, back to the magic list: the press is the menu's.
        assert!(!frame(&mut h, true, true, true));
        assert!(!frame(&mut h, true, false, false));
        // The magic list, back to the top.
        assert!(!frame(&mut h, true, true, true));
        assert!(!frame(&mut h, false, false, true));
        assert!(!frame(&mut h, false, false, false));
        assert!(!h.held_back());
    }

    #[test]
    fn the_top_level_jumps_as_normal() {
        let mut h = JumpHold::default();
        assert!(frame(&mut h, false, true, true));
        assert!(frame(&mut h, false, false, true));
        assert!(!frame(&mut h, false, false, false));
    }

    #[test]
    fn a_press_logged_once() {
        let mut h = JumpHold::default();
        assert!(h.menu_step(true, true));
        // (The same press seen again on a later step, as KH1's repeat bits: not a new one.)
        assert!(!h.menu_step(true, true));
    }
}
