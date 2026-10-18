//! The particle-data player: emitters, instances and the program machine
//! (`docs/kh2-effects.md` §5, §7).
//!
//! One [`Player`] runs one particle data. A tick is split as the game
//! splits it: the **calc pass** spawns instances and runs every program
//! without a draw slot; the **draw pass** runs the calc part of the draw
//! programs (MegaBirth, shape animation, texture scroll, ribbon recording)
//! and then the frames advance. Geometry is produced later, from this
//! state, by [`crate::draw`], because only that needs the camera.

use glam::{Mat4, Vec3};
use kh2_formats::dpd::{EmitterDef, Key, ParticleData, Program, NEVER, SPAWNED, TICK};

use crate::library::Library;
use crate::math;
use crate::megabirth::MegaBirth;
use crate::rng::Rng;
use crate::shipoly::Shipoly;

/// Program types (indices into the game's table, names in
/// [`kh2_formats::dpd::PROGRAM_NAMES`]).
pub(crate) mod k {
    pub const ACCELE: u32 = 28;
    pub const ANG_ACCELE: u32 = 29;
    pub const SCL_ACCELE: u32 = 30;
    pub const COL_ACCELE: u32 = 31;
    pub const MOVE: u32 = 36;
    pub const ANG_MOVE: u32 = 37;
    pub const SCL_MOVE: u32 = 38;
    pub const COL_MOVE: u32 = 39;
    pub const POINT: u32 = 40;
    pub const ANGLE: u32 = 41;
    pub const SCALE: u32 = 42;
    pub const COLOR: u32 = 43;
    /// Vertex morph between two models (drawn by a later DrawMdl*).
    pub const VT_MIME: u32 = 49;
    pub const RAND_FLOAT: u32 = 51;
    pub const RAND_IV: u32 = 66;
    pub const MATRIX_XYZ: u32 = 87;
    pub const MATRIX_YXZ: u32 = 90;
    pub const MATRIX_ZXY: u32 = 91;
    pub const MATRIX_SCL: u32 = 94;
    pub const PAR_MATRIX: u32 = 105;
    pub const DRAW_MATRIX: u32 = 112;
    pub const DRAW_MATRIX_FRONT: u32 = 113;
    /// Billboard turning only about the root's own y axis.
    pub const DRAW_MATRIX_WOOD: u32 = 115;
    /// `D` through the root matrix saved when the instance was spawned.
    pub const RYJ_DRAW_MATRIX_WORLD: u32 = 120;
    pub const KE_Z_CRCT: u32 = 124;
    pub const KE_Z_CRCT_SHP: u32 = 125;
    pub const DRAW_MDL3: u32 = 137;
    pub const DRAW_MDL_TS3: u32 = 139;
    pub const DRAW_SHAPE: u32 = 141;
    pub const DRAW_SHAPE_X: u32 = 142;
    /// VertexApLc with the vertex carried through the spawner's matrix.
    pub const VERTEX_AP: u32 = 174;
    pub const VERTEX_AP_LC: u32 = 177;
    pub const MEGA_BIRTH: u32 = 201;
    /// Same routine as MatrixXYZ.
    pub const MATRIX_LOOP: u32 = 216;
    pub const DRAW_MATRIX_LOOP: u32 = 217;
    pub const DRAW_MDL_LOOP_Z: u32 = 219;
    /// Same routine as VertexApLc.
    pub const VERTEX_AP_LC_LOOP: u32 = 236;
    pub const SHIPOLY: u32 = 241;
}

/// Program types this crate runs.
pub fn supported(kind: u32) -> bool {
    matches!(
        kind,
        28..=31 | 36..=43 | 49 | 51 | 66 | 84 | 87 | 90 | 91 | 94 | 105 | 112 | 113 | 115 | 120 | 124 | 125 | 137 | 139 | 141 | 142 | 174 | 177 | 201 | 216 | 217 | 219 | 236 | 241
    )
}

/// Programs with a draw slot: they run in the draw pass.
pub(crate) fn has_draw(kind: u32) -> bool {
    matches!(kind, k::VT_MIME | k::DRAW_MDL3 | k::DRAW_MDL_TS3 | k::DRAW_SHAPE | k::DRAW_SHAPE_X | k::MEGA_BIRTH | k::DRAW_MDL_LOOP_Z | k::SHIPOLY)
}

/// Programs skipped while an instance is fast-forwarded after a loop.
fn skipped_in_fast_forward(kind: u32) -> bool {
    matches!(kind, k::MEGA_BIRTH | k::SHIPOLY)
}

/// The instance's work area (the bytes after the 0xA0-byte header).
#[derive(Clone, Debug)]
pub(crate) struct Work(Vec<u8>);

