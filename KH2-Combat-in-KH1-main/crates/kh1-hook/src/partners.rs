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
}
