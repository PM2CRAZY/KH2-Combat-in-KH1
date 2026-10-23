//! Secondary motion for KH1 Sora's loose parts while KH2's moves pose him: the hair spikes,
//! the hood and the Keyblade's keychain lag, swing and settle, with a light breeze.
//!
//! KH1 has no physics for these: their movement is animated by hand in each of its own motions,
//! so under KH2's motions they stayed rigid (the user: "sora's keychain, hair, hood etc dont have
//! wind physics during any KH2 moves"). Ours, not either game's: each chain is a string of
//! particles in world space (so running, turning and swinging carry them), pulled back toward
//! where the pose puts them, with gravity and a breeze, kept at the bones' lengths; each bone is
//! then turned to point at its simulated child.
//!
//! KH2's own animation of these parts now drives them under KH2's moves (`secondary`);
//! this swing is left for chains KH2's data doesn't animate (`Jiggle::apply_except`).

use glam::{Mat4, Quat, Vec3};

/// One chain: its bones from the one fixed to its parent to the tip, and how it moves.
#[derive(Clone, Debug)]
pub struct Chain {
    pub bones: Vec<usize>,
    /// Pull back toward the posed position each step (0..1).
    pub stiffness: f32,
    /// Velocity lost each step (0..1).
    pub damping: f32,
    /// Gravity's share (1 = full).
    pub gravity: f32,
    /// The breeze's strength (acceleration, units / s²).
    pub wind: f32,
    /// The most a bone may turn away from its posed direction (radians): no swing so wide it
    /// clips into him (the user's video, s185: hair and hood through his head).
    pub max_turn: f32,
}

/// KH1 Sora's chains (bones from `xa_ex_0010.mdls`'s bind: under the head (151) fourteen hair
/// spikes of a root, a joint and a tip, three bones each, the joint at the root's place; the
/// hood 79..84 from the neck base, 79 and 80 at one place; the keychain 283..292 from the
/// Keyblade's pommel, swung from 284 on: 283 may carry the pommel itself).
pub fn sora_chains() -> Vec<Chain> {
    let mut v = Vec::new();
    // (Toned down from s185, the user: "needs to be toned down a lot, there's a lot of
    // clipping": stiffer, little gravity on the hair and hood, a light breeze, swings capped.)
    for root in (152..=191).step_by(3) {
        v.push(Chain { bones: vec![root + 1, root + 2], stiffness: 0.55, damping: 0.25, gravity: 0.0, wind: 40.0, max_turn: 10f32.to_radians() });
    }
    v.push(Chain { bones: (80..=84).collect(), stiffness: 0.45, damping: 0.25, gravity: 0.1, wind: 30.0, max_turn: 12f32.to_radians() });
    // The keychain hangs free from the pommel, a pendulum (s188, the user: its movement is good
    // but it "looks like the chain is kind of floating": KH1's rest pose holds it straight out
    // of the pommel along the blade, and the pull back there with the 35° cap kept it so).
    v.push(Chain { bones: (284..=292).collect(), stiffness: 0.0, damping: 0.12, gravity: 1.0, wind: 20.0, max_turn: 170f32.to_radians() });
    v
}

/// Gravity (units: KH1's, about a centimetre; per second²), along the model's down in the world.
const GRAVITY: f32 = 980.0;
/// Simulation step (the game's frame).
const STEP: f32 = 1.0 / 60.0;
/// Steps a chain takes at once when it starts over, to settle.
const SETTLE: usize = 120;
/// A jump this far between frames is a warp (a room change, a cut): the chains start over.
const WARP: f32 = 300.0;

#[derive(Clone, Debug, Default)]
struct State {
    p: Vec<Vec3>,
    prev: Vec<Vec3>,
}

/// The chains and their particles.
pub struct Jiggle {
    chains: Vec<Chain>,
    state: Vec<State>,
    /// The breeze's clock.
    clock: f32,
}

impl Jiggle {
    pub fn new(chains: Vec<Chain>) -> Jiggle {
        let n = chains.len();
        Jiggle { chains, state: vec![State::default(); n], clock: 0.0 }
    }

    /// Starts every chain over at its posed place (no swing carried in).
    pub fn reset(&mut self) {
        for s in &mut self.state {
            s.p.clear();
        }
    }