impl Work {
    fn new(size: usize) -> Work {
        Work(vec![0; size])
    }
    fn get<const N: usize>(&self, o: i32) -> [u8; N] {
        let o = o as usize;
        match self.0.get(o..o + N) {
            Some(s) if (o as i32) >= 0 => s.try_into().unwrap(),
            _ => [0; N],
        }
    }
    fn set<const N: usize>(&mut self, o: i32, v: [u8; N]) {
        if o < 0 {
            return;
        }
        let o = o as usize;
        if let Some(s) = self.0.get_mut(o..o + N) {
            s.copy_from_slice(&v);
        }
    }
    pub fn f32(&self, o: i32) -> f32 {
        f32::from_le_bytes(self.get(o))
    }
    pub fn set_f32(&mut self, o: i32, v: f32) {
        self.set(o, v.to_le_bytes())
    }
    pub fn i32(&self, o: i32) -> i32 {
        i32::from_le_bytes(self.get(o))
    }
    pub fn set_i32(&mut self, o: i32, v: i32) {
        self.set(o, v.to_le_bytes())
    }
    pub fn u16(&self, o: i32) -> u16 {
        u16::from_le_bytes(self.get(o))
    }
    pub fn set_u16(&mut self, o: i32, v: u16) {
        self.set(o, v.to_le_bytes())
    }
    pub fn set_u8(&mut self, o: i32, v: u8) {
        self.set(o, [v])
    }
    pub fn vec3(&self, o: i32) -> Vec3 {
        Vec3::new(self.f32(o), self.f32(o + 4), self.f32(o + 8))
    }
    pub fn set_vec3(&mut self, o: i32, v: Vec3) {
        self.set_f32(o, v.x);
        self.set_f32(o + 4, v.y);
        self.set_f32(o + 8, v.z);
    }
    /// The four colour bytes a Color program writes at its offset + 8.
    pub fn rgba(&self, o: i32) -> [u8; 4] {
        self.get(o + 8)
    }
    fn zero(&mut self, o: i32, n: usize) {
        if o < 0 {
            return;
        }
        let o = o as usize;
        let end = (o + n).min(self.0.len());
        if o < end {
            self.0[o..end].fill(0);
        }
    }
}

/// How the draw matrix `D` was last built.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub(crate) enum DKind {
    #[default]
    None,
    /// `D = P·V·Root·M`: oriented in 3D.
    Full,
    /// Billboard: `M`'s axes (times the root scale lengths) used directly
    /// in view space at the view position of `M`'s origin.
    Front,
    /// Upright billboard (pppDrawMatrixWood): `M`'s axes times the root
    /// scale lengths, and `M`'s translation, put through the frame that
    /// keeps the root's y axis and turns x to face the screen.
    Wood,
    /// `D = P·V·R·M` with `R` the root matrix saved at spawn
    /// (pppRyjDrawMatrixWorld): the instance stays where it was born.
    World(Mat4),
}

/// Per-program state that the game keeps behind pointers in the work area.
#[derive(Clone, Debug, Default)]
pub(crate) enum ProgState {
    #[default]
    None,
    MegaBirth(Box<MegaBirth>),
    Shipoly(Box<Shipoly>),
}

/// One live instance of an emitter ("pObj").
#[derive(Clone, Debug)]
pub(crate) struct Instance {
    pub uid: u64,
    /// (emitter, uid) of the instance that spawned this one.
    pub parent: Option<(usize, u64)>,
    /// Instance frame, ×4096.
    pub frame: i32,
    /// This tick has been computed (the game's frame bit 0).
    pub computed: bool,
    /// Spawned by a program this tick: skipped until the next one.
    pub spawned: bool,
    /// Loop restarted with loop start 0: reset again before the next run.
    pub reset_pending: bool,
    pub m: Mat4,
    pub d: DKind,
    /// The program (index) whose VtMime vertices replace the model's in
    /// this instance's model draws (the game's vertex override at +0x90).
    pub mime: Option<usize>,
    pub work: Work,
    /// Current key per program.
    pub cursor: Vec<usize>,
    pub state: Vec<ProgState>,
}

impl Instance {
    /// Program constructs that do more than zero their work: RyjDrawMatrixWorld
    /// saves the current root matrix at its `w1`.
    fn constructed(mut self, def: &EmitterDef, root: Mat4) -> Instance {
        for prog in &def.programs {
            if prog.kind == k::RYJ_DRAW_MATRIX_WORLD {
                let o = w(prog, 1);
                for (i, v) in root.to_cols_array().into_iter().enumerate() {
                    self.work.set_f32(o + i as i32 * 4, v);
                }
            }
        }
        self
    }

    fn saved_matrix(&self, o: i32) -> Mat4 {
        let mut a = [0.0; 16];
        for (i, v) in a.iter_mut().enumerate() {
            *v = self.work.f32(o + i as i32 * 4);
        }
        Mat4::from_cols_array(&a)
    }

    pub fn key<'a>(&self, prog: &'a Program, pi: usize) -> Option<&'a Key> {
        prog.keys.get(self.cursor[pi]).or(prog.keys.last())
    }
}

/// Effect-wide values the programs read.
#[derive(Clone, Copy, Debug)]
pub(crate) struct EffectView {
    pub root: Mat4,
    pub rs: Vec3,
    /// RGBA multiplier for the Color programs (element fade in alpha).
    pub colour: [f32; 4],
}

/// One emitter's runtime state.
#[derive(Clone, Debug)]
struct EmitterRt {
    /// Effect frame at which the clock spawns it (`SPAWNED` once done).
    start: i32,
    /// Instances in the game's group order: the first spawned, then the
    /// newest children.
    group: Vec<Instance>,
}

