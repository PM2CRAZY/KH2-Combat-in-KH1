//! The traps the spell scripts call, written from plain-language notes on
//! the game's trap handlers (see `docs/magic-model.md`).
//!
//! Script-visible objects are 16-byte records in script memory whose
//! second word holds their own handle, mirroring how the game hands
//! objects to scripts (a script copies the caster's header and passes the
//! copy's address; traps read the object handle at +4). Vectors the traps
//! return live in per-trap static slots, as in the game, so a script that
//! keeps the pointer sees it overwritten by the next call.

use glam::{Mat4, Quat, Vec3, Vec4};
use kh2_bdx::{BdxError, Cell, CtxId, Host, Vm};

use crate::{flip, Cast, Effect, Event, Magic, MagicAttack, ObjId, Tables, World};

#[derive(Clone, Copy, Debug, Default)]
pub struct Statics {
    add: u32,
    sub: u32,
    mul: u32,
    roty: u32,
    effect_pos: u32,
    shared: u32,
}

impl Statics {
    pub fn new(vm: &mut Vm) -> Statics {
        let mut slot = || {
            let a = vm.mem.alloc(16);
            vm.mem.set_f32(a + 12, 1.0).ok();
            a
        };
        Statics { add: slot(), sub: slot(), mul: slot(), roty: slot(), effect_pos: slot(), shared: slot() }
    }
}

/// The record for object `o`, made on first use.
pub(crate) fn obj_record(vm: &mut Vm, t: &mut Tables, o: ObjId) -> u32 {
    if let Some(r) = t.obj_rec.get(&o) {
        return *r;
    }
    let r = vm.mem.alloc(16);
    vm.mem.set_u32(r + 4, r).ok();
    t.obj_rec.insert(o, r);
    t.objs.insert(r, o);
    r
}

/// Starts PAX element `id` of the file's effects (the record `pax` must be
/// the file's). Returns the effect record, or 0.
fn start_effect_in(magic: &mut Magic, vm: &mut Vm, t: &mut Tables, world: &dyn World, pax: u32, id: u16, owner: Option<ObjId>) -> u32 {
    if pax != t.pax_rec {
        return 0;
    }
    let spell = t.spell;
    let file = &magic.spells[spell];
    let Some(element) = file.pax.element(id).cloned() else { return 0 };
    let timing = file.pax.effect(element.effect).cloned().unwrap_or(kh2_formats::pax::PaxEffect { number: element.effect as u32, emitters: Vec::new() });
    let eid = magic.fresh_id();
    let e = Effect::new(eid, spell, element, timing, owner, world);
    magic.effects.push(e);
    let rec = vm.mem.alloc(16);
    t.effects.insert(rec, eid);
    rec
}

/// The muzzle effect of an object's frame trigger 1 (its own effect list).
pub(crate) fn start_object_effect(magic: &mut Magic, vm: &mut Vm, t: &mut Tables, id: u16, owner: ObjId, world: &dyn World) -> u32 {
    let pax = t.pax_rec;
    start_effect_in(magic, vm, t, world, pax, id, Some(owner))
}

pub(crate) fn run(magic: &mut Magic, cast: &mut Cast, world: &mut dyn World) -> Result<(), BdxError> {
    let main = cast.main;
    let vm = &mut cast.vm;
    let mut b = Bridge { magic, t: &mut cast.t, world };
    vm.owner = None;
    let main_done = vm.run_tree(main, &mut b)?;
    let mut i = 0;
    while i < b.t.threads.len() {
        let c = b.t.threads[i];
        vm.owner = None;
        if vm.run_tree(c, &mut b)? {
            b.t.threads.remove(i);
        } else {
            i += 1;
        }
    }
    cast.finished = main_done && cast.t.threads.is_empty();
    Ok(())
}

struct Bridge<'a> {
    magic: &'a mut Magic,
    t: &'a mut Tables,
    world: &'a mut dyn World,
}

/// Reads a vector argument, zeroing NaN components in place as the game
/// does.
fn vec_in(vm: &mut Vm, addr: u32) -> Result<Vec4, BdxError> {
    let mut v = vm.mem.vec4(addr)?;
    let mut fixed = false;
    for x in v.iter_mut() {
        if x.is_nan() {
            *x = 0.0;
            fixed = true;
        }
    }
    if fixed {
        vm.mem.set_vec4(addr, v)?;
    }
    Ok(Vec4::from(v))
}

fn put(vm: &mut Vm, addr: u32, v: Vec4) -> Result<(), BdxError> {
    vm.mem.set_vec4(addr, v.into())
}

