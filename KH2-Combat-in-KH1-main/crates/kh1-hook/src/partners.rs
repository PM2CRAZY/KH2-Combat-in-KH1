//! The Drive forms' partners (the user: "Let's make drive forms require their specific party
//! members to cast. For example, wisdom form needs donald alive to activate in KH2"). In KH2 a form
//! draws on its partners: Valor on Goofy, Wisdom on Donald, Master and Final on both; Limit (KH2
//! FM) on neither, the partners staying by Sora's side. Anti Form is the secret code's and needs no
//! one. Here a partner counts when he is in KH1's field party now (one of the two actors after
//! Sora) and not knocked out; how far away he is doesn't matter (KH2 doesn't ask). A world guest
//! (Tarzan, Aladdin, Ariel, Jack, Peter Pan, Beast) in his place doesn't stand in for him.
//!
//! Who an actor is: KH1's party member id, its object's `+0x4C` (`win::form`, `docs/drive-model.md`,
//! "Partners"), the same number as the save's party records (0 Sora, 1 Donald, 2 Goofy, then the
//! guests).

use crate::story::Form;

/// KH1's party member ids (the save's party records, `docs/kh1-equipment.md`).
pub const SORA: u16 = 0;
pub const DONALD: u16 = 1;
pub const GOOFY: u16 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Partner {
    Donald,
    Goofy,
}

impl Partner {
    pub fn member(self) -> u16 {
        match self {
            Partner::Donald => DONALD,
            Partner::Goofy => GOOFY,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Partner::Donald => "Donald",
            Partner::Goofy => "Goofy",
        }
    }
}

/// The partners form `form` needs (Anti, not a Drive list form, needs none).
pub fn needs(form: Form) -> &'static [Partner] {
    match form {
        Form::Valor => &[Partner::Goofy],
        Form::Wisdom => &[Partner::Donald],
        Form::Master | Form::Final => &[Partner::Donald, Partner::Goofy],
        Form::Limit => &[],
    }
}

/// The partners a form takes while it lasts (KH2: they vanish in it, back when it ends): those it
/// needs (`needs`); Anti Form (`None`, not a Drive list form) both, as KH2's Anti. Only who is
/// there goes (`win::vanish` finds them by member id: a guest never vanishes).
pub fn taken(form: Option<Form>) -> &'static [Partner] {
    match form {
        Some(f) => needs(f),
        None => &[Partner::Donald, Partner::Goofy],
    }
}

/// One of the two actors after Sora in KH1's field party: his member id and whether he is down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Member {
    pub id: u16,
    pub down: bool,
}

/// A partner's state now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    Here,
    KnockedOut,
    /// Not in the field party (a guest in his place, or no one).
    Away,
}

/// Where `p` is in the field party `party` (the slots after Sora's; `None` an empty slot).
pub fn state(party: &[Option<Member>], p: Partner) -> State {
    match party.iter().flatten().find(|m| m.id == p.member()) {
        Some(m) if m.down => State::KnockedOut,
        Some(_) => State::Here,
        None => State::Away,
    }
}

/// Whether `form` (named `name`) may start with the field party `party`, and why not.
pub fn check(form: Form, name: &str, party: &[Option<Member>]) -> Result<(), String> {
    let need = needs(form);
    let missing: Vec<String> = need
        .iter()
        .filter_map(|&p| match state(party, p) {
            State::Here => None,
            State::KnockedOut => Some(format!("{} is knocked out", p.name())),
            State::Away => Some(format!("{} isn't in the party", p.name())),
        })
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    let names: Vec<&str> = need.iter().map(|p| p.name()).collect();
    Err(format!("{name} needs {}: {}", names.join(" and "), missing.join(", ")))
}

/// A member's name for the log (the guests by number: their order isn't checked).
pub fn member_name(id: u16) -> String {
    match id {
        SORA => "Sora".into(),
        DONALD => "Donald".into(),
        GOOFY => "Goofy".into(),
        n => format!("guest {n}"),
    }
}

