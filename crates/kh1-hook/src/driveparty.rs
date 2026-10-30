//! Which partners a Drive Form takes, and whether it can start (the owner, the main-branch port's
//! round 7, row 11: answers 1 (a), 2 (a); "make a toggle in drives for parties being required to go
//! into drive forms"). Notes: `docs/kh1-drive-forms-native.md` §18.
//!
//! KH2 (Steam KH2, Checked (decomp)): the same rule picks whom a form takes (`0x140400B20`) and
//! whether it can be chosen (`0x140400740`, `0x1404005E0`), slot by slot over the two party slots:
//! Valor (1) needs Goofy, Wisdom (2) Donald, Limit (3) no one, Master, Final and Anti (4..6) whoever is
//! in each slot. A needed slot that is empty: the form can't be chosen (2); a needed partner who
//! can't act (KO): greyed (1); no partner needed at all but the form isn't Limit: can't (2).
//!
//! KH1 (Checked (decomp)): the party slots are `save + 0x48E..` (Sora, then two; 0xFF none), each a
//! member id; the members are the save's records, Sora 0, Donald 1, Goofy 2 (KH1's weapon kind
//! `2 + member`: Donald's staffs kind 3, Goofy's shields kind 4), guests after them.
//!
//! Pure Rust, unit tested; the game side is `win::driveparty`.

pub const DONALD: u8 = 1;
pub const GOOFY: u8 = 2;
/// KH2's form ids (`formown::form_id`).
pub const VALOR: u8 = 1;
pub const WISDOM: u8 = 2;
pub const LIMIT: u8 = 3;

/// Whether form `form` takes the member `member` in a party slot.
pub fn takes(form: u8, member: u8) -> bool {
    match form {
        VALOR => member == GOOFY,
        WISDOM => member == DONALD,
        LIMIT => false,
        4..=6 => true,
        _ => false,
    }
}

/// Can form `form` start with these two slots (`None` empty, else the member and whether he can act)?
/// 0 yes, 1 a partner it needs is down, 2 a partner it needs isn't there (KH2's `0x140400740`).
pub fn availability(form: u8, slots: [Option<(u8, bool)>; 2]) -> u8 {
    if form == LIMIT {
        return 0;
    }
    let mut found = 0;
    let mut out = 0;
    for slot in slots {
        let member = slot.map(|s| s.0);
        let needed = match member {
            Some(m) => takes(form, m),
            // (KH2: an empty slot counts as member 0, which only Master, Final and Anti take.)
            None => matches!(form, 4..=6),
        };
        if !needed {
            continue;
        }
        match slot {
            None => return 2,
            Some((_, able)) => {
                if !able {
                    out = 1;
                }
                found += 1;
            }
        }
    }
    if found == 0 {
        2
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kh2s_rule() {
        let both = [Some((DONALD, true)), Some((GOOFY, true))];
        for f in 1..=6 {
            assert_eq!(availability(f, both), 0, "form {f}");
        }
        // Valor without Goofy (a guest in his place), Wisdom without Donald.
        let guest = [Some((DONALD, true)), Some((5, true))];
        assert_eq!((availability(VALOR, guest), availability(WISDOM, guest), availability(4, guest)), (2, 0, 0));
        // A slot empty: Master can't; Limit always can.
        assert_eq!((availability(4, [Some((DONALD, true)), None]), availability(LIMIT, [None, None])), (2, 0));
        // Goofy down: Valor greyed.
        assert_eq!(availability(VALOR, [Some((DONALD, true)), Some((GOOFY, false))]), 1);
        assert!(takes(4, 5) && !takes(VALOR, DONALD) && !takes(LIMIT, GOOFY));
    }
}
