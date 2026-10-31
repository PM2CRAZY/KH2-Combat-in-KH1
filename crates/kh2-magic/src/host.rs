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
    /// KH2's one result vector of `trap_vector_inter` / `_outer_product` / `_rot` (`0x1407535C8`).
    vec: u32,
}

impl Statics {
    pub fn new(vm: &mut Vm) -> Statics {
        let mut slot = || {
            let a = vm.mem.alloc(16);
            vm.mem.set_f32(a + 12, 1.0).ok();
            a
        };
        Statics { add: slot(), sub: slot(), mul: slot(), roty: slot(), effect_pos: slot(), shared: slot(), vec: slot() }
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
fn start_effect_in(magic: &mut Magic, vm: &mut Vm, t: &mut Tables, world: &mut dyn World, pax: u32, id: u16, owner: Option<ObjId>) -> u32 {
    if pax != t.pax_rec {
        return 0;
    }
    let spell = t.spell;
    let file = &magic.spells[spell];
    let Some(element) = file.pax.element(id).cloned() else { return 0 };
    let timing = file.pax.effect(element.effect).cloned().unwrap_or(kh2_formats::pax::PaxEffect { number: element.effect as u32, emitters: Vec::new() });
    let eid = magic.fresh_id();
    let mut e = Effect::new(eid, spell, element, timing, owner, &*world);
    // (The host's own instance for it, started by this call, t820.)
    e.host = world.effect_start(&e);
    magic.effects.push(e);
    let rec = vm.mem.alloc(16);
    t.effects.insert(rec, eid);
    rec
}

/// The muzzle effect of an object's frame trigger 1 (its own effect list).
pub(crate) fn start_object_effect(magic: &mut Magic, vm: &mut Vm, t: &mut Tables, id: u16, owner: ObjId, world: &mut dyn World) -> u32 {
    let pax = t.pax_rec;
    start_effect_in(magic, vm, t, world, pax, id, Some(owner))
}

pub(crate) fn run(magic: &mut Magic, cast: &mut Cast, world: &mut dyn World) -> Result<(), BdxError> {
    run_ctx(magic, cast, None, world)
}

/// `run`, or (`only`) one of the cast's threads on the host's child processes alone.
pub(crate) fn run_ctx(magic: &mut Magic, cast: &mut Cast, only: Option<CtxId>, world: &mut dyn World) -> Result<(), BdxError> {
    let main = cast.main;
    let vm = &mut cast.vm;
    // The teamwork blocks this cast holds: KH2's are one block per key for every script
    // (`trap_teamwork_alloc`), ours a copy in each cast's memory, brought up to date before it
    // runs and handed back after (scripts run one after another, so this is the same).
    for (key, addr) in &cast.t.teamwork {
        if let Some(block) = magic.teamwork.get(key) {
            vm.mem.write(*addr, block)?;
        }
    }
    let result = match only {
        None => run_trees(magic, vm, &mut cast.t, main, world).map(|_| ()),
        Some(c) => run_one(magic, vm, &mut cast.t, c, world),
    };
    for (key, addr) in &cast.t.teamwork {
        if let Some(block) = magic.teamwork.get_mut(key) {
            let n = block.len() as u32;
            block.copy_from_slice(vm.mem.bytes(*addr, n)?);
        }
    }
    result?;
    let main_done = vm.ctxs[main].done && vm.ctxs[main].children.is_empty();
    cast.finished = main_done && cast.t.threads.is_empty() && cast.t.ext_threads.is_empty();
    Ok(())
}

/// Trigger `key` of the cast's program run at once to its end on a context of its own with the main
/// context's work memory (a limit's follow-up, `Magic::limit_command`). False: no such trigger.
pub(crate) fn run_trigger(magic: &mut Magic, cast: &mut Cast, key: i32, args: &[Cell], world: &mut dyn World) -> Result<bool, BdxError> {
    let Some(entry) = cast.vm.prog.trigger(key) else { return Ok(false) };
    let wp = cast.vm.ctxs[cast.main].wp;
    let c = cast.vm.spawn(Some(wp));
    cast.vm.start_at(c, entry, args)?;
    let mut b = Bridge { magic, t: &mut cast.t, world };
    cast.vm.owner = Some(cast.main);
    let st = cast.vm.run(c, &mut b);
    cast.vm.ctxs[c].done = true;
    cast.vm.owner = None;
    match st? {
        kh2_bdx::Status::Yield => b.magic.events.push(Event::Error(format!("trigger {key:#x} halted (not run on)"))),
        _ => {}
    }
    Ok(true)
}

/// One thread on the host's child process (its tree), dropped from the cast's once done.
fn run_one(magic: &mut Magic, vm: &mut Vm, t: &mut Tables, c: CtxId, world: &mut dyn World) -> Result<(), BdxError> {
    let mut b = Bridge { magic, t, world };
    vm.owner = None;
    if vm.run_tree(c, &mut b)? {
        b.t.ext_threads.retain(|x| *x != c);
    }
    Ok(())
}

/// Runs the cast's main tree and its threads; whether the main one is done.
fn run_trees(magic: &mut Magic, vm: &mut Vm, t: &mut Tables, main: CtxId, world: &mut dyn World) -> Result<bool, BdxError> {
    let mut b = Bridge { magic, t, world };
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
    Ok(main_done)
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
        let id = match self.t.effects.get(&h) {
            Some(id) => *id,
            // Another cast's effect, by its handle (Reflect's chain: the new cast's release ends
            // the last one's barrier element through the teamwork block they share, as KH2 does;
            // until t820 it was looked up in the new cast's own effects only and stayed). Only on
            // the host's memory, where every cast's handles are its own addresses.
            None if self.t.native_id.is_some() => self.magic.casts.iter().filter(|c| c.t.native_id.is_some()).find_map(|c| c.t.effects.get(&h).copied())?,
            None => return None,
        };
        self.magic.effects.iter_mut().find(|e| e.id == id)
    }

    fn start_effect(&mut self, vm: &mut Vm, pax: u32, id: u16, owner: Option<ObjId>) -> u32 {
        start_effect_in(self.magic, vm, self.t, &mut *self.world, pax, id, owner)
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
            freed_seen: false,
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

    /// A thread of the cast's at `pc` with `args` (on a child of the host's process when the cast is
    /// on one, as `trap_magic_start_thread`'s): the context and whether it is the host's. `None`: the
    /// host had no process for it.
    fn thread_at(&mut self, vm: &mut Vm, ctx: CtxId, pc: u32, args: &[Cell]) -> Result<Option<(CtxId, bool)>, BdxError> {
        let owner = vm.owner.unwrap_or(ctx);
        let wp = vm.ctxs[owner].wp;
        let id = self.t.native_id;
        let hosted = match (id, self.magic.native.as_mut()) {
            (Some(id), Some(n)) => Some(n.thread(id, vm, wp)),
            _ => None,
        };
        if let Some(None) = hosted {
            return Ok(None);
        }
        let ext = hosted.flatten();
        let c = ext.unwrap_or_else(|| vm.spawn(Some(wp)));
        vm.start_at(c, pc, args)?;
        match (ext, id, self.magic.native.as_mut()) {
            (Some(_), Some(id), Some(n)) => {
                n.store(id, vm, c);
                self.t.ext_threads.push(c);
                Ok(Some((c, true)))
            }
            _ => {
                self.t.threads.push(c);
                Ok(Some((c, false)))
            }
        }
    }

    /// A target record pointed at `o` (none: 0), its other words kept.
    fn set_target(&mut self, vm: &mut Vm, rec: u32, o: Option<ObjId>) -> Result<(), BdxError> {
        let h = match o {
            Some(o) => obj_record(vm, self.t, o),
            None => 0,
        };
        vm.mem.set_u32(rec, h)?;
        vm.mem.set_u32(rec + 8, 0)
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
                let now = self.effect(a(0).v).map(|e| {
                    e.placed = Some(host(v));
                    e.pos = host(v);
                    e.clone()
                });
                if let Some(e) = now {
                    self.world.effect_place(&e);
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
                let now = self.effect(a(0).v).map(|e| {
                    e.dir = flip(v.truncate());
                    e.clone()
                });
                if let Some(e) = now {
                    self.world.effect_place(&e);
                }
                None
            }
            (0, 44) => {
                let now = self.effect(a(0).v).map(|e| {
                    e.loop_end();
                    e.clone()
                });
                if let Some(e) = now {
                    if self.world.effect_end(&e) {
                        if let Some(e) = self.effect(a(0).v) {
                            e.host_ended = true;
                        }
                    }
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
                let now = self.effect(a(0).v).map(|e| {
                    e.kill();
                    e.clone()
                });
                if let Some(e) = now {
                    self.world.effect_kill(&e);
                }
                None
            }
            (0, 52) => {
                let frames = a(1).f();
                let now = self.effect(a(0).v).map(|e| {
                    e.fadeout(frames);
                    e.clone()
                });
                if let Some(e) = now {
                    if self.world.effect_fade(&e, frames) {
                        if let Some(e) = self.effect(a(0).v) {
                            e.host_fading = true;
                        }
                    }
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
            // obj_star(obj, frames): the object takes no hit for that long (KH2's `+0xD70`).
            (1, 91) => {
                if let Some(o) = self.obj_of_ref(vm, a(0).v) {
                    self.world.star(o, a(1).f());
                }
                None
            }
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
                let now = self.attack(a(0).v).map(|x| {
                    x.radius = r;
                    x.height = if h > 0.0 { h } else { r };
                    x.clone()
                });
                if let Some(x) = now {
                    self.world.attack_set(&x);
                }
                None
            }
            (2, 12) => {
                let p = host(vec_in(vm, a(1).v)?);
                let now = self.attack(a(0).v).map(|x| {
                    x.set_pos(p, ft);
                    x.clone()
                });
                if let Some(x) = now {
                    self.world.attack_set(&x);
                }
                None
            }
            (2, 13) => {
                let now = self.attack(a(0).v).map(|x| {
                    x.freed = true;
                    x.clone()
                });
                if let Some(x) = now {
                    self.world.attack_free(&x);
                }
                None
            }
            (2, 14) => {
                let x = self.attack(a(0).v).map(|x| x.clone());
                Some(b(x.is_some_and(|x| x.hit || x.hit_bg || x.reflected || self.world.attack_hit(&x) || self.world.attack_hit_bg(&x))))
            }
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
                // (On the host's machine: a child process of the cast's, stepped after it. No
                // fallback onto our arena then: a thread the host can't make doesn't start and the
                // call gives a null record, t886.)
                let id = self.t.native_id;
                let hosted = match (id, self.magic.native.as_mut()) {
                    (Some(id), Some(n)) => Some(n.thread(id, vm, wp)),
                    _ => None,
                };
                match hosted {
                    Some(None) => Some(Cell::addr(0)),
                    _ => {
                        let c = hosted.flatten().unwrap_or_else(|| vm.spawn(Some(wp)));
                        vm.start_at(c, a(0).v, &[a(1)])?;
                        match (hosted.flatten(), id, self.magic.native.as_mut()) {
                            (Some(_), Some(id), Some(n)) => {
                                n.store(id, vm, c);
                                self.t.ext_threads.push(c);
                            }
                            _ => self.t.threads.push(c),
                        }
                        let rec = vm.mem.alloc(16);
                        vm.mem.set_u32(rec, c as u32)?;
                        Some(Cell::addr(rec))
                    }
                }
            }
            (2, 21) => {
                // KH2's `trap_teamwork_alloc (key, size)` (Steam `0x1403F6DF0`, Checked (decomp)): one
                // block per key for every script, made zeroed `size + 1` bytes the first time;
                // asked again, its first byte is set to 1 and the same block comes back. Reflect's
                // script keeps there its newest barrier's element and the hits counted, so a new
                // Reflect ends the last one's barrier and its counter counts both (`docs/kh1-reflect-spell.md`).
                let (key, size) = (a(0).v, a(1).i().max(0) as usize);
                match self.magic.teamwork.get_mut(&key) {
                    Some(block) => block[0] = 1,
                    None => {
                        self.magic.teamwork.insert(key, vec![0; size + 1]);
                    }
                }
                let block = self.magic.teamwork[&key].clone();
                let r = match self.t.teamwork.get(&key) {
                    Some(r) => *r,
                    None => {
                        let r = vm.mem.alloc((block.len() as u32).max(4));
                        self.t.teamwork.insert(key, r);
                        r
                    }
                };
                vm.mem.write(r, &block)?;
                Some(Cell::addr(r))
            }
            (2, 23) => {
                let o = self.obj_of_ref(vm, a(0).v).unwrap_or(self.t.caster);
                Some(Cell::addr(self.target_record(vm, o)?))
            }
            (2, 29) => {
                let x = self.attack(a(0).v).map(|x| x.clone());
                Some(b(x.is_some_and(|x| x.hit_bg || self.world.attack_hit_bg(&x))))
            }
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
            // --- a limit's script (Limit Form's `limi`, `Magic::limit`; Steam KH2's traps, Checked
            // (decomp) where noted in `docs/kh1-drive-forms-native.md` §23) ---
            // (`trap_puts`: a debug print.)
            (0, 2) => None,
            (1, 4) => {
                let o = self.obj_of_ref(vm, a(0).v);
                let d = vec_in(vm, a(1).v)?;
                if let Some(o) = o {
                    self.world.wish_dir(o, Vec3::new(d.x, d.y, d.z));
                }
                None
            }
            (1, 11) | (1, 13) => {
                if let Some(o) = self.obj_of_handle(a(0).v) {
                    let (m, blend) = (a(1).v as u16, a(2).f());
                    if index == 11 {
                        self.world.motion_start(o, m, blend);
                    } else {
                        self.world.motion_push(o, m, blend);
                    }
                }
                None
            }
            (1, 14) => Some(b(self.obj_of_handle(a(0).v).is_some_and(|o| self.world.motion_ended(o)))),
            (1, 23) => Some(Cell::int(self.world.area_world())),
            // `trap_obj_hook (obj, pc, arg)` (`0x14042CA20`): a thread at `pc` with the object (and
            // `arg` when there is one), the object's hook; a new hook ends its last one.
            (1, 112) => {
                if let Some(o) = self.obj_of_ref(vm, a(0).v) {
                    if let Some(i) = self.t.hooks.iter().position(|h| h.0 == o) {
                        let (_, c, ext) = self.t.hooks.remove(i);
                        if c != ctx {
                            if ext {
                                self.t.ext_threads.retain(|x| *x != c);
                            } else {
                                self.t.threads.retain(|x| *x != c);
                            }
                            vm.ctxs[c].done = true;
                        }
                    }
                    let args = if a(2).v != 0 { vec![a(0), a(2)] } else { vec![a(0)] };
                    if let Some((c, ext)) = self.thread_at(vm, ctx, a(1).v, &args)? {
                        self.t.hooks.push((o, c, ext));
                    }
                }
                None
            }
            (1, 150) => Some(b(self.obj_of_ref(vm, a(0).v).is_some_and(|o| self.world.motion_range(o, a(1).i().clamp(0, 255) as u8)))),
            (1, 165) => {
                if let Some(o) = self.obj_of_ref(vm, a(2).v) {
                    self.world.command_override(a(0).i(), a(1).v as u16, o);
                }
                None
            }
            // `trap_obj_effect_start_bind (obj, element, .., ..)`: the script's effect element on the
            // object, following it (Last Arcanum's 35 / 36 on Sora).
            (1, 87) => {
                let o = self.obj_of_ref(vm, a(0).v);
                let pax = self.t.pax_rec;
                let rec = match o {
                    Some(o) => self.start_effect(vm, pax, a(1).v as u16, Some(o)),
                    None => 0,
                };
                Some(Cell::addr(rec))
            }
            (1, 272) => Some(b(!self.obj_of_ref(vm, a(0).v).is_some_and(|o| self.world.exists(o)))),
            // `trap_effect_loop_end_kill`: its loop ended (it dies when that ends).
            (0, 85) => {
                let now = self.effect(a(0).v).map(|e| {
                    e.loop_end();
                    e.clone()
                });
                if let Some(e) = now {
                    if self.world.effect_end(&e) {
                        if let Some(e) = self.effect(a(0).v) {
                            e.host_ended = true;
                        }
                    }
                }
                None
            }
            // --- Infinity's shots (`limi`'s trigger 0x11; Steam KH2, Checked (decomp)) ---
            // `trap_vector_normalize (v)` (`0x14041CFA0` → `0x140142080`): v's xyz made unit in place,
            // its length into w and returned.
            (0, 7) => {
                let v = vec_in(vm, a(0).v)?;
                let len = v.truncate().length();
                let n = if len != 0.0 { v.truncate() / len } else { v.truncate() };
                put(vm, a(0).v, n.extend(len))?;
                Some(Cell::float(len))
            }
            // `trap_random_get (n)` / `_getf (x)` / `_range (a, b)`: the host's uniform draw (their
            // KH2 bodies aren't in the decomp; Assumption: [0, n), [0, x), [a, b)).
            (0, 16) => {
                let n = a(0).i().max(0);
                let r = self.world.random();
                Some(Cell::int(((r * n as f32) as i32).min((n - 1).max(0))))
            }
            (0, 17) => {
                let r = self.world.random();
                Some(Cell::float(r * a(0).f()))
            }
            (0, 18) => {
                let r = self.world.random();
                let (lo, hi) = (a(0), a(1));
                if matches!(lo.tag, kh2_bdx::Tag::Int) && matches!(hi.tag, kh2_bdx::Tag::Int) {
                    Some(Cell::int(lo.i() + (r * (hi.i() - lo.i()) as f32) as i32))
                } else {
                    Some(Cell::float(lo.f() + r * (hi.f() - lo.f())))
                }
            }
            // `trap_vector_inter (a, b, t)` (`0x14041D490` → `0x140140230`): a·(1 − t) + b·t.
            (0, 78) => {
                let (va, vb, t) = (vec_in(vm, a(0).v)?, vec_in(vm, a(1).v)?, a(2).f());
                put(vm, s.vec, va * (1.0 - t) + vb * t)?;
                Some(Cell::addr(s.vec))
            }
            // `trap_vector_outer_product (a, b)` (`0x14041D570` → `0x140142120`): a × b, w 0.
            (0, 82) => {
                let (va, vb) = (vec_in(vm, a(0).v)?.truncate(), vec_in(vm, a(1).v)?.truncate());
                put(vm, s.vec, va.cross(vb).extend(0.0))?;
                Some(Cell::addr(s.vec))
            }
            // `trap_vector_rot (axis, v, angle)` (`0x14041D650` → `0x1403B7560`): v turned about the
            // axis (Rodrigues; Assumption: the sense, `0x1403B7560` isn't in the decomp; Infinity
            // spreads its shots evenly round, so either sense gives the same ring).
            (0, 83) => {
                let k = vec_in(vm, a(0).v)?.truncate().normalize_or_zero();
                let v = vec_in(vm, a(1).v)?;
                let (sn, cs) = a(2).f().sin_cos();
                let r = v.truncate() * cs + k.cross(v.truncate()) * sn + k * k.dot(v.truncate()) * (1.0 - cs);
                put(vm, s.vec, r.extend(v.w))?;
                Some(Cell::addr(s.vec))
            }
            // `trap_target_set_obj (target, obj)` (`0x140430470`).
            (1, 160) => {
                let o = self.obj_of_ref(vm, a(1).v);
                self.set_target(vm, a(0).v, o)?;
                None
            }
            // `trap_btlobj_lockon_target (obj)` (`0x140431AD0`: the player's lock-on target record).
            (2, 56) => {
                let o = self.obj_of_ref(vm, a(0).v).and_then(|o| self.world.lock_target(o)).filter(|o| self.world.exists(*o));
                let rec = vm.mem.alloc(16);
                vm.mem.set_u32(rec + 4, 0)?;
                vm.mem.set_u32(rec + 12, 2)?;
                self.set_target(vm, rec, o)?;
                Some(Cell::addr(rec))
            }
            // `trap_target_searcher_init (searcher, n)` (`0x14053DCE0` → `0x14053FB60`): emptied.
            (6, 3) => {
                self.t.searchers.insert(a(0).v, Vec::new());
                None
            }
            // `trap_target_seracher_search (searcher, target, pos, dir, .., .., ..)` (`0x14053DD20` →
            // `0x14053FB80`): the next object the searcher hasn't found yet into the target, within
            // the searcher's distances (`+0x10` / `+0x14`, the script's own), nearest first
            // (Assumption: KH2's direction and flag tests left out).
            (6, 5) => {
                let (sr, rec) = (a(0).v, a(1).v);
                let from = host(vec_in(vm, a(2).v)?);
                let (lo, hi) = (vm.mem.f32(sr + 0x10).unwrap_or(0.0), vm.mem.f32(sr + 0x14).unwrap_or(1000.0));
                let found = self.t.searchers.get(&sr).cloned().unwrap_or_default();
                let mut c: Vec<(f32, ObjId)> = self.world.enemies(from, lo, hi).into_iter().filter(|o| !found.contains(o)).map(|o| (self.world.target_point(o).distance(from), o)).collect();
                c.sort_by(|x, y| x.0.total_cmp(&y.0));
                let pick = c.first().map(|x| x.1);
                if let Some(o) = pick {
                    self.t.searchers.entry(sr).or_default().push(o);
                }
                self.set_target(vm, rec, pick)?;
                None
            }
            // `trap_target_dist (target, pos)` (`0x14053E3E0` → `0x1403BDD90`).
            (6, 9) => {
                let h = vm.mem.u32(a(0).v)?;
                let p = host(vec_in(vm, a(1).v)?);
                let d = self.obj_of_handle(h).map_or(f32::MAX, |o| self.world.target_point(o).distance(p));
                Some(Cell::float(d))
            }
            // `trap_target_searcher_get_old (searcher, target, i)` (`0x14053E4F0`): the i-th found.
            (6, 12) => {
                let o = self.t.searchers.get(&a(0).v).and_then(|v| v.get(a(2).i().max(0) as usize).copied());
                self.set_target(vm, a(1).v, o)?;
                None
            }
            // `trap_target_clear (target)` (`0x14053E540` → `0x1403BDA50`).
            (6, 16) => {
                self.set_target(vm, a(0).v, None)?;
                None
            }
            // `trap_target_get_group (target)` (`0x14053E5B0`: its `+4`).
            (6, 57) => Some(Cell::int(vm.mem.u32(a(0).v + 4)? as i32)),
            // `trap_party_get_weapon (obj, n)` (`0x140431310`, `obj + 0xD60 + n × 8`) / `trap_obj_is_attach
            // (obj)` (`0x14042B980`, `+0x6A0` ≠ 0), t988: until the thrown Keyblade is an object of its own (its
            // version), no weapon object and every weapon still in the hand, so Strike Raid's
            // script plays its moves with the Keyblade held (its flight loop runs while the weapon
            // isn't attached, `limi` pc 3418, Checked (files)).
            (1, 234) => {
                let w = self.obj_of_ref(vm, a(0).v).and_then(|o| self.world.weapon(o, a(1).v));
                Some(Cell::addr(match w {
                    Some(w) => obj_record(vm, self.t, w),
                    None => 0,
                }))
            }
            (1, 161) => Some(b(self.obj_of_ref(vm, a(0).v).map_or(true, |o| self.world.attached(o)))),
            // --- Strike Raid's thrown Keyblade (t992; Steam KH2, Checked (decomp),
            // `docs/kh1-strike-raid-research.md` §1) ---
            // `trap_obj_attach (child, parent, bone, flags, n, ptr)` / `trap_obj_detach (obj)`.
            (1, 18) | (1, 102) => {
                if let Some(o) = self.obj_of_ref(vm, a(0).v) {
                    self.world.attach(o, index == 18);
                }
                None
            }
            // `trap_obj_motion_sync` / `_unsync (master, slave)`: the weapon on Sora's motion while
            // held; the host's weapon is drawn on his own bones then (nothing to do).
            (1, 126) | (1, 236) => None,
            // `trap_party_hand_to_bone (obj, hand)`: the hand's bone (the host attaches to the hand).
            (1, 235) => Some(Cell::int(0)),
            // `trap_obj_set_pos (obj, pos)` (`0x1403B64D0`: `+0x670`).
            (1, 148) => {
                let p = host(vec_in(vm, a(1).v)?);
                if let Some(o) = self.obj_of_ref(vm, a(0).v) {
                    self.world.set_pos(o, p);
                }
                None
            }
            // `trap_obj_move_request (obj, dir, speed)` (`0x1403B9A70`, used once by the move step
            // `0x1403B89A0`: position += normalize(dir) × speed × the frame time).
            (1, 89) => {
                let d = flip(vec_in(vm, a(1).v)?.truncate()).normalize_or_zero();
                if let Some(o) = self.obj_of_ref(vm, a(0).v) {
                    let p = self.world.pos(o) + d * a(2).f() * ft;
                    self.world.set_pos(o, p);
                }
                None
            }
            // `trap_obj_set_xyzrot (obj, rot)` (`0x1403B65F0`: `+0x680`).
            (8, 2) => {
                let r = vec_in(vm, a(1).v)?;
                if let Some(o) = self.obj_of_ref(vm, a(0).v) {
                    self.world.set_rot(o, Vec3::new(r.x, r.y, r.z));
                }
                None
            }
            // `trap_strike_raid_calc_xyzrot (dir, up)` (`0x14053EF90`): d = unit dir, u = unit up ((0,
            // -1, 0) if the same); r0 = unit(u × d), r2 = -d, r1 = unit(r2 × r0); `up` overwritten
            // with d × r0; the basis' angles returned (KH2's `0x1401AB290`; ours glam's XYZ angles of
            // it, read back the same way by the host's `set_rot`, Assumption: the convention).
            (6, 68) => {
                let d = vec_in(vm, a(0).v)?.truncate().normalize_or_zero();
                let mut u = vec_in(vm, a(1).v)?.truncate().normalize_or_zero();
                if (u - d).length() < 1e-6 {
                    u = Vec3::new(0.0, -1.0, 0.0);
                }
                let r0 = u.cross(d).normalize_or_zero();
                let r2 = -d;
                let r1 = r2.cross(r0).normalize_or_zero();
                put(vm, a(1).v, d.cross(r0).extend(0.0))?;
                let (x, y, z) = Quat::from_mat3(&glam::Mat3::from_cols(r0, r1, r2)).normalize().to_euler(glam::EulerRot::XYZ);
                put(vm, s.vec, Vec4::new(x, y, z, 0.0))?;
                Some(Cell::addr(s.vec))
            }
            // --- Strike Raid's trail, catch flash and hits (t993; Steam KH2, Checked (decomp)) ---
            // `trap_obj_effect_start_bind_other (obj, id, other, .., ..)` (`0x1403B6670`): obj's
            // effect element bound to `other` (the trail, 32, on the thrown Keyblade).
            (1, 149) => {
                let other = self.obj_of_ref(vm, a(2).v);
                let pax = self.t.pax_rec;
                let rec = match other {
                    Some(o) => self.start_effect(vm, pax, a(1).v as u16, Some(o)),
                    None => 0,
                };
                Some(Cell::addr(rec))
            }
            // `trap_obj_effect_start_pos (obj, id, pos, .., ..)` (`0x1403B66C0`): obj's effect element
            // at `pos` (the catch flash, 34, where the Keyblade was).
            (1, 22) => {
                let p = host(vec_in(vm, a(2).v)?);
                let pax = self.t.pax_rec;
                let rec = self.start_effect(vm, pax, a(1).v as u16, None);
                let now = self.effect(rec).map(|e| {
                    e.placed = Some(p);
                    e.pos = p;
                    e.clone()
                });
                if let Some(e) = now {
                    self.world.effect_place(&e);
                }
                Some(Cell::addr(rec))
            }
            // `trap_attack_set_obj_pax (attack, obj)` (`0x140432A90`: the hit's effects from that
            // object's effect file): the host's hit effects come from the cast's package already
            // (the limit's, `kh2fx::LIMIT`; Assumption: Raid's weapon effects are in it).
            (2, 22) => None,
            // `trap_vector_inner_prodcut (a, b)` (`0x14041D8B0`): the cosine of unit copies.
            (0, 91) => {
                let (va, vb) = (vec_in(vm, a(0).v)?.truncate().normalize_or_zero(), vec_in(vm, a(1).v)?.truncate().normalize_or_zero());
                Some(Cell::float(va.dot(vb)))
            }
            // `trap_fixrad (a)` (`0x14041E5E0` → `0x1403B6F20`): into [-pi, pi]; not finite: 0.
            (0, 43) => {
                let v = a(0).f();
                Some(Cell::float(if v.is_finite() { wrap(v) } else { 0.0 }))
            }
            (6, 67) => {
                if let Some(o) = self.obj_of_ref(vm, a(0).v) {
                    self.world.party_attack(o, a(1).v as u16);
                }
                None
            }
            (1, 151) => Some(b(self.obj_of_ref(vm, a(0).v).is_some_and(|o| self.world.motion_trigger(o, a(1).i().clamp(0, 255) as u8)))),
            // `trap_obj_effect_unbind (obj, effect)` (`0x14042D650` → `0x1402C7340`): the effect no
            // longer follows the object (it stays where it is).
            (1, 169) => {
                if let Some(e) = self.effect(a(1).v) {
                    e.owner = None;
                }
                None
            }
            // `trap_obj_set_movement (obj, dir, speed)` (`0x14042FDA0` → `0x1403B9DA0`: the move
            // direction `+0x10` and speed `+0x1C`, Checked (decomp)): Infinity's follow-ups set 0
            // (stand still in the air) before their recoil (t1010: to the host, `World::set_movement`).
            (1, 111) => {
                let o = self.obj_of_ref(vm, a(0).v);
                let d = vec_in(vm, a(1).v)?;
                if let Some(o) = o {
                    let s = a(2).f();
                    self.world.set_movement(o, Vec3::new(d.x, d.y, d.z), if s.is_finite() { s } else { 0.0 });
                }
                None
            }
            (1, 183) => {
                self.world.light_color(a(0).v, [a(1).i(), a(2).i(), a(3).i()], a(4).f());
                None
            }
            (1, 222) => Some(Cell::int(self.obj_of_ref(vm, a(0).v).map_or(0, |o| self.world.entry_id(o)))),
            // `trap_limit_player (limit)` / `trap_limit_friend (limit, n)`: the limit's player (the
            // owner); a solo limit has no friend.
            (2, 48) => {
                let caster = self.t.caster;
                Some(Cell::addr(obj_record(vm, self.t, caster)))
            }
            (2, 49) => Some(Cell::addr(0)),
            // `trap_limit_start_thread (pc, arg)`: a thread at `pc` with `arg` (t984: the code at `limi`'s
            // pc 5595 / 4978 pushes the pc first, Checked (files); t980 had them the other way).
            (2, 54) => {
                self.thread_at(vm, ctx, a(0).v, &[a(1)])?;
                None
            }
            // `trap_limit_light (limit)`: the limit's light (one per limit: 1).
            (2, 55) => Some(Cell::int(1)),
            // `trap_limit_time (limit)` (limit `+0x118`): a solo limit has no partner time.
            (2, 60) => Some(Cell::float(0.0)),
            // `trap_limit_reference (limit)` (limit `+0x50`): nobody else holds a solo limit.
            (2, 63) => Some(Cell::int(0)),
            // `trap_obj_stop_end_all`.
            (6, 47) => None,
            // `trap_limit_start_command` (`0x1403D89F0`): the limit's command.
            (7, 27) => Some(Cell::int(self.t.limit_command.map_or(0, |c| c as i32))),
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
