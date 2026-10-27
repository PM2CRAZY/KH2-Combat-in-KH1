//! KH1's event script code, as the script step (`0x1401A2320`) reads it: 4-byte instructions, the
//! class in the last byte and a 24-bit operand. Patterns the In-game Scene Skip looks for
//! (`win/ingameskip.rs`, `docs/kh1-ingame-skip.md`). Pure: no game memory.

/// Class 5 ends a sub-script, class 9 pushes its operand, class 0x18 calls the event function its
/// operand names (Checked (decomp + files)).
pub const CLASS_END: u8 = 5;
pub const CLASS_PUSH: u8 = 9;
pub const CLASS_CALL: u8 = 0x18;
/// The Trinity command a mark offers in the Triangle box (Checked (files): offered in 17 rooms of six
/// worlds, each one starting the party's Trinity scene; that KH1 names it "Trinity" is an Assumption).
pub const CMD_TRINITY: u32 = 0x53;
/// Event function 454 puts a command in the room's object table (`0x142382938`, 8 slots of 0x20: the
/// context's actor, the command at `+0xC`, a flag at `+0x14`; the Triangle-box scan `0x1401A4D00`
/// offers it). Event function 126 starts an event entry (under 100) of the room's script file
/// (`0x1401A38A0`). Checked (exe + decomp).
pub const FN_OFFER: u32 = 454;
pub const FN_START_ENTRY: u32 = 126;

/// What three instructions in a row do, for the Trinity reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Call {
    /// `push 0x53, push 1, call 454`: the context offers the Trinity command.
    OfferTrinity,
    /// `.., push n, call 126`: the context starts event entry `n`.
    StartEntry(u32),
}

/// With `last` the last three instructions read (newest last), the Trinity offer or an entry start
/// they make, if any.
pub fn trinity_call(last: &[u32; 3]) -> Option<Call> {
    let op = |w: u32| (w >> 24) as u8;
    let arg = |w: u32| w & 0xFF_FFFF;
    let [a, b, c] = *last;
    if op(c) != CLASS_CALL {
        return None;
    }
    if arg(c) == FN_OFFER && op(a) == CLASS_PUSH && arg(a) == CMD_TRINITY && op(b) == CLASS_PUSH && arg(b) == 1 {
        return Some(Call::OfferTrinity);
    }
    if arg(c) == FN_START_ENTRY && op(b) == CLASS_PUSH {
        return Some(Call::StartEntry(arg(b)));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn ins(class: u8, arg: u32) -> u32 {
        (class as u32) << 24 | arg
    }

    #[test]
    fn reads_a_trinity_mark() {
        // `push 0x53, push 1, call 454`: the mark offers the Trinity command.
        let offer = [ins(CLASS_PUSH, 0x53), ins(CLASS_PUSH, 1), ins(CLASS_CALL, 454)];
        assert_eq!(trinity_call(&offer), Some(Call::OfferTrinity));
        // Taking the command back (`push 0x53, push 0, call 454`) or another command is not an offer.
        assert_eq!(trinity_call(&[ins(CLASS_PUSH, 0x53), ins(CLASS_PUSH, 0), ins(CLASS_CALL, 454)]), None);
        assert_eq!(trinity_call(&[ins(CLASS_PUSH, 0x37), ins(CLASS_PUSH, 1), ins(CLASS_CALL, 454)]), None);
        // `push 50, call 126` starts entry 50.
        let start = [ins(0x0D, 0xD58), ins(CLASS_PUSH, 50), ins(CLASS_CALL, 126)];
        assert_eq!(trinity_call(&start), Some(Call::StartEntry(50)));
        // A push of 126 isn't a call; an entry started from a variable isn't read.
        assert_eq!(trinity_call(&[ins(CLASS_PUSH, 0x53), ins(CLASS_PUSH, 50), ins(CLASS_PUSH, 126)]), None);
        assert_eq!(trinity_call(&[ins(CLASS_PUSH, 1), ins(0x0A, 0x12), ins(CLASS_CALL, 126)]), None);
    }
}
