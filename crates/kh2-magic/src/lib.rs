//! KH2's spells, run from the spell files' own BDX scripts.
//!
//! A spell file (`magic/FIRE_1.mag`, ...) is a BAR holding a PAX effect
//! package and a script. Casting starts the script's trigger 20 with the
//! caster, the spell's resources, the magic "sub" (level, finisher, Drive
//! form) and that sub's `magi` parameters; the cast motion's magic-cast
//! event sends trigger 21, which releases the spell. From then on the
//! script creates attacks, places them each frame (often on an effect's
//! position), starts threads for projectiles, and frees everything when
//! it is done. This crate runs those scripts and implements the traps
//! they call; the host engine answers questions about the world through
//! [`World`] and tests the attacks against its own objects.
//!
//! Coordinates: the host's world is y-up; the game's is y-down. The
//! conversion is a flip of y at this boundary (see [`flip`]).
//!
//! See `docs/magic-model.md` for what each part is based on.

pub mod effect;
mod host;

use std::collections::HashMap;

use combat_core::{Shape, ShapeKind};
use glam::{Mat4, Vec3};
use kh2_bdx::{BdxError, Cell, CtxId, Program, Vm};
use kh2_formats::bar::Bar;
use kh2_formats::pax::Pax;

pub use effect::Effect;

pub type ObjId = u32;

/// y-up (host) <-> y-down (game).
pub fn flip(v: Vec3) -> Vec3 {
    Vec3::new(v.x, -v.y, v.z)
}

/// What the spell scripts ask about the world. Positions are the host's
/// (y-up); yaw 0 faces +z and `(sin, cos)` of the yaw is the facing in x/z.
pub trait World {
    fn exists(&self, o: ObjId) -> bool;
    /// Feet position.
    fn pos(&self, o: ObjId) -> Vec3;
    fn facing(&self, o: ObjId) -> f32;
    /// The motion slot playing.
    fn motion(&self, o: ObjId) -> u16;
    fn airborne(&self, o: ObjId) -> bool;
    /// Airborne from a jump (not a fall or a launch).
    fn jumping(&self, o: ObjId) -> bool;
    /// A bone's world transform. `code` is a bone index or a special
    /// anchor: 0x4000 the object (feet, facing), 0x4001 its body centre,
    /// 0x4003/0x4004 a weapon's tip, 0x4005/0x4006 its base, 0x4007/0x4008
    /// its middle.
    fn bone(&self, o: ObjId, code: u16) -> Option<Mat4>;
    /// Where to aim at this object (its target-type collision shape).
    fn target_point(&self, o: ObjId) -> Vec3;
    /// The object's current target (lock-on, else the auto-target).
    fn target_of(&self, o: ObjId) -> Option<ObjId>;
    /// Party member `n`: 0 is the player, then the companions.
    fn party(&self, n: u32) -> Option<ObjId>;
    /// Ability bit `n` (an ability number) equipped.
    fn has_ability(&self, o: ObjId, n: u32) -> bool;
    /// The floor under `from`, at most `depth` below.
    fn ground(&self, from: Vec3, depth: f32) -> Option<Vec3>;
    /// A segment hits walls or floor.
    fn bg_hit(&self, from: Vec3, to: Vec3) -> bool;
    /// Push an object (the caster's recoil).
    fn hitback(&mut self, o: ObjId, dir: Vec3, dist: f32);
    /// Uniform in [0, 1).
    fn random(&mut self) -> f32;
}

/// One loaded spell file.
#[derive(Clone, Debug)]
pub struct SpellFile {
    pub name: String,
    pub program: Program,
    pub pax: Pax,
}

impl SpellFile {
    /// An object's own script and effects: the BDX entry of its model
    /// (Wisdom Form's `wisd`) and the PAX of its effect archive.
    pub fn from_parts(name: &str, script: &[u8], pax: &[u8]) -> Result<SpellFile, String> {
        Ok(SpellFile { name: name.to_string(), program: Program::parse(script).map_err(|e| e.to_string())?, pax: Pax::parse(pax).map_err(|e| e.to_string())? })
    }

