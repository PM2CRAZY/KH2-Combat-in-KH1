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
/// The script machine (for a host that runs it on its own memory, `NativeHost`).
pub use kh2_bdx;

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
    /// KH2's star (`trap_obj_star`): `o` takes no hit for `frames` (60 Hz frames).
    fn star(&mut self, _o: ObjId, _frames: f32) {}
    /// An attack the script made a shape for, sized or moved (`trap_attack_set_radius`,
    /// `trap_attack_set_pos`): a host with attack objects of its own makes or moves its object for
    /// it now.
    fn attack_set(&mut self, _a: &MagicAttack) {}
    /// An attack the script freed (`trap_attack_free`): the host's object for it goes.
    fn attack_free(&mut self, _a: &MagicAttack) {}
    /// An effect the script started (`trap_pax_start*`, `trap_obj_effect_start`): a host that plays
    /// effects itself starts its own instance now and returns its handle (`Effect::host`).
    fn effect_start(&mut self, _e: &Effect) -> Option<u32> {
        None
    }
    /// An effect the script killed (`trap_effect_kill`): the host's instance stops now.
    fn effect_kill(&mut self, _e: &Effect) {}
    /// An effect whose loop the script ended (`trap_effect_loop_end`): true if the host ended its
    /// instance now (`Effect::host_ended`).
    fn effect_end(&mut self, _e: &Effect) -> bool {
        false
    }
    /// An effect the script fades out over `frames` (`trap_effect_fadeout`): true if the host runs
    /// that fade on its instance (`Effect::host_fading`).
    fn effect_fade(&mut self, _e: &Effect, _frames: f32) -> bool {
        false
    }
    /// An effect the script put somewhere or turned (`trap_effect_set_pos`, `trap_effect_set_dir`):
    /// the host's instance goes there now.
    fn effect_place(&mut self, _e: &Effect) {}
    /// Whether `a`'s last move met the background (`trap_attack_is_hit_bg`; KH2 marks it as its
    /// attack moves).
    fn attack_hit_bg(&self, _a: &MagicAttack) -> bool {
        false
    }
    /// Whether the host's object for `a` has hit something (`trap_attack_is_hit`).
    fn attack_hit(&self, _a: &MagicAttack) -> bool {
        false
    }
    /// Uniform in [0, 1).
    fn random(&mut self) -> f32;
    /// `trap_sysobj_motion_start (obj, motion, blend)`: `o` plays `motion` now, its queue cleared
    /// (a limit's moves: Limit Form's script).
    fn motion_start(&mut self, _o: ObjId, _motion: u16, _blend: f32) {}
    /// `trap_sysobj_motion_push (obj, motion, blend)`: `motion` queued after the one playing.
    fn motion_push(&mut self, _o: ObjId, _motion: u16, _blend: f32) {}
    /// `trap_sysobj_motion_is_end (obj)`: the motion playing has ended.
    fn motion_ended(&self, _o: ObjId) -> bool {
        false
    }
    /// `trap_obj_wish_dir (obj, dir)`: the way `o` should face (the game's vector).
    fn wish_dir(&mut self, _o: ObjId, _dir: Vec3) {}
    /// `trap_light_set_color (light, r, g, b, frames)`: the limit light (`trap_limit_light`) fades to
    /// the colour (128 = unchanged) over `frames`.
    fn light_color(&mut self, _light: u32, _rgb: [i32; 3], _frames: f32) {}
    /// `trap_obj_motion_check_range (obj, n)`: script range `n` (`clip::WindowKind::ScriptRange`) of
    /// the motion `o` plays is open now.
    fn motion_range(&self, _o: ObjId, _n: u8) -> bool {
        false
    }
    /// `trap_command_override (slot, command, obj, 1)`: `command` offered in the player's reaction
    /// slot this frame (a limit's follow-up).
    fn command_override(&mut self, _slot: i32, _command: u16, _o: ObjId) {}
    /// `trap_obj_set_movement (obj, dir, speed)` (Steam KH2 `0x14042FDA0` → `0x1403B9DA0`: the
    /// object's move direction `+0x10` and speed `+0x1C`, Checked (decomp)).
    fn set_movement(&mut self, _o: ObjId, _dir: Vec3, _speed: f32) {}
    /// `trap_obj_party_attack (obj, row)` (Steam KH2 `0x14053EF20` → `0x1403D74E0` / `0x1403D78D0`):
    /// `o`'s moveset's PTYA row `row` started on it (its motion, toward its target).
    fn party_attack(&mut self, _o: ObjId, _row: u16) {}
    /// `trap_obj_motion_check_trigger (obj, n)` (`0x14042D230`): the motion's frame trigger 8 + n
    /// (KH2's frame triggers 9..12, `0x14040DD70`) fired this frame.
    fn motion_trigger(&self, _o: ObjId, _n: u8) -> bool {
        false
    }
    /// The objects a target search may find (`trap_target_seracher_search`): the host's enemies
    /// whose distance from `from` is within `min..=max` (host units).
    fn enemies(&self, _from: Vec3, _min: f32, _max: f32) -> Vec<ObjId> {
        Vec::new()
    }
    /// `o`'s lock-on target (`trap_btlobj_lockon_target`), none without a lock.
    fn lock_target(&self, _o: ObjId) -> Option<ObjId> {
        None
    }
    /// `trap_party_get_weapon (obj, n)`: `o`'s weapon `n` as an object of its own, if the host has
    /// one (Strike Raid's thrown Keyblade).
    fn weapon(&self, _o: ObjId, _n: u32) -> Option<ObjId> {
        None
    }
    /// `trap_obj_is_attach (obj)`: `o` hangs on another (a weapon in its hand). Every object is
    /// unless the host says otherwise.
    fn attached(&self, _o: ObjId) -> bool {
        true
    }
    /// `trap_obj_attach (obj, parent, ..)` / `trap_obj_detach (obj)`: `o` back on its parent, or
    /// left free where it is.
    fn attach(&mut self, _o: ObjId, _on: bool) {}
    /// `trap_obj_set_pos (obj, pos)` / `trap_obj_move_request`'s landing: a free object's place
    /// (host units).
    fn set_pos(&mut self, _o: ObjId, _p: Vec3) {}
    /// `trap_obj_set_xyzrot (obj, rot)`: a free object's turn (the game's angles, as
    /// `trap_strike_raid_calc_xyzrot` made them).
    fn set_rot(&mut self, _o: ObjId, _rot: Vec3) {}
    /// `trap_area_world`: the game's world number (KH2's numbering), or -1.
    fn area_world(&self) -> i32 {
        -1
    }
    /// `trap_obj_get_entry_id (obj)`: `o`'s object-table entry (KH2's numbering), or 0.
    fn entry_id(&self, _o: ObjId) -> i32 {
        0
    }
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
    /// Freed and already shown to the host by an `update` (it goes at the next one).
    pub(crate) freed_seen: bool,
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

