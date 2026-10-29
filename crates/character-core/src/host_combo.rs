//! KH2's Attack press on a host's own combo state (the KH1 port: KH2's row walk inside KH1's own
//! row pickers, on KH1's own count, length, finisher count, target and lock). The same rules as
//! [`Character`](crate::Character)'s press (`plan_press`): Steam KH2's Attack count step
//! `0x1403D6CA0` and its row walk `0x140402D20` (`combo::select`), but every input is given by the
//! host as it has it, and nothing is kept between presses: no controller, no copy of the host's
//! state. The hop a row starts is not worked out here.
//!
//! What the host gives (`HostPress`): KH2's row table and which of its motions there are clips of
//! (KH2's data), the host's position, facing, primary target (its own target for the press) and
//! every target it would measure (each with a point and a size), the abilities equipped, the jump
//! height KH2's flag 2 reads, the combo's length and most finishers, and the motion playing; for
//! a row's hop (`HostPress::hop`), what `HopIn` holds.

use crate::combo::{condition, in_range, select, ComboRow, Situation, TargetGeom, FLAG_JUMP_ADJUST};
use crate::{Launch, KH2_GRAVITY};

/// A target as the host has it: its point (y up) and its size (radius; height for the height
/// window).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HostTarget {
    pub id: u32,
    pub pos: [f32; 3],
    pub radius: f32,
    pub height: f32,
}

/// One Attack press on the host's state (see the module docs).
pub struct HostPress<'a> {
    /// KH2's row table of the moveset now (base Sora's or a Drive Form's), in table order.
    pub rows: &'a [ComboRow],
    /// KH2 has a clip of this motion in the moveset (a row with none is no attack, as KH2's).
    pub has_motion: &'a dyn Fn(u16) -> bool,
    pub air: bool,
    /// The host's position (feet, y up) and facing (yaw, as `Character::facing`).
    pub pos: [f32; 3],
    pub facing: f32,
    /// The host's own target for this press (KH2's primary target `+0xDC8`: kind-1 rows are
    /// measured against it, the selector faces it).
    pub primary: Option<HostTarget>,
    /// Every target the host measures (KH2's kind-2 pass runs on each).
    pub targets: &'a [HostTarget],
    pub equipped: &'a [u16],
    /// KH2's jump height (row flag 2 raises the height window by it).
    pub jump_height: f32,
    /// The combo's length here (ground or air; Combo Plus, Air Combo Plus, Negative Combo in it).
    pub max_count: u8,
    /// The most finishers in a row (KH2's `[+0x5C0] + 0x202`: one plus Finishing Plus's copies).
    pub max_finishers: u8,
    /// MP charging with Berserk Charge equipped: no finisher row, every step goes on.
    pub berserk: bool,
    /// On the ground, the floor gives out ahead (KH2 marks row 1).
    pub drop_ahead: bool,
    /// The motion of the host's attack playing (KH2's anti-repeat passes over its rows).
    pub playing: Option<u16>,
}

fn len2(v: [f32; 2]) -> f32 {
    v[0].hypot(v[1])
}

fn norm(v: [f32; 2]) -> [f32; 2] {
    let l = len2(v);
    if l > 0.0 {
        [v[0] / l, v[1] / l]
    } else {
        [0.0, 0.0]
    }
}

