//! pppKeShpTail2X (type 155) and pppKeShpTail3X (type 157): a trail of
//! sprites along the instance's recent positions (`docs/kh2-effects.md`
//! §7.5).
//!
//! Every tick the program writes the instance position (its `M`
//! translation, or that through the root with the key's "world" byte) into
//! a ring of past positions, newest first. At draw time it walks back along
//! that polyline and puts a camera-facing sprite every `spacing` units:
//! the first at the newest point (if the key says so), the others shrinking
//! linearly from the start size to the end size, their colour going from
//! the head colour to the tail colour. The spacing shrinks in proportion to
//! the size, so the sprites stay overlapping as they get smaller. The walk
//! stops when the sprite count, the spacing or the history runs out.
//!
//! The two programs differ in where the colours come from and in the
//! extras: 2X takes fixed head and tail colours from its key and draws the
//! one animated shape step it keeps itself; 3X integrates both colours in
//! its work area (value, velocity and acceleration, like ColAccele and
//! ColMove) and gives every sprite a pseudo-random size change, rotation
//! and animation phase, seeded once per instance so they stay put from
//! frame to frame.
//!
//! The work areas are laid out as the game's (offsets from the program's
//! first work offset):
//!
//! - 2X: u8 ring length (31, set when the instance is made), u8 newest
//!   index, u16 animation time, u16 animation step, u16 step drawn, then
//!   the ring at +0x10 (16 bytes per point).
//! - 3X: four i16 head colour channels (8.8, 0x4000 = 0x80), four tail
//!   channels, their velocities and accelerations (+0x10..+0x2F), the ring
//!   of 28 points at +0x30, u32 animation time at +0x1F0, u16 random seed at
//!   +0x1F8, u8 newest index at +0x1FA.

use glam::{Mat4, Vec3, Vec4};
use kh2_formats::dpd::{Key, Shape};

use crate::player::Work;

/// Points in a 3X ring (fixed in the game).
pub(crate) const TAIL3_LEN: usize = 28;
/// A 2X ring's length, written by its construct.
pub(crate) const TAIL2_LEN: u8 = 31;
const TAIL3_RING: i32 = 0x30;
const TAIL2_RING: i32 = 0x10;
pub(crate) const TAIL3_TIME: i32 = 0x1f0;
pub(crate) const TAIL3_SEED: i32 = 0x1f8;
const TAIL3_HEAD: i32 = 0x1fa;

/// The game stops a walk when the history runs out; this only bounds a
/// count that wrapped (a zero spacing with a growing size).
const MAX_SPRITES: u32 = 4096;

fn vec4(wk: &Work, o: i32) -> Vec4 {
    Vec4::new(wk.f32(o), wk.f32(o + 4), wk.f32(o + 8), wk.f32(o + 12))
}

fn set_vec4(wk: &mut Work, o: i32, v: Vec4) {
    wk.set_vec3(o, v.truncate());
    wk.set_f32(o + 12, v.w);
}

/// The position a tail records this tick: `M`'s translation, or that
/// through the root (the key's world byte) with w forced to 1.
fn recorded(m: &Mat4, root: &Mat4, world: bool) -> Vec4 {
    if world {
        root.transform_point3(m.w_axis.truncate()).extend(1.0)
    } else {
        m.w_axis
    }
}