    /// The first BAR entry of `kind` (3 script, 18 PAX) in a file.
    pub fn bar_entry(file: &[u8], kind: u16) -> Option<Vec<u8>> {
        let bar = Bar::parse(file).ok()?;
        bar.entries.iter().find(|e| e.kind == kind).map(|e| e.data.to_vec())
    }

    pub fn parse(name: &str, mag: &[u8]) -> Result<SpellFile, String> {
        let bar = Bar::parse(mag).map_err(|e| e.to_string())?;
        let script = bar.entries.iter().find(|e| e.kind == 3).ok_or("no script in the spell file")?;
        let pax = bar.entries.iter().find(|e| e.kind == 18).ok_or("no effects in the spell file")?;
        Ok(SpellFile {
            name: name.to_string(),
            program: Program::parse(script.data).map_err(|e| e.to_string())?,
            pax: Pax::parse(pax.data).map_err(|e| e.to_string())?,
        })
    }
}

/// A spell's attack: a sphere or ellipsoid the script places.
#[derive(Clone, Debug)]
pub struct MagicAttack {
    /// This attack object.
    pub key: u64,
    /// The hit record's key. A duplicated attack shares its original's id
    /// and group (the game passes them on), so a target hit by one is
    /// treated as hit by both.
    pub id: u64,
    /// Whether ending this attack frees its id (originals do, duplicates
    /// don't).
    pub owns_id: bool,
    /// A fresh group per attack (the game's default); the host may use
    /// the ATKP row's combo group instead when it has one.
    pub group: i64,
    pub owner: ObjId,
    pub spell: usize,
    pub atk: u16,
    pub sub: u8,
    pub pos: Option<Vec3>,
    pub prev: Option<Vec3>,
    pub moved: bool,
    pub radius: f32,
    pub height: f32,
    pub hit: bool,
    pub hit_bg: bool,
    pub reflected: bool,
    pub reflect_count: u8,
    pub freed: bool,
}

impl MagicAttack {
    fn set_pos(&mut self, p: Vec3, frametime: f32) {
        match self.pos {
            Some(old) => {
                self.prev = Some(old);
                self.moved = old.distance(p) > frametime * 2.0;
            }
            None => {
                self.prev = Some(p);
                self.moved = false;
            }
        }
        self.pos = Some(p);
    }

    /// This frame's shapes: the shape at its position, then copies stepped
    /// back toward last frame's position (one per `min(radius, height)`)
    /// when it moved, like the game.
    pub fn shapes(&self) -> Vec<Shape> {
        let Some(pos) = self.pos else { return Vec::new() };
        let step = self.radius.min(self.height);
        if step <= 0.0 {
            return Vec::new();
        }
        let kind = if self.radius == self.height { ShapeKind::Sphere } else { ShapeKind::Ellipsoid };
        let mut out = vec![Shape::new(kind, pos.into(), self.radius, self.height)];
        if let (true, Some(prev)) = (self.moved, self.prev) {
            let d = prev - pos;
            let mut left = d.length();
            let dir = d.normalize_or_zero() * step;
            let mut c = pos;
            while left - step > step * 0.5 && out.len() < 64 {
                c += dir;
                left -= step;
                out.push(Shape::new(kind, c.into(), self.radius, self.height));
            }
        }
        out
    }
}

/// Something the host should act on.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The script applied an attack to an object directly (Cure).
    Strike { attack: u64, target: ObjId },
    /// A trap this crate does not implement was called (it returned 0).
    Missing(&'static str),
    Error(String),
}

