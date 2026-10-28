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

/// Class `0x15` pushes the context its operand names and class `0x16` wakes that context's sub-script
/// its operand names (the pair `21/c 22/s`; `0x1401A0AF0` pops the context and the priority and sets
/// the woken context's next instruction to the sub-script's start, `+0x90 + s * 4`). Checked (decomp +
/// files).
pub const CLASS_CTX: u8 = 0x15;
pub const CLASS_WAKE: u8 = 0x16;
/// Event function 514 turns event mode off (bit `0x400` of `0x142382594`, `0x1401A7750`); 512 gives
/// Sora back to the player (clears bit 1 of `0x1423AB2D0`, `0x1401A7660`). Checked (decomp + exe).
pub const FN_EVENT_OFF: u32 = 514;
pub const FN_RELEASE: u32 = 512;
/// The most sub-scripts and instructions a follow-up read looks at.
pub const FOLLOW_SUBS: usize = 128;
pub const FOLLOW_MAX: usize = 8192;

/// What the room script does after an event entry it started has ended, read from where it carries on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Follow {
    /// It ends event mode (514) or gives Sora back (512) with no choice, shop or menu on the way
    /// (instructions read).
    Skip(usize),
    /// A choice, shop or menu (the event function) on the way, or in a sub-script it wakes.
    Plays(u32),
    /// Its sub-script ends without ending event mode: there is no end to skip to.
    NoEnd,
    /// The code couldn't be read.
    Unread,
}

/// Reads the room script's follow-up from context `ctx`'s instruction `pc` to its first 514 or 512 (or
/// the end of the sub-script), and every sub-script it wakes on the way (`21/c 22/s`) in full.
/// `word(ctx, i)` is context `ctx`'s instruction `i`; `sub(ctx, s)` the first instruction of its
/// sub-script `s`; `plays` the event functions that make it play.
pub fn read_follow_up(ctx: usize, pc: usize, word: impl Fn(usize, usize) -> Option<u32>, sub: impl Fn(usize, usize) -> Option<usize>, plays: &[u32]) -> Follow {
    walk_follow_up(ctx, pc, word, sub, plays, &mut Vec::new())
}

/// `read_follow_up`, with every event function called on the way noted in `seen` (once each).
fn walk_follow_up(ctx: usize, pc: usize, word: impl Fn(usize, usize) -> Option<u32>, sub: impl Fn(usize, usize) -> Option<usize>, plays: &[u32], seen: &mut Vec<u32>) -> Follow {
    let op = |w: u32| (w >> 24) as u8;
    let arg = |w: u32| w & 0xFF_FFFF;
    // (context, first instruction, the main path)
    let mut todo = vec![(ctx, pc, true)];
    let mut woken: Vec<(usize, usize)> = Vec::new();
    let mut total = 0usize;
    let mut ended = false;
    while let Some((c, start, main)) = todo.pop() {
        let mut last_ctx = None;
        let mut i = start;
        loop {
            if i - start >= FOLLOW_MAX {
                return Follow::Unread;
            }
            let Some(w) = word(c, i) else { return Follow::Unread };
            total += 1;
            i += 1;
            match op(w) {
                CLASS_END => break,
                CLASS_CALL => {
                    let f = arg(w);
                    if !seen.contains(&f) {
                        seen.push(f);
                    }
                    if plays.contains(&f) {
                        return Follow::Plays(f);
                    }
                    if main && (f == FN_EVENT_OFF || f == FN_RELEASE) {
                        ended = true;
                        break;
                    }
                }
                CLASS_CTX => last_ctx = Some(arg(w) as usize),
                CLASS_WAKE => {
                    if let Some(to) = last_ctx {
                        let s = arg(w) as usize;
                        if !woken.contains(&(to, s)) {
                            if woken.len() >= FOLLOW_SUBS {
                                return Follow::Unread;
                            }
                            woken.push((to, s));
                            let Some(first) = sub(to, s) else { return Follow::Unread };
                            todo.push((to, first, false));
                        }
                    }
                }
                _ => {}
            }
        }
        if main && !ended {
            return Follow::NoEnd;
        }
    }
    Follow::Skip(total)
}

/// Class 1 with operand 6 compares the two values on top of the stack, class 3 jumps past the code that
/// follows when they differ (Checked (files): every command handler of the room scripts reads
/// `push <command>, 1/6, 3/<skip>` and runs that command's part right after it).
pub const CLASS_CMP: u8 = 1;
pub const CMP_EQ: u32 = 6;
pub const CLASS_JUMP_NOT: u8 = 3;
/// The event functions a "box" handler is made of: 511 takes the player for the script (KH1's game
/// mode "event"), 258 adds an item to the stock (`push item, push count`), 583 gives a reward record
/// (`push record`). Checked (exe + decomp).
pub const FN_HOLD: u32 = 511;
pub const FN_ADD_ITEM: u32 = 258;
pub const FN_GIVE_REWARD: u32 = 583;