struct Spawn {
    emitter: usize,
    parent: (usize, u64),
    offset: i32,
    value: Vec3,
}

/// Runs one particle data.
#[derive(Clone, Debug)]
pub(crate) struct Player {
    pub dpd: usize,
    pub pd: usize,
    /// Effect frame, ×4096.
    pub frame: i32,
    /// Ended from outside: no more loops, endless instances go.
    pub ended: bool,
    pub finished: bool,
    pub fx: EffectView,
    emitters: Vec<EmitterRt>,
    /// Emitter indices in priority order.
    priority: Vec<usize>,
    next_uid: u64,
}

fn w(p: &Program, i: usize) -> i32 {
    p.work.get(i).copied().unwrap_or(0)
}

impl Player {
    pub fn new(lib: &Library, dpd: usize, pd: usize) -> Player {
        let data = &lib.dpds[dpd].particles[pd];
        let emitters = data.emitters.iter().map(|e| EmitterRt { start: e.start, group: Vec::new() }).collect();
        let mut priority: Vec<usize> = (0..data.emitters.len()).collect();
        priority.sort_by_key(|&i| data.emitters[i].priority);
        Player {
            dpd,
            pd,
            frame: 0,
            ended: false,
            finished: false,
            fx: EffectView { root: Mat4::IDENTITY, rs: Vec3::ONE, colour: [1.0; 4] },
            emitters,
            priority,
            next_uid: 1,
        }
    }