/// Host point -> game vector (w = 1).
fn game(p: Vec3) -> Vec4 {
    flip(p).extend(1.0)
}

/// Game vector -> host point.
fn host(v: Vec4) -> Vec3 {
    flip(v.truncate())
}

fn b(v: bool) -> Cell {
    Cell::int(v as i32)
}

impl Bridge<'_> {
    fn obj_of_ref(&self, vm: &Vm, objref: u32) -> Option<ObjId> {
        let h = vm.mem.u32(objref.checked_add(4)?).ok()?;
        self.t.objs.get(&h).copied()
    }

    fn obj_of_handle(&self, h: u32) -> Option<ObjId> {
        self.t.objs.get(&h).copied()
    }

    fn attack(&mut self, h: u32) -> Option<&mut MagicAttack> {
        let key = *self.t.attacks.get(&h)?;
        self.magic.attacks.iter_mut().find(|a| a.key == key)
    }

    fn effect(&mut self, h: u32) -> Option<&mut Effect> {
        let id = *self.t.effects.get(&h)?;
        self.magic.effects.iter_mut().find(|e| e.id == id)
    }

    fn start_effect(&mut self, vm: &mut Vm, pax: u32, id: u16, owner: Option<ObjId>) -> u32 {
        start_effect_in(self.magic, vm, self.t, &*self.world, pax, id, owner)
    }

    fn new_attack(&mut self, vm: &mut Vm, owner: ObjId, atk: u16, sub: u8, swing: Option<(u64, i64)>) -> u32 {
        let key = self.magic.fresh_id();
        let (id, group, owns_id) = match swing {
            Some((id, group)) => (id, group, false),
            None => (key, self.magic.fresh_group(), true),
        };
        self.magic.attacks.push(MagicAttack {
            key,
            id,
            group,
            owns_id,
            owner,
            spell: self.t.spell,
            atk,
            sub,
            pos: None,
            prev: None,
            moved: false,
            radius: 0.0,
            height: 0.0,
            hit: false,
            hit_bg: false,
            reflected: false,
            reflect_count: 0,
            freed: false,
        });
        let rec = vm.mem.alloc(16);
        self.t.attacks.insert(rec, key);
        rec
    }

    fn target_record(&mut self, vm: &mut Vm, of: ObjId) -> Result<u32, BdxError> {
        let rec = match self.t.targets.get(&of) {
            Some(r) => *r,
            None => {
                let r = vm.mem.alloc(16);
                self.t.targets.insert(of, r);
                r
            }
        };
        // The caster's target was picked when the cast started; other
        // objects use their own.
        let target = if of == self.t.caster { self.t.target } else { self.world.target_of(of) };
        let target = target.filter(|o| self.world.exists(*o));
        let h = match target {
            Some(o) => obj_record(vm, self.t, o),
            None => 0,
        };
        vm.mem.set_u32(rec, h)?;
        vm.mem.set_u32(rec + 4, 0)?;
        vm.mem.set_u32(rec + 8, 0)?;
        vm.mem.set_u32(rec + 12, 2)?;
        Ok(rec)
    }

    fn missing(&mut self, table: u8, index: u16) {
        let name = kh2_bdx::trap_name(table, index).unwrap_or("unknown trap");
        if !self.magic.events.iter().any(|e| *e == Event::Missing(name)) {
            self.magic.events.push(Event::Missing(name));
        }
    }
}

/// Rotates `v` about the vertical axis so that a facing of yaw `y` turns
/// into `y + a` (the game's convention: facing = (sin, 0, cos)).
fn roty(v: Vec4, a: f32) -> Vec4 {
    let (s, c) = wrap(a).sin_cos();
    Vec4::new(v.x * c + v.z * s, v.y, -v.x * s + v.z * c, v.w)
}

/// Angles wrapped into [-pi, pi) the way the game does.
fn wrap(a: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    if a > 0.0 {
        (a + PI) % TAU - PI
    } else {
        PI - (PI - a) % TAU
    }
}