    /// Moves the chains on by `steps` game frames (0: the same frame drawn again) and turns their
    /// bones in `pose` (model space, bones in parent-before-child order, `parents` their
    /// parents) toward where they swing to. `to_world`: the model's place in the world, y up.
    pub fn apply(&mut self, pose: &mut [Mat4], parents: &[i16], to_world: &Mat4, steps: usize) {
        self.apply_except(pose, parents, to_world, steps, |_| false);
    }

    /// As `apply`, leaving out every chain with a bone `skip` names (posed another way: KH2's
    /// own animation of them, `secondary`). A chain left out starts over when it comes back.
    pub fn apply_except(&mut self, pose: &mut [Mat4], parents: &[i16], to_world: &Mat4, steps: usize, skip: impl Fn(usize) -> bool) {
        let from_world = to_world.inverse();
        if !from_world.is_finite() {
            return;
        }
        let at = |pose: &[Mat4], b: usize| to_world.transform_point3(pose[b].w_axis.truncate());
        // Down is the model's down in the world (KH1's world has y pointing down: the user, s188,
        // "the keychain is now floating upwards").
        let down = -to_world.transform_vector3(Vec3::Y).normalize_or(Vec3::Y);
        for ci in 0..self.chains.len() {
            let c = self.chains[ci].clone();
            if c.bones.iter().any(|&b| b >= pose.len()) || c.bones.len() < 2 {
                continue;
            }
            if c.bones.iter().any(|&b| skip(b)) {
                self.state[ci].p.clear();
                continue;
            }
            let target: Vec<Vec3> = c.bones.iter().map(|&b| at(pose, b)).collect();
            let len: Vec<f32> = target.windows(2).map(|w| (w[1] - w[0]).length()).collect();
            let st = &mut self.state[ci];
            let fresh = st.p.len() != target.len() || (st.p[0] - target[0]).length() > WARP || st.p.iter().any(|p| !p.is_finite());
            if fresh {
                st.p = target.clone();
                st.prev = target.clone();
            }
            // Starting over, a chain first settles where it rests (a free one hanging), so it
            // isn't seen dropping from the pose's place (`SETTLE` steps, no breeze).
            let settle = if fresh { SETTLE } else { 0 };
            for k in 0..steps + settle {
                let k = k.saturating_sub(settle);
                let t = self.clock + k as f32 * STEP;
                // A breeze from one side, swelling and dropping, with a quicker flutter on top.
                let gust = 0.6 + 0.4 * (t * 0.9).sin() + 0.15 * (t * 7.3).sin() * (t * 2.1).cos();
                let wind = if settle > 0 && k == 0 { Vec3::ZERO } else { Vec3::new((t * 0.13).cos(), 0.0, (t * 0.13).sin()) * (c.wind * gust) };
                let acc = down * (GRAVITY * c.gravity) + wind;
                st.p[0] = target[0];
                st.prev[0] = target[0];
                for i in 1..st.p.len() {
                    let p = st.p[i];
                    let v = (p - st.prev[i]) * (1.0 - c.damping);
                    let mut n = p + v + acc * STEP * STEP;
                    n += (target[i] - n) * c.stiffness;
                    st.prev[i] = p;
                    st.p[i] = n;
                }
                // Each link back to its bone's length.
                for i in 1..st.p.len() {
                    let d = st.p[i] - st.p[i - 1];
                    let l = d.length();
                    if l > 1e-4 && len[i - 1] > 1e-4 {
                        st.p[i] = st.p[i - 1] + d * (len[i - 1] / l);
                    } else {
                        st.p[i] = target[i];
                    }
                }
            }
            // Each bone turned toward its simulated child, its subtree with it.
            for i in 0..c.bones.len() - 1 {
                let (b, child) = (c.bones[i], c.bones[i + 1]);
                let here = pose[b].w_axis.truncate();
                let cur = pose[child].w_axis.truncate() - here;
                let want = from_world.transform_point3(st.p[i + 1]) - here;
                if cur.length() < 1e-3 || want.length() < 1e-3 {
                    continue;
                }
                let q = Quat::from_rotation_arc(cur.normalize(), want.normalize());
                if !q.is_finite() {
                    continue;
                }
                // At most `max_turn` from the posed direction (the simulated tip pulled back
                // to match, so the swing doesn't build up past it).
                let (axis, angle) = q.to_axis_angle();
                let q = if angle > c.max_turn && axis.is_finite() {
                    let q = Quat::from_axis_angle(axis, c.max_turn);
                    let tip = here + q * cur.normalize() * want.length();
                    st.p[i + 1] = to_world.transform_point3(tip);
                    q
                } else {
                    q
                };
                let turn = Mat4::from_translation(here) * Mat4::from_quat(q) * Mat4::from_translation(-here);
                for j in 0..pose.len() {
                    if j == b || under(parents, j, b) {
                        pose[j] = turn * pose[j];
                    }
                }
            }
        }
        self.clock += steps as f32 * STEP;
    }
}