impl HostPress<'_> {
    /// The selector's direction (Steam KH2 `0x1404028D0`): flat, to the primary target, or the
    /// host's facing with none. x / z.
    pub fn aim(&self) -> [f32; 2] {
        self.primary
            .map(|t| [t.pos[0] - self.pos[0], t.pos[2] - self.pos[2]])
            .filter(|d| len2(*d) > 1e-3)
            .map(norm)
            .unwrap_or_else(|| [self.facing.sin(), self.facing.cos()])
    }

    /// A target as KH2's row checks measure it (as `Character`'s: its distance, its height over
    /// the host's feet, its size, and how far off the selector's direction it is).
    fn geom(&self, t: &HostTarget, aim: [f32; 2]) -> TargetGeom {
        let d = [t.pos[0] - self.pos[0], t.pos[2] - self.pos[2]];
        let n = norm(d);
        TargetGeom {
            id: t.id,
            distance: len2(d),
            dy: t.pos[1] - self.pos[1],
            radius: t.radius,
            height: t.height,
            facing_dot: if len2(d) > 1e-3 { (n[0] * aim[0] + n[1] * aim[1]).min(1.0) } else { 1.0 },
        }
    }

    /// Every target and the primary one, as the row checks measure them (the primary one dead
    /// ahead: the selector faces it).
    fn geoms(&self) -> (Vec<TargetGeom>, Option<TargetGeom>) {
        let aim = self.aim();
        let all = self.targets.iter().map(|t| self.geom(t, aim)).collect();
        let primary = self.primary.map(|t| TargetGeom { facing_dot: 1.0, ..self.geom(&t, aim) });
        (all, primary)
    }

    fn situation<'s>(&'s self, all: &'s [TargetGeom], primary: Option<TargetGeom>, count: u8, finishers: u8, finisher_window: bool) -> Situation<'s> {
        Situation {
            air: self.air,
            count,
            finishers,
            max_count: self.max_count,
            current_motion: self.playing.unwrap_or(u16::MAX),
            primary,
            all,
            equipped: self.equipped,
            jump_height: self.jump_height,
            finisher_window,
            berserk: self.berserk,
            drop_ahead: self.drop_ahead && !self.air,
        }
    }

    /// The walk's row, if KH2 has a clip of it (else no attack, as KH2's press).
    fn walk(&self, count: u8, finishers: u8, finisher_window: bool) -> Option<ComboRow> {
        let (all, primary) = self.geoms();
        let sit = self.situation(&all, primary, count, finishers, finisher_window);
        let c = select(self.rows, &sit)?;
        (self.has_motion)(c.row.motion).then(|| c.row.clone())
    }

    /// An Attack press in the combo (KH2's combo window with the swing connected, or a new combo):
    /// `count` is the host's count with this press counted (1 for a new combo). The row KH2 takes,
    /// or none (KH2 refuses the press).
    pub fn press(&self, count: u8) -> Option<ComboRow> {
        self.walk(count.max(1), 1, false)
    }

    /// An Attack press in a finisher's Finishing Plus window (`0x1403D6CA0`'s first branch: the
    /// swing connected and fewer than the most finishers so far): `finishers` is the finisher count
    /// before this press (KH2's `+0xD75`, at least 1), `count` the host's count before it. KH2's
    /// next finisher, or none (none left: no attack until the finisher ends, as KH2's).
    ///
    /// With none left (or no count to chain on from) KH2's count step starts a new combo (counts 1
    /// / 1, `0x101`), as `Character`'s press does from a finisher that isn't playing; its row is
    /// taken only if it is a finisher (the combo's length 1, Negative Combo), as before.
    pub fn next_finisher(&self, count: u8, finishers: u8) -> Option<ComboRow> {
        let finishers = finishers.max(1);
        let row = if count > 0 && finishers < self.max_finishers {
            self.walk(count.saturating_add(1), finishers + 1, true)
        } else {
            self.walk(1, 1, false)
        };
        row.filter(|r| r.is_finisher())
    }

    /// Whether `next_finisher` chains on (a finisher left): KH2's `+0xD75` under the most.
    pub fn finisher_left(&self, finishers: u8) -> bool {
        finishers.max(1) < self.max_finishers
    }

    /// A row's range checks alone (kind 1 the primary target, kind 2 any target, kind 3 none).
    fn range_of(r: &ComboRow, all: &[TargetGeom], primary: Option<TargetGeom>, jump_height: f32) -> bool {
        match r.kind {
            1 => primary.is_some_and(|t| in_range(r, &t, jump_height)),
            2 => all.iter().any(|t| in_range(r, t, jump_height)),
            _ => true,
        }
    }

    /// Whether row `id`'s range checks alone pass now (as `Character::row_in_range`): false with
    /// no such row or a kind-0 row.
    pub fn row_in_range(&self, id: u8) -> bool {
        let Some(r) = self.rows.iter().find(|r| r.id == id) else { return false };
        if !matches!(r.kind, 1..=3) {
            return false;
        }
        let (all, primary) = self.geoms();
        Self::range_of(r, &all, primary, self.jump_height)
    }

    /// One of KH2's ability rows by id as an extra candidate of the host's own picker (as
    /// `Character::ability_row_id`): its ability equipped, its kind (`finisher`), ground or air, a
    /// row the selector picks (kind 1..3), not the motion playing, playable by the host
    /// (`usable`), KH2's range checks, and a clip of it.
    pub fn ability_row_id(&self, id: u8, finisher: bool, usable: &dyn Fn(&ComboRow) -> bool) -> Option<ComboRow> {
        let r = self.rows.iter().find(|r| r.id == id)?;
        let (all, primary) = self.geoms();
        let count = if finisher { self.max_count.max(1) } else { 1 };
        let sit = self.situation(&all, primary, count, 1, false);
        let pass = r.ability != 0
            && matches!(r.kind, 1..=3)
            && r.is_finisher() == finisher
            && condition(r, &sit)
            && Some(r.motion) != self.playing
            && usable(r)
            && Self::range_of(r, &all, primary, self.jump_height)
            && (self.has_motion)(r.motion);
        pass.then(|| r.clone())
    }

    /// The first of KH2's ability rows in table order that passes `ability_row_id` (as
    /// `Character::ability_row`).
    pub fn ability_row(&self, finisher: bool, usable: &dyn Fn(&ComboRow) -> bool) -> Option<ComboRow> {
        self.rows.iter().filter(|r| r.ability != 0).find_map(|r| self.ability_row_id(r.id, finisher, usable))
    }

    /// KH2's normal finisher (no ability) for the host's next finisher (as
    /// `Character::normal_finisher`): KH2's walk over its finisher rows that need no ability, in
    /// table order: the first that passes and whose motion isn't the one playing (or whose `sub`
    /// is its own id); with none, the `sub` row of the first passing one whose motion is playing.
    pub fn normal_finisher(&self) -> Option<ComboRow> {
        let (all, primary) = self.geoms();
        let sit = Situation { berserk: false, drop_ahead: false, ..self.situation(&all, primary, self.max_count.max(1), 1, false) };
        let mut fallback: Option<u8> = None;
        for r in self.rows.iter().filter(|r| r.ability == 0 && matches!(r.kind, 1..=3) && r.is_finisher()) {
            if !condition(r, &sit) || !Self::range_of(r, &all, primary, self.jump_height) {
                continue;
            }
            if r.sub == r.id as i8 || Some(r.motion) != self.playing {
                return (self.has_motion)(r.motion).then(|| r.clone());
            }
            if fallback.is_none() && r.sub >= 0 {
                fallback = Some(r.sub as u8);
            }
        }
        let id = fallback?;
        self.rows.iter().find(|r| r.id == id).cloned()
    }
}

