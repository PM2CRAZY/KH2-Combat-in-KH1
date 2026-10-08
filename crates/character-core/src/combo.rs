//! Combo-tree rows (KH2 PTYA) and the rule that picks the next attack.
//!
//! The row fields are data; how the original picks between them is not
//! documented. The selection below is a hypothesis built from patterns in
//! the data (see `docs/movement-model.md`, "Combo selection").

/// One attack choice.
#[derive(Clone, Debug, PartialEq)]
pub struct ComboRow {
    pub id: u8,
    /// 1 and 2: needs a target within `[near, far]` and the height window.
    /// 0 and 3: no target requirement.
    pub kind: u8,
    pub sub: i8,
    pub flag: u32,
    pub motion: u16,
    /// Motion to return to afterwards (4 = fall for air rows, 0 = idle).
    pub next_motion: u16,
    /// Hop height when this air attack starts.
    pub jump: f32,
    pub speed_min: f32,
    pub speed_max: f32,
    pub near: f32,
    pub far: f32,
    pub low: f32,
    pub high: f32,
    /// Required ability item id (0 = none).
    pub ability: u16,
    pub score: u16,
}

pub const FLAG_FINISHER: u32 = 0x4;

impl ComboRow {
    pub fn is_finisher(&self) -> bool {
        self.flag & FLAG_FINISHER != 0
    }

    /// Air rows give the character a hop; ground rows never do.
    pub fn is_air(&self) -> bool {
        self.jump > 0.0
    }

    pub fn needs_target(&self) -> bool {
        matches!(self.kind, 1 | 2)
    }
}

/// Where the target is relative to the attacker.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetGeom {
    /// Horizontal distance.
    pub distance: f32,
    /// target feet y - (attacker feet y + attack_y_offset).
    pub dy: f32,
}

pub struct Situation<'a> {
    pub air: bool,
    pub finisher: bool,
    pub target: Option<TargetGeom>,
    pub equipped: &'a [u16],
}

/// Picks the highest-scoring row that fits. On equal score a row whose
/// target requirement is met beats a no-requirement fallback; then the
/// earlier row wins.
pub fn select<'a>(rows: &'a [ComboRow], s: &Situation) -> Option<&'a ComboRow> {
    let mut best: Option<&ComboRow> = None;
    for r in rows {
        if r.is_air() != s.air || r.is_finisher() != s.finisher {
            continue;
        }
        if r.ability != 0 && !s.equipped.contains(&r.ability) {
            continue;
        }
        if r.needs_target() {
            let Some(t) = s.target else { continue };
            if t.distance < r.near || t.distance > r.far {
                continue;
            }
            let (lo, hi) = (r.low.min(r.high), r.low.max(r.high));
            if t.dy < lo || t.dy > hi {
                continue;
            }
        }
        let rank = |x: &ComboRow| (x.score, x.needs_target() as u8);
        if best.map_or(true, |b| rank(r) > rank(b)) {
            best = Some(r);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: u8, kind: u8, flag: u32, motion: u16, near: f32, far: f32, score: u16) -> ComboRow {
        ComboRow {
            id, kind, sub: -1, flag, motion, next_motion: 0, jump: 0.0, speed_min: 0.0, speed_max: 0.0,
            near, far, low: -20.0, high: -110.0, ability: 0, score,
        }
    }

    #[test]
    fn picks_by_distance_and_score() {
        let rows = vec![
            row(5, 3, 0, 151, 0.0, 0.0, 1),
            row(4, 2, 0, 152, 0.0, 180.0, 1),
            row(6, 1, 0, 153, 180.0, 300.0, 5),
            row(3, 3, 4, 155, 0.0, 0.0, 10),
        ];
        let sit = |t: Option<f32>, fin: bool| Situation {
            air: false,
            finisher: fin,
            target: t.map(|d| TargetGeom { distance: d, dy: -60.0 }),
            equipped: &[],
        };
        assert_eq!(select(&rows, &sit(None, false)).unwrap().motion, 151);
        assert_eq!(select(&rows, &sit(Some(250.0), false)).unwrap().motion, 153);
        // near target: 152 and the no-target fallback 151 tie on score;
        // the row whose requirement is met wins
        assert_eq!(select(&rows, &sit(Some(100.0), false)).unwrap().motion, 152);
        assert_eq!(select(&rows, &sit(None, true)).unwrap().motion, 155);
    }
}