    pub fn data<'a>(&self, lib: &'a Library) -> &'a ParticleData {
        &lib.dpds[self.dpd].particles[self.pd]
    }

    /// Effect root: `T(pos)·R·S(scale)` times the particle data's own
    /// offset, angle and scale; `rs` are its axis lengths.
    pub fn set_root(&mut self, lib: &Library, pos: Vec3, rot: glam::Mat3, scale: Vec3) {
        let d = self.data(lib);
        let header = math::trs(Vec3::from(d.offset), math::euler_xyz_i(d.angle), Vec3::from(d.scale));
        let root = math::trs(pos, rot, scale) * header;
        self.fx.root = root;
        self.fx.rs = math::axis_lengths(&root);
    }

    pub fn instance_count(&self) -> usize {
        self.emitters.iter().map(|e| e.group.len()).sum()
    }

    /// Instances of emitter `e` in draw order.
    pub(crate) fn group(&self, e: usize) -> &[Instance] {
        &self.emitters[e].group
    }

    fn find(&self, (e, uid): (usize, u64)) -> Option<&Instance> {
        self.emitters.get(e)?.group.iter().find(|i| i.uid == uid)
    }

    fn new_instance(&mut self, def: &EmitterDef) -> Instance {
        let uid = self.next_uid;
        self.next_uid += 1;
        let size = (def.work_size.max(0xa0) as usize - 0xa0) + 64;
        let n = def.programs.len();
        Instance {
            uid,
            parent: None,
            frame: 0,
            computed: false,
            spawned: false,
            reset_pending: false,
            m: Mat4::IDENTITY,
            d: DKind::None,
            mime: None,
            work: Work::new(size),
            cursor: vec![0; n],
            state: vec![ProgState::None; n],
        }
        .constructed(def, self.fx.root)
    }

    /// Spawn emitters whose start frame has come (each fires once).
    fn spawn_clock(&mut self, lib: &Library) {
        let data = &lib.dpds[self.dpd].particles[self.pd];
        for (e, def) in data.emitters.iter().enumerate() {
            if self.emitters[e].start == SPAWNED || self.emitters[e].start > self.frame {
                continue;
            }
            self.emitters[e].start = SPAWNED;
            let inst = self.new_instance(def);
            insert(&mut self.emitters[e].group, inst);
        }
    }

    fn apply_spawns(&mut self, lib: &Library, spawns: Vec<Spawn>) {
        let data = &lib.dpds[self.dpd].particles[self.pd];
        for s in spawns {
            let Some(def) = data.emitters.get(s.emitter) else { continue };
            let mut inst = self.new_instance(def);
            inst.parent = Some(s.parent);
            inst.spawned = true;
            inst.work.set_vec3(s.offset, s.value);
            insert(&mut self.emitters[s.emitter].group, inst);
        }
    }

    /// The calc pass: spawn, then every program without a draw slot.
    pub fn calc_pass(&mut self, lib: &Library, rng: &mut Rng) {
        self.spawn_clock(lib);
        let data = &lib.dpds[self.dpd].particles[self.pd];
        for (e, def) in data.emitters.iter().enumerate() {
            for (pi, prog) in def.programs.iter().enumerate() {
                if has_draw(prog.kind) {
                    continue;
                }
                let mut spawns = Vec::new();
                for ii in 0..self.emitters[e].group.len() {
                    if self.emitters[e].group[ii].spawned {
                        continue;
                    }
                    let parent_m = self.emitters[e].group[ii].parent.and_then(|p| self.find(p)).map(|p| p.m);
                    let fx = self.fx;
                    let inst = &mut self.emitters[e].group[ii];
                    prepare(inst, def, pi, prog);
                    calc(lib, self.dpd, rng, &fx, inst, (e, pi), prog, parent_m, false, &mut spawns);
                }
                self.apply_spawns(lib, spawns);
            }
        }
    }

    /// The draw pass minus the drawing: the draw programs' calc parts. On a
    /// repeat frame (`repeat`, the game's first of the two frames a tick is
    /// shown) MegaBirth does not step its particles.
    pub fn draw_pass(&mut self, lib: &Library, rng: &mut Rng, repeat: bool) {
        let data = &lib.dpds[self.dpd].particles[self.pd];
        let fx = self.fx;
        for (e, def) in data.emitters.iter().enumerate() {
            for (pi, prog) in def.programs.iter().enumerate() {
                if !has_draw(prog.kind) {
                    continue;
                }
                for inst in self.emitters[e].group.iter_mut() {
                    if inst.spawned {
                        continue;
                    }
                    prepare(inst, def, pi, prog);
                    draw_calc(lib, self.dpd, rng, &fx, inst, pi, prog, repeat);
                }
            }
            if repeat {
                for inst in self.emitters[e].group.iter_mut() {
                    if !inst.spawned {
                        inst.computed = true;
                    }
                }
            }
        }
    }

    /// End of a shown tick: frames advance, instances loop or expire, the
    /// effect frame advances. Sets `finished` when the effect is over.
    pub fn advance(&mut self, lib: &Library, rng: &mut Rng) {
        let data = &lib.dpds[self.dpd].particles[self.pd];
        for em in self.emitters.iter_mut() {
            for inst in em.group.iter_mut() {
                if inst.spawned {
                    inst.spawned = false;
                } else {
                    inst.frame = ((inst.frame & !0xfff) + TICK).min(0x6fff_f000);
                    inst.computed = false;
                }
            }
        }
        let mut totals: u32 = 0;
        for &e in &self.priority.clone() {
            let def = &data.emitters[e];
            let mut i = 0;
            while i < self.emitters[e].group.len() {
                let f = self.emitters[e].group[i].frame;
                let loops = def.loop_start < NEVER && def.loop_end < NEVER;
                if !self.ended && loops && def.loop_end <= f {
                    self.restart(lib, rng, e, i);
                    i += 1;
                    continue;
                }
                totals |= def.total as u32;
                if def.total as u32 <= f as u32 {
                    self.emitters[e].group.remove(i);
                } else {
                    i += 1;
                }
            }
        }
        if data.looping && self.ended && totals == NEVER as u32 {
            for em in self.emitters.iter_mut() {
                em.group.clear();
            }
        }
        self.frame = self.frame.saturating_add(TICK);
        if !data.looping {
            if data.end <= self.frame {
                self.finished = true;
            }
        } else if self.ended {
            if self.instance_count() == 0 {
                self.finished = true;
            }
        } else if self.frame > 0x6fff_ffff {
            self.frame = NEVER;
        }
        if self.finished {
            for em in self.emitters.iter_mut() {
                em.group.clear();
            }
        }
    }

    /// Instance loop: back to frame 0, reset, fast-forward to the loop start.
    fn restart(&mut self, lib: &Library, rng: &mut Rng, e: usize, i: usize) {
        let data = &lib.dpds[self.dpd].particles[self.pd];
        let def = &data.emitters[e];
        {
            let inst = &mut self.emitters[e].group[i];
            inst.frame = 0;
            reset(inst, def);
        }
        let mut guard = 0;
        loop {
            self.fast_step(lib, rng, e, i);
            guard += 1;
            let inst = &mut self.emitters[e].group[i];
            if def.loop_start == 0 {
                inst.reset_pending = true;
                break;
            }
            if inst.frame >= def.loop_start || guard > 4096 {
                break;
            }
        }
    }

    /// One calc-only step of one instance (no MegaBirth, no ribbons, no
    /// spawning), then its frame advances.
    fn fast_step(&mut self, lib: &Library, rng: &mut Rng, e: usize, i: usize) {
        let data = &lib.dpds[self.dpd].particles[self.pd];
        let def = &data.emitters[e];
        let parent_m = self.emitters[e].group[i].parent.and_then(|p| self.find(p)).map(|p| p.m);
        let fx = self.fx;
        let inst = &mut self.emitters[e].group[i];
        for (pi, prog) in def.programs.iter().enumerate() {
            if skipped_in_fast_forward(prog.kind) {
                continue;
            }
            advance_cursor(inst, pi, prog);
            if has_draw(prog.kind) {
                draw_calc_ff(lib, self.dpd, inst, pi, prog);
            } else {
                let mut none = Vec::new();
                calc(lib, self.dpd, rng, &fx, inst, (e, pi), prog, parent_m, true, &mut none);
            }
        }
        inst.frame = ((inst.frame & !0xfff) + TICK).min(0x6fff_f000);
        inst.computed = false;
    }
}

/// New instances of an emitter: the first one heads the group, later ones
/// go right after the head (the game links them after the group's first).
fn insert(group: &mut Vec<Instance>, inst: Instance) {
    if group.is_empty() {
        group.push(inst);
    } else {
        group.insert(1, inst);
    }
}

fn advance_cursor(inst: &mut Instance, pi: usize, prog: &Program) {
    if inst.computed {
        return;
    }
    let c = inst.cursor[pi];
    if let Some(next) = prog.keys.get(c + 1) {
        if next.frame() == inst.frame {
            inst.cursor[pi] = c + 1;
        }
    }
}

