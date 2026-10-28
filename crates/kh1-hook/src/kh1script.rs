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

/// Event functions 0 and 177 open a message window (the window's number pushed right before the call);
/// window 7 is the one the "Obtained" lines use (the treasure script's, the evidence boxes', Destiny
/// Islands' gathered items). Checked (exe + files).
pub const FN_OPEN_WINDOW: u32 = 0;
pub const FN_OPEN_WINDOW_AT: u32 = 177;
pub const WINDOW_OBTAINED: u32 = 7;

/// t795: a room script's part that a Triangle command woke through KH1's event dispatch (`0x1401A1A10`:
/// event 0 wakes the object's context's sub-script 5, event 4 its sub-script 3), read as an item pickup:
/// the whole sub-script from `start` to its first 512 (or 514), with every sub-script it wakes. A pickup
/// takes the player (511), gives (258 / 583 / 458 / 444 / 570) and gives him back (512) with no scene,
/// choice, shop or menu (`plays`), and every window its own sub-script opens is window 7 (the "Obtained"
/// one: a talk opens its own windows). Destiny Islands' gathered items read `511, 113, 115, wake (Sora's
/// motion), 2, 644, 258, 364, window 7 (4 / 5 / 6 / 83 / 80 / 3, 0, 1), 353 0x1F, 107, 2, wake, 112, 114,
/// 512` (Checked (files): `di01`).
pub fn read_pickup(ctx: usize, start: usize, word: impl Fn(usize, usize) -> Option<u32>, sub: impl Fn(usize, usize) -> Option<usize>, plays: &[u32]) -> BoxOpen {
    let op = |w: u32| (w >> 24) as u8;
    let arg = |w: u32| w & 0xFF_FFFF;
    // The windows its own sub-script opens, up to its 512 / 514 or its end.
    let mut last: Option<u32> = None;
    let mut i = start;
    loop {
        if i - start >= FOLLOW_MAX {
            return BoxOpen::Unread;
        }
        let Some(w) = word(ctx, i) else { return BoxOpen::Unread };
        i += 1;
        match op(w) {
            CLASS_END => break,
            CLASS_CALL => {
                let f = arg(w);
                if f == FN_OPEN_WINDOW || f == FN_OPEN_WINDOW_AT {
                    match last {
                        Some(w) if op(w) == CLASS_PUSH && arg(w) == WINDOW_OBTAINED => {}
                        _ => return BoxOpen::Not(f),
                    }
                }
                if f == FN_RELEASE || f == FN_EVENT_OFF {
                    break;
                }
            }
            _ => {}
        }
        last = Some(w);
    }
    let mut seen = Vec::new();
    match walk_follow_up(ctx, start, &word, &sub, plays, &mut seen) {
        Follow::Skip(n) if seen.contains(&FN_HOLD) && seen.contains(&FN_RELEASE) && seen.iter().any(|f| TAIL_GIVES.contains(f)) => BoxOpen::Give(n),
        Follow::Skip(_) | Follow::NoEnd => BoxOpen::Not(0),
        Follow::Plays(f) => BoxOpen::Not(f),
        Follow::Unread => BoxOpen::Unread,
    }
}

/// t793: what's left of a scene after its event mode ends (514) while the player is still held: from
/// context `ctx`'s instruction `pc` to its 512 (the player given back), with every sub-script it wakes.
/// `Some(n)` when it gives something (258 / 583 an item or reward, 458 a spell, 444 munny, 570 a Gummi
/// Block) and has no choice, shop, menu or pause skip (`plays`): the scene's give and its "Obtained" window
/// (the Coliseum's Entry Pass after Phil's scene: 514, then 258, window 7, 353, 107, 2, 512).
pub fn read_give_tail(ctx: usize, pc: usize, word: impl Fn(usize, usize) -> Option<u32>, sub: impl Fn(usize, usize) -> Option<usize>, plays: &[u32]) -> Option<usize> {
    let mut seen = Vec::new();
    match walk_follow_up(ctx, pc, &word, &sub, plays, &mut seen) {
        Follow::Skip(n) if seen.contains(&FN_RELEASE) && !seen.contains(&FN_EVENT_OFF) && seen.iter().any(|f| TAIL_GIVES.contains(f)) => Some(n),
        _ => None,
    }
}

