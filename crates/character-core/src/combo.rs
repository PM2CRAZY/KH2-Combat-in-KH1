//! Combo-tree rows (KH2 PTYA) and the rule that picks the next attack.
//!
//! The selection rules below were read from KH2 Final Mix's executable and
//! are described in plain language in `docs/movement-model.md`
//! ("Combo selection"). In short: the first row in table order whose
//! condition and range checks pass wins, with one anti-repeat swap.

/// One attack choice.
#[derive(Clone, Debug, PartialEq)]
pub struct ComboRow {
    pub id: u8,
    /// 0: only reachable by id (fallbacks). 1: range-checked against the
    /// primary target. 2: range-checked against every target. 3: condition
    /// checks only.
    pub kind: u8,
    /// Row id to use instead when this row's motion is already playing.
    pub sub: i8,
    /// Added to the combo count when testing a finisher row.
    pub combo_offset: i8,
    pub flag: u32,
    pub motion: u16,
    /// Motion to queue afterwards (4 = fall for air rows, 0 = idle).
    pub next_motion: u16,
    /// Hop height. > 0 makes the attack a jump.
    pub jump: f32,
    pub jump_min: f32,
    pub jump_max: f32,
    pub speed_min: f32,
    pub speed_max: f32,
    pub near: f32,
    pub far: f32,
    /// Height window, in the original's y-down convention (see [`select`]).
    pub low: f32,
    pub high: f32,
    /// Window on dot(facing, direction to target).
    pub inner_min: f32,
    pub inner_max: f32,
    pub blend_time: f32,
    pub distance_adjust: f32,
    /// Required ability item id (0 = none).
    pub ability: u16,
    /// Present in the table but not read by the selector.
    pub score: u16,
}

/// Row only valid while airborne (otherwise only on the ground).
pub const FLAG_AIR: u32 = 0x1;
/// Jump height window is raised by the character's jump height.
pub const FLAG_JUMP_ADJUST: u32 = 0x2;
pub const FLAG_FINISHER: u32 = 0x4;
/// Lower bound of the height window shrinks by the target's height.
pub const FLAG_LOW_SUB: u32 = 0x8;
/// Upper bound of the height window grows by the target's height.
pub const FLAG_HIGH_ADD: u32 = 0x10;

impl ComboRow {
    pub fn is_finisher(&self) -> bool {
        self.flag & FLAG_FINISHER != 0
    }

    /// Valid only in the air.
    pub fn is_air(&self) -> bool {
        self.flag & FLAG_AIR != 0
    }

    /// Starts a jump (hop) when used.
    pub fn is_jump(&self) -> bool {
        self.jump > 0.0
    }

    pub fn needs_target(&self) -> bool {
        matches!(self.kind, 1 | 2)
    }
}

/// A possible target, relative to the attacker.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetGeom {
    pub id: u32,
    /// Horizontal distance between centres.
    pub distance: f32,
    /// Target feet y minus attacker feet y (y up).
    pub dy: f32,
    /// Target collision radius and height.
    pub radius: f32,
    pub height: f32,
    /// dot(attacker facing, horizontal direction to target).
    pub facing_dot: f32,
}

pub struct Situation<'a> {
    pub air: bool,
    /// Current combo count and finisher count (after this press's update).
    pub count: u8,
    pub finishers: u8,
    /// Combo length for the current state (ground or air).
    pub max_count: u8,
    /// The motion currently playing; used for the anti-repeat swap.
    pub current_motion: u16,
    pub primary: Option<TargetGeom>,
    pub all: &'a [TargetGeom],
    pub equipped: &'a [u16],
    /// Character's jump height (for [`FLAG_JUMP_ADJUST`]).
    pub jump_height: f32,
    /// The press chains on from a finisher (Steam exe: the character's `+0x194` bit 5, which the
    /// count step `0x1403D6CA0` takes as the finisher's chain, count and finisher count up to the
    /// most finishers): finisher rows pass whatever the count, normal rows only while one
    /// finisher has run (`0x1403D7780` / `0x1403D7850`).
    pub finisher_window: bool,
    /// Berserk Charge equipped (ability number 107, its bit in `+0x5C0` block `+0x1DC` bit 11) and
    /// MP charging (`0x1403C1D10`: the status's MP Charge time `+0x1C0` above 0): no finisher
    /// row passes, every normal row does (exe, the same two tests).
    pub berserk: bool,
    /// On the ground, the floor gives out ahead of him (the host's test of KH2's line ahead, exe
    /// `0x140402D20` -> `0x1403B7BD0` / `0x1403BA750`): row 1 is marked, whatever its conditions.
    pub drop_ahead: bool,
}