/// Before a program runs on an instance: a pending loop reset, then the
/// key cursor.
fn prepare(inst: &mut Instance, def: &EmitterDef, pi: usize, prog: &Program) {
    if !inst.computed && inst.reset_pending {
        inst.frame = 0;
        reset(inst, def);
        inst.reset_pending = false;
    }
    advance_cursor(inst, pi, prog);
}

/// Program reset at an instance loop (the construct values again).
fn reset(inst: &mut Instance, def: &EmitterDef) {
    for (pi, prog) in def.programs.iter().enumerate() {
        inst.cursor[pi] = 0;
        let w0 = w(prog, 0);
        match prog.kind {
            k::ACCELE | k::SCL_ACCELE | k::MOVE | k::SCL_MOVE | k::POINT | k::SCALE | k::ANG_ACCELE | k::ANG_MOVE | k::ANGLE => inst.work.zero(w0, 12),
            k::COL_ACCELE | k::COL_MOVE | k::COLOR => inst.work.zero(w0, 8),
            k::DRAW_SHAPE | k::DRAW_SHAPE_X => inst.work.zero(w0, 6),
            k::DRAW_MDL_TS3 | k::DRAW_MDL_LOOP_Z => inst.work.zero(w(prog, 2), 28),
            // VertexAp has no reset slot: its countdown carries over a loop.
            k::VERTEX_AP_LC | k::VERTEX_AP_LC_LOOP => inst.work.zero(w0, 4),
            // Reset clears t, its rate and acceleration (the buffer stays).
            k::VT_MIME => inst.work.zero(w0, 12),
            k::SHIPOLY => {
                if let ProgState::Shipoly(s) = &mut inst.state[pi] {
                    s.reset();
                }
            }
            _ => {}
        }
    }
}

/// A delta key applies on the exact tick its frame equals the instance's.
fn on_key(inst: &Instance, key: &Key) -> bool {
    !inst.computed && key.frame() == inst.frame
}