/// The gives a held tail can have (t793).
pub const TAIL_GIVES: [u32; 5] = [258, 583, 458, 444, 570];

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

/// t786: the "Obtained" window after a skipped scene, as KH1's treasure script shows it (`exchange/
/// <lang>_treasure.ev`, read in the owner's files, nothing of it copied here): window 7 set up with event
/// functions 3 / 4 / 5 / 6 / 83 / 80, then per line 365 (window, text, x, y: placed at (-220, -150)), 0
/// (open), 1 (the line), 107 (wait while it shows) and 2 (close). Its "Obtained" texts are: `08`, `0A 00 00
/// 00`, `07 0C 00`, the words, a colour `0C 04`, an icon, the item, `0C FF`, a full stop (0x68) and `06 52 00`
/// (the window closes itself after 0x52 frames), ended by 0. Checked (files).
pub const CLASS_OP: u8 = 1;
pub const OP_NEGATE: u32 = 5;
const WINDOW: u32 = 7;
/// t790: event function 353 plays a sound (`push id, push 0, call 353`; `0x1401C8F40` -> `0x1401787D0`, not
/// in skip mode). Sound 0x1F is the one the room scripts play with a give: next to the item message (605)
/// 112 times, next to 258 25 times and to 570 13 times across the scripts read, and in the Blizzard scene
/// after its "learned" line (`aw02`). Checked (files + decomp).
pub const FN_SOUND: u32 = 353;
pub const SOUND_GET: u32 = 0x1F;

/// One "Obtained" line in the treasure script's form: `name` in KH1's text codes (no end byte), `icon`
/// the glyph before it (the treasure texts use 0x82 / 0x83 / 0x84 / 0x89 for items, 0x86 for Sora's
/// abilities), if any.
pub fn obtained_text(name: &[u8], icon: Option<u8>) -> Vec<u8> {
    let mut t = vec![0x08, 0x0A, 0x00, 0x00, 0x00, 0x07, 0x0C, 0x00];
    // "Obtained " in KH1's codes (0x2B.. A-Z, 0x45.. a-z, 1 a space).
    t.extend("Obtained ".bytes().map(|c| match c {
        b'A'..=b'Z' => 0x2B + (c - b'A'),
        b'a'..=b'z' => 0x45 + (c - b'a'),
        _ => 0x01,
    }));
    t.extend_from_slice(&[0x0C, 0x04]);
    if let Some(i) = icon {
        t.push(i);
    }
    t.extend(name.iter().copied().filter(|&b| b != 0 && b != 0x04));
    t.extend_from_slice(&[0x0C, 0xFF, 0x68, 0x06, 0x52, 0x00, 0x00]);
    t
}