/// Whether bone `j` is below bone `root`.
fn under(parents: &[i16], j: usize, root: usize) -> bool {
    let mut p = parents.get(j).copied().unwrap_or(-1);
    for _ in 0..64 {
        if p < 0 {
            return false;
        }
        if p as usize == root {
            return true;
        }
        p = parents.get(p as usize).copied().unwrap_or(-1);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A straight chain of 4 bones hanging out along +x from bone 0 at the origin.
    fn rig() -> (Vec<Mat4>, Vec<i16>) {
        let pose = (0..4).map(|i| Mat4::from_translation(Vec3::new(i as f32 * 10.0, 100.0, 0.0))).collect();
        (pose, vec![-1, 0, 1, 2])
    }

    fn chain() -> Chain {
        Chain { bones: vec![0, 1, 2, 3], stiffness: 0.2, damping: 0.1, gravity: 0.0, wind: 0.0, max_turn: 1.5 }
    }

    #[test]
    fn still_pose_stays_put() {
        let (pose, parents) = rig();
        let mut j = Jiggle::new(vec![chain()]);
        let mut p = pose.clone();
        for _ in 0..120 {
            p = pose.clone();
            j.apply(&mut p, &parents, &Mat4::IDENTITY, 1);
        }
        for (a, b) in p.iter().zip(&pose) {
            assert!((a.w_axis - b.w_axis).length() < 1e-3, "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn moving_chain_trails_then_settles() {
        let (pose, parents) = rig();
        let mut j = Jiggle::new(vec![chain()]);
        let mut x = 0.0;
        let mut tip = Vec3::ZERO;
        // Carried along +z at 300 units/s: the tip trails behind (-z of where the pose puts it).
        for _ in 0..20 {
            x += 300.0 * STEP;
            let w = Mat4::from_translation(Vec3::new(0.0, 0.0, x));
            let mut p = pose.clone();
            j.apply(&mut p, &parents, &w, 1);
            tip = p[3].w_axis.truncate();
        }
        assert!(tip.z < -1.0, "tip didn't trail: {tip:?}");
        // Lengths kept.
        assert!((tip.length() - Vec3::new(30.0, 100.0, 0.0).length()).abs() < 5.0 || tip.distance(Vec3::new(0.0, 100.0, 0.0)) < 30.5);
        // Stopped, it settles back.
        let w = Mat4::from_translation(Vec3::new(0.0, 0.0, x));
        for _ in 0..300 {
            let mut p = pose.clone();
            j.apply(&mut p, &parents, &w, 1);
            tip = p[3].w_axis.truncate();
        }
        assert!((tip - Vec3::new(30.0, 100.0, 0.0)).length() < 0.5, "didn't settle: {tip:?}");
    }

    #[test]
    fn a_warp_restarts_the_chain() {
        let (pose, parents) = rig();
        let mut j = Jiggle::new(vec![chain()]);
        let mut p = pose.clone();
        j.apply(&mut p, &parents, &Mat4::IDENTITY, 1);
        let mut p = pose.clone();
        j.apply(&mut p, &parents, &Mat4::from_translation(Vec3::new(5000.0, 0.0, 0.0)), 1);
        assert!((p[3].w_axis - pose[3].w_axis).length() < 1e-3);
    }
}

#[cfg(test)]
mod cap_tests {
    use super::*;

    #[test]
    fn swing_capped() {
        let pose: Vec<Mat4> = (0..2).map(|i| Mat4::from_translation(Vec3::new(i as f32 * 10.0, 100.0, 0.0))).collect();
        let parents = vec![-1, 0];
        let c = Chain { bones: vec![0, 1], stiffness: 0.05, damping: 0.0, gravity: 0.0, wind: 0.0, max_turn: 0.2 };
        let mut j = Jiggle::new(vec![c]);
        let mut worst = 0.0f32;
        let mut z = 0.0;
        for _ in 0..60 {
            z += 2000.0 * STEP;
            let w = Mat4::from_translation(Vec3::new(0.0, 0.0, z));
            let mut p = pose.clone();
            j.apply(&mut p, &parents, &w, 1);
            let d = (p[1].w_axis - p[0].w_axis).truncate();
            worst = worst.max(d.angle_between(Vec3::X));
        }
        assert!(worst <= 0.2 + 1e-3, "turned {worst}");
        assert!(worst > 0.1, "never swung");
    }
}

#[cfg(test)]
mod hang_tests {
    use super::*;

    #[test]
    fn a_free_chain_hangs_down() {
        // Out along +x from a still root; no stiffness: it ends up hanging below the root.
        let pose: Vec<Mat4> = (0..4).map(|i| Mat4::from_translation(Vec3::new(i as f32 * 5.0, 100.0, 0.0))).collect();
        let parents = vec![-1, 0, 1, 2];
        let c = Chain { bones: vec![0, 1, 2, 3], stiffness: 0.0, damping: 0.12, gravity: 1.0, wind: 0.0, max_turn: 170f32.to_radians() };
        let mut j = Jiggle::new(vec![c]);
        let mut tip = Vec3::ZERO;
        for _ in 0..600 {
            let mut p = pose.clone();
            j.apply(&mut p, &parents, &Mat4::IDENTITY, 1);
            tip = p[3].w_axis.truncate();
        }
        assert!(tip.y < 100.0 - 13.0 && tip.x.abs() < 3.0, "not hanging: {tip:?}");
    }
}

#[cfg(test)]
mod down_tests {
    use super::*;

    #[test]
    fn hangs_down_in_a_y_down_world() {
        // KH1's world: y down (to_world flips y). The chain must hang toward the model's down.
        let pose: Vec<Mat4> = (0..4).map(|i| Mat4::from_translation(Vec3::new(i as f32 * 5.0, 100.0, 0.0))).collect();
        let parents = vec![-1, 0, 1, 2];
        let c = Chain { bones: vec![0, 1, 2, 3], stiffness: 0.0, damping: 0.12, gravity: 1.0, wind: 0.0, max_turn: 170f32.to_radians() };
        let mut j = Jiggle::new(vec![c]);
        let flip = Mat4::from_scale(Vec3::new(1.0, -1.0, 1.0));
        let mut tip = Vec3::ZERO;
        for _ in 0..600 {
            let mut p = pose.clone();
            j.apply(&mut p, &parents, &flip, 1);
            tip = p[3].w_axis.truncate();
        }
        assert!(tip.y < 100.0 - 13.0, "not hanging in model space: {tip:?}");
    }
}

#[cfg(test)]
mod skip_tests {
    use super::*;

    #[test]
    fn a_skipped_chain_is_left_as_posed() {
        let pose: Vec<Mat4> = (0..4).map(|i| Mat4::from_translation(Vec3::new(i as f32 * 5.0, 100.0, 0.0))).collect();
        let parents = vec![-1, 0, 1, 2];
        let c = Chain { bones: vec![0, 1, 2, 3], stiffness: 0.0, damping: 0.12, gravity: 1.0, wind: 0.0, max_turn: 170f32.to_radians() };
        let mut j = Jiggle::new(vec![c]);
        for _ in 0..60 {
            let mut p = pose.clone();
            j.apply_except(&mut p, &parents, &Mat4::IDENTITY, 1, |b| b == 2);
            assert!(p == pose);
        }
        // Not skipped, it hangs.
        let mut p = pose.clone();
        j.apply(&mut p, &parents, &Mat4::IDENTITY, 1);
        assert!(p[3].w_axis.y < 100.0 - 1.0);
    }
}