/// The vanish's decisions for one partner on one frame (`win::vanish`; `docs/drive-model.md`,
/// "Partners vanish in the form"), kept apart from the game's memory for the tests.
///
/// The bits are KH1's actor flags word (`+0`): `ACTS` (its actor step runs him), `MODEL` and
/// `SHADOW` (drawn), `NOT_FOE` and `UNTOUCHABLE`. KH1's actor reset (`0x1402904D0`: its actor
/// script's "place at a point" command, its "reset every actor" command, the partners' placement
/// and catch-up warp) keeps only `RESET_KEEPS` of the word and turns `ACTS` on: our away bits
/// `NOT_FOE | UNTOUCHABLE` go, while the model and shadow bits we turned off stay off (s333).
pub mod vanish {
    pub const ACTS: u32 = 0x8000;
    pub const MODEL: u32 = 0x1;
    pub const SHADOW: u32 = 0x2;
    pub const NOT_FOE: u32 = 0x200;
    pub const UNTOUCHABLE: u32 = 0x40;
    /// On while here, off while away.
    pub const HERE_BITS: u32 = ACTS | MODEL | SHADOW;
    /// On while away.
    pub const AWAY_BITS: u32 = NOT_FOE | UNTOUCHABLE;
    pub const MASK: u32 = HERE_BITS | AWAY_BITS;
    /// What KH1's actor reset keeps of the flags word (it then sets `ACTS`).
    pub const RESET_KEEPS: u32 = 0x0100_4083;

