//! The traps the spell scripts call, written from plain-language notes on
//! the game's trap handlers (see `docs/magic-model.md`).
//!
//! Script-visible objects are 16-byte records in script memory whose
//! second word holds their own handle, mirroring how the game hands
//! objects to scripts (a script copies the caster's header and passes the
//! copy's address; traps read the object handle at +4). Vectors the traps
//! return live in per-trap static slots, as in the game, so a script that
//! keeps the pointer sees it overwritten by the next call.

use glam::{EulerRot, Mat3, Mat4, Quat, Vec3, Vec4};
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
    outer: u32,
    rot: u32,
    inter: u32,
    xyzrot: u32,
}

impl Statics {
    pub fn new(vm: &mut Vm) -> Statics {
        let mut slot = || {
            let a = vm.mem.alloc(16);
            vm.mem.set_f32(a + 12, 1.0).ok();
            a
        };
        Statics { add: slot(), sub: slot(), mul: slot(), roty: slot(), effect_pos: slot(), shared: slot(), outer: slot(), rot: slot(), inter: slot(), xyzrot: slot() }
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
    // Objects' hooks first (`trap_obj_hook`): they run in the object's own update, before the
    // scripts that run after it (ours, from Limit Form's script: its main loop ends the limit on
    // a motion its hook hasn't taken over yet, so the hook must have run first).
    let hooks: Vec<CtxId> = b.t.hooks.values().copied().collect();
    for c in hooks {
        vm.owner = None;
        if vm.run_tree(c, &mut b)? {
            b.t.hooks.retain(|_, x| *x != c);
        }
    }
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
    cast.finished = main_done && cast.t.threads.is_empty() && cast.t.hooks.is_empty();
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

    /// A target record set to object `o` (or cleared).
    fn set_target(&mut self, vm: &mut Vm, rec: u32, o: Option<ObjId>) -> Result<(), BdxError> {
        let h = match o {
            Some(o) => obj_record(vm, self.t, o),
            None => 0,
        };
        vm.mem.set_u32(rec, h)?;
        vm.mem.set_u32(rec + 8, 0)
    }

    /// Where a target record points (its attack's place, else its object's aim point).
    fn target_at(&mut self, vm: &Vm, rec: u32) -> Result<Option<Vec3>, BdxError> {
        let h = vm.mem.u32(rec)?;
        let atk = vm.mem.u32(rec + 8)?;
        if let Some(p) = (atk != 0).then(|| self.attack(atk).and_then(|x| x.pos)).flatten() {
            return Ok(Some(p));
        }
        Ok(self.obj_of_handle(h).filter(|o| self.world.exists(*o)).map(|o| self.world.target_point(o)))
    }

    /// A thread at code `pc` with `arg`, sharing the work memory of the tree running now.
    fn thread(&mut self, vm: &mut Vm, ctx: CtxId, pc: u32, arg: Cell) -> Result<CtxId, BdxError> {
        let owner = vm.owner.unwrap_or(ctx);
        let wp = vm.ctxs[owner].wp;
        let c = vm.spawn(Some(wp));
        vm.start_at(c, pc, &[arg])?;
        self.t.threads.push(c);
        Ok(c)
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
                let c = self.thread(vm, ctx, a(0).v, a(1))?;
                let rec = vm.mem.alloc(16);
                vm.mem.set_u32(rec, c as u32)?;
                Some(Cell::addr(rec))
            }
            // trap_limit_start_thread(code, arg): as a magic thread (no handle back).
            (2, 54) => {
                self.thread(vm, ctx, a(0).v, a(1))?;
                None
            }

            // --- a limit (KH2's Limit Form `limi`, `Magic::limit`) ---
            // trap_limit_start_command: the limit chosen.
            (7, 27) => Some(Cell::int(self.t.limit_command as i32)),
            // trap_limit_player(limit): its user's object; trap_limit_friend(limit, n): none
            // (Limit Form's limits are Sora's alone).
            (2, 48) => Some(Cell::addr(obj_record(vm, self.t, self.t.caster))),
            (2, 49) => Some(Cell::int(0)),
            // trap_limit_time(limit): the team limits' time; trap_limit_reference(limit): what
            // still holds the limit. Neither applies to a solo limit (ours: 0, so the script's
            // waits on them end at once).
            (2, 60) => Some(Cell::float(0.0)),
            (2, 63) => Some(Cell::int(0)),
            // trap_limit_light / trap_light_set_color: the scene's light during a limit, and
            // trap_area_world (the script picks other colours in world 13): not modelled.
            (2, 55) => Some(Cell::int(0)),
            (1, 183) => None,
            (1, 23) => Some(Cell::int(0)),
            // trap_obj_stop_end_all(kind): not modelled.
            (6, 47) => None,
            // trap_obj_hook(obj, code, _): the object's hook, a thread with the object as its
            // argument; a new one replaces the object's last.
            (1, 112) => {
                if let Some(o) = self.obj_of_ref(vm, a(0).v) {
                    if let Some(old) = self.t.hooks.remove(&o) {
                        vm.ctxs[old].done = true;
                        vm.ctxs[old].children.clear();
                    }
                    // (Run before the others each frame, `run`: not in the thread list.)
                    let c = self.thread(vm, ctx, a(1).v, a(0))?;
                    self.t.threads.retain(|x| *x != c);
                    self.t.hooks.insert(o, c);
                }
                None
            }
            // trap_sysobj_motion_start / _push (handle, motion, blend), trap_sysobj_motion_is_end.
            (1, 11) | (1, 13) => {
                if let Some(o) = self.obj_of_handle(a(0).v) {
                    if index == 11 {
                        self.world.motion_start(o, a(1).v as u16, a(2).f());
                    } else {
                        self.world.motion_push(o, a(1).v as u16, a(2).f());
                    }
                }
                None
            }
            (1, 14) => Some(b(self.obj_of_handle(a(0).v).map_or(true, |o| self.world.motion_ended(o)))),
            // trap_obj_motion_check_range / _check_trigger (obj, n).
            (1, 150) => Some(b(self.obj_of_ref(vm, a(0).v).is_some_and(|o| self.world.motion_range(o, a(1).v as u8)))),
            (1, 151) => Some(b(self.obj_of_ref(vm, a(0).v).is_some_and(|o| self.world.motion_trigger(o, a(1).v as u8)))),
            // trap_command_override(slot, command, obj, on): the command on the reaction button.
            (1, 165) => {
                if a(3).v != 0 {
                    if let Some(o) = self.obj_of_ref(vm, a(2).v) {
                        self.world.offer_command(o, a(1).v as u16);
                    }
                }
                None
            }
            // trap_obj_can_decide_command(obj, command, _): ours, yes (the friend's offers).
            (1, 221) => Some(b(true)),
            (1, 222) => Some(Cell::int(self.obj_of_ref(vm, a(0).v).map_or(0, |o| self.world.entry_id(o)) as i32)),
            // trap_obj_wish_dir(obj, dir).
            (1, 4) => {
                let d = host(vec_in(vm, a(1).v)?);
                if let Some(o) = self.obj_of_ref(vm, a(0).v) {
                    self.world.face(o, d);
                }
                None
            }
            // trap_obj_set_movement(obj, dir, speed): ours, nothing (the motions' root motion
            // moves him; the scripts set speed 0 before a motion of their own).
            (1, 111) => None,
            (1, 272) => Some(b(self.obj_of_ref(vm, a(0).v).map_or(true, |o| !self.world.exists(o)))),
            // trap_obj_party_attack(obj, row).
            (6, 67) => {
                if let Some(o) = self.obj_of_ref(vm, a(0).v) {
                    self.world.party_attack(o, a(1).v as u16);
                }
                None
            }
            // trap_obj_effect_start_bind(obj, element, _, _): as `trap_obj_effect_start`.
            (1, 87) => {
                let owner = self.obj_of_ref(vm, a(0).v);
                let pax = self.t.pax_rec;
                Some(Cell::addr(self.start_effect(vm, pax, a(1).v as u16, owner)))
            }
            // trap_obj_effect_unbind(obj, effect): it stays where it is.
            (1, 169) => {
                if let Some(e) = self.effect(a(1).v) {
                    e.placed = Some(e.pos);
                }
                None
            }
            // trap_effect_loop_end_kill(effect).
            (0, 85) => {
                if let Some(e) = self.effect(a(0).v) {
                    e.loop_end();
                }
                None
            }
            // trap_puts: the script's own log lines.
            (0, 2) => None,

            // --- vectors and random numbers (Limit Form's Strike Raid and Infinity) ---
            // trap_vector_normalize(v): in place; the length it had.
            (0, 7) => {
                let v = vec_in(vm, a(0).v)?;
                let len = v.truncate().length();
                if len > 0.0 {
                    put(vm, a(0).v, (v.truncate() / len).extend(v.w))?;
                }
                Some(Cell::float(len))
            }
            // trap_random_get(n): 0..n; trap_random_getf(x): 0..x; trap_random_range(a, b).
            (0, 16) => {
                let n = a(0).i().max(1);
                Some(Cell::int(((self.world.random() * n as f32) as i32).clamp(0, n - 1)))
            }
            (0, 17) => Some(Cell::float(self.world.random() * a(0).f())),
            (0, 18) => {
                let (lo, hi) = (a(0).f(), a(1).f());
                Some(Cell::float(lo + (hi - lo) * self.world.random()))
            }
            (0, 43) => Some(Cell::float(wrap(a(0).f()))),
            // trap_vector_inter(a, b, t): a toward b by t.
            (0, 78) => {
                let (x, y) = (vec_in(vm, a(0).v)?, vec_in(vm, a(1).v)?);
                put(vm, s.inter, x.lerp(y, a(2).f()))?;
                Some(Cell::addr(s.inter))
            }
            // trap_vector_outer_product(a, b): a × b.
            (0, 82) => {
                let (x, y) = (vec_in(vm, a(0).v)?, vec_in(vm, a(1).v)?);
                put(vm, s.outer, x.truncate().cross(y.truncate()).extend(0.0))?;
                Some(Cell::addr(s.outer))
            }
            // trap_vector_rot(v, axis, angle): v turned about the axis.
            (0, 83) => {
                let (v, axis) = (vec_in(vm, a(0).v)?, vec_in(vm, a(1).v)?.truncate().normalize_or_zero());
                let r = if axis == Vec3::ZERO { v.truncate() } else { Quat::from_axis_angle(axis, a(2).f()) * v.truncate() };
                put(vm, s.rot, r.extend(v.w))?;
                Some(Cell::addr(s.rot))
            }
            (0, 91) => {
                let (x, y) = (vec_in(vm, a(0).v)?, vec_in(vm, a(1).v)?);
                Some(Cell::float(x.truncate().dot(y.truncate())))
            }

            // --- an object's weapon as an object (Strike Raid's thrown Keyblade, `Loose`) ---
            // trap_party_get_weapon(obj, n): its handle.
            (1, 234) => {
                let w = self.obj_of_ref(vm, a(0).v).and_then(|o| self.world.weapon(o, a(1).v));
                Some(Cell::addr(match w {
                    Some(w) => obj_record(vm, self.t, w),
                    None => 0,
                }))
            }
            // trap_party_hand_to_bone(obj, n): the hand's bone (ours: none to name, the host's
            // weapon goes back to its own hand).
            (1, 235) => Some(Cell::int(0)),
            (1, 161) => Some(b(self.obj_of_ref(vm, a(0).v).map_or(true, |o| self.world.attached(o)))),
            (1, 102) => {
                if let Some(o) = self.obj_of_ref(vm, a(0).v) {
                    self.world.detach(o);
                }
                None
            }
            // trap_obj_attach(obj, to, bone, ...).
            (1, 18) => {
                if let (Some(o), Some(to)) = (self.obj_of_ref(vm, a(0).v), self.obj_of_ref(vm, a(1).v)) {
                    self.world.attach(o, to);
                }
                None
            }
            // trap_obj_motion_sync / _unsync (obj, weapon): the weapon's motion following its
            // holder's; ours, nothing (the host poses a held weapon by its holder's motion).
            (1, 126) | (1, 236) => None,
            (1, 148) => {
                let p = host(vec_in(vm, a(1).v)?);
                if let Some(o) = self.obj_of_ref(vm, a(0).v) {
                    self.world.set_pos(o, p);
                }
                None
            }
            // trap_obj_move_request(obj, dir, speed): moved by dir × speed this frame.
            (1, 89) => {
                let d = flip(vec_in(vm, a(1).v)?.truncate()).normalize_or_zero();
                if let Some(o) = self.obj_of_ref(vm, a(0).v) {
                    self.world.move_by(o, d * a(2).f() * ft);
                }
                None
            }
            // trap_strike_raid_calc_xyzrot(dir, up) (exe `0x14053EF90`): the turn of an object
            // facing `dir` with `up` as its up, as Euler angles (ours: X, then Y, then Z, as the
            // bones' and `trap_obj_set_xyzrot`'s); `up` is written back square to `dir`.
            (6, 68) => {
                let (dir, up) = (a(0).v, a(1).v);
                let d = vec_in(vm, dir)?.truncate().normalize_or_zero();
                let mut r = vec_in(vm, up)?.truncate().normalize_or_zero();
                if r == d {
                    r = Vec3::NEG_Y;
                }
                let mut x = r.cross(d).normalize_or_zero();
                // (Ours: an `up` along `dir` is no up at all; the world's is taken. The game's
                // cross product is then zero.)
                if x == Vec3::ZERO {
                    x = Vec3::NEG_Y.cross(d).normalize_or_zero();
                    if x == Vec3::ZERO {
                        x = Vec3::X;
                    }
                }
                let z = -d;
                put(vm, up, d.cross(x).extend(0.0))?;
                let y = z.cross(x).normalize_or_zero();
                let m = Mat3::from_cols(x, y, z);
                let (rz, ry, rx) = Quat::from_mat3(&m).normalize().to_euler(EulerRot::ZYX);
                put(vm, s.xyzrot, Vec4::new(rx, ry, rz, 0.0))?;
                Some(Cell::addr(s.xyzrot))
            }
            // trap_obj_set_xyzrot(obj, rot): its turn (game axes) to ours.
            (8, 2) => {
                let v = vec_in(vm, a(1).v)?;
                let m = Mat3::from_quat(Quat::from_euler(EulerRot::ZYX, v.z, v.y, v.x));
                let f = Mat3::from_diagonal(Vec3::new(1.0, -1.0, 1.0));
                if let Some(o) = self.obj_of_ref(vm, a(0).v) {
                    self.world.set_rotation(o, f * m * f);
                }
                None
            }
            // trap_obj_effect_start_bind_other(obj, element, other, ...): obj's element, bound to
            // `other` (the thrown Keyblade's trail).
            (1, 149) => {
                let owner = self.obj_of_ref(vm, a(2).v);
                let pax = self.t.pax_rec;
                Some(Cell::addr(self.start_effect(vm, pax, a(1).v as u16, owner)))
            }
            // trap_obj_effect_start_pos(obj, element, pos, ...): unbound, there.
            (1, 22) => {
                let p = host(vec_in(vm, a(2).v)?);
                let pax = self.t.pax_rec;
                let rec = self.start_effect(vm, pax, a(1).v as u16, None);
                if let Some(e) = self.effect(rec) {
                    e.placed = Some(p);
                    e.pos = p;
                }
                Some(Cell::addr(rec))
            }
            // trap_attack_set_obj_pax(attack, obj): its hit effects from obj's (not drawn).
            (2, 22) => None,

            // --- targets (Infinity's shots, Strike Raid's throw) ---
            // trap_btlobj_lockon_target(obj): a record holding its lock-on target (0: none).
            (2, 56) => {
                let t = self.obj_of_ref(vm, a(0).v).and_then(|o| self.world.target_of(o)).filter(|o| self.world.exists(*o));
                let rec = vm.mem.alloc(16);
                let h = match t {
                    Some(o) => obj_record(vm, self.t, o),
                    None => 0,
                };
                vm.mem.set_u32(rec, h)?;
                Some(Cell::addr(rec))
            }
            // trap_target_set_obj(target, obj).
            (1, 160) => {
                let o = if a(1).v == 0 { None } else { self.obj_of_ref(vm, a(1).v) };
                self.set_target(vm, a(0).v, o)?;
                None
            }
            (6, 16) => {
                self.set_target(vm, a(0).v, None)?;
                None
            }
            (6, 57) => Some(Cell::int(0)),
            // trap_target_dist(target, pos).
            (6, 9) => {
                let p = host(vec_in(vm, a(1).v)?);
                let d = self.target_at(vm, a(0).v)?.map_or(f32::MAX, |t| t.distance(p));
                Some(Cell::float(d))
            }
            // trap_target_searcher_init(searcher, n); _search(searcher, target, pos, dir, ...): the
            // nearest enemy within the searcher's range ([+16], [+20]) it hasn't found yet (ours:
            // KH2's own choice isn't traced); _get_old(searcher, target, i): its find i.
            (6, 3) => {
                self.t.searchers.insert(a(0).v, Vec::new());
                None
            }
            (6, 5) => {
                let rec = a(0).v;
                let (lo, hi) = (vm.mem.f32(rec + 16)?, vm.mem.f32(rec + 20)?);
                let p = host(vec_in(vm, a(2).v)?);
                let found = self.t.searchers.get(&rec).cloned().unwrap_or_default();
                let pick = self
                    .world
                    .foes()
                    .into_iter()
                    .filter(|o| self.world.exists(*o) && !found.contains(o))
                    .map(|o| (o, self.world.target_point(o).distance(p)))
                    .filter(|(_, d)| *d >= lo && (hi <= lo || *d <= hi))
                    .min_by(|x, y| x.1.total_cmp(&y.1))
                    .map(|x| x.0);
                if let Some(o) = pick {
                    self.t.searchers.entry(rec).or_default().push(o);
                }
                self.set_target(vm, a(1).v, pick)?;
                None
            }
            (6, 12) => {
                let o = self.t.searchers.get(&a(0).v).and_then(|l| l.get(a(2).i().max(0) as usize).copied());
                self.set_target(vm, a(1).v, o)?;
                None
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