#[allow(clippy::too_many_arguments)]
fn calc(
    lib: &Library,
    dpd: usize,
    rng: &mut Rng,
    fx: &EffectView,
    inst: &mut Instance,
    (_e, pi): (usize, usize),
    prog: &Program,
    parent_m: Option<Mat4>,
    fast_forward: bool,
    spawns: &mut Vec<Spawn>,
) {
    let Some(key) = inst.key(prog, pi) else { return };
    let (w0, w1, w2) = (w(prog, 0), w(prog, 1), w(prog, 2));
    match prog.kind {
        // Accele/Move: on a key tick w1 += key, then w0 += w1 every tick.
        k::ACCELE | k::SCL_ACCELE | k::MOVE | k::SCL_MOVE => {
            if inst.computed {
                return;
            }
            if key.frame() == inst.frame {
                let v = inst.work.vec3(w1) + Vec3::from(key.vec3(4));
                inst.work.set_vec3(w1, v);
            }
            let v = inst.work.vec3(w0) + inst.work.vec3(w1);
            inst.work.set_vec3(w0, v);
        }
        k::ANG_ACCELE | k::ANG_MOVE => {
            if inst.computed {
                return;
            }
            let on = key.frame() == inst.frame;
            for c in 0..3 {
                let o = c * 4;
                if on {
                    inst.work.set_i32(w1 + o, inst.work.i32(w1 + o).wrapping_add(key.i32(4 + o as usize)));
                }
                inst.work.set_i32(w0 + o, inst.work.i32(w0 + o).wrapping_add(inst.work.i32(w1 + o)));
            }
        }
        k::COL_ACCELE | k::COL_MOVE => {
            if inst.computed {
                return;
            }
            let on = key.frame() == inst.frame;
            for c in 0..4 {
                let o = c * 2;
                if on {
                    inst.work.set_u16(w1 + o, inst.work.u16(w1 + o).wrapping_add(key.u16(4 + o as usize)));
                }
                inst.work.set_u16(w0 + o, inst.work.u16(w0 + o).wrapping_add(inst.work.u16(w1 + o)));
            }
        }
        k::POINT | k::SCALE => {
            if on_key(inst, key) {
                let v = inst.work.vec3(w0) + Vec3::from(key.vec3(4));
                inst.work.set_vec3(w0, v);
            }
        }
        k::ANGLE => {
            if on_key(inst, key) {
                for c in 0..3 {
                    let o = c * 4;
                    inst.work.set_i32(w0 + o, inst.work.i32(w0 + o).wrapping_add(key.i32(4 + o as usize)));
                }
            }
        }
        k::COLOR => {
            if on_key(inst, key) {
                for c in 0..4 {
                    let o = c * 2;
                    inst.work.set_u16(w0 + o, inst.work.u16(w0 + o).wrapping_add(key.u16(4 + o as usize)));
                }
            }
            // Every call: the colour bytes (0x80 = 1.0) at w0 + 8.
            for c in 0..4 {
                let v = (inst.work.u16(w0 + c * 2) as f32 * fx.colour[c as usize]).min(32767.0) / 128.0;
                inst.work.set_u8(w0 + 8 + c, v as i32 as u8);
            }
        }
        k::RAND_FLOAT => {
            if inst.computed {
                return;
            }
            let r = if inst.frame == 0 {
                let r = if key.u8(12) == 0 { rng.rand01() * 2.0 } else { rng.rand01() + rng.rand01() };
                inst.work.set_f32(w0, r);
                r
            } else if key.frame() == inst.frame {
                inst.work.f32(w0)
            } else {
                return;
            };
            let target = key.i32(4);
            if target != -1 {
                let range = key.f32(8);
                inst.work.set_f32(target, range * r - range + inst.work.f32(target));
            }
        }
        k::RAND_IV => {
            if !on_key(inst, key) {
                return;
            }
            let r = if key.u8(20) == 0 { rng.rand01() * 2.0 } else { rng.rand01() + rng.rand01() };
            inst.work.set_f32(w0, r);
            let target = key.i32(4);
            if target != -1 {
                for c in 0..3 {
                    let range = key.i32(8 + c * 4) as f32;
                    let o = target + c as i32 * 4;
                    inst.work.set_i32(o, inst.work.i32(o).wrapping_add((range * r - range) as i32));
                }
            }
        }
        k::MATRIX_XYZ | k::MATRIX_LOOP | k::MATRIX_YXZ | k::MATRIX_ZXY => {
            let a = [inst.work.i32(w1), inst.work.i32(w1 + 4), inst.work.i32(w1 + 8)];
            let r = match prog.kind {
                k::MATRIX_YXZ => math::euler_yxz_i(a),
                k::MATRIX_ZXY => math::euler_zxy_i(a),
                _ => math::euler_xyz_i(a),
            };
            inst.m = math::trs(inst.work.vec3(w0), r, inst.work.vec3(w2));
        }
        k::MATRIX_SCL => {
            inst.m = math::trs(inst.work.vec3(w0), glam::Mat3::IDENTITY, inst.work.vec3(w1));
        }
        k::PAR_MATRIX => {
            if let Some(pm) = parent_m {
                let t = pm.transform_point3(inst.m.w_axis.truncate());
                inst.m.w_axis = t.extend(inst.m.w_axis.w);
            }
        }
        k::DRAW_MATRIX | k::DRAW_MATRIX_LOOP => inst.d = DKind::Full,
        k::DRAW_MATRIX_FRONT => inst.d = DKind::Front,
        k::DRAW_MATRIX_WOOD => inst.d = DKind::Wood,
        k::RYJ_DRAW_MATRIX_WORLD => inst.d = DKind::World(inst.saved_matrix(w1)),
        k::VERTEX_AP | k::VERTEX_AP_LC | k::VERTEX_AP_LC_LOOP => {
            if inst.computed || fast_forward || key.i16(4) < 0 {
                return;
            }
            let mut count = inst.work.u16(w0 + 2);
            if count == 0 {
                let d = &lib.dpds[dpd];
                if let Some(vs) = d.vertex_set(key.i16(4) as i32) {
                    let verts = d.model(vs.model).map(|m| m.vertices.as_slice()).unwrap_or(&[]);
                    let n = key.u8(6);
                    // VertexAp hands the child the vertex through the
                    // spawner's own matrix; VertexApLc the raw vertex.
                    let carry = (prog.kind == k::VERTEX_AP).then_some(inst.m);
                    let mut pick = |idx: usize| {
                        let v = vs.indices.get(idx).and_then(|&i| verts.get(i as usize)).copied().unwrap_or([0; 3]);
                        let v = Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
                        spawns.push(Spawn {
                            emitter: key.i32(12) as usize,
                            parent: (_e, inst.uid),
                            offset: key.i32(16),
                            value: carry.map_or(v, |m| m.transform_point3(v)),
                        });
                    };
                    if key.i32(12) != -1 && !vs.indices.is_empty() {
                        if key.u8(8) == 0 {
                            for _ in 0..n {
                                let mut c = inst.work.u16(w0);
                                if vs.indices.len() as u16 <= c {
                                    c = 0;
                                }
                                inst.work.set_u16(w0, c + 1);
                                pick(c as usize);
                            }
                        } else if key.u8(8) == 1 {
                            for _ in 0..n {
                                let r = rng.rand01();
                                pick((vs.indices.len() as f32 * r) as usize);
                            }
                        }
                    }
                }
                count = key.u8(7) as u16;
            }
            inst.work.set_u16(w0 + 2, count.wrapping_sub(1));
        }
        // S-matrix pointer (unused on PC), Z correction (built at draw time).
        _ => {}
    }
}