/// What a row's hop needs besides the press (`HostPress::hop`, Steam KH2 `0x1403D78D0`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HopIn {
    /// KH2's own target shape: its centre's height over his feet and its radius (the plyr
    /// record's, `AttackParams::target_lift` / `target_radius`).
    pub target_lift: f32,
    pub target_radius: f32,
    /// The moveset's gravity (the launch's).
    pub gravity: f32,
    /// The stick (x / z, scaled by its tilt): with no target, a combo's first press faces it.
    pub stick: [f32; 2],
    /// The press starts a combo (KH2's count from 0: nothing carried), so with no target the stick
    /// turns him first.
    pub first: bool,
    /// The row's motion moves as he moves (an allow-movement range: Wisdom's air shots): the hop
    /// keeps `vel_h`, his sideways speed now, instead of the row's.
    pub moving: bool,
    pub vel_h: [f32; 2],
}

impl HostPress<'_> {
    /// The hop row `row` starts at this press (as `Character`'s press: Steam KH2 `0x1403D78D0`,
    /// Checked (decomp)): measured to the primary target, from his own target shape's centre to
    /// the target's point: the height between the two within `jump_min .. jump_max` (plus the jump
    /// height with flag 2); the sideways speed the gap (distance less both radii, plus
    /// `distance_adjust`) over KH2's rise time for that height (KH2's own gravity constant), within
    /// `speed_min .. speed_max`, toward the target. With no target: the row's `jump` and
    /// `speed_max` along his facing (the stick's on a combo's first press). `None` for a row with no
    /// hop.
    pub fn hop(&self, row: &ComboRow, h: &HopIn) -> Option<Launch> {
        if !row.is_jump() {
            return None;
        }
        let (_, primary) = self.geoms();
        let mut dir = [self.facing.sin(), self.facing.cos()];
        if primary.is_none() && h.first && len2(h.stick) > 0.0 {
            dir = norm(h.stick);
        }
        let (height, speed, dir) = match (primary, self.primary) {
            (Some(t), Some(p)) => {
                let extra = if row.flag & FLAG_JUMP_ADJUST != 0 { self.jump_height } else { 0.0 };
                let bound = |v: f32, lo: f32, hi: f32| if !(lo <= v) { lo } else if v <= hi { v } else { hi };
                let height = bound(t.dy - h.target_lift, row.jump_min, row.jump_max + extra);
                let gap = t.distance - (t.radius + h.target_radius) + row.distance_adjust;
                let up = (2.0 * height / KH2_GRAVITY).abs().sqrt();
                let speed = bound(gap / up, row.speed_min, row.speed_max);
                let to = [p.pos[0] - self.pos[0], p.pos[2] - self.pos[2]];
                (height, speed, if len2(to) > 1e-6 { norm(to) } else { dir })
            }
            _ => (row.jump, row.speed_max, dir),
        };
        let velocity = if h.moving { h.vel_h } else { [dir[0] * speed, dir[1] * speed] };
        Some(Launch { height: height.max(0.0), velocity, gravity: h.gravity })
    }

    /// One row by id as a state's own move forces it (Counterguard, Retaliating Slash, the guard:
    /// `Character::would_take_row`, the exe's own forced rows such as `0x1403D6030`): a new combo's
    /// press (counts 1 / 1), the row's conditions, a Square move only with a swing left, and a clip
    /// of it. `square`: it is one of KH2's Square moves (Finishing Leap, Upper Slash, Horizontal
    /// Slash) that isn't a form's Square finisher.
    pub fn forced(&self, id: u8, square: bool) -> Option<ComboRow> {
        let r = self.rows.iter().find(|r| r.id == id)?;
        let (all, primary) = self.geoms();
        let sit = self.situation(&all, primary, 1, 1, false);
        let ok = condition(r, &sit) && (!square || r.is_finisher() || sit.count < sit.max_count) && (self.has_motion)(r.motion);
        ok.then(|| r.clone())
    }

    /// KH2's Square moves in a combo as the exe reaches them (`Character::would_take_square_exe`:
    /// Steam KH2's action-button handler `0x1403A81F0` tries rows 0x25, 0x0C, 0x26 through
    /// `0x1403A9950`, Checked (decomp)): `count` / `finishers` the combo before the press, `hit`
    /// the swing connected (or Combo Master), `combo` / `finisher` KH2's combo / finisher window
    /// open, `free` KH2's free state, `skip` row ids left out. The row and whether the press starts
    /// a combo (KH2's count step went to a free state: `HopIn::first`).
    pub fn square(&self, count: u8, finishers: u8, hit: bool, combo: bool, finisher: bool, free: bool, skip: &[u8]) -> Option<(ComboRow, bool)> {
        const ORDER: [u8; 3] = [0x25, 0x0C, 0x26];
        if !(combo || finisher) {
            return None;
        }
        let max = self.max_count;
        let fin = finishers.max(1);
        let fresh = if finisher && hit && fin < self.max_finishers {
            false
        } else if combo && hit {
            false
        } else if free {
            true
        } else {
            return None;
        };
        let gate = |r: &ComboRow| -> bool {
            if r.motion == 0 || (r.ability != 0 && !self.equipped.contains(&r.ability)) || r.is_air() != self.air {
                return false;
            }
            if r.is_finisher() {
                !self.berserk && (finisher || count as i32 + r.combo_offset as i32 >= max as i32)
            } else {
                self.berserk || (!finisher && count < max)
            }
        };
        ORDER.into_iter().filter(|id| !skip.contains(id)).find_map(|id| {
            let r = self.rows.iter().find(|r| r.id == id)?;
            (gate(r) && (self.has_motion)(r.motion)).then(|| (r.clone(), fresh))
        })
    }
}