/// A script file in KH1's form (the layout `0x1401A43A0` reads: no variables, one entry, the texts at the
/// offset in word 3) that shows `texts` one after another in window 7, as the treasure script does, and
/// ends: entry 0, one context, sub-script 0 sets the window up (run at once by the start), sub-script 1
/// shows the lines. No hold on the player (no 511 / 512, 113..115).
pub fn obtained_script(texts: &[Vec<u8>]) -> Vec<u8> {
    let push = |a: u32| (CLASS_PUSH as u32) << 24 | (a & 0xFF_FFFF);
    let call = |f: u32| (CLASS_CALL as u32) << 24 | f;
    let neg = (CLASS_OP as u32) << 24 | OP_NEGATE;
    let end = (CLASS_END as u32) << 24 | 0x10;
    let settings = |c: &mut Vec<u32>| {
        c.extend([push(WINDOW), push(0), push(1), call(4), push(WINDOW), push(1), call(5), push(WINDOW), push(0), call(6)]);
        c.extend([push(WINDOW), push(0), call(83), push(WINDOW), push(0), call(80)]);
    };
    let mut s0 = vec![push(WINDOW), push(0x73), neg, push(0x87), neg, call(3)];
    settings(&mut s0);
    s0.push(end);
    let mut s1 = Vec::new();
    for k in 0..texts.len() as u32 {
        settings(&mut s1);
        s1.extend([push(WINDOW), push(k), push(0xDC), neg, push(0x96), neg, call(365)]);
        s1.extend([push(SOUND_GET), push(0), call(FN_SOUND)]);
        s1.extend([push(WINDOW), call(0), push(WINDOW), push(k), call(1), push(WINDOW), call(107), push(WINDOW), call(2)]);
    }
    s1.push(end);
    // Header: 0 variables, 0, 1 entry, the texts' offset, entry 0's offset.
    let mut f: Vec<u8> = Vec::new();
    let word = |f: &mut Vec<u8>, w: u32| f.extend_from_slice(&w.to_le_bytes());
    for _ in 0..5 {
        word(&mut f, 0);
    }
    let entry = f.len() as u32;
    f.extend_from_slice(b"OBT\0");
    word(&mut f, 1);
    word(&mut f, 1);
    f.extend_from_slice(&[1, 2, 0, 0, 0]);
    for w in s0.iter().chain(s1.iter()) {
        word(&mut f, *w);
    }
    let text_at = f.len() as u32;
    word(&mut f, texts.len() as u32);
    for t in texts {
        f.extend_from_slice(t);
    }
    f[12..16].copy_from_slice(&text_at.to_le_bytes());
    f[16..20].copy_from_slice(&entry.to_le_bytes());
    f
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
    fn reads_a_pickup() {
        let plays = [88u32, 126];
        // Context 0's sub-script 3 (woken by the Triangle command): a guard, hold, wake context 1's
        // sub-script 1 (Sora's motion), give item 0xC0, window 7 with its line, give the player back.
        let c0 = vec![
            ins(0x10, 0x958), ins(CLASS_PUSH, 0), ins(CLASS_CMP, 6), ins(CLASS_JUMP_NOT, 0x15),
            ins(CLASS_CALL, 511), ins(CLASS_CALL, 113), ins(CLASS_PUSH, 6), ins(CLASS_CTX, 1), ins(CLASS_WAKE, 1),
            ins(CLASS_PUSH, 0xC0), ins(CLASS_PUSH, 1), ins(CLASS_CALL, 258),
            ins(CLASS_PUSH, 7), ins(CLASS_CALL, 0), ins(CLASS_PUSH, 7), ins(CLASS_PUSH, 0x1EF), ins(CLASS_CALL, 1),
            ins(CLASS_PUSH, 7), ins(CLASS_CALL, 107), ins(CLASS_CALL, 112), ins(CLASS_CALL, 512), ins(CLASS_END, 0x10),
        ];
        let c1 = vec![ins(CLASS_CALL, 55), ins(CLASS_END, 0x10), ins(CLASS_PUSH, 0xA6), ins(CLASS_CALL, 108), ins(CLASS_CALL, 103), ins(CLASS_END, 0x10)];
        let ctxs = [c0.clone(), c1.clone()];
        let word = |c: usize, i: usize| ctxs.get(c).and_then(|v| v.get(i)).copied();
        let sub = |c: usize, s: usize| match (c, s) { (0, 3) => Some(0), (1, 0) => Some(0), (1, 1) => Some(2), _ => None };
        assert_eq!(read_pickup(0, 0, word, sub, &plays), BoxOpen::Give(25));
        // A talk's window (window 0) isn't a pickup's.
        let mut c0b = c0.clone();
        c0b[12] = ins(CLASS_PUSH, 0);
        let ctxs = [c0b, c1.clone()];
        let word = |c: usize, i: usize| ctxs.get(c).and_then(|v| v.get(i)).copied();
        assert_eq!(read_pickup(0, 0, word, sub, &plays), BoxOpen::Not(0));
        // A choice plays; a part that gives nothing, or never takes the player, isn't a pickup.
        let mut c0c = c0.clone();
        c0c[18] = ins(CLASS_CALL, 88);
        let ctxs = [c0c, c1.clone()];
        let word = |c: usize, i: usize| ctxs.get(c).and_then(|v| v.get(i)).copied();
        assert_eq!(read_pickup(0, 0, word, sub, &plays), BoxOpen::Not(88));
        let mut c0d = c0.clone();
        c0d[11] = ins(CLASS_CALL, 459);
        let ctxs = [c0d, c1.clone()];
        let word = |c: usize, i: usize| ctxs.get(c).and_then(|v| v.get(i)).copied();
        assert_eq!(read_pickup(0, 0, word, sub, &plays), BoxOpen::Not(0));
        let mut c0e = c0.clone();
        c0e[4] = ins(CLASS_CALL, 459);
        let ctxs = [c0e, c1];
        let word = |c: usize, i: usize| ctxs.get(c).and_then(|v| v.get(i)).copied();
        assert_eq!(read_pickup(0, 0, word, sub, &plays), BoxOpen::Not(0));
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

    #[test]
    fn builds_an_obtained_script() {
        let texts = vec![obtained_text(&[0x3B, 0x52, 0x57], Some(0x82)), obtained_text(&[0x3F], None)];
        let f = obtained_script(&texts);
        let w = |o: usize| u32::from_le_bytes(f[o..o + 4].try_into().unwrap());
        // As `0x1401A43A0` reads it: entry 0's record at word (n1 + n0 + 1 + 0) + 3, its context count at
        // +0xC, the context's sub-script count at +0xD and its code at +0x11.
        let (n0, n1) = (w(0) as usize, w(4) as usize);
        let e = w((n1 + n0 + 1 + 3) * 4) as usize;
        assert_eq!(f[e + 0xC], 1);
        assert_eq!(f[e + 0xD], 2);
        let code = e + 0x11;
        let mut i = code;
        let mut subs = 0;
        let mut calls = Vec::new();
        while subs < 2 {
            let ins = w(i);
            if (ins >> 24) as u8 == CLASS_END {
                subs += 1;
            } else if (ins >> 24) as u8 == CLASS_CALL {
                calls.push(ins & 0xFF_FFFF);
            }
            i += 4;
        }
        // The texts follow the code: their count, then each one.
        let t = w(12) as usize;
        assert_eq!(t, i);
        assert_eq!(w(t), 2);
        assert_eq!(&f[t + 4..t + 4 + texts[0].len()], &texts[0][..]);
        // Each line: 365, open, line, wait, close.
        assert_eq!(calls.iter().filter(|&&c| c == 365).count(), 2);
        assert_eq!(calls.iter().filter(|&&c| c == 107).count(), 2);
        assert!(!calls.contains(&511) && !calls.contains(&512) && !calls.contains(&113));
        // The give's sound with each line.
        assert_eq!(calls.iter().filter(|&&c| c == FN_SOUND).count(), 2);
        // "Obtained" as the treasure texts spell it; a text ends at its 0 after `06 52 00`.
        assert_eq!(&texts[0][8..17], &[0x39, 0x46, 0x58, 0x45, 0x4D, 0x52, 0x49, 0x48, 0x01]);
        assert_eq!(&texts[0][texts[0].len() - 4..], &[0x06, 0x52, 0x00, 0x00]);
    }

    #[test]
    fn reads_a_give_tail() {
        let plays = [88u32, 139];
        // After 514: a wait, the give (258), the window, 512.
        let c0 = vec![
            ins(CLASS_PUSH, 10), ins(CLASS_CALL, 8), ins(CLASS_PUSH, 0xE5), ins(CLASS_PUSH, 1), ins(CLASS_CALL, 258),
            ins(CLASS_PUSH, 7), ins(CLASS_CALL, 0), ins(CLASS_PUSH, 7), ins(CLASS_CALL, 107), ins(CLASS_CALL, 512), ins(CLASS_END, 0x10),
        ];
        let ctxs = [c0.clone()];
        let word = |c: usize, i: usize| ctxs.get(c).and_then(|v| v.get(i)).copied();
        let sub = |_: usize, _: usize| None;
        assert_eq!(read_give_tail(0, 0, word, sub, &plays), Some(10));
        // No give: nothing to do. A choice: it plays. No 512: nothing.
        let none = [vec![ins(CLASS_PUSH, 7), ins(CLASS_CALL, 0), ins(CLASS_CALL, 512), ins(CLASS_END, 0x10)]];
        assert_eq!(read_give_tail(0, 0, |c: usize, i: usize| none.get(c).and_then(|v| v.get(i)).copied(), sub, &plays), None);
        let choice = [vec![ins(CLASS_CALL, 258), ins(CLASS_CALL, 88), ins(CLASS_CALL, 512), ins(CLASS_END, 0x10)]];
        assert_eq!(read_give_tail(0, 0, |c: usize, i: usize| choice.get(c).and_then(|v| v.get(i)).copied(), sub, &plays), None);
        let open = [vec![ins(CLASS_CALL, 258), ins(CLASS_END, 0x10)]];
        assert_eq!(read_give_tail(0, 0, |c: usize, i: usize| open.get(c).and_then(|v| v.get(i)).copied(), sub, &plays), None);
    }
}