pub(crate) struct Tables {
    pub caster: ObjId,
    pub spell: usize,
    pub objs: HashMap<u32, ObjId>,
    pub obj_rec: HashMap<ObjId, u32>,
    pub pax_rec: u32,
    pub attacks: HashMap<u32, u64>,
    pub effects: HashMap<u32, u64>,
    pub targets: HashMap<ObjId, u32>,
    pub teamwork: HashMap<(u32, u32), u32>,
    pub statics: host::Statics,
    pub threads: Vec<CtxId>,
    /// The caster's battle target, picked when the cast starts.
    pub target: Option<ObjId>,
}

pub struct Cast {
    pub(crate) vm: Vm,
    pub(crate) t: Tables,
    pub(crate) main: CtxId,
    pub released: bool,
    pub finished: bool,
    /// Started but not run yet.
    fresh: bool,
    /// An object script run (not a spell cast).
    pub object: bool,
    /// Held by the host (`Magic::hold`): its script doesn't run and its effects don't age.
    pub held: bool,
}

impl Cast {
    pub fn caster(&self) -> ObjId {
        self.t.caster
    }
    pub fn spell(&self) -> usize {
        self.t.spell
    }
}

/// Every spell in flight, with their attacks and effects.
pub struct Magic {
    pub spells: Vec<SpellFile>,
    pub casts: Vec<Cast>,
    pub attacks: Vec<MagicAttack>,
    pub effects: Vec<Effect>,
    pub frametime: f32,
    /// Print every trap call (research).
    pub trace: bool,
    next_id: u64,
    next_group: i64,
    pub(crate) events: Vec<Event>,
}

impl Default for Magic {
    fn default() -> Self {
        Self::new()
    }
}

impl Magic {
    pub fn new() -> Magic {
        // Attack groups the game makes up start at 0x100.
        Magic { spells: Vec::new(), casts: Vec::new(), attacks: Vec::new(), effects: Vec::new(), frametime: 1.0, trace: std::env::var_os("KH2_MAGIC_TRACE").is_some(), next_id: 1, next_group: 0x100, events: Vec::new() }
    }

    pub fn add_spell(&mut self, file: SpellFile) -> usize {
        self.spells.push(file);
        self.spells.len() - 1
    }