impl Host for Bridge<'_> {
    fn syscall(&mut self, vm: &mut Vm, ctx: CtxId, table: u8, index: u16, args: &mut [Cell]) -> Result<(), BdxError> {
        let ft = self.magic.frametime;
        if self.magic.trace {
            let name = kh2_bdx::trap_name(table, index).unwrap_or("?");
            let shown: Vec<String> = args.iter().map(|c| format!("{:?}:{:#x}/{}", c.tag, c.v, c.f())).collect();
            eprintln!("  [{}] ctx{ctx} pc{} {name}({})", self.magic.spells[self.t.spell].name, vm.ctxs[ctx].pc, shown.join(", "));
        }
        let a = |i: usize| args.get(i).copied().unwrap_or(Cell::int(0));
        let s = self.t.statics;
        let out: Option<Cell> = match (table, index) {
            // --- table 0: system ---
            (0, 3) => Some(Cell::float(ft)),
            (0, 4) | (0, 5) => {
                let (x, y) = (vec_in(vm, a(0).v)?, vec_in(vm, a(1).v)?);
                let (slot, r) = if index == 4 { (s.add, x + y) } else { (s.sub, x - y) };
                put(vm, slot, r)?;
                Some(Cell::addr(slot))
            }
            (0, 6) => Some(Cell::float(vec_in(vm, a(0).v)?.truncate().length())),
            (0, 13) => {
                let v = vec_in(vm, a(0).v)?;
                put(vm, s.roty, roty(v, a(1).f()))?;
                Some(Cell::addr(s.roty))
            }
            (0, 36) => {
                let v = vec_in(vm, a(0).v)?;
                put(vm, s.mul, v * a(1).f())?;
                Some(Cell::addr(s.mul))
            }
            (0, 38) => {
                let v = vec_in(vm, a(1).v)?;
                if let Some(e) = self.effect(a(0).v) {
                    e.placed = Some(host(v));
                    e.pos = host(v);
                }
                None
            }
            (0, 39) => {
                let v = vec_in(vm, a(1).v)?;
                if let Some(e) = self.effect(a(0).v) {
                    e.scale = v.truncate();
                }
                None
            }
            (0, 41) => {
                let v = vec_in(vm, a(1).v)?;
                if let Some(e) = self.effect(a(0).v) {
                    e.dir = flip(v.truncate());
                }
                None
            }
            (0, 44) => {
                if let Some(e) = self.effect(a(0).v) {
                    e.loop_end();
                }
                None
            }
            // v += normalize(dir) * speed * frametime
            (0, 45) => {
                let v = vec_in(vm, a(0).v)?;
                let d = vec_in(vm, a(1).v)?;
                let n = d.truncate().normalize_or_zero().extend(d.w);
                put(vm, a(0).v, v + n * a(2).f() * ft)?;
                None
            }
            // Turns v toward a direction by at most `max * frametime`
            // radians; the result is a unit vector.
            (0, 46) => {
                let v = vec_in(vm, a(0).v)?.truncate().normalize_or_zero();
                let to = vec_in(vm, a(1).v)?.truncate().normalize_or_zero();
                let cos = v.dot(to);
                let angle = if cos < 1.0 && cos > -1.0 { cos.acos() } else if cos >= 1.0 { 0.0 } else { std::f32::consts::PI };
                let max = a(2).f() * ft;
                let turn = if angle <= max && -max <= angle { angle } else { max };
                let axis = v.cross(to).normalize_or_zero();
                let r = if axis == Vec3::ZERO { v } else { Quat::from_axis_angle(axis, turn) * v };
                put(vm, a(0).v, r.extend(0.0))?;
                None
            }
            (0, 47) => Some(Cell::addr(vm.mem.alloc(a(0).i().max(4) as u32))),
            (0, 48) => None,
            (0, 49) | (0, 50) => {
                let alive = self.effect(a(0).v).is_some_and(|e| if index == 49 { e.alive() } else { e.active() });
                Some(b(alive))
            }
            (0, 51) => {
                if let Some(e) = self.effect(a(0).v) {
                    e.kill();
                }
                None
            }
            (0, 52) => {
                let frames = a(1).f();
                if let Some(e) = self.effect(a(0).v) {
                    e.fadeout(frames);
                }
                None
            }
            (0, 53) => {
                let p = self.effect(a(0).v).map(|e| e.pos).unwrap_or(Vec3::ZERO);
                put(vm, s.effect_pos, game(p))?;
                Some(Cell::addr(s.effect_pos))
            }
            // A save-file counter (friend recoveries).
            (0, 100) => None,

            // --- table 1: objects, effects, targets, background ---
            (1, 15) => Some(Cell::int(self.obj_of_handle(a(0).v).map_or(0, |o| self.world.motion(o) as i32))),
            (1, 57) => Some(b(self.obj_of_ref(vm, a(0).v).is_some_and(|o| self.world.jumping(o)))),
            (1, 60) => Some(b(self.obj_of_ref(vm, a(0).v).is_some_and(|o| self.world.airborne(o)))),
            (1, 83) => {
                let v = vec_in(vm, a(2).v)?;
                let o = self.obj_of_ref(vm, a(0).v);
                let m = o.and_then(|o| self.world.bone(o, a(1).v as u16)).unwrap_or(Mat4::IDENTITY);
                put(vm, s.shared, game(m.transform_point3(flip(v.truncate()))))?;
                Some(Cell::addr(s.shared))
            }
            (1, 88) => {
                let p = self.obj_of_ref(vm, a(0).v).map(|o| self.world.target_point(o)).unwrap_or(Vec3::ZERO);
                put(vm, s.shared, game(p))?;
                Some(Cell::addr(s.shared))
            }
            // MP orbs ("stars") for the caster: not modelled.
            (1, 91) => None,
            (1, 93) => {
                let h = match self.world.party(a(0).v) {
                    Some(o) => obj_record(vm, self.t, o),
                    None => 0,
                };
                Some(Cell::addr(h))
            }
            (1, 94) => Some(b(self.obj_of_handle(a(0).v).is_some_and(|o| self.world.exists(o)))),
            (10, 31) => Some(Cell::int(0)),
            (1, 120) => {
                let rec = a(0).v;
                let h = vm.mem.u32(rec)?;
                let atk = vm.mem.u32(rec + 8)?;
                let p = if let Some(p) = (atk != 0).then(|| self.attack(atk).and_then(|x| x.pos)).flatten() {
                    game(p)
                } else if let Some(o) = self.obj_of_handle(h).filter(|o| self.world.exists(*o)) {
                    game(self.world.target_point(o))
                } else {
                    Vec4::new(0.0, 0.0, 0.0, 1.0)
                };
                put(vm, s.shared, p)?;
                Some(Cell::addr(s.shared))
            }
            (1, 130) => {
                let (x, y) = (a(0).v, a(1).v);
                let r = if x != 0 && y != 0 { vm.mem.u32(x + 4)?.wrapping_sub(vm.mem.u32(y + 4)?) as i32 } else { 0 };
                Some(Cell::int(r))
            }
            (1, 131) => {
                let r = vm.mem.alloc(16);
                vm.mem.copy(r, a(0).v, 16)?;
                Some(Cell::addr(r))
            }
            (1, 132) => None,
            (1, 138) => Some(Cell::addr(self.start_effect(vm, a(0).v, a(1).v as u16, None))),
            // obj_effect_start(obj, element, _, _): the object's own effect
            // list (an object script's file).
            (1, 21) => {
                let owner = self.obj_of_ref(vm, a(0).v);
                let pax = self.t.pax_rec;
                Some(Cell::addr(self.start_effect(vm, pax, a(1).v as u16, owner)))
            }
            (1, 139) => {
                let owner = self.obj_of_ref(vm, a(2).v);
                Some(Cell::addr(self.start_effect(vm, a(0).v, a(1).v as u16, owner)))
            }
            (1, 140) => {
                let rec = a(0).v;
                let h = vm.mem.u32(rec)?;
                let atk = vm.mem.u32(rec + 8)?;
                Some(b(atk != 0 || self.obj_of_handle(h).is_some_and(|o| self.world.exists(o))))
            }
            (1, 141) => {
                let p = host(vec_in(vm, a(0).v)?);
                let hit = self.world.ground(p, a(2).f());
                put(vm, a(1).v, game(hit.unwrap_or(p)))?;
                Some(b(hit.is_some()))
            }
            (1, 146) => {
                let mut d = flip(vec_in(vm, a(1).v)?.truncate());
                let mut dist = a(2).f();
                if dist < 0.0 {
                    d = -d;
                    dist = -dist;
                }
                if let Some(o) = self.obj_of_ref(vm, a(0).v) {
                    self.world.hitback(o, d.normalize_or_zero(), dist);
                }
                None
            }
            (1, 147) => {
                let p = self.obj_of_ref(vm, a(0).v).map(|o| self.world.pos(o)).unwrap_or(Vec3::ZERO);
                put(vm, s.shared, game(p))?;
                Some(Cell::addr(s.shared))
            }
            // Prize (orb) attraction on/off.
            (1, 187) | (1, 188) => None,
            (1, 201) => {
                let f = self.obj_of_ref(vm, a(0).v).map(|o| self.world.facing(o)).unwrap_or(0.0);
                put(vm, s.shared, Vec4::new(f.sin(), 0.0, f.cos(), f))?;
                Some(Cell::addr(s.shared))
            }
            // A random point around `pos`: horizontal distance in [a, b],
            // height offset in [c, d] (approximation of the game's search).
            (1, 248) => {
                let p = host(vec_in(vm, a(0).v)?);
                let (r0, r1, y0, y1) = (a(1).f(), a(2).f(), a(3).f(), a(4).f());
                let ang = self.world.random() * std::f32::consts::TAU;
                let r = r0 + (r1 - r0) * self.world.random();
                let dy = y0 + (y1 - y0) * self.world.random();
                let q = p + Vec3::new(ang.sin() * r, -dy, ang.cos() * r);
                put(vm, s.shared, game(q))?;
                Some(Cell::addr(s.shared))
            }
            (1, 343) => Some(b(self.obj_of_ref(vm, a(0).v).is_some_and(|o| self.world.has_ability(o, a(1).v)))),

            // --- table 2: battle ---
            (2, 10) => {
                let owner = self.obj_of_ref(vm, a(0).v).unwrap_or(self.t.caster);
                Some(Cell::addr(self.new_attack(vm, owner, a(1).v as u16, a(3).v as u8, None)))
            }
            (2, 11) => {
                let (r, h) = (a(1).f(), a(2).f());
                if let Some(x) = self.attack(a(0).v) {
                    x.radius = r;
                    x.height = if h > 0.0 { h } else { r };
                }
                None
            }
            (2, 12) => {
                let p = host(vec_in(vm, a(1).v)?);
                if let Some(x) = self.attack(a(0).v) {
                    x.set_pos(p, ft);
                }
                None
            }
            (2, 13) => {
                if let Some(x) = self.attack(a(0).v) {
                    x.freed = true;
                }
                None
            }
            (2, 14) => Some(b(self.attack(a(0).v).is_some_and(|x| x.hit || x.hit_bg || x.reflected))),
            (2, 17) => {
                let target = self.obj_of_ref(vm, a(1).v);
                let id = self.attack(a(0).v).map(|x| x.key);
                if let (Some(attack), Some(target)) = (id, target) {
                    self.magic.events.push(Event::Strike { attack, target });
                }
                None
            }
            (2, 20) => {
                let owner = vm.owner.unwrap_or(ctx);
                let wp = vm.ctxs[owner].wp;
                let c = vm.spawn(Some(wp));
                vm.start_at(c, a(0).v, &[a(1)])?;
                self.t.threads.push(c);
                let rec = vm.mem.alloc(16);
                vm.mem.set_u32(rec, c as u32)?;
                Some(Cell::addr(rec))
            }
            (2, 21) => {
                let key = (a(0).v, a(1).v);
                let r = match self.t.teamwork.get(&key) {
                    Some(r) => *r,
                    None => {
                        let r = vm.mem.alloc(64);
                        self.t.teamwork.insert(key, r);
                        r
                    }
                };
                Some(Cell::addr(r))
            }
            (2, 23) => {
                let o = self.obj_of_ref(vm, a(0).v).unwrap_or(self.t.caster);
                Some(Cell::addr(self.target_record(vm, o)?))
            }
            (2, 29) => Some(b(self.attack(a(0).v).is_some_and(|x| x.hit_bg))),
            // Hit effects come from the spell's own effects; not drawn.
            (2, 30) | (2, 94) => None,
            (2, 31) => {
                let atk = a(1).v as u16;
                let src = self.attack(a(0).v).map(|x| (x.owner, x.sub, x.id, x.group));
                let h = match src {
                    Some((owner, sub, id, group)) => self.new_attack(vm, owner, atk, sub, Some((id, group))),
                    None => 0,
                };
                Some(Cell::addr(h))
            }
            (2, 39) => Some(b(self.attack(a(0).v).is_some_and(|x| x.reflected))),
            (2, 66) => Some(Cell::int(self.attack(a(0).v).map_or(0, |x| x.reflect_count as i32))),
            _ => {
                self.missing(table, index);
                Some(Cell::int(0))
            }
        };
        if let Some(c) = out {
            args[0] = c;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roty_turns_facing() {
        let f = 0.3f32;
        let v = Vec4::new(f.sin(), 0.0, f.cos(), 0.0);
        let r = roty(v, 0.5);
        assert!((r.x - 0.8f32.sin()).abs() < 1e-5 && (r.z - 0.8f32.cos()).abs() < 1e-5);
        assert!((wrap(4.0) - (4.0 - std::f32::consts::TAU)).abs() < 1e-5);
    }
}