/// pppKeShpTail3X calc (once per tick): fill the ring on the instance's
/// first tick (not after a loop), record this tick's point, integrate the
/// two colours and add the key's deltas on a key tick.
pub(crate) fn tail3_calc(wk: &mut Work, o: i32, key: &Key, frame: i32, looped: bool, m: &Mat4, root: &Mat4) {
    let world = key.u8(0x53) != 0;
    let p = recorded(m, root, world);
    if frame == 0 && !looped {
        let fill = p.truncate().extend(1.0);
        for i in 0..TAIL3_LEN as i32 {
            set_vec4(wk, o + TAIL3_RING + i * 16, fill);
        }
    }
    let head = match wk.u8(o + TAIL3_HEAD) {
        0 => TAIL3_LEN as u8,
        h => h,
    } - 1;
    wk.set_u8(o + TAIL3_HEAD, head);
    set_vec4(wk, o + TAIL3_RING + head as i32 * 16, p);
    // Per channel: velocity += acceleration, value += velocity, for the
    // head colour (value +0, velocity +16, acceleration +24) and the tail
    // colour (+8, +32, +40); wrapping 16-bit arithmetic.
    let on_key = key.frame() == frame;
    for c in 0..4 {
        let at = |base: i32| o + base + c * 2;
        let add = |wk: &mut Work, at: i32, v: i16| wk.set_u16(at, (wk.u16(at) as i16).wrapping_add(v) as u16);
        for (value, vel, acc) in [(0, 16, 24), (8, 32, 40)] {
            let a = wk.u16(at(acc)) as i16;
            add(wk, at(vel), a);
            let v = wk.u16(at(vel)) as i16;
            add(wk, at(value), v);
        }
        if on_key {
            let k = |off: i32| key.i16((off + c * 2) as usize);
            add(wk, at(0), k(0x20));
            add(wk, at(16), k(0x30));
            add(wk, at(24), k(0x38));
            add(wk, at(8), k(0x28));
            add(wk, at(32), k(0x40));
            add(wk, at(40), k(0x48));
        }
    }
}

/// pppKeShpTail2X calc (once per tick): the ring as for 3X (its own
/// length), then the shape animation. Unlike DrawShapeX this one reads the
/// wait and loop bit of the step it is on, and keeps the step it was on
/// before advancing as the one to draw (+6).
#[allow(clippy::too_many_arguments)]
pub(crate) fn tail2_calc(wk: &mut Work, o: i32, key: &Key, frame: i32, looped: bool, m: &Mat4, root: &Mat4, shape: Option<&Shape>) {
    let world = key.u8(0x26) != 0;
    let len = wk.u8(o);
    if frame == 0 && !looped {
        let fill = recorded(m, root, world).truncate().extend(1.0);
        for i in 0..len as i32 {
            set_vec4(wk, o + TAIL2_RING + i * 16, fill);
        }
    }
    let head = match wk.u8(o + 1) {
        0 => len,
        h => h,
    }
    .wrapping_sub(1);
    wk.set_u8(o + 1, head);
    let at = o + TAIL2_RING + head as i32 * 16;
    if world {
        set_vec4(wk, at, recorded(m, root, true));
    } else {
        // Only x, y and z are copied; w keeps the 1.0 of the first fill.
        wk.set_vec3(at, m.w_axis.truncate());
    }
    let Some(sh) = shape else { return };
    if sh.sequence.is_empty() {
        return;
    }
    let step = wk.u16(o + 4);
    wk.set_u16(o + 6, step);
    let rec = &sh.sequence[(step as usize).min(sh.sequence.len() - 1)];
    let time = wk.u16(o + 2).wrapping_add(key.i16(8) as u16);
    wk.set_u16(o + 2, time);
    let wait = rec.wait as i16 as i32;
    if wait <= time as i32 {
        let next = step.wrapping_add(1);
        wk.set_u16(o + 4, next);
        wk.set_u16(o + 2, time.wrapping_sub(wait as u16));
        if sh.sequence.len() as i32 <= next as i32 {
            wk.set_u16(o + 2, 0);
            wk.set_u16(o + 4, if rec.loops { 0 } else { step });
        }
    }
}

/// One sprite of a trail: where (in the recorded space), how big (before
/// the root scale) and what colour (bytes, 0x80 = 1.0).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TailSprite {
    pub pos: Vec3,
    pub size: f32,
    pub colour: [u8; 4],
}

/// The walk both programs share.
struct Walk {
    ring: Vec<Vec4>,
    head: usize,
    count: u32,
    draw_first: bool,
    size: f32,
    end_size: f32,
    spacing: f32,
    head_colour: [f32; 4],
    tail_colour: [f32; 4],
}

/// Colour floats to the bytes the packet takes (truncated, low byte).
fn bytes(c: [f32; 4]) -> [u8; 4] {
    c.map(|v| v as i64 as u8)
}

