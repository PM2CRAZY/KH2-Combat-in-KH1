//! An action clip: how long a move lasts, how it moves the character
//! (root motion), and which windows and events it carries. Frames are 60 Hz.

/// What a window allows or does while it is open.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WindowKind {
    /// An attack hitbox is live. `group` picks the collision shapes.
    /// `weapon`: the shapes are the weapon model's (else the character's own).
    Hitbox { attack: u16, group: u8, combo: Option<i16>, weapon: u8 },
    /// The next combo hit (or another action) may start.
    Combo,
    /// Like `Combo` but only for magic.
    ComboMagic,
    /// Another finisher may start (Finishing Plus).
    Finisher,
    /// Rotate toward the locked/auto target, radians per frame.
    TurnToLock(f32),
    TurnToTarget(f32),
    /// Cannot be hit.
    Invulnerable,
    NoGravity,
    Air,
    Grounded,
    AllowMovement,
    KeepMomentum,
    AllowReactionCommand,
    WeaponTrail,
    /// Guard is active: incoming non-piercing hits from the front are blocked.
    /// The guard's shapes are the attack-group shapes of `group` (on the
    /// Keyblade if `weapon`), as the game builds them.
    Guard { attack: u16, group: u8, weapon: u8 },
    /// The object's collision group `n` is switched on / off (reaction
    /// command areas open this way).
    EnableGroup(u8),
    DisableGroup(u8),
    /// A range an object's script asks about by number (`n` of 1..3): the controller does
    /// nothing with it (KH2's Limit Form follow-up windows).
    ScriptRange(u8),
    /// A command offered on the reaction button while open (KH2's range 20: Limit Form's
    /// follow-ups that its motions carry). The controller does nothing with it.
    OfferCommand(u16),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Window {
    pub start: u32,
    /// `None` = until the clip ends.
    pub end: Option<u32>,
    pub kind: WindowKind,
}

impl Window {
    pub fn contains(&self, frame: u32) -> bool {
        frame >= self.start && self.end.map_or(true, |e| frame < e)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EventKind {
    /// The jump impulse is applied on this frame.
    Jump,
    MagicCast(i16),
    QuickRunCheck,
    FallCheck,
    RevengeCheck,
    /// Start the object's effect element `n`; an object with a script
    /// (Wisdom Form) runs its trigger 0x10 with it (frame trigger 1).
    Effect(i16),
    /// Play sound effect `bank * 1000 + index` (frame trigger 8, e.g. the Keyblade's swing:
    /// bank 500 stands for the equipped weapon's own bank).
    Sound(u32),
    /// Play the object's battle voice `n` (frame triggers 13 / 14: `00battle` VTBL id).
    Voice(i16),
    /// A footstep (frame trigger 2): its kind, the same numbering KH1's material sounds use
    /// (`0x1402939B0`): 0 run, 4 walk, 10 landing.
    Footstep(i16),
    Other(u8),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Event {
    pub frame: u32,
    pub kind: EventKind,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Clip {
    pub motion: u16,
    /// Length in 60 Hz frames.
    pub length: u32,
    /// Cumulative root translation at each frame (`length + 1` entries),
    /// in the character's local space: +Z forward, +Y up, +X right.
    pub root: Vec<[f32; 3]>,
    pub windows: Vec<Window>,
    pub events: Vec<Event>,
}

impl Clip {
    /// A clip with no root motion and no windows.
    pub fn plain(motion: u16, length: u32) -> Clip {
        Clip { motion, length, root: vec![[0.0; 3]; length as usize + 1], windows: Vec::new(), events: Vec::new() }
    }

    /// Root displacement from `frame` to `frame + 1`.
    pub fn root_delta(&self, frame: u32) -> [f32; 3] {
        let i = frame as usize;
        match (self.root.get(i), self.root.get(i + 1)) {
            (Some(a), Some(b)) => [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
            _ => [0.0; 3],
        }
    }

    pub fn open(&self, frame: u32) -> impl Iterator<Item = &Window> {
        self.windows.iter().filter(move |w| w.contains(frame))
    }

    pub fn has(&self, frame: u32, pred: impl Fn(&WindowKind) -> bool) -> bool {
        self.open(frame).any(|w| pred(&w.kind))
    }

    /// First frame at which a window matching `pred` opens.
    pub fn first(&self, pred: impl Fn(&WindowKind) -> bool) -> Option<u32> {
        self.windows.iter().filter(|w| pred(&w.kind)).map(|w| w.start).min()
    }

    pub fn event_at(&self, frame: u32, pred: impl Fn(&EventKind) -> bool) -> bool {
        self.events.iter().any(|e| e.frame == frame && pred(&e.kind))
    }

    /// Total root distance on the ground plane.
    pub fn root_distance(&self) -> f32 {
        let (Some(a), Some(b)) = (self.root.first(), self.root.last()) else { return 0.0 };
        ((b[0] - a[0]).powi(2) + (b[2] - a[2]).powi(2)).sqrt()
    }
}