    /// His actor now: its flags word, its flag stack's depth (KH1's game over pushes it) and KH1's
    /// own leaving mark (`+0x374` bit 28, its script op `0xCF`).
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Seen {
        pub flags: u32,
        pub depth: u32,
        pub left: bool,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Act {
        Nothing,
        /// Take him (our away bits on his actor now).
        Hide,
        /// Away already: kept so (his model, shadow and HUD entry hidden again if KH1 shows them).
        Keep,
        /// Back: acting, model and shadow on, on his actor now.
        Restore,
    }

    /// Our hide is still on this very actor: we hid it and KH1 hasn't given it `ACTS` back since
    /// (a reset of it, or the pop of its game over). `ours_actor` the actor we hid.
    pub fn held(ours_actor: Option<usize>, actor: usize, flags: u32) -> bool {
        ours_actor == Some(actor) && flags & ACTS == 0
    }

    /// Whether a hide of ours may still be on him (`owed`: we turned his model and shadow off
    /// and no one has turned his model on since), from what his actor shows now. This is the
    /// test for "we hid him" against "KH1 hides him": KH1's reset keeps the model bit as it was,
    /// so an actor of his found acting but undrawn after a hide of ours is still ours to show;
    /// one KH1 hid itself, never hidden by us since, isn't. Our exact away bits (acting, model
    /// and shadow off, both away bits on, no game over's stack) are ours whatever we remember.
    pub fn owed(owed: bool, held: bool, s: Seen) -> bool {
        if s.left {
            return false;
        }
        if held {
            return true;
        }
        if s.flags & MODEL != 0 {
            // (Drawn: KH1 showed him itself, or a new actor came in whole.)
            return false;
        }
        owed || (s.depth == 0 && s.flags & MASK == AWAY_BITS)
    }

    /// The frame's act. `away`: the form takes him and KH1's script doesn't have Sora.
    pub fn decide(away: bool, held: bool, owed: bool, s: Seen) -> Act {
        if s.left {
            return Act::Nothing;
        }
        if !away {
            return if held || owed { Act::Restore } else { Act::Nothing };
        }
        if held {
            return Act::Keep;
        }
        // (KH1's game over holds his flags: its own.)
        if s.depth != 0 {
            return Act::Nothing;
        }
        // Acting and drawn: taken. Acting but undrawn after a hide of ours (KH1 reset him): taken
        // again. Undrawn by KH1 itself (never ours): left to KH1 until it shows him.
        if s.flags & (ACTS | MODEL) == ACTS | MODEL || (owed && (s.flags & ACTS != 0 || s.flags & MASK == AWAY_BITS)) {
            return Act::Hide;
        }
        Act::Nothing
    }

    /// His flags taken.
    pub fn hidden(flags: u32) -> u32 {
        (flags & !HERE_BITS) | AWAY_BITS
    }

    /// His flags back: acting, model and shadow on whatever was saved. The away bits: as saved
    /// when our hide is still on this actor (`saved`); else cleared if he shows exactly ours, else
    /// as KH1 left them (its reset clears them itself).
    pub fn restored(flags: u32, saved: Option<u32>) -> u32 {
        let away = match saved {
            Some(was) => was & AWAY_BITS,
            None if flags & MASK == AWAY_BITS => 0,
            None => flags & AWAY_BITS,
        };
        (flags & !MASK) | away | HERE_BITS
    }

    /// KH1's actor reset on a flags word (for the tests: what it does to a partner we hid).
    pub fn kh1_reset(flags: u32) -> u32 {
        (flags & RESET_KEEPS) | ACTS
    }

    /// The bits that matter here, for the log: `+` on, `-` off.
    pub fn describe(flags: u32) -> String {
        let bit = |b: u32, name: &str| format!("{}{name}", if flags & b != 0 { '+' } else { '-' });
        [bit(ACTS, "acts"), bit(MODEL, "model"), bit(SHADOW, "shadow"), bit(NOT_FOE, "notfoe"), bit(UNTOUCHABLE, "untouchable")].join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Form; 5] = [Form::Valor, Form::Wisdom, Form::Limit, Form::Master, Form::Final];

    fn here(id: u16) -> Option<Member> {
        Some(Member { id, down: false })
    }
    fn down(id: u16) -> Option<Member> {
        Some(Member { id, down: true })
    }
    fn ok(form: Form, party: &[Option<Member>]) -> bool {
        check(form, "Form", party).is_ok()
    }

    #[test]
    fn the_table() {
        assert_eq!(needs(Form::Valor), &[Partner::Goofy]);
        assert_eq!(needs(Form::Wisdom), &[Partner::Donald]);
        assert_eq!(needs(Form::Limit), &[]);
        assert_eq!(needs(Form::Master), &[Partner::Donald, Partner::Goofy]);
        assert_eq!(needs(Form::Final), &[Partner::Donald, Partner::Goofy]);
    }

    #[test]
    fn who_vanishes() {
        assert_eq!(taken(Some(Form::Valor)), &[Partner::Goofy]);
        assert_eq!(taken(Some(Form::Wisdom)), &[Partner::Donald]);
        assert_eq!(taken(Some(Form::Limit)), &[]);
        assert_eq!(taken(Some(Form::Master)), &[Partner::Donald, Partner::Goofy]);
        assert_eq!(taken(Some(Form::Final)), &[Partner::Donald, Partner::Goofy]);
        // Anti Form.
        assert_eq!(taken(None), &[Partner::Donald, Partner::Goofy]);
    }

    #[test]
    fn the_full_party_opens_every_form() {
        for f in ALL {
            assert!(ok(f, &[here(DONALD), here(GOOFY)]), "{f:?}");
            assert!(ok(f, &[here(GOOFY), here(DONALD)]), "{f:?}: slot order doesn't matter");
        }
    }

    #[test]
    fn a_guest_doesnt_stand_in() {
        // A guest for Goofy (Deep Jungle's Tarzan).
        let p = [here(DONALD), here(3)];
        assert!(!ok(Form::Valor, &p));
        assert!(ok(Form::Wisdom, &p));
        assert!(ok(Form::Limit, &p));
        assert!(!ok(Form::Master, &p));
        assert!(!ok(Form::Final, &p));
        // Donald out for a guest.
        let p = [here(4), here(GOOFY)];
        assert!(ok(Form::Valor, &p));
        assert!(!ok(Form::Wisdom, &p));
        assert!(!ok(Form::Master, &p));
        assert!(!ok(Form::Final, &p));
    }

    #[test]
    fn knocked_out_counts_as_gone() {
        let p = [down(DONALD), here(GOOFY)];
        assert!(!ok(Form::Wisdom, &p));
        assert!(ok(Form::Valor, &p));
        assert!(!ok(Form::Master, &p));
        assert!(ok(Form::Limit, &p));
        let p = [here(DONALD), down(GOOFY)];
        assert!(!ok(Form::Valor, &p));
        assert!(ok(Form::Wisdom, &p));
        assert!(!ok(Form::Final, &p));
    }

    #[test]
    fn alone_only_limit() {
        for p in [[None, None], [down(DONALD), down(GOOFY)]] {
            for f in ALL {
                assert_eq!(ok(f, &p), f == Form::Limit, "{f:?}");
            }
        }
    }

    #[test]
    fn the_reason_names_the_partner() {
        assert_eq!(check(Form::Wisdom, "Wisdom Form", &[None, here(GOOFY)]), Err("Wisdom Form needs Donald: Donald isn't in the party".into()));
        assert_eq!(check(Form::Valor, "Valor Form", &[here(DONALD), down(GOOFY)]), Err("Valor Form needs Goofy: Goofy is knocked out".into()));
        assert_eq!(
            check(Form::Master, "Master Form", &[down(DONALD), here(5)]),
            Err("Master Form needs Donald and Goofy: Donald is knocked out, Goofy isn't in the party".into())
        );
    }

    mod vanishing {
        use crate::partners::vanish::*;

        /// A partner in the field, acting and drawn (the s333 log's flags for a party member).
        const HERE: u32 = 0x0000_c003;
        const A: usize = 0x7000_0000;
        const B: usize = 0x7000_04b0;

        fn seen(flags: u32) -> Seen {
            Seen { flags, depth: 0, left: false }
        }

        /// One frame as `vanish::frame` runs it: owed updated, then the act applied to the flags.
        /// Returns the new flags, the act, and the new (ours_actor, owed).
        fn step(away: bool, ours: Option<usize>, owed_was: bool, actor: usize, s: Seen) -> (u32, Act, Option<usize>, bool) {
            let h = held(ours, actor, s.flags);
            let o = owed(owed_was, h, s);
            let act = decide(away, h, o, s);
            match act {
                Act::Hide | Act::Keep => (hidden(s.flags), act, Some(actor), true),
                Act::Restore => (restored(s.flags, if h { Some(0) } else { None }), act, None, false),
                Act::Nothing => (s.flags, act, if h { ours } else { None }, o),
            }
        }

        #[test]
        fn taken_and_back() {
            let (f, act, ours, owed) = step(true, None, false, A, seen(HERE));
            assert_eq!(act, Act::Hide);
            assert_eq!(f & MASK, AWAY_BITS);
            let (f2, act, ours, owed) = step(true, ours, owed, A, seen(f));
            assert_eq!(act, Act::Keep);
            assert_eq!(f2, f);
            let (f3, act, ours, owed) = step(false, ours, owed, A, seen(f2));
            assert_eq!(act, Act::Restore);
            assert_eq!(f3, HERE);
            assert_eq!((ours, owed), (None, false));
            // Nothing more to do once he is back.
            assert_eq!(step(false, ours, owed, A, seen(f3)).1, Act::Nothing);
        }

        /// The s333 bug: KH1 reset him (a Coliseum round's placing) while away. The model and
        /// shadow stay off, acting comes back, our away bits go.
        #[test]
        fn a_kh1_reset_while_away_is_still_ours() {
            let (f, _, ours, owed) = step(true, None, false, A, seen(HERE));
            let reset = kh1_reset(f);
            assert_eq!(reset & MASK, ACTS, "acting, undrawn, a foe and touchable: {}", describe(reset));
            // The form goes on: taken again.
            let (f2, act, ours2, owed2) = step(true, ours, owed, A, seen(reset));
            assert_eq!(act, Act::Hide);
            assert_eq!(f2 & MASK, AWAY_BITS);
            // The revert: shown.
            let (f3, act, _, owed3) = step(false, ours2, owed2, A, seen(kh1_reset(f2)));
            assert_eq!(act, Act::Restore);
            assert_eq!(f3 & MASK, HERE_BITS);
            assert!(!owed3);
            // Reset during a scene (the form still on, KH1's script has Sora): shown for it.
            let (f4, act, _, _) = step(false, ours, owed, A, seen(reset));
            assert_eq!(act, Act::Restore);
            assert_eq!(f4 & MASK, HERE_BITS);
        }

        /// Before s333: the record dropped on the reset, and nothing showed him again. The bits
        /// the old safety net looked for (exactly our away bits) aren't what the reset leaves.
        #[test]
        fn the_old_net_missed_it() {
            let reset = kh1_reset(hidden(HERE));
            assert_ne!(reset & MASK, AWAY_BITS);
            // ...and the old re-take wanted acting and drawn.
            assert_ne!(reset & (ACTS | MODEL), ACTS | MODEL);
        }

        /// A new actor for him (another address): it came in whole, nothing of ours on it.
        #[test]
        fn a_new_actor_comes_in_whole() {
            let (_, _, ours, owed) = step(true, None, false, A, seen(HERE));
            // The form goes on: the new one is taken.
            let (f, act, ours2, _) = step(true, ours, owed, B, seen(HERE));
            assert_eq!(act, Act::Hide);
            assert_eq!(ours2, Some(B));
            assert_eq!(f & MASK, AWAY_BITS);
            // Out of the form: nothing to undo on it.
            let (f, act, _, owed) = step(false, ours, owed, B, seen(HERE));
            assert_eq!((f, act, owed), (HERE, Act::Nothing, false));
        }

        /// A new actor carrying our hidden bits (the old safety net's case) is shown.
        #[test]
        fn our_exact_bits_are_ours() {
            let ours_bits = hidden(HERE);
            let (f, act, _, _) = step(false, None, false, B, seen(ours_bits));
            assert_eq!(act, Act::Restore);
            assert_eq!(f, HERE);
        }

        /// KH1 hides him itself (a scene's "model off"), never hidden by us: left alone, in and
        /// out of a form; once KH1 shows him the form takes him.
        #[test]
        fn kh1s_own_hide_is_left_alone() {
            let kh1_hidden = HERE & !MODEL & !SHADOW;
            assert_eq!(step(false, None, false, A, seen(kh1_hidden)).1, Act::Nothing);
            assert_eq!(step(true, None, false, A, seen(kh1_hidden)).1, Act::Nothing);
            // Frozen by its game over (no `ACTS`): left too.
            assert_eq!(step(true, None, false, A, Seen { flags: HERE & !ACTS, depth: 1, left: false }).1, Act::Nothing);
            assert_eq!(step(true, None, false, A, seen(HERE)).1, Act::Hide);
        }

        /// After our return KH1 hides him for a scene: ours no more, not fought.
        #[test]
        fn shown_once_then_kh1s() {
            let (f, _, ours, owed) = step(true, None, false, A, seen(HERE));
            let (f, act, ours, owed) = step(false, ours, owed, A, seen(f));
            assert_eq!(act, Act::Restore);
            let kh1_hidden = f & !MODEL;
            assert_eq!(step(false, ours, owed, A, seen(kh1_hidden)).1, Act::Nothing);
        }

        /// KH1 shows him itself while a hide of ours was lost (a model-on command): ours no more.
        #[test]
        fn shown_by_kh1_clears_ours() {
            assert!(!owed(true, false, seen(HERE)));
            assert!(owed(true, false, seen(kh1_reset(hidden(HERE)))));
            assert!(owed(true, true, seen(hidden(HERE) | MODEL)));
        }

        /// KH1's own leaving (op 0xCF) is never touched.
        #[test]
        fn left_for_good() {
            let s = Seen { flags: hidden(HERE), depth: 0, left: true };
            assert_eq!(decide(false, true, true, s), Act::Nothing);
            assert_eq!(decide(true, false, true, s), Act::Nothing);
            assert!(!owed(true, true, s));
        }

        #[test]
        fn restored_bits() {
            // Saved away bits kept (a partner who was untouchable before).
            assert_eq!(restored(hidden(HERE), Some(UNTOUCHABLE)), HERE | UNTOUCHABLE);
            assert_eq!(restored(hidden(HERE), Some(0)), HERE);
            // No record: our exact bits cleared, KH1's left.
            assert_eq!(restored(hidden(HERE), None), HERE);
            assert_eq!(restored(ACTS | NOT_FOE | 0x4000, None), HERE | NOT_FOE);
            // Other bits untouched.
            assert_eq!(restored(0x0100_0000 | hidden(HERE), Some(0)), 0x0100_0000 | HERE);
        }

        #[test]
        fn the_log_names_the_bits() {
            assert_eq!(describe(HERE), "+acts +model +shadow -notfoe -untouchable");
            assert_eq!(describe(kh1_reset(hidden(HERE))), "+acts -model -shadow -notfoe -untouchable");
        }
    }
}