/// Berserk Charge's ability number (03system item 0x18B; PTYA rows and the equipped bitset name
/// abilities by it).
pub const BERSERK_CHARGE: u16 = 107;
/// The row KH2 marks when the floor gives out ahead (`Situation::drop_ahead`).
pub const DROP_AHEAD_ROW: u8 = 1;

/// Row-level checks that don't depend on a target (exe `0x140402750`: the ability, ground or
/// air, then the finisher test `0x1403D7780` or the normal test `0x1403D7850`).
pub fn condition(r: &ComboRow, s: &Situation) -> bool {
    if r.motion == 0 || (r.ability != 0 && !s.equipped.contains(&r.ability)) {
        return false;
    }
    if r.is_air() != s.air {
        return false;
    }
    if s.berserk {
        return !r.is_finisher();
    }
    if r.is_finisher() {
        s.finisher_window || s.count as i32 + r.combo_offset as i32 >= s.max_count as i32
    } else {
        // Refused only past the combo's length (or chaining on from a finisher) once more than one
        // finisher has run.
        (!s.finisher_window && s.count < s.max_count) || s.finishers <= 1
    }
}

/// Range checks against one target. The gap is the distance less the target's radius, never
/// below 0 (Steam exe `0x1404029F0`, Checked): a target closer than its own radius is at gap 0,
/// so rows with `near` 0 still take it.
pub fn in_range(r: &ComboRow, t: &TargetGeom, jump_height: f32) -> bool {
    let d = (t.distance - t.radius).max(0.0);
    if !(r.near <= d && d < r.far) {
        return false;
    }
    // y-down, as the original: valid when high' < dy <= low'.
    let dy = -t.dy;
    let mut low = r.low;
    let mut high = r.high;
    if r.flag & FLAG_JUMP_ADJUST != 0 {
        high -= jump_height;
    }
    if r.flag & FLAG_LOW_SUB != 0 { low -= t.height } else { low += t.height }
    if r.flag & FLAG_HIGH_ADD != 0 { high += t.height } else { high -= t.height }
    if !(high < dy && dy <= low) {
        return false;
    }
    r.inner_min < t.facing_dot && t.facing_dot <= r.inner_max
}

/// What [`select`] chose: the row and the target its move is aimed at (the
/// primary target; kind 1 only passes against it, kind 2 against any target).
#[derive(Clone, Copy, Debug)]
pub struct Choice<'a> {
    pub row: &'a ComboRow,
    pub target: Option<TargetGeom>,
}