impl Walk {
    /// Steps back along the history, newest first, calling `emit` for every
    /// sprite placed (the game's arithmetic, including its quirk that the
    /// next sprite's offset grows by the current spacing while the history
    /// check uses the new one).
    fn run(&self, mut emit: impl FnMut(TailSprite)) {
        let n = self.ring.len();
        if n < 2 || self.head >= n {
            return;
        }
        let last = n - 1;
        let steps = self.count as i32 - 1;
        let steps = steps as f32;
        let dcol: [f32; 4] = std::array::from_fn(|i| (self.head_colour[i] - self.tail_colour[i]) / steps);
        let size_step = (self.size - self.end_size) / steps;
        let spacing_step = size_step / self.size * self.spacing;
        let mut count = if self.spacing == 0.0 { 0 } else { self.count };
        let (mut size, mut spacing, mut colour) = (self.size, self.spacing, self.head_colour);
        let mut p_cur = self.ring[self.head];
        let mut idx = if self.head != last { self.head + 1 } else { 0 };
        let mut p_next = self.ring[idx];
        let mut d = p_next - p_cur;
        let mut seg = d.truncate().length();
        let mut avail = seg;
        let mut offset = spacing;
        let mut sample = p_cur;
        let mut first = self.draw_first;
        let mut guard = 0;
        loop {
            if first {
                emit(TailSprite { pos: sample.truncate(), size, colour: bytes(colour) });
            }
            first = true;
            count = count.wrapping_sub(1);
            guard += 1;
            if count == 0 || guard > MAX_SPRITES {
                return;
            }
            for (c, d) in colour.iter_mut().zip(dcol) {
                *c -= d;
            }
            spacing -= spacing_step;
            size -= size_step;
            if spacing <= 0.0 {
                return;
            }
            while avail < spacing {
                idx = if idx != last { idx + 1 } else { 0 };
                if idx == self.head {
                    return;
                }
                p_cur = p_next;
                p_next = self.ring[idx];
                d = p_next - p_cur;
                offset -= seg;
                seg = d.truncate().length();
                avail += seg;
            }
            sample = d * (offset / seg) + p_cur;
            offset += spacing;
            avail -= spacing;
        }
    }
}

/// The instance alpha (the Color program's byte at `w1 + 0xB`) as a
/// factor (0x80 = 1.0).
fn alpha_factor(colour: [u8; 4]) -> f32 {
    colour[3] as f32 / 128.0
}

/// A 3X sprite also carries the pseudo-random value it was given.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Tail3Sprite {
    pub sprite: TailSprite,
    pub seed: u16,
}

/// The sprites of a 3X trail. `colour` is the instance colour (for its
/// alpha). Each sprite draws the next number of a 16-bit generator
/// (`seed · 0x80D + 7`) started from the instance's seed; its size is
/// `size · (1 − seed/65535 · key@0x14)`.
pub(crate) fn tail3_sprites(wk: &Work, o: i32, key: &Key, colour: [u8; 4]) -> Vec<Tail3Sprite> {
    let af = alpha_factor(colour);
    // Colour channels are 8.8 (0x4000 = 0x80): `>> 7` gives bytes.
    let ch = |at: i32| (wk.u16(at) as i16 >> 7) as f32;
    let mut a: [f32; 4] = std::array::from_fn(|i| ch(o + i as i32 * 2));
    let mut b: [f32; 4] = std::array::from_fn(|i| ch(o + 8 + i as i32 * 2));
    a[3] *= af;
    b[3] *= af;
    let walk = Walk {
        ring: (0..TAIL3_LEN as i32).map(|i| vec4(wk, o + TAIL3_RING + i * 16)).collect(),
        head: wk.u8(o + TAIL3_HEAD) as usize,
        count: key.u8(0x1c) as u32,
        draw_first: key.u8(0x1d) != 0,
        size: key.f32(0xc),
        end_size: key.f32(0x10),
        spacing: key.f32(0x18),
        head_colour: a,
        tail_colour: b,
    };
    let jitter = key.f32(0x14);
    let mut seed = wk.u16(o + TAIL3_SEED);
    let mut out = Vec::new();
    walk.run(|s| {
        seed = seed.wrapping_mul(0x80d).wrapping_add(7);
        let size = (1.0 - seed as f32 / 65535.0 * jitter) * s.size;
        out.push(Tail3Sprite { sprite: TailSprite { size, ..s }, seed });
    });
    out
}