/// A host whose own script machine runs the spells' scripts (the mod: KH1's script processes,
/// `docs/kh1-magic-native.md` §8). With one set (`Magic::native`), a cast's machine sits on the
/// host's memory and its main context is one of the host's processes: the host steps it
/// (`Magic::step_external`) when its own machine steps that process, and `update` leaves it alone.
/// The context's registers live in the host's process between steps (`load` / `store`).
pub trait NativeHost: Send {
    /// A machine for spell `spell`'s script on the host's memory, with its main context on a new
    /// process of the host's, for trigger `key`. `None`: the cast doesn't start (no fallback onto
    /// our own arena while a host is set).
    fn machine(&mut self, file: &SpellFile, spell: usize, key: i32) -> Option<(Vm, CtxId)>;
    /// The cast `id` is set up (its trigger's frame and arguments in the context): its process is
    /// handed the registers and steps it from now on. False: the host couldn't take it (the cast
    /// doesn't start; the host ends the process it made).
    fn started(&mut self, id: u64, vm: &Vm, main: CtxId) -> bool;
    /// A thread the cast `id`'s script starts (`trap_magic_start_thread`): a context on a new
    /// child process of the cast's process (its stack the child's, its work memory `wp`, shared),
    /// which the host's machine steps after its parent. `None`: the thread doesn't start (the
    /// call returns a null record).
    fn thread(&mut self, id: u64, vm: &mut Vm, wp: u32) -> Option<CtxId>;
    /// A context's registers from its host process (before a step).
    fn load(&mut self, id: u64, vm: &mut Vm, ctx: CtxId);
    /// A context's registers into its host process (after a step, a trigger, a thread start).
    fn store(&mut self, id: u64, vm: &Vm, ctx: CtxId);
    /// The host's process is still there (a room change or reset may have dropped it).
    fn alive(&self, id: u64) -> bool;
    /// The cast is over: its process ends at its next step and its memory goes back.
    fn ended(&mut self, id: u64);
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
    /// The teamwork blocks this cast holds (`trap_teamwork_alloc`): key, its copy in this cast's
    /// memory (`Magic::teamwork` holds the shared bytes).
    pub teamwork: HashMap<u32, u32>,
    pub statics: host::Statics,
    pub threads: Vec<CtxId>,
    /// Threads on the host's child processes (`NativeHost::thread`), stepped by the host.
    pub ext_threads: Vec<CtxId>,
    /// The cast, when its main context is on the host's process (`NativeHost`).
    pub native_id: Option<u64>,
    /// The caster's battle target, picked when the cast starts.
    pub target: Option<ObjId>,
    /// A limit's script (`Magic::limit`): the limit's command (`trap_limit_start_command`).
    pub limit_command: Option<u16>,
    /// Each object's hook thread (`trap_obj_hook`): the object, the thread, on the host's process.
    pub hooks: Vec<(ObjId, CtxId, bool)>,
    /// Each target searcher's found objects (`trap_target_searcher_init` / `_search`), by its record.
    pub searchers: HashMap<u32, Vec<ObjId>>,
    /// Hook threads of a limit's stepped ahead of its main context this host frame (`step_external`).
    pub early: Vec<(CtxId, Option<u64>)>,
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
    /// This cast, for the host (`NativeHost`).
    pub id: u64,
    /// Its main context is one of the host's processes, stepped by the host (`step_external`).
    pub external: bool,
}