/// Picks the next attack (exe): every row that passes is marked, then the rows are walked in
/// table order and the first one whose motion is not the one playing is taken (or whose `sub`
/// is itself). A row whose motion is already playing is passed over; the first such row's
/// `sub` row is kept as the answer for when nothing else passes. With nothing at all, row 9
/// (air) or 5 (ground) is marked too and the walk is done again.
///
/// (Before b48 the first passing row was taken and swapped for its `sub` when its motion was
/// playing; a row with no `sub`, such as Explosion, then repeated on every press. The exe walks
/// on instead, which is where KH2's mix of finishers and modifiers comes from.)
pub fn select<'a>(rows: &'a [ComboRow], s: &Situation) -> Option<Choice<'a>> {
    let by_id = |id: u8| rows.iter().find(|r| r.id == id);
    let passes = |r: &'a ComboRow| -> Option<Choice<'a>> {
        if !condition(r, s) {
            return None;
        }
        let target = match r.kind {
            1 => match s.primary {
                Some(t) if in_range(r, &t, s.jump_height) => Some(t),
                _ => return None,
            },
            // The kind-2 pass (`0x1404029F0`) only marks the row: whichever target passed it, the
            // move's hop and lunge are measured to the primary target (`+0xDC8`, `0x1403D78D0`;
            // Steam KH2, Checked (decomp)).
            2 => {
                s.all.iter().find(|t| in_range(r, t, s.jump_height))?;
                s.primary
            }
            3 => s.primary,
            _ => return None,
        };
        Some(Choice { row: r, target })
    };
    let walk = |valid: &[Choice<'a>]| -> Option<Choice<'a>> {
        let mut alt: Option<Choice<'a>> = None;
        for c in valid {
            if c.row.sub as i32 == c.row.id as i32 || c.row.motion != s.current_motion {
                return Some(*c);
            }
            if alt.is_none() {
                if let Some(a) = u8::try_from(c.row.sub).ok().and_then(by_id) {
                    alt = Some(Choice { row: a, target: c.target });
                }
            }
        }
        alt
    };
    let mut valid: Vec<Choice<'a>> = rows.iter().filter_map(passes).collect();
    // The floor giving out ahead: row 1 joins the marked ones in its table place, unchecked (exe).
    if s.drop_ahead && !s.air {
        if let Some(r) = by_id(DROP_AHEAD_ROW).filter(|r| !valid.iter().any(|c| std::ptr::eq(c.row, *r))) {
            mark(rows, &mut valid, Choice { row: r, target: s.primary });
        }
    }
    if let Some(c) = walk(&valid) {
        return Some(c);
    }
    // Nothing: the default row joins the marked ones (in its table place) and the walk runs again.
    let Some(fallback) = by_id(if s.air { 9 } else { 5 }) else {
        // A table without the default rows (test fixtures): repeat the first passing row.
        return valid.first().copied();
    };
    mark(rows, &mut valid, Choice { row: fallback, target: s.primary });
    walk(&valid)
}