/// What a room script's handler for a picked Triangle command does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoxOpen {
    /// A box: it takes the player (511), gives an item or a reward (258 / 583) and gives the player back
    /// (512) with no scene, choice, shop or menu on the way (instructions read).
    Give(usize),
    /// Not a box (why: the event function that plays, 0 when it gives nothing or doesn't hold).
    Not(u32),
    /// No part for that command in the handler, or the code couldn't be read.
    Unread,
}

/// Reads context `ctx`'s command handler (its sub-script starting at `start`) for command `cmd`: the
/// part after `push cmd, compare, jump if different`, to its first 512 (or 514), with every sub-script
/// it wakes. `plays` are the event functions that make it a scene (a choice, a shop, a menu, an event
/// entry, a pause-skippable section).
pub fn read_box(ctx: usize, start: usize, cmd: u32, word: impl Fn(usize, usize) -> Option<u32>, sub: impl Fn(usize, usize) -> Option<usize>, plays: &[u32]) -> BoxOpen {
    let op = |w: u32| (w >> 24) as u8;
    let arg = |w: u32| w & 0xFF_FFFF;
    // The command's part: `push cmd, 1/6, 3/n` inside the handler's sub-script.
    let mut i = start;
    let part = loop {
        if i - start >= FOLLOW_MAX {
            return BoxOpen::Unread;
        }
        let Some(w) = word(ctx, i) else { return BoxOpen::Unread };
        if op(w) == CLASS_END {
            return BoxOpen::Unread;
        }
        if op(w) == CLASS_PUSH && arg(w) == cmd {
            if let (Some(a), Some(b)) = (word(ctx, i + 1), word(ctx, i + 2)) {
                if op(a) == CLASS_CMP && arg(a) == CMP_EQ && op(b) == CLASS_JUMP_NOT {
                    break i + 3;
                }
            }
        }
        i += 1;
    };
    let mut seen = Vec::new();
    match walk_follow_up(ctx, part, &word, &sub, plays, &mut seen) {
        Follow::Skip(n) => {
            let gives = seen.contains(&FN_ADD_ITEM) || seen.contains(&FN_GIVE_REWARD);
            if seen.contains(&FN_HOLD) && gives && seen.contains(&FN_RELEASE) {
                BoxOpen::Give(n)
            } else {
                BoxOpen::Not(0)
            }
        }
        Follow::Plays(f) => BoxOpen::Not(f),
        Follow::NoEnd => BoxOpen::Not(0),
        Follow::Unread => BoxOpen::Unread,
    }
}