impl Cast {
    pub fn caster(&self) -> ObjId {
        self.t.caster
    }
    pub fn spell(&self) -> usize {
        self.t.spell
    }
    /// Its main context.
    pub fn main_ctx(&self) -> CtxId {
        self.main
    }
    /// Its threads on the host's child processes (`NativeHost::thread`), in the order started.
    pub fn host_threads(&self) -> &[CtxId] {
        &self.t.ext_threads
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
    /// KH2's teamwork blocks (`trap_teamwork_alloc`, Steam `0x1403F6DF0`): one per key, shared by
    /// every script.
    pub(crate) teamwork: HashMap<u32, Vec<u8>>,
    /// The host's own script machine, when it runs the casts (`NativeHost`).
    pub native: Option<Box<dyn NativeHost>>,
    /// The host's frame number, when its machine steps casts outside `update`: the effects then
    /// age once per frame, before the first script of the frame runs (as `update` has it), whether
    /// that is `update` or `step_external`. `None`: every `update` ages them.
    pub frame: Option<u64>,
    ticked: Option<u64>,
}

impl Default for Magic {
    fn default() -> Self {
        Self::new()
    }
}

impl Magic {
    pub fn new() -> Magic {
        // Attack groups the game makes up start at 0x100.
        Magic { spells: Vec::new(), casts: Vec::new(), attacks: Vec::new(), effects: Vec::new(), frametime: 1.0, trace: std::env::var_os("KH2_MAGIC_TRACE").is_some(), next_id: 1, next_group: 0x100, events: Vec::new(), teamwork: HashMap::new(), native: None, frame: None, ticked: None }
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
        // On the host's own script machine when it has one (its memory, its process); no fallback
        // onto our arena then.
        let made = match self.native.as_mut() {
            Some(n) => Some(n.machine(file, spell, 20).ok_or_else(|| BdxError::Trap("the host has no process for it".into()))?),
            None => None,
        };
        let (mut vm, native_main) = match made {
            Some((vm, main)) => (vm, Some(main)),
            None => (Vm::new(file.program.clone()), None),
        };
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
            ext_threads: Vec::new(),
            native_id: None,
            limit_command: None,
            hooks: Vec::new(),
            searchers: HashMap::new(),
            early: Vec::new(),
            target: if matches!(element, 0 | 1 | 2 | 4) { world.target_of(caster) } else { None },
        };
        let player = host::obj_record(&mut vm, &mut t, caster);
        t.pax_rec = vm.mem.alloc(16);
        let magi_at = vm.mem.alloc(magi.len() as u32);
        vm.mem.write(magi_at, magi)?;
        let main = native_main.unwrap_or_else(|| vm.spawn(None));
        vm.trigger(main, 20, &[Cell::addr(player), Cell::addr(t.pax_rec), Cell::int(sub), Cell::addr(magi_at)])?;
        let id = self.fresh_id();
        let external = native_main.is_some();
        if external && !self.native.as_mut().is_some_and(|n| n.started(id, &vm, main)) {
            return Err(BdxError::Trap("the host couldn't take it".into()));
        }
        if external {
            t.native_id = Some(id);
        }
        self.casts.push(Cast { vm, t, main, released: false, finished: false, fresh: true, object: false, held: false, id, external });
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
        // On the host's own script machine when it has one, as a cast (t827); no fallback onto
        // our arena then.
        let made = match self.native.as_mut() {
            Some(n) => Some(n.machine(f, file, 0x10).ok_or_else(|| BdxError::Trap("the host has no process for it".into()))?),
            None => None,
        };
        let (mut vm, native_main) = match made {
            Some((vm, main)) => (vm, Some(main)),
            None => (Vm::new(f.program.clone()), None),
        };
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
            ext_threads: Vec::new(),
            native_id: None,
            limit_command: None,
            hooks: Vec::new(),
            searchers: HashMap::new(),
            early: Vec::new(),
            target: world.target_of(owner),
        };
        let obj = host::obj_record(&mut vm, &mut t, owner);
        t.pax_rec = vm.mem.alloc(16);
        let effect = host::start_object_effect(self, &mut vm, &mut t, n as u16, owner, world);
        let main = native_main.unwrap_or_else(|| vm.spawn(None));
        vm.trigger(main, 0x10, &[Cell::addr(obj), Cell::int(n as i32), Cell::addr(effect)])?;
        let id = self.fresh_id();
        let external = native_main.is_some();
        if external && !self.native.as_mut().is_some_and(|h| h.started(id, &vm, main)) {
            return Err(BdxError::Trap("the host couldn't take it".into()));
        }
        if external {
            t.native_id = Some(id);
        }
        self.casts.push(Cast { vm, t, main, released: true, finished: false, fresh: false, object: true, held: false, id, external });
        // The trigger runs at once (same frame), like the game's call.
        let i = self.casts.len() - 1;
        self.run_cast(i, world);
        // (On the host's process: a thread it started there runs in this same step, as the host's
        // machine runs a child started during its parent's step, `NativeHost::thread`.)
        if external {
            let mut k = 0;
            while let Some(ctx) = self.casts.iter().find(|c| c.id == id).and_then(|c| c.t.ext_threads.get(k).copied()) {
                if self.step_external(id, ctx, world) || self.casts.iter().find(|c| c.id == id).and_then(|c| c.t.ext_threads.get(k).copied()) == Some(ctx) {
                    k += 1;
                }
            }
        }
        Ok(i)
    }

    /// A limit's script (Limit Form's `limi`, the model's own script, loaded as `file`): KH2's start
    /// of a solo limit (Steam KH2 `0x1403D7FA0`, Checked (decomp)) runs the player's script's
    /// trigger 0x16 with the limit object; the script asks the limit's command
    /// (`trap_limit_start_command`), its player (`trap_limit_player`, the owner) and its light, and
    /// plays the limit's moves on the player (`trap_obj_hook`, `trap_sysobj_motion_*`). Run at once,
    /// as `object_effect`. Returns the cast's index.
    pub fn limit(&mut self, file: usize, owner: ObjId, command: u16, world: &mut dyn World) -> Result<usize, BdxError> {
        let f = self.spells.get(file).ok_or_else(|| BdxError::Trap("no such object script".into()))?;
        let made = match self.native.as_mut() {
            Some(n) => Some(n.machine(f, file, 0x16).ok_or_else(|| BdxError::Trap("the host has no process for it".into()))?),
            None => None,
        };
        let (mut vm, native_main) = match made {
            Some((vm, main)) => (vm, Some(main)),
            None => (Vm::new(f.program.clone()), None),
        };
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
            ext_threads: Vec::new(),
            native_id: None,
            target: world.target_of(owner),
            limit_command: Some(command),
            hooks: Vec::new(),
            searchers: HashMap::new(),
            early: Vec::new(),
        };
        host::obj_record(&mut vm, &mut t, owner);
        t.pax_rec = vm.mem.alloc(16);
        // (KH2's limit object is 0x128 bytes, `0x1403D7FA0`; the script only hands it to the
        // limit's calls, which answer from the cast's tables.)
        let limit = vm.mem.alloc(0x128);
        let main = native_main.unwrap_or_else(|| vm.spawn(None));
        vm.trigger(main, 0x16, &[Cell::addr(limit)])?;
        let id = self.fresh_id();
        let external = native_main.is_some();
        if external && !self.native.as_mut().is_some_and(|h| h.started(id, &vm, main)) {
            return Err(BdxError::Trap("the host couldn't take it".into()));
        }
        if external {
            t.native_id = Some(id);
        }
        self.casts.push(Cast { vm, t, main, released: true, finished: false, fresh: false, object: true, held: false, id, external });
        let i = self.casts.len() - 1;
        self.run_cast(i, world);
        if external {
            let mut k = 0;
            while let Some(ctx) = self.casts.iter().find(|c| c.id == id).and_then(|c| c.t.ext_threads.get(k).copied()) {
                if self.step_external(id, ctx, world) || self.casts.iter().find(|c| c.id == id).and_then(|c| c.t.ext_threads.get(k).copied()) == Some(ctx) {
                    k += 1;
                }
            }
        }
        Ok(i)
    }

    /// A limit's follow-up chosen (KH2: `0x1403D8B40` sends a command with no `limt` row of its own
    /// to the running limit thread as trigger 0x17 with the command, `0x1403E1840`, Steam KH2,
    /// Checked (decomp)): trigger 0x17 runs at once on a context of its own with the limit's work
    /// memory (its code to the follow-up's hook has no `halt`, Checked (files)). Returns whether a
    /// limit of `file` took it.
    pub fn limit_command(&mut self, file: usize, command: u16, world: &mut dyn World) -> Result<bool, BdxError> {
        self.limit_run(file, 0x17, |_| vec![Cell::int(command as i32)], world)
    }

    /// The limit's player's motion fired frame trigger 7 with `(p0, p1)` (Steam KH2 `0x14040DD70`
    /// case 7: the object's script's trigger 0x11 with the object and both, Checked (decomp)): run
    /// at once as `limit_command`'s trigger (Infinity's shots: its code starts a thread a shot).
    pub fn limit_object_trigger(&mut self, file: usize, p0: i16, p1: i16, world: &mut dyn World) -> Result<bool, BdxError> {
        self.limit_run(file, 0x11, |c| {
            let caster = c.t.caster;
            let obj = host::obj_record(&mut c.vm, &mut c.t, caster);
            vec![Cell::addr(obj), Cell::int(p0 as i32), Cell::int(p1 as i32)]
        }, world)
    }

    fn limit_run(&mut self, file: usize, key: i32, args: impl FnOnce(&mut Cast) -> Vec<Cell>, world: &mut dyn World) -> Result<bool, BdxError> {
        let Some(i) = self.casts.iter().position(|c| c.t.spell == file && c.t.limit_command.is_some() && !c.finished) else { return Ok(false) };
        let mut cast = std::mem::replace(&mut self.casts[i], placeholder());
        if cast.external {
            if let Some(n) = self.native.as_mut() {
                n.load(cast.id, &mut cast.vm, cast.main);
            }
        }
        let args = args(&mut cast);
        let r = host::run_trigger(self, &mut cast, key, &args, world);
        if cast.external {
            if let Some(n) = self.native.as_mut() {
                n.store(cast.id, &cast.vm, cast.main);
            }
        }
        self.casts[i] = cast;
        // (A hook it started on the host's process runs this same step, as `object_effect`'s.)
        let id = self.casts[i].id;
        if self.casts[i].external {
            let mut k = 0;
            while let Some(ctx) = self.casts.iter().find(|c| c.id == id).and_then(|c| c.t.ext_threads.get(k).copied()) {
                if self.step_external(id, ctx, world) || self.casts.iter().find(|c| c.id == id).and_then(|c| c.t.ext_threads.get(k).copied()) == Some(ctx) {
                    k += 1;
                }
            }
        }
        r
    }

    /// Whether a limit's script (`limit`) of `file` still runs.
    pub fn limit_running(&self, file: usize) -> bool {
        self.casts.iter().any(|c| c.t.spell == file && c.t.limit_command.is_some() && !c.finished)
    }

    /// Ends the limit scripts of `file` (out of the form).
    pub fn limit_end(&mut self, file: usize) -> usize {
        let mut n = 0;
        for c in self.casts.iter_mut().filter(|c| c.t.spell == file && c.t.limit_command.is_some() && !c.finished) {
            c.finished = true;
            n += 1;
        }
        n
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
        // (On the host's process: its registers brought here first, and the trigger's frame
        // handed back to it.)
        if c.external {
            if let Some(n) = self.native.as_mut() {
                n.load(c.id, &mut c.vm, main);
            }
        }
        let r = c.vm.trigger(main, 21, &[]);
        if c.external {
            if let Some(n) = self.native.as_mut() {
                n.store(c.id, &c.vm, main);
            }
        }
        match r {
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
        if cast.external {
            if let Some(n) = self.native.as_mut() {
                n.load(cast.id, &mut cast.vm, cast.main);
            }
        }
        let result = host::run(self, &mut cast, world);
        if cast.external {
            if let Some(n) = self.native.as_mut() {
                n.store(cast.id, &cast.vm, cast.main);
            }
        }
        if let Err(e) = result {
            self.events.push(Event::Error(format!("{}: {e}", self.spells[cast.t.spell].name)));
            cast.finished = true;
        }
        self.casts[i] = cast;
    }

    /// One frame: effects age and follow their bones, then every script
    /// runs. Returns what the host should act on.
    /// The effects age one frame (once per host frame when `frame` is set).
    fn tick(&mut self, world: &mut dyn World) {
        if self.frame.is_some() && self.ticked == self.frame {
            return;
        }
        self.ticked = self.frame;
        let ft = self.frametime;
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
    }

    pub fn update(&mut self, world: &mut dyn World) -> Vec<Event> {
        // Attacks freed before the last update are gone now (the host has seen them freed;
        // one freed since, by the host's machine too, stays for it to see this time).
        self.attacks.retain(|a| !(a.freed && a.freed_seen));
        self.tick(world);
        for i in 0..self.casts.len() {
            // (A cast on the host's process is stepped by the host's machine, `step_external`;
            // one whose process is gone, a room change or a reset, is over.)
            if self.casts[i].external {
                let gone = self.native.as_ref().map_or(true, |n| !n.alive(self.casts[i].id));
                if gone && !self.casts[i].finished {
                    self.casts[i].finished = true;
                    self.events.push(Event::Error(format!("{}: its script process is gone", self.spells[self.casts[i].t.spell].name)));
                }
                continue;
            }
            if !self.casts[i].finished && !self.casts[i].held {
                self.run_cast(i, world);
            }
        }
        // A finished cast's attacks go too (the scripts free their own).
        let live: Vec<bool> = self.casts.iter().map(|c| !c.finished).collect();
        for c in self.casts.iter().filter(|c| c.finished && c.external) {
            if let Some(n) = self.native.as_mut() {
                n.ended(c.id);
            }
        }
        let mut keep = live.iter();
        self.casts.retain(|_| *keep.next().unwrap());
        // Attacks of casts that ended are freed with them.
        for a in &mut self.attacks {
            if !self.casts.iter().any(|c| c.t.attacks.values().any(|k| *k == a.key)) {
                a.freed = true;
            }
            if a.freed {
                a.freed_seen = true;
            }
        }
        std::mem::take(&mut self.events)
    }

    /// The host's machine steps the process of cast `id` (`NativeHost`): its script runs once, as
    /// `update` runs a cast of ours. Returns whether the process goes on (false: the cast is over,
    /// or no longer here, and the process ends).
    /// `ctx` is the cast's main context (its tree and any thread of ours run) or one of its threads
    /// on the host's child processes (that thread alone runs; false once it is done).
    pub fn step_external(&mut self, id: u64, ctx: CtxId, world: &mut dyn World) -> bool {
        let Some(i) = self.casts.iter().position(|c| c.id == id && c.external) else { return false };
        if self.casts[i].finished {
            return false;
        }
        let main = self.casts[i].main;
        if ctx != main && !self.casts[i].t.ext_threads.contains(&ctx) {
            return false;
        }
        // The frame's effect ages first, as `update` has it before any script.
        self.tick(world);
        // Held: it waits, its process with it.
        if self.casts[i].held {
            return true;
        }
        if ctx == main {
            // (A limit's script, t998: its hooks first, then its main context. Its main loop ends the
            // limit once the player's motion isn't the one the hook last started (`limi` pc 5644 /
            // 5792 against wp0, set by pc 1994); the hook starts its next motion the frame the queue
            // reaches it (pc 2629..2663). Run after it, the main context saw the queued motion before
            // the hook restarted it and ended every limit after its first move (the owner's log:
            // "its script is over" ~30 frames in). Assumption: KH2 runs an object's hook threads,
            // attached to it (`0x1403D76D0`), before the limit's thread; the script needs it.)
            if self.casts[i].t.limit_command.is_some() {
                let frame = self.frame;
                let hooks: Vec<CtxId> = self.casts[i].t.hooks.iter().filter(|h| h.2).map(|h| h.1).collect();
                for h in hooks {
                    if !self.casts[i].t.ext_threads.contains(&h) || self.casts[i].t.early.iter().any(|e| e.0 == h && e.1 == frame && frame.is_some()) {
                        continue;
                    }
                    self.run_thread(i, id, h, world);
                    if frame.is_some() {
                        let c = &mut self.casts[i];
                        c.t.early.retain(|e| e.0 != h);
                        c.t.early.push((h, frame));
                    }
                }
                if self.casts[i].finished {
                    return false;
                }
            }
            self.run_cast(i, world);
            return !self.casts[i].finished;
        }
        // (A hook already stepped ahead of its limit's main context this frame: not again.)
        if self.frame.is_some() && self.casts[i].t.early.iter().any(|e| e.0 == ctx && e.1 == self.frame) {
            return !self.casts[i].finished && self.casts[i].t.ext_threads.contains(&ctx);
        }
        self.run_thread(i, id, ctx, world)
    }

    /// One of a cast's threads on the host's child processes run once (its registers from and back
    /// to its process). Whether it goes on.
    fn run_thread(&mut self, i: usize, id: u64, ctx: CtxId, world: &mut dyn World) -> bool {
        let mut cast = std::mem::replace(&mut self.casts[i], placeholder());
        if let Some(n) = self.native.as_mut() {
            n.load(id, &mut cast.vm, ctx);
        }
        let result = host::run_ctx(self, &mut cast, Some(ctx), world);
        if let Some(n) = self.native.as_mut() {
            n.store(id, &cast.vm, ctx);
        }
        if let Err(e) = result {
            self.events.push(Event::Error(format!("{}: {e}", self.spells[cast.t.spell].name)));
            cast.finished = true;
        }
        let goes = !cast.finished && cast.t.ext_threads.contains(&ctx);
        self.casts[i] = cast;
        goes
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
            ext_threads: Vec::new(),
            native_id: None,
            limit_command: None,
            hooks: Vec::new(),
            searchers: HashMap::new(),
            early: Vec::new(),
            target: None,
        },
        main: 0,
        released: true,
        finished: true,
        fresh: false,
        object: false,
        held: false,
        id: 0,
        external: false,
    }
}