/// `c` marked too, in its table place among the marked rows (the exe's marks are bits by table
/// index, walked in table order).
fn mark<'a>(rows: &'a [ComboRow], valid: &mut Vec<Choice<'a>>, c: Choice<'a>) {
    let at = rows.iter().position(|r| std::ptr::eq(r, c.row)).unwrap_or(rows.len());
    let before = valid.iter().filter(|v| rows.iter().position(|r| std::ptr::eq(r, v.row)).unwrap_or(0) < at).count();
    valid.insert(before, c);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: u8, kind: u8, sub: i8, flag: u32, motion: u16, near: f32, far: f32) -> ComboRow {
        ComboRow {
            id, kind, sub, combo_offset: 0, flag, motion, next_motion: 0, jump: 0.0, jump_min: 0.0, jump_max: 0.0,
            speed_min: 0.0, speed_max: 0.0, near, far, low: 100.0, high: -300.0, inner_min: -2.0, inner_max: 2.0,
            blend_time: 0.0, distance_adjust: 0.0, ability: 0, score: 0,
        }
    }

    fn geom(d: f32) -> TargetGeom {
        TargetGeom { id: 1, distance: d, dy: 0.0, radius: 0.0, height: 50.0, facing_dot: 1.0 }
    }

    fn sit(count: u8, motion: u16, t: Option<TargetGeom>) -> Situation<'static> {
        Situation {
            air: false, count, finishers: 1, max_count: 3, current_motion: motion, primary: t, all: &[],
            equipped: &[], jump_height: 0.0, finisher_window: false, berserk: false, drop_ahead: false,
        }
    }

    #[test]
    fn berserk_charge_and_the_finishers_chain() {
        let rows = vec![row(1, 3, 1, FLAG_FINISHER, 155, 0.0, 0.0), row(5, 3, 5, 0, 151, 0.0, 0.0)];
        // At the combo's end: the finisher.
        assert_eq!(select(&rows, &sit(3, 0, None)).unwrap().row.motion, 155);
        // Berserk Charge while MP charges: no finisher, the swing goes on.
        let s = Situation { berserk: true, ..sit(3, 0, None) };
        assert_eq!(select(&rows, &s).unwrap().row.motion, 151);
        // Chaining on from a finisher: a finisher at any count; a normal row once two have run.
        let s = Situation { finisher_window: true, ..sit(1, 0, None) };
        assert_eq!(select(&rows, &s).unwrap().row.motion, 155);
        let s = Situation { finishers: 2, ..sit(1, 0, None) };
        assert!(condition(&rows[1], &s), "under the length a normal row passes, finishers or not");
        let s = Situation { finishers: 2, finisher_window: true, ..sit(1, 0, None) };
        assert!(!condition(&rows[1], &s));
    }

    /// A kind-2 row passes on any target, and its move is aimed at the primary target (KH2's
    /// kind-2 pass only marks the row; the tester, combo row selection m23 answer 1 (a)).
    #[test]
    fn group_row_passes_on_any_target_and_aims_at_the_primary() {
        let rows = vec![row(26, 2, -1, FLAG_FINISHER, 166, 0.0, 300.0), row(3, 3, 3, FLAG_FINISHER, 155, 0.0, 0.0)];
        let primary = TargetGeom { id: 7, ..geom(900.0) };
        let beside = [primary, TargetGeom { id: 9, ..geom(100.0) }];
        let s = Situation { all: &beside, ..sit(3, 0, Some(primary)) };
        let c = select(&rows, &s).unwrap();
        assert_eq!(c.row.id, 26);
        assert_eq!(c.target.map(|t| t.id), Some(7));
        // Only the primary target there (out of reach): the normal finisher.
        let alone = [primary];
        let s = Situation { all: &alone, ..sit(3, 0, Some(primary)) };
        assert_eq!(select(&rows, &s).unwrap().row.id, 3);
    }

    #[test]
    fn drop_ahead_marks_row_one() {
        let mut hop = row(1, 0, -1, 0, 181, 0.0, 0.0);
        hop.jump = 21.0;
        let rows = vec![row(3, 3, 3, FLAG_FINISHER, 155, 0.0, 0.0), hop, row(5, 3, 5, 0, 151, 0.0, 0.0)];
        assert_eq!(select(&rows, &sit(1, 0, None)).unwrap().row.motion, 151);
        let s = Situation { drop_ahead: true, ..sit(1, 0, None) };
        assert_eq!(select(&rows, &s).unwrap().row.id, 1);
        // Not in the air.
        let s = Situation { drop_ahead: true, air: true, ..sit(1, 0, None) };
        assert!(select(&rows, &s).map_or(true, |c| c.row.id != 1));
    }

    #[test]
    fn table_order_finishers_and_alternation() {
        let rows = vec![
            row(1, 1, 1, FLAG_FINISHER, 155, 0.0, 200.0),
            row(2, 1, 3, 0, 151, 0.0, 200.0),
            row(3, 1, 2, 0, 153, 0.0, 200.0),
            row(5, 3, 5, 0, 150, 0.0, 0.0),
        ];
        // in range, count 1: first non-finisher row
        assert_eq!(select(&rows, &sit(1, 0, Some(geom(100.0)))).unwrap().row.motion, 151);
        // same motion already playing: swap to sub row 3
        assert_eq!(select(&rows, &sit(2, 151, Some(geom(100.0)))).unwrap().row.motion, 153);
        // count reached max: finisher row is first in the table
        assert_eq!(select(&rows, &sit(3, 153, Some(geom(100.0)))).unwrap().row.motion, 155);
        // out of range: condition-only row
        assert_eq!(select(&rows, &sit(1, 0, Some(geom(900.0)))).unwrap().row.motion, 150);
        assert_eq!(select(&rows, &sit(1, 0, None)).unwrap().row.motion, 150);
    }

    #[test]
    fn gap_never_below_zero() {
        // Inside the target's radius: gap 0, so a row starting at 0 takes it.
        let r = row(1, 1, 1, 0, 1, 0.0, 200.0);
        let t = TargetGeom { radius: 80.0, ..geom(30.0) };
        assert!(in_range(&r, &t, 0.0));
    }

    #[test]
    fn height_and_facing_windows() {
        let r = row(1, 1, 1, 0, 1, 0.0, 200.0);
        let mut t = geom(50.0);
        assert!(in_range(&r, &t, 0.0));
        t.dy = 400.0; // far above: dy(y-down) = -400 <= high' (-350)
        assert!(!in_range(&r, &t, 0.0));
        t.dy = 0.0;
        t.facing_dot = -1.0;
        let mut front = r.clone();
        front.inner_min = 0.5;
        assert!(!in_range(&front, &t, 0.0));
    }
}