/// The calc part of the draw programs.
#[allow(clippy::too_many_arguments)]
fn draw_calc(lib: &Library, dpd: usize, rng: &mut Rng, fx: &EffectView, inst: &mut Instance, pi: usize, prog: &Program, repeat: bool) {
    let Some(key) = inst.key(prog, pi) else { return };
    match prog.kind {
        k::DRAW_SHAPE | k::DRAW_SHAPE_X => {
            if !inst.computed {
                shape_anim(lib, dpd, &mut inst.work, w(prog, 0), key.i32(4), key.i16(8));
            }
        }
        k::DRAW_MDL_TS3 => {
            if inst.computed {
                return;
            }
            let o = w(prog, 2);
            let wk = &mut inst.work;
            let (u, du, ddu, v, dv, ddv) = (wk.f32(o), wk.f32(o + 4), wk.f32(o + 8), wk.f32(o + 12), wk.f32(o + 16), wk.f32(o + 20));
            let du = du + ddu;
            let u = u + du;
            let dv = dv + ddv;
            let v = v + dv;
            let on = key.frame() == inst.frame;
            let add = |i: usize| if on { key.f32(8 + i * 4) } else { 0.0 };
            for (i, val) in [u, du, ddu, v, dv, ddv].into_iter().enumerate() {
                wk.set_f32(o + i as i32 * 4, val + add(i));
            }
        }
        k::DRAW_MDL_LOOP_Z => loop_z(inst, prog, key, false),
        k::VT_MIME => {
            if !inst.computed {
                vt_mime_step(&mut inst.work, w(prog, 0), key, key.frame() == inst.frame);
            }
            // The draw half points the instance's model draws at the morph.
            if key.i32(4) != 0xffff || key.i32(8) != 0xffff {
                inst.mime = Some(pi);
            }
        }
        k::MEGA_BIRTH => {
            let alpha_bytes = inst.work.rgba(w(prog, 1));
            let m = inst.m;
            if !matches!(inst.state[pi], ProgState::MegaBirth(_)) {
                inst.state[pi] = ProgState::MegaBirth(Box::new(MegaBirth::new(key)));
            }
            if let ProgState::MegaBirth(mb) = &mut inst.state[pi] {
                mb.calc(lib, dpd, rng, fx, key, m, alpha_bytes, repeat);
            }
        }
        k::SHIPOLY => {
            let (m, frame, computed) = (inst.m, inst.frame, inst.computed);
            if !matches!(inst.state[pi], ProgState::Shipoly(_)) {
                if computed {
                    return;
                }
                inst.state[pi] = ProgState::Shipoly(Box::new(Shipoly::new(key, rng)));
            }
            if let ProgState::Shipoly(s) = &mut inst.state[pi] {
                if !computed {
                    s.record(key, frame, m, fx);
                }
                if !repeat {
                    s.scroll(key);
                }
            }
        }
        _ => {}
    }
}

/// Draw programs' calc while fast-forwarding (no ribbons or particles).
fn draw_calc_ff(lib: &Library, dpd: usize, inst: &mut Instance, pi: usize, prog: &Program) {
    let Some(key) = inst.key(prog, pi) else { return };
    match prog.kind {
        k::DRAW_SHAPE | k::DRAW_SHAPE_X => shape_anim(lib, dpd, &mut inst.work, w(prog, 0), key.i32(4), key.i16(8)),
        k::DRAW_MDL_LOOP_Z => loop_z(inst, prog, key, true),
        _ => {}
    }
}

/// pppVtMime calc: `w0` holds `t`, its rate and its acceleration. Every
/// tick `rate += accel; t += rate`; on a key tick `t += @12`, `rate +=
/// @16` (unless @16 is below -1e38, the "keep" marker) and `accel += @20`.
pub(crate) fn vt_mime_step(wk: &mut Work, o: i32, key: &Key, on_key: bool) {
    let (t, rate, accel) = (wk.f32(o), wk.f32(o + 4), wk.f32(o + 8));
    let rate = rate + accel;
    let t = t + rate;
    if on_key {
        wk.set_f32(o, t + key.f32(12));
        let dv = key.f32(16);
        wk.set_f32(o + 4, if dv as f64 >= -1e38 { rate + dv } else { rate });
        wk.set_f32(o + 8, accel + key.f32(20));
    } else {
        wk.set_f32(o, t);
        wk.set_f32(o + 4, rate);
    }
}

/// pppVtMime draw: model A's vertices moved toward model B's by `t`,
/// `trunc(a + (b - a)·t)` per component as i16 (A's vertex count).
pub(crate) fn vt_mime_vertices(a: &[[i16; 3]], b: &[[i16; 3]], t: f32) -> Vec<[i16; 3]> {
    a.iter()
        .enumerate()
        .map(|(i, va)| {
            let vb = b.get(i).copied().unwrap_or([0; 3]);
            std::array::from_fn(|c| ((vb[c] as i32 - va[c] as i32) as f32 * t + va[c] as f32) as i32 as i16)
        })
        .collect()
}

/// pppDrawMdlLoopZ scroll: positions wrap at 32768, velocities pick up
/// the key deltas on key ticks and their accelerations every tick.
fn loop_z(inst: &mut Instance, prog: &Program, key: &Key, fast_forward: bool) {
    let o = w(prog, 2);
    let wk = &mut inst.work;
    if wk.i32(o) == 0 {
        wk.set_i32(o, 1);
        wk.set_f32(o + 4, key.f32(8));
        wk.set_f32(o + 16, key.f32(20));
        for i in [8, 12, 20, 24] {
            wk.set_f32(o + i, 0.0);
        }
    }
    if inst.computed {
        return;
    }
    if key.frame() == inst.frame {
        wk.set_f32(o + 8, wk.f32(o + 8) + key.f32(12));
        wk.set_f32(o + 12, wk.f32(o + 12) + key.f32(16));
        wk.set_f32(o + 20, wk.f32(o + 20) + key.f32(24));
        wk.set_f32(o + 24, wk.f32(o + 24) + key.f32(28));
    }
    if !fast_forward {
        let du = wk.f32(o + 12) + wk.f32(o + 8);
        let dv = wk.f32(o + 24) + wk.f32(o + 20);
        wk.set_f32(o + 8, du);
        wk.set_f32(o + 20, dv);
        wk.set_f32(o + 4, (du + wk.f32(o + 4)) % 32768.0);
        wk.set_f32(o + 16, (dv + wk.f32(o + 16)) % 32768.0);
    }
}