/// The sprites of a 2X trail (head and tail colours from the key bytes
/// @0x14 and @0x18, alpha times the instance alpha).
pub(crate) fn tail2_sprites(wk: &Work, o: i32, key: &Key, colour: [u8; 4]) -> Vec<TailSprite> {
    let af = alpha_factor(colour);
    let mut a: [f32; 4] = std::array::from_fn(|i| key.u8(0x14 + i) as f32);
    let mut b: [f32; 4] = std::array::from_fn(|i| key.u8(0x18 + i) as f32);
    a[3] *= af;
    b[3] *= af;
    let len = wk.u8(o) as i32;
    let walk = Walk {
        ring: (0..len).map(|i| vec4(wk, o + TAIL2_RING + i * 16)).collect(),
        head: wk.u8(o + 1) as usize,
        count: key.u16(0x20) as u32,
        draw_first: key.u8(0x22) != 0,
        size: key.f32(0xc),
        end_size: key.f32(0x10),
        spacing: key.f32(0x1c),
        head_colour: a,
        tail_colour: b,
    };
    let mut out = Vec::new();
    walk.run(|s| out.push(s));
    out
}

/// The animation step a 3X sprite shows: `((time + seed) / wait of step
/// 0) mod steps`, in unsigned 32-bit arithmetic.
pub(crate) fn tail3_step(sh: &Shape, time: u32, seed: u16) -> usize {
    let n = sh.sequence.len() as u32;
    let wait = sh.sequence.first().map(|s| s.wait as i16 as i32 as u32).unwrap_or(0);
    if n == 0 || wait == 0 {
        return 0;
    }
    ((time.wrapping_add(seed as u32) / wait) % n) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(len: usize, fields: &[(usize, &[u8])]) -> Key {
        let mut data = vec![0u8; len];
        for (at, b) in fields {
            data[*at..*at + b.len()].copy_from_slice(b);
        }
        Key { data }
    }

    fn at_x(x: f32) -> Mat4 {
        Mat4::from_translation(Vec3::new(x, 0.0, 0.0))
    }

    #[test]
    fn tail3_ring_and_colours() {
        let mut wk = Work::new(0x200);
        // Head colour +0x4000 red at key 0, velocity +0x100 per tick on red.
        let k = key(0x58, &[(0x20, &0x4000i16.to_le_bytes()), (0x30, &0x100i16.to_le_bytes())]);
        tail3_calc(&mut wk, 0, &k, 0, false, &at_x(5.0), &Mat4::IDENTITY);
        // The first tick fills the ring and records at the last slot.
        assert_eq!(wk.u8(TAIL3_HEAD), 27);
        for i in 0..28 {
            assert_eq!(vec4(&wk, TAIL3_RING + i * 16), Vec4::new(5.0, 0.0, 0.0, 1.0));
        }
        assert_eq!(wk.u16(0), 0x4000);
        assert_eq!(wk.u16(16), 0x100);
        tail3_calc(&mut wk, 0, &k, 4096, false, &at_x(9.0), &Mat4::IDENTITY);
        assert_eq!(wk.u8(TAIL3_HEAD), 26);
        assert_eq!(vec4(&wk, TAIL3_RING + 26 * 16).x, 9.0);
        // Not a key tick: value += velocity.
        assert_eq!(wk.u16(0), 0x4100);
        // A looped instance does not refill on frame 0.
        tail3_calc(&mut wk, 0, &k, 0, true, &at_x(100.0), &Mat4::IDENTITY);
        assert_eq!(vec4(&wk, TAIL3_RING + 3 * 16).x, 5.0);
    }

    #[test]
    fn tail2_animation_reads_current_step() {
        use kh2_formats::dpd::{ShapeFrame, ShapeStep};
        let frame = ShapeFrame { free_quads: false, tex0: kh2_formats::dpd::Tex0(0), sprites: vec![] };
        let sh = Shape { number: 1, sequence: vec![ShapeStep { wait: 2, loops: false, frame: frame.clone() }, ShapeStep { wait: 1, loops: true, frame }] };
        let mut wk = Work::new(0x200);
        wk.set_u8(0, TAIL2_LEN);
        let k = key(0x2c, &[(8, &1i16.to_le_bytes())]);
        let mut seen = Vec::new();
        for t in 0..6 {
            tail2_calc(&mut wk, 0, &k, t * 4096, false, &at_x(0.0), &Mat4::IDENTITY, Some(&sh));
            seen.push((wk.u16(4), wk.u16(6)));
        }
        // Step 0 waits 2 ticks, step 1 one tick and loops back.
        assert_eq!(seen, vec![(0, 0), (1, 0), (0, 1), (0, 0), (1, 0), (0, 1)]);
        assert_eq!(wk.u8(1), 25);
    }

    #[test]
    fn walk_spaces_sprites_along_history() {
        // A straight history 10 units apart, newest at x = 0 going to +x.
        let mut ring = vec![Vec4::ZERO; 28];
        let head = 5;
        for (i, p) in ring.iter_mut().enumerate() {
            let back = (i + 28 - head) % 28;
            *p = Vec4::new(back as f32 * 10.0, 0.0, 0.0, 1.0);
        }
        let walk = Walk {
            ring,
            head,
            count: 4,
            draw_first: true,
            size: 2.0,
            end_size: 2.0,
            spacing: 15.0,
            head_colour: [96.0, 0.0, 0.0, 128.0],
            tail_colour: [0.0, 0.0, 96.0, 32.0],
        };
        let mut got = Vec::new();
        walk.run(|s| got.push(s));
        assert_eq!(got.len(), 4);
        let xs: Vec<f32> = got.iter().map(|s| s.pos.x).collect();
        assert_eq!(xs, vec![0.0, 15.0, 30.0, 45.0]);
        // Colour goes from head to tail in count - 1 steps.
        assert_eq!(got[0].colour, [96, 0, 0, 128]);
        assert_eq!(got[1].colour, [64, 0, 32, 96]);
        assert_eq!(got[3].colour, [0, 0, 96, 32]);
        // No sprite without a first one, and the walk stops at the history's end.
        let short = Walk { draw_first: false, count: 100, ..walk };
        let mut n = 0;
        short.run(|_| n += 1);
        // 27 segments of 10 units hold 18 sprites 15 apart (270 / 15).
        assert_eq!(n, 18);
    }

    #[test]
    fn walk_shrinks_size_and_spacing() {
        let ring: Vec<Vec4> = (0..28).map(|i| Vec4::new(i as f32 * 4.0, 0.0, 0.0, 1.0)).collect();
        let walk = Walk {
            ring,
            head: 0,
            count: 3,
            draw_first: true,
            size: 2.0,
            end_size: 1.0,
            spacing: 8.0,
            head_colour: [0.0; 4],
            tail_colour: [0.0; 4],
        };
        let mut got = Vec::new();
        walk.run(|s| got.push((s.pos.x, s.size)));
        // size 2 → 1.5 → 1; spacing 8 → 6 → 4. The second sprite sits at
        // the first spacing (8), the third 6 further (the game's offsets).
        assert_eq!(got, vec![(0.0, 2.0), (8.0, 1.5), (14.0, 1.0)]);
    }

    #[test]
    fn tail3_sprites_jitter_and_step() {
        let mut wk = Work::new(0x200);
        let k = key(0x58, &[(0xc, &1.0f32.to_le_bytes()), (0x10, &1.0f32.to_le_bytes()), (0x14, &0.5f32.to_le_bytes()), (0x18, &1.0f32.to_le_bytes()), (0x1c, &[2, 1])]);
        for t in 0..3 {
            tail3_calc(&mut wk, 0, &k, t * 4096, false, &at_x(t as f32 * 10.0), &Mat4::IDENTITY);
        }
        wk.set_u16(TAIL3_SEED, 1);
        let s = tail3_sprites(&wk, 0, &k, [0x80; 4]);
        assert_eq!(s.len(), 2);
        // seed 1 → 0x80D + 7 = 0x814.
        assert_eq!(s[0].seed, 0x814);
        assert!((s[0].sprite.size - (1.0 - 0x814 as f32 / 65535.0 * 0.5)).abs() < 1e-6);
        assert_eq!(s[0].sprite.pos, Vec3::new(20.0, 0.0, 0.0));
        assert_eq!(s[1].sprite.pos, Vec3::new(19.0, 0.0, 0.0));
    }
}