/// Whether the instruction before `pc` is `call 126` with `push entry` before it: the context started
/// `entry` there and carries on at `pc` when it has ended (126 from the room script advances past the
/// call, `0x1401A9D90` returns 10). `word(i)` is the context's instruction `i`.
pub fn started_entry(pc: usize, entry: u32, word: impl Fn(usize) -> Option<u32>) -> bool {
    if pc < 2 {
        return false;
    }
    match (word(pc - 2), word(pc - 1)) {
        (Some(a), Some(b)) => trinity_call(&[0, a, b]) == Some(Call::StartEntry(entry)),
        _ => false,
    }
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

    #[test]
    fn reads_a_box() {
        let plays = [88u32, 126];
        // Context 0's sub-script 9: a talk part for command 0x56, then the box part for 0x55: hold,
        // wake context 1's sub-script 1 (Sora's motion), give item 0xDF, a line, give the player back.
        let c0 = vec![
            ins(CLASS_PUSH, 0x56), ins(CLASS_CMP, 6), ins(CLASS_JUMP_NOT, 3), ins(CLASS_CALL, 88), ins(CLASS_CALL, 512),
            ins(CLASS_PUSH, 0x55), ins(CLASS_CMP, 6), ins(CLASS_JUMP_NOT, 12),
            ins(CLASS_CALL, 511), ins(CLASS_PUSH, 5), ins(CLASS_CTX, 1), ins(CLASS_WAKE, 1),
            ins(CLASS_PUSH, 0xDF), ins(CLASS_PUSH, 1), ins(CLASS_CALL, 258), ins(CLASS_PUSH, 7), ins(CLASS_CALL, 1),
            ins(CLASS_CALL, 112), ins(CLASS_CALL, 512), ins(CLASS_END, 0x10),
        ];
        let c1 = vec![ins(CLASS_CALL, 55), ins(CLASS_END, 0x10), ins(CLASS_PUSH, 0xA6), ins(CLASS_CALL, 108), ins(CLASS_CALL, 103), ins(CLASS_END, 0x10)];
        let ctxs = [c0.clone(), c1.clone()];
        let word = |c: usize, i: usize| ctxs.get(c).and_then(|v| v.get(i)).copied();
        let sub = |c: usize, s: usize| match (c, s) { (0, 0) => Some(0), (1, 0) => Some(0), (1, 1) => Some(2), _ => None };
        assert_eq!(read_box(0, 0, 0x55, word, sub, &plays), BoxOpen::Give(15));
        // The talk part (a choice) plays; a command the handler has no part for isn't read.
        assert_eq!(read_box(0, 0, 0x56, word, sub, &plays), BoxOpen::Not(88));
        assert_eq!(read_box(0, 0, 0x37, word, sub, &plays), BoxOpen::Unread);
        // A part that starts an event entry plays; one that gives nothing isn't a box.
        let mut c0b = c0.clone();
        c0b[14] = ins(CLASS_CALL, 126);
        let ctxs = [c0b, c1.clone()];
        let word = |c: usize, i: usize| ctxs.get(c).and_then(|v| v.get(i)).copied();
        assert_eq!(read_box(0, 0, 0x55, word, sub, &plays), BoxOpen::Not(126));
        let mut c0c = c0.clone();
        c0c[14] = ins(CLASS_CALL, 459);
        let ctxs = [c0c, c1];
        let word = |c: usize, i: usize| ctxs.get(c).and_then(|v| v.get(i)).copied();
        assert_eq!(read_box(0, 0, 0x55, word, sub, &plays), BoxOpen::Not(0));
    }

    #[test]
    fn reads_a_follow_up() {
        let plays = [88u32, 222];
        // Context 0 carries on after `push 1, call 126`: an item, a message, a wake of context 7's
        // sub-script 1 (messages only), then event mode off.
        let c0 = vec![
            ins(CLASS_PUSH, 1), ins(CLASS_CALL, 126),
            ins(CLASS_PUSH, 2), ins(CLASS_CALL, 458), ins(CLASS_PUSH, 7), ins(CLASS_CALL, 1),
            ins(CLASS_PUSH, 5), ins(CLASS_CTX, 1), ins(CLASS_WAKE, 1),
            ins(CLASS_CALL, 514), ins(CLASS_CALL, 512), ins(CLASS_END, 0x10),
        ];
        // Context 1: sub-script 0 has a choice (88), sub-script 1 only lines.
        let c1 = vec![ins(CLASS_CALL, 88), ins(CLASS_END, 0x10), ins(CLASS_CALL, 1), ins(CLASS_CALL, 107), ins(CLASS_END, 0x10)];
        let ctxs = [c0.clone(), c1.clone()];
        let word = |c: usize, i: usize| ctxs.get(c).and_then(|v| v.get(i)).copied();
        let sub = |c: usize, s: usize| match (c, s) { (0, 0) => Some(0), (1, 0) => Some(0), (1, 1) => Some(2), _ => None };
        assert!(started_entry(2, 1, |i| c0.get(i).copied()));
        assert!(!started_entry(2, 3, |i| c0.get(i).copied()));
        assert_eq!(read_follow_up(0, 2, word, sub, &plays), Follow::Skip(11));
        // Waking sub-script 0 (the choice) makes it play.
        let mut c0b = c0.clone();
        c0b[8] = ins(CLASS_WAKE, 0);
        let ctxs = [c0b, c1.clone()];
        let word = |c: usize, i: usize| ctxs.get(c).and_then(|v| v.get(i)).copied();
        assert_eq!(read_follow_up(0, 2, word, sub, &plays), Follow::Plays(88));
        // No 514 or 512 before the sub-script ends: nothing to end the skip at.
        let c0c = vec![ins(CLASS_PUSH, 7), ins(CLASS_CALL, 1), ins(CLASS_END, 0x10)];
        let ctxs = [c0c, c1];
        let word = |c: usize, i: usize| ctxs.get(c).and_then(|v| v.get(i)).copied();
        assert_eq!(read_follow_up(0, 0, word, sub, &plays), Follow::NoEnd);
        // Unreadable code.
        assert_eq!(read_follow_up(0, 0, |_, _| None, sub, &plays), Follow::Unread);
    }
}
