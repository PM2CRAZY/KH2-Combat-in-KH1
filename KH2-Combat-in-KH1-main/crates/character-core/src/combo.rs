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
    /// Every target near (kind-2 rows), measured from the direction to the primary target.
    pub all: &'a [TargetGeom],
    /// The host's lock-on, if any (ours): a kind-2 row that needs no ability (the base
    /// finishers and swings) is measured against the locked target alone, so another foe beside
    /// it doesn't swap the locked one's finisher for the base one (the user, s206). Rows an
    /// ability brings (Vicinity Break, Magnet Burst, Explosion, Dodge Slash...) see every target
    /// near, as KH2's do: they are about the foes around him.
    pub lock: Option<u32>,
    pub equipped: &'a [u16],
    /// Character's jump height (for [`FLAG_JUMP_ADJUST`]).
    pub jump_height: f32,
}

/// Row-level checks that don't depend on a target.
pub fn condition(r: &ComboRow, s: &Situation) -> bool {
    if r.motion == 0 || (r.ability != 0 && !s.equipped.contains(&r.ability)) {
        return false;
    }
    if r.is_air() != s.air {
        return false;
    }
    if r.is_finisher() {
        s.count as i32 + r.combo_offset as i32 >= s.max_count as i32
    } else {
        s.finishers <= 1
    }
}

/// Range checks against one target.
pub fn in_range(r: &ComboRow, t: &TargetGeom, jump_height: f32) -> bool {
    let d = t.distance - t.radius;
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

/// The targets a kind-2 row is checked against (`Situation::lock`).
pub fn candidates<'s>(r: &'s ComboRow, s: &'s Situation) -> impl Iterator<Item = &'s TargetGeom> + 's {
    s.all.iter().filter(move |t| r.ability != 0 || s.lock.is_none_or(|id| t.id == id))
}

/// Whether a row passes its conditions and range checks, and the target it passed on.
pub fn passes(r: &ComboRow, s: &Situation) -> Option<Option<TargetGeom>> {
    if !condition(r, s) {
        return None;
    }
    match r.kind {
        1 => s.primary.filter(|t| in_range(r, t, s.jump_height)).map(Some),
        2 => candidates(r, s).find(|t| in_range(r, t, s.jump_height)).map(|t| Some(*t)),
        3 => Some(s.primary),
        _ => None,
    }
}

/// Why one target fails a row's range checks (`in_range`), in words; `None` when it passes.
pub fn range_miss(r: &ComboRow, t: &TargetGeom, jump_height: f32) -> Option<String> {
    let d = t.distance - t.radius;
    if !(r.near <= d && d < r.far) {
        return Some(format!("{d:.0} away (needs {:.0}..{:.0})", r.near, r.far));
    }
    let mut low = r.low;
    let mut high = r.high;
    if r.flag & FLAG_JUMP_ADJUST != 0 {
        high -= jump_height;
    }
    if r.flag & FLAG_LOW_SUB != 0 { low -= t.height } else { low += t.height }
    if r.flag & FLAG_HIGH_ADD != 0 { high += t.height } else { high -= t.height }
    if !(high < -t.dy && -t.dy <= low) {
        return Some(format!("{:.0} up (needs {:.0}..{:.0})", t.dy, -low, -high));
    }
    if !(r.inner_min < t.facing_dot && t.facing_dot <= r.inner_max) {
        let deg = |c: f32| c.clamp(-1.0, 1.0).acos().to_degrees();
        return Some(format!("{:.0} deg from the aim (needs {:.0}..{:.0})", deg(t.facing_dot), deg(r.inner_max), deg(r.inner_min)));
    }
    None
}

/// Why a row doesn't pass (`passes`), in words; `None` when it does.
pub fn miss(r: &ComboRow, s: &Situation) -> Option<String> {
    if r.motion == 0 {
        return Some("no motion".into());
    }
    if r.ability != 0 && !s.equipped.contains(&r.ability) {
        return Some(format!("ability {} not equipped", r.ability));
    }
    if r.is_air() != s.air {
        return Some(if r.is_air() { "an air row, on the ground".into() } else { "a ground row, in the air".into() });
    }
    if r.is_finisher() && (s.count as i32 + r.combo_offset as i32) < s.max_count as i32 {
        return Some(format!("a finisher, the combo is at {} of {}", s.count, s.max_count));
    }
    if !r.is_finisher() && s.finishers > 1 {
        return Some("not a finisher, the combo is in its finishers".into());
    }
    match r.kind {
        1 => match s.primary {
            None => Some("no primary target".into()),
            Some(t) => range_miss(r, &t, s.jump_height).map(|m| format!("primary target {m}")),
        },
        2 => {
            let seen: Vec<String> = candidates(r, s)
                .map(|t| range_miss(r, t, s.jump_height).map(|m| format!("#{:x} {m}", t.id)))
                .collect::<Option<Vec<_>>>()?;
            if seen.is_empty() {
                return Some(format!("no target ({} in all)", s.all.len()));
            }
            let more = if seen.len() > 4 { format!(" and {} more", seen.len() - 4) } else { String::new() };
            Some(format!("{} target(s), none fits: {}{more}", seen.len(), seen[..seen.len().min(4)].join("; ")))
        }
        3 => None,
        _ => Some("reached only by id".into()),
    }
}

/// What [`select`] chose: the row and, for range-checked rows, the target
/// it was checked against.
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
    let pass = |r: &'a ComboRow| -> Option<Choice<'a>> { passes(r, s).map(|target| Choice { row: r, target }) };
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
    let mut valid: Vec<Choice<'a>> = rows.iter().filter_map(pass).collect();
    if let Some(c) = walk(&valid) {
        return Some(c);
    }
    // Nothing: the default row joins the marked ones (in its table place) and the walk runs again.
    let Some(fallback) = by_id(if s.air { 9 } else { 5 }) else {
        // A table without the default rows (test fixtures): repeat the first passing row.
        return valid.first().copied();
    };
    let at = rows.iter().position(|r| std::ptr::eq(r, fallback)).unwrap_or(rows.len());
    let before = valid.iter().filter(|c| rows.iter().position(|r| std::ptr::eq(r, c.row)).unwrap_or(0) < at).count();
    valid.insert(before, Choice { row: fallback, target: s.primary });
    walk(&valid)
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
            lock: None, equipped: &[], jump_height: 0.0,
        }
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