    pub(crate) fn fresh_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }

    pub(crate) fn fresh_group(&mut self) -> i64 {
        self.next_group += 1;
        if self.next_group > 0x1000_0000 {
            self.next_group = 0x100;
        }
        self.next_group
    }

    /// Starts a cast: trigger 20 with the caster, the spell's resources,
    /// `sub` and its `magi` record. Call when the cast motion starts.
    /// `element` is the MAGC element (0 Fire .. 5 Reflect): the game picks
    /// a target first for Fire, Blizzard, Thunder and Magnet only.
    pub fn cast(&mut self, spell: usize, caster: ObjId, element: u8, sub: i32, magi: &[u8], world: &mut dyn World) -> Result<usize, BdxError> {
        let file = self.spells.get(spell).ok_or_else(|| BdxError::Trap("no such spell".into()))?;
        let mut vm = Vm::new(file.program.clone());
        vm.frametime = self.frametime;
        let statics = host::Statics::new(&mut vm);
        let mut t = Tables {
            caster,
            spell,
            objs: HashMap::new(),
            obj_rec: HashMap::new(),
            pax_rec: 0,
            attacks: HashMap::new(),
            effects: HashMap::new(),
            targets: HashMap::new(),
            teamwork: HashMap::new(),
            statics,
            threads: Vec::new(),
            target: if matches!(element, 0 | 1 | 2 | 4) { world.target_of(caster) } else { None },
        };
        let player = host::obj_record(&mut vm, &mut t, caster);
        t.pax_rec = vm.mem.alloc(16);
        let magi_at = vm.mem.alloc(magi.len() as u32);
        vm.mem.write(magi_at, magi)?;
        let main = vm.spawn(None);
        vm.trigger(main, 20, &[Cell::addr(player), Cell::addr(t.pax_rec), Cell::int(sub), Cell::addr(magi_at)])?;
        self.casts.push(Cast { vm, t, main, released: false, finished: false, fresh: true, object: false, held: false });
        // It first runs with the next `update` (the game's magic manager
        // runs every script once per frame, after the player).
        Ok(self.casts.len() - 1)
    }

    /// An object's motion fired frame trigger 1 with effect id `n` (exe):
    /// the object's effect element `n` starts, and its script (loaded as
    /// `file`) runs trigger 0x10 with the object, `n` and that effect.
    /// Wisdom Form's shots come from here.
    pub fn object_effect(&mut self, file: usize, owner: ObjId, n: i16, world: &mut dyn World) -> Result<usize, BdxError> {
        let f = self.spells.get(file).ok_or_else(|| BdxError::Trap("no such object script".into()))?;
        let mut vm = Vm::new(f.program.clone());
        vm.frametime = self.frametime;
        let statics = host::Statics::new(&mut vm);
        let mut t = Tables {
            caster: owner,
            spell: file,
            objs: HashMap::new(),
            obj_rec: HashMap::new(),
            pax_rec: 0,
            attacks: HashMap::new(),
            effects: HashMap::new(),
            targets: HashMap::new(),
            teamwork: HashMap::new(),
            statics,
            threads: Vec::new(),
            target: world.target_of(owner),
        };
        let obj = host::obj_record(&mut vm, &mut t, owner);
        t.pax_rec = vm.mem.alloc(16);
        let effect = host::start_object_effect(self, &mut vm, &mut t, n as u16, owner, world);
        let main = vm.spawn(None);
        vm.trigger(main, 0x10, &[Cell::addr(obj), Cell::int(n as i32), Cell::addr(effect)])?;
        self.casts.push(Cast { vm, t, main, released: true, finished: false, fresh: false, object: true, held: false });
        // The trigger runs at once (same frame), like the game's call.
        let i = self.casts.len() - 1;
        self.run_cast(i, world);
        Ok(i)
    }

    /// The cast motion reached its magic-cast event: trigger 21 on the
    /// caster's latest cast (it runs with the next `update`). A cast whose
    /// trigger 20 has not run yet (release on the motion's first frame)
    /// runs it first.
    pub fn release(&mut self, caster: ObjId, world: &mut dyn World) -> bool {
        let Some(i) = self.casts.iter().rposition(|c| c.t.caster == caster && !c.released && !c.object) else { return false };
        if self.casts[i].fresh {
            self.run_cast(i, world);
        }
        let c = &mut self.casts[i];
        c.released = true;
        let main = c.main;
        match c.vm.trigger(main, 21, &[]) {
            Ok(true) => {
                c.finished = false;
                true
            }
            Ok(false) => false,
            Err(e) => {
                self.events.push(Event::Error(e.to_string()));
                false
            }
        }
    }

    fn run_cast(&mut self, i: usize, world: &mut dyn World) {
        let mut cast = std::mem::replace(&mut self.casts[i], placeholder());
        cast.fresh = false;
        let result = host::run(self, &mut cast, world);
        if let Err(e) = result {
            self.events.push(Event::Error(format!("{}: {e}", self.spells[cast.t.spell].name)));
            cast.finished = true;
        }
        self.casts[i] = cast;
    }

    /// One frame: effects age and follow their bones, then every script
    /// runs. Returns what the host should act on.
    pub fn update(&mut self, world: &mut dyn World) -> Vec<Event> {
        let ft = self.frametime;
        // Attacks freed last frame are gone now.
        self.attacks.retain(|a| !a.freed);
        // A held cast's effects stand still in time (they still follow their owner).
        let held: Vec<u64> = self.casts.iter().filter(|c| c.held).flat_map(|c| c.t.effects.values().copied()).collect();
        for e in &mut self.effects {
            if e.alive() {
                e.tick(if held.contains(&e.id) { 0.0 } else { ft }, world);
            } else {
                e.linger -= ft;
            }
        }
        // Dead effects stay a little so scripts holding them read "dead".
        self.effects.retain(|e| e.alive() || e.linger > 0.0);
        for i in 0..self.casts.len() {
            if !self.casts[i].finished && !self.casts[i].held {
                self.run_cast(i, world);
            }
        }
        // A finished cast's attacks go too (the scripts free their own).
        let live: Vec<bool> = self.casts.iter().map(|c| !c.finished).collect();
        let mut keep = live.iter();
        self.casts.retain(|_| *keep.next().unwrap());
        // Attacks of casts that ended are freed with them.
        for a in &mut self.attacks {
            if !self.casts.iter().any(|c| c.t.attacks.values().any(|k| *k == a.key)) {
                a.freed = true;
            }
        }
        std::mem::take(&mut self.events)
    }

    /// Events from casting or releasing outside `update`.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    pub fn attack_mut(&mut self, key: u64) -> Option<&mut MagicAttack> {
        self.attacks.iter_mut().find(|a| a.key == key)
    }

    /// Attacks freed this frame (the host should forget their hits).
    pub fn freed(&self) -> impl Iterator<Item = &MagicAttack> {
        self.attacks.iter().filter(|a| a.freed)
    }

    /// Holds `caster`'s spell casts in place (`on`), or lets them go on: a held cast's script
    /// doesn't run (its waits don't count down) and its effects don't age, while its attacks
    /// stay as they are. For a host that stretches a phase of a spell (Reflect's barrier while
    /// attacks keep hitting it). Returns whether any cast was there.
    pub fn hold(&mut self, caster: ObjId, on: bool) -> bool {
        let mut any = false;
        for c in self.casts.iter_mut().filter(|c| c.t.caster == caster && !c.object) {
            c.held = on;
            any = true;
        }
        any
    }

    /// Ends `caster`'s spell casts at once (a host's choice: a new cast of the same spell taking
    /// over, Reflect spammed): their scripts stop, their effects are killed and their attacks go
    /// with the next `update`. Returns how many casts there were.
    pub fn cancel(&mut self, caster: ObjId) -> usize {
        let mut n = 0;
        let mut effects = Vec::new();
        let mut attacks = Vec::new();
        for c in self.casts.iter_mut().filter(|c| c.t.caster == caster && !c.object && !c.finished) {
            c.finished = true;
            effects.extend(c.t.effects.values().copied());
            attacks.extend(c.t.attacks.values().copied());
            n += 1;
        }
        for e in self.effects.iter_mut().filter(|e| effects.contains(&e.id)) {
            e.killed = true;
        }
        // Its attacks go now, not at the next `update`: a cast started straight after must not
        // find them (the host's Reflect chain counted the carried hits on the old barrier, and
        // the new one burst for none).
        for a in self.attacks.iter_mut().filter(|a| attacks.contains(&a.key)) {
            a.freed = true;
        }
        n
    }

    /// Whether `caster` has a cast in flight.
    pub fn busy(&self, caster: ObjId) -> bool {
        self.casts.iter().any(|c| c.t.caster == caster && !c.object)
    }
}

fn placeholder() -> Cast {
    let mut b = vec![0u8; 0x24];
    b[0] = b'-';
    let vm = Vm::new(Program::parse(&b).expect("empty program"));
    Cast {
        vm,
        t: Tables {
            caster: 0,
            spell: 0,
            objs: HashMap::new(),
            obj_rec: HashMap::new(),
            pax_rec: 0,
            attacks: HashMap::new(),
            effects: HashMap::new(),
            targets: HashMap::new(),
            teamwork: HashMap::new(),
            statics: host::Statics::default(),
            threads: Vec::new(),
            target: None,
        },
        main: 0,
        released: true,
        finished: true,
        fresh: false,
        object: false,
        held: false,
    }
}