/// Shape animation (pppDrawShapeX calc): `time += speed`; past the wait
/// of the step, move on; at the end loop or hold. The wait and loop flag
/// are read from the step that was drawn last (the game's lag), and the
/// drawn step is the one before this advance (`w0 + 4`).
pub(crate) fn shape_anim(lib: &Library, dpd: usize, wk: &mut Work, o: i32, shape: i32, speed: i16) {
    if shape == 0xffff {
        return;
    }
    let Some(sh) = lib.dpds[dpd].shape(shape) else { return };
    let (time, step, prev) = (wk.u16(o), wk.u16(o + 2), wk.u16(o + 4));
    let (t, s, p) = anim_step(sh, time, step, prev, speed);
    wk.set_u16(o, t);
    wk.set_u16(o + 2, s);
    wk.set_u16(o + 4, p);
}

/// One animation step: returns (time, step, drawn step).
pub(crate) fn anim_step(sh: &kh2_formats::dpd::Shape, time: u16, step: u16, prev: u16, speed: i16) -> (u16, u16, u16) {
    let n = sh.sequence.len();
    if n == 0 {
        return (time, step, prev);
    }
    let rec = &sh.sequence[(prev as usize).min(n - 1)];
    let drawn = step;
    let mut time = time.wrapping_add(speed as u16);
    let mut step = step;
    let wait = rec.wait as i16 as i32;
    if wait <= time as i32 {
        time = time.wrapping_sub(wait as u16);
        step = drawn.wrapping_add(1);
        if n as i32 <= step as i32 {
            time = 0;
            step = if rec.loops { 0 } else { drawn };
        }
    }
    (time, step, drawn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn work_access() {
        let mut wk = Work::new(32);
        wk.set_vec3(4, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(wk.vec3(4), Vec3::new(1.0, 2.0, 3.0));
        wk.set_u16(20, 0xfff0);
        wk.set_u16(20, wk.u16(20).wrapping_add(0x20));
        assert_eq!(wk.u16(20), 0x10);
        // Out of range reads are zero, writes are dropped.
        assert_eq!(wk.f32(100), 0.0);
        wk.set_f32(-4, 1.0);
        wk.set_f32(30, 1.0);
    }

    #[test]
    fn supported_set() {
        for kind in [28, 39, 43, 51, 66, 87, 105, 113, 124, 137, 142, 177, 201, 219, 241] {
            assert!(supported(kind), "{kind}");
        }
        assert!(!supported(1) && !supported(200));
        assert!(has_draw(201) && !has_draw(113));
        for kind in [49, 115, 120, 174] {
            assert!(supported(kind), "{kind}");
        }
        assert!(has_draw(k::VT_MIME) && !has_draw(k::DRAW_MATRIX_WOOD) && !has_draw(k::RYJ_DRAW_MATRIX_WORLD) && !has_draw(k::VERTEX_AP));
    }

    fn key(frame: i32, words: &[f32]) -> Key {
        let mut data = frame.to_le_bytes().to_vec();
        for v in words {
            data.extend_from_slice(&v.to_le_bytes());
        }
        Key { data }
    }

    #[test]
    fn vt_mime_integrates_t() {
        let mut wk = Work::new(16);
        // Models (ignored here), then t, rate and acceleration deltas.
        let k0 = key(0, &[0.0, 0.0, 0.25, 0.5, -0.125]);
        vt_mime_step(&mut wk, 0, &k0, true);
        assert_eq!((wk.f32(0), wk.f32(4), wk.f32(8)), (0.25, 0.5, -0.125));
        // Next tick: rate += accel, then t += rate.
        vt_mime_step(&mut wk, 0, &k0, false);
        assert_eq!((wk.f32(0), wk.f32(4), wk.f32(8)), (0.625, 0.375, -0.125));
        // A rate delta of -FLT_MAX leaves the rate alone.
        let k1 = key(0, &[0.0, 0.0, 0.0, f32::MIN, 0.125]);
        vt_mime_step(&mut wk, 0, &k1, true);
        assert_eq!((wk.f32(0), wk.f32(4), wk.f32(8)), (0.875, 0.25, 0.0));
    }

    #[test]
    fn vt_mime_morphs_and_truncates() {
        let a = [[0i16, 10, -10], [100, 0, 0]];
        let b = [[10i16, 20, -20]];
        let m = vt_mime_vertices(&a, &b, 0.55);
        // trunc(a + (b - a)·t), toward zero; a vertex B lacks goes to 0.
        assert_eq!(m[0], [5, 15, -15]);
        assert_eq!(m[1], [45, 0, 0]);
        assert_eq!(vt_mime_vertices(&a, &b, 0.0), a.to_vec());
    }
}
