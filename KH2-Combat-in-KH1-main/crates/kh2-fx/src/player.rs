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
use crate::tail;

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
    /// RandFloat for three floats with one factor.
    pub const RAND_FV: u32 = 63;
    /// RandFV with the factor in -1..0: each float moves by 0..-range.
    pub const RAND_DOWN_FV: u32 = 65;
    pub const RAND_IV: u32 = 66;
    /// Three floats, a factor each, uniform in ±range.
    pub const S_RAND_FV: u32 = 75;
    /// Three floats, a factor each, in 0..range.
    pub const S_RAND_UP_FV: u32 = 76;
    /// A rotation by the change of an angle, kept for the S-matrix pointer.
    pub const KE_MAT_SN: u32 = 85;
    pub const MATRIX_XYZ: u32 = 87;
    pub const MATRIX_YZX: u32 = 89;
    pub const MATRIX_YXZ: u32 = 90;
    pub const MATRIX_ZXY: u32 = 91;
    pub const MATRIX_SCL: u32 = 94;
    /// `M` with the camera's inverse rotation: always faces the camera.
    pub const RYJ_MATRIX_NO_ROT: u32 = 96;
    pub const PAR_MATRIX: u32 = 105;
    /// ParMatrix that also inherits the parent's rotation.
    pub const KE_PAR_MAT_R: u32 = 106;
    pub const DRAW_MATRIX: u32 = 112;
    pub const DRAW_MATRIX_FRONT: u32 = 113;
    /// Billboard turning only about the root's own y axis.
    pub const DRAW_MATRIX_WOOD: u32 = 115;
    /// DrawMatrix that also keeps the view-space matrix in the work area.
    pub const KE_D_MAT: u32 = 116;
    /// DrawMatrixFront, likewise.
    pub const KE_D_MAT_FR: u32 = 117;
    /// `D` through the root matrix saved when the instance was spawned.
    pub const RYJ_DRAW_MATRIX_WORLD: u32 = 120;
    pub const KE_Z_CRCT: u32 = 124;
    pub const KE_Z_CRCT_SHP: u32 = 125;
    pub const DRAW_MDL3: u32 = 137;
    /// DrawMdl3 with the blend taken from the key.
    pub const DRAW_MDL_SEMI3: u32 = 138;
    pub const DRAW_MDL_TS3: u32 = 139;
    pub const DRAW_SHAPE: u32 = 141;
    pub const DRAW_SHAPE_X: u32 = 142;
    /// A trail of sprites along past positions (`crate::tail`).
    pub const KE_SHP_TAIL2X: u32 = 155;
    pub const KE_SHP_TAIL3X: u32 = 157;
    /// Screen distortion quad (reads the frame buffer; not drawn here).
    pub const KE_SHP_DTT: u32 = 160;
    /// VertexApLc with the vertex carried through the spawner's matrix.
    pub const VERTEX_AP: u32 = 174;
    pub const VERTEX_AP_LC: u32 = 177;
    /// Spawns children at vertices, each also turned to face its vertex.
    pub const KE_BORN_RND6: u32 = 184;
    pub const MEGA_BIRTH: u32 = 201;
    /// The Loop integrators keep their values over an instance loop.
    pub const MOVE_LOOP: u32 = 210;
    pub const ANG_MOVE_LOOP: u32 = 211;
    /// Same routine as MoveLoop (three floats).
    pub const SCL_MOVE_LOOP: u32 = 212;
    pub const POINT_LOOP: u32 = 213;
    pub const ANGLE_LOOP: u32 = 214;
    pub const SCALE_LOOP: u32 = 215;
    /// Same routine as MatrixXYZ.
    pub const MATRIX_LOOP: u32 = 216;
    pub const DRAW_MATRIX_LOOP: u32 = 217;
    /// DrawMdlLoopZ without the Z matrix.
    pub const DRAW_MDL_LOOP: u32 = 218;
    pub const DRAW_MDL_LOOP_Z: u32 = 219;
    /// Same routine as MatrixZXY.
    pub const MATRIX_LOOP_ZXY: u32 = 234;
    /// Same routine as VertexApLc.
    pub const VERTEX_AP_LC_LOOP: u32 = 236;
    /// Model distortion by its normals (reads the frame buffer; not drawn).
    pub const SNO_MDL_DTT_NORM: u32 = 238;
    pub const SHIPOLY: u32 = 241;
}

/// Program types this crate runs.
///
/// Two of them, pppKeShpDtt (160) and pppSnoMdlDttNorm (238), are the
/// game's screen distortion: they bend the already drawn frame behind a
/// quad or a model. Their state runs, but they draw nothing, because the
/// crate's output is textured triangles and cannot read the frame
/// ([`draws_nothing`]).
pub fn supported(kind: u32) -> bool {
    matches!(
        kind,
        28..=31
            | 36..=43
            | 49
            | 51
            | 63
            | 65
            | 66
            | 75
            | 76
            | 84
            | 85
            | 87
            | 89
            | 90
            | 91
            | 94
            | 96
            | 105
            | 106
            | 112
            | 113
            | 115..=117
            | 120
            | 124
            | 125
            | 137..=139
            | 141
            | 142
            | 155
            | 157
            | 160
            | 174
            | 177
            | 184
            | 201
            | 210..=219
            | 234
            | 236
            | 238
            | 241
    )
}

/// Supported programs whose drawing reads the frame buffer (a screen
/// distortion), which this crate's triangle output cannot do: they run
/// but emit nothing.
pub fn draws_nothing(kind: u32) -> bool {
    matches!(kind, k::KE_SHP_DTT | k::SNO_MDL_DTT_NORM)
}

/// Programs with a draw slot: they run in the draw pass.
pub(crate) fn has_draw(kind: u32) -> bool {
    matches!(
        kind,
        k::VT_MIME
            | k::DRAW_MDL3
            | k::DRAW_MDL_SEMI3
            | k::DRAW_MDL_TS3
            | k::DRAW_SHAPE
            | k::DRAW_SHAPE_X
            | k::KE_SHP_TAIL2X
            | k::KE_SHP_TAIL3X
            | k::KE_SHP_DTT
            | k::MEGA_BIRTH
            | k::DRAW_MDL_LOOP
            | k::DRAW_MDL_LOOP_Z
            | k::SNO_MDL_DTT_NORM
            | k::SHIPOLY
    )
}

/// Programs skipped while an instance is fast-forwarded after a loop.
fn skipped_in_fast_forward(kind: u32) -> bool {
    matches!(kind, k::MEGA_BIRTH | k::SHIPOLY)
}

/// The instance's work area (the bytes after the 0xA0-byte header).
#[derive(Clone, Debug)]
pub(crate) struct Work(Vec<u8>);

impl Work {
    pub(crate) fn new(size: usize) -> Work {
        Work(vec![0; size])
    }
    fn get<const N: usize>(&self, o: i32) -> [u8; N] {
        let o = o as usize;
        match self.0.get(o..o + N) {
            Some(s) if (o as i32) >= 0 => s.try_into().unwrap(),
            _ => [0; N],
        }
    }
    pub(crate) fn set<const N: usize>(&mut self, o: i32, v: [u8; N]) {
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
    pub fn u8(&self, o: i32) -> u8 {
        self.get::<1>(o)[0]
    }
    pub fn set_u8(&mut self, o: i32, v: u8) {
        self.set(o, [v])
    }
    /// A 4×4 matrix as the game stores it (its rows are glam's columns).
    pub fn mat4(&self, o: i32) -> Mat4 {
        Mat4::from_cols_array(&std::array::from_fn(|i| self.f32(o + i as i32 * 4)))
    }
    pub fn set_mat4(&mut self, o: i32, m: Mat4) {
        for (i, v) in m.to_cols_array().into_iter().enumerate() {
            self.set_f32(o + i as i32 * 4, v);
        }
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
    pub(crate) fn zero(&mut self, o: i32, n: usize) {
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
    /// KeShpTail3X: the animation time its draw last used (the game draws
    /// with the time from before the tick's increment).
    TailTime(u32),
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
    /// Has looped at least once (the game's +0x9D): the tails then keep
    /// their history instead of refilling it on frame 0.
    pub looped: bool,
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
    /// saves the current root matrix at its `w1`; KeShpTail2X sets its ring
    /// length; KeShpTail3X draws its random seed (one C `rand()`).
    fn constructed(mut self, def: &EmitterDef, root: Mat4, rng: &mut Rng) -> Instance {
        for prog in &def.programs {
            match prog.kind {
                k::RYJ_DRAW_MATRIX_WORLD => self.work.set_mat4(w(prog, 1), root),
                k::KE_SHP_TAIL2X => self.work.set_u8(w(prog, 0), tail::TAIL2_LEN),
                k::KE_SHP_TAIL3X => self.work.set_u16(w(prog, 0) + tail::TAIL3_SEED, rng.rand() as u16),
                _ => {}
            }
        }
        self
    }

    fn saved_matrix(&self, o: i32) -> Mat4 {
        self.work.mat4(o)
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
    /// The camera view of the last frame built (the game reads the current
    /// one; the programs that need it in their calc, RyjMatrixNoRot and
    /// KeDMat(Fr), run before this frame's camera is known).
    pub view: Mat4,
}

impl EffectView {
    /// `V · Root`.
    pub fn vr(&self) -> Mat4 {
        self.view * self.root
    }

    /// The billboard matrix of pppDrawMatrixFront for `m`, in view space:
    /// `m`'s axes times the root scale lengths, at the view position of its
    /// origin.
    pub fn front(&self, m: Mat4) -> Mat4 {
        Mat4::from_cols(m.x_axis * self.rs.x, m.y_axis * self.rs.y, m.z_axis * self.rs.z, self.vr().transform_point3(m.w_axis.truncate()).extend(1.0))
    }
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

/// A child a program asks for, with the words to write into its work area
/// (in order: later writes win where they overlap).
struct Spawn {
    emitter: usize,
    parent: (usize, u64),
    writes: Vec<(i32, [u8; 4])>,
}

impl Spawn {
    fn new(emitter: usize, parent: (usize, u64)) -> Spawn {
        Spawn { emitter, parent, writes: Vec::new() }
    }
    fn vec3(mut self, offset: i32, v: Vec3) -> Spawn {
        for (i, c) in v.to_array().into_iter().enumerate() {
            self.writes.push((offset + i as i32 * 4, c.to_le_bytes()));
        }
        self
    }
    fn i32(mut self, offset: i32, v: i32) -> Spawn {
        self.writes.push((offset, v.to_le_bytes()));
        self
    }
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
            fx: EffectView { root: Mat4::IDENTITY, rs: Vec3::ONE, colour: [1.0; 4], view: Mat4::IDENTITY },
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

    fn new_instance(&mut self, def: &EmitterDef, rng: &mut Rng) -> Instance {
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
            looped: false,
            m: Mat4::IDENTITY,
            d: DKind::None,
            mime: None,
            work: Work::new(size),
            cursor: vec![0; n],
            state: vec![ProgState::None; n],
        }
        .constructed(def, self.fx.root, rng)
    }

    /// Spawn emitters whose start frame has come (each fires once).
    fn spawn_clock(&mut self, lib: &Library, rng: &mut Rng) {
        let data = &lib.dpds[self.dpd].particles[self.pd];
        for (e, def) in data.emitters.iter().enumerate() {
            if self.emitters[e].start == SPAWNED || self.emitters[e].start > self.frame {
                continue;
            }
            self.emitters[e].start = SPAWNED;
            let inst = self.new_instance(def, rng);
            insert(&mut self.emitters[e].group, inst);
        }
    }

    fn apply_spawns(&mut self, lib: &Library, rng: &mut Rng, spawns: Vec<Spawn>) {
        let data = &lib.dpds[self.dpd].particles[self.pd];
        for s in spawns {
            let Some(def) = data.emitters.get(s.emitter) else { continue };
            let mut inst = self.new_instance(def, rng);
            inst.parent = Some(s.parent);
            inst.spawned = true;
            for (o, b) in s.writes {
                inst.work.set(o, b);
            }
            insert(&mut self.emitters[s.emitter].group, inst);
        }
    }

    /// The calc pass: spawn, then every program without a draw slot.
    pub fn calc_pass(&mut self, lib: &Library, rng: &mut Rng) {
        self.spawn_clock(lib, rng);
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
                    // KeBornRnd6 takes its vertices from a VtMime morph when
                    // the instance has one (the game's vertex override).
                    let morph = (prog.kind == k::KE_BORN_RND6).then(|| mime_vertices(&lib.dpds[self.dpd], &def.programs, inst)).flatten();
                    let ctx = CalcCtx { lib, dpd: self.dpd, fx: &fx, parent_m, morph: morph.as_deref(), fast_forward: false };
                    calc(&ctx, rng, inst, (e, pi), prog, &mut spawns);
                }
                self.apply_spawns(lib, rng, spawns);
            }
        }
    }

    /// The draw pass minus the drawing: the draw programs' calc parts. On a
    /// repeat frame (`repeat`, the game's first of the two frames a tick is
    /// shown) MegaBirth does not step its particles. `shown` is false for
    /// the catch-up ticks that run without drawing: the game does not call
    /// draw slots then, and some programs keep state in them.
    pub fn draw_pass(&mut self, lib: &Library, rng: &mut Rng, repeat: bool, shown: bool) {
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
                    draw_calc(lib, self.dpd, rng, &fx, inst, pi, prog, repeat, shown);
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
            inst.looped = true;
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
                draw_calc_ff(lib, self.dpd, &fx, inst, pi, prog);
            } else {
                let mut none = Vec::new();
                let ctx = CalcCtx { lib, dpd: self.dpd, fx: &fx, parent_m, morph: None, fast_forward: true };
                calc(&ctx, rng, inst, (e, pi), prog, &mut none);
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
            k::DRAW_MDL_TS3 => inst.work.zero(w(prog, 2), 24),
            // Only the scroll velocities and accelerations: the position
            // and the "initialised" word carry over the loop.
            k::DRAW_MDL_LOOP | k::DRAW_MDL_LOOP_Z => {
                inst.work.zero(w(prog, 2) + 8, 8);
                inst.work.zero(w(prog, 2) + 20, 8);
            }
            // The Loop integrators: Move keeps its value, only the
            // velocity restarts; Point, Angle and Scale take back the key
            // they started from, which the replay from frame 0 adds again,
            // so the value runs on across the loop.
            k::MOVE_LOOP | k::SCL_MOVE_LOOP | k::ANG_MOVE_LOOP => inst.work.zero(w(prog, 1), 12),
            k::POINT_LOOP | k::SCALE_LOOP | k::ANGLE_LOOP => {
                let first = inst.work.i32(w0 + 12);
                if let Some(key) = first.checked_sub(1).and_then(|i| prog.keys.get(i as usize)) {
                    for c in 0..3 {
                        let o = w0 + c * 4;
                        if prog.kind == k::ANGLE_LOOP {
                            inst.work.set_i32(o, inst.work.i32(o).wrapping_sub(key.i32(4 + c as usize * 4)));
                        } else {
                            inst.work.set_f32(o, inst.work.f32(o) - key.f32(4 + c as usize * 4));
                        }
                    }
                }
            }
            // The colours and their rates; the ring and seed stay.
            k::KE_SHP_TAIL3X => inst.work.zero(w0, 48),
            // The distortion strength, its rate and acceleration.
            k::KE_SHP_DTT | k::SNO_MDL_DTT_NORM => inst.work.zero(w0, 12),
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

/// What a calc-pass program reads besides its instance.
struct CalcCtx<'a> {
    lib: &'a Library,
    dpd: usize,
    fx: &'a EffectView,
    /// The parent instance's `M` (children).
    parent_m: Option<Mat4>,
    /// The instance's VtMime vertices, when it has some.
    morph: Option<&'a [[i16; 3]]>,
    /// Calc-only steps after an instance loop (the game's flag at
    /// 0x142B0EE24).
    fast_forward: bool,
}

/// The vertices a VtMime program last pointed the instance at (the game's
/// vertex override at +0x90), if any.
pub(crate) fn mime_vertices(dpd: &kh2_formats::dpd::Dpd, progs: &[Program], inst: &Instance) -> Option<Vec<[i16; 3]>> {
    let pi = inst.mime?;
    let prog = progs.get(pi)?;
    let key = inst.key(prog, pi)?;
    let a = dpd.model(key.i32(4))?;
    let b = dpd.model(key.i32(8))?;
    Some(vt_mime_vertices(&a.vertices, &b.vertices, inst.work.f32(w(prog, 0))))
}

/// One factor of the S-random programs: `2u` (mode 0) or `u + u` for
/// SRandFV, uniform in ±range after `range·r − range`; `u` or `(u + u)/2`
/// for SRandUpFV, in 0..range after `range·r`.
fn s_rand(rng: &mut Rng, mode: u8, up: bool) -> f32 {
    let u = rng.rand01();
    match (mode != 0, up) {
        (false, false) => u * 2.0,
        (true, false) => u + rng.rand01(),
        (false, true) => u,
        (true, true) => (rng.rand01() + u) * 0.5,
    }
}

fn calc(cx: &CalcCtx, rng: &mut Rng, inst: &mut Instance, (_e, pi): (usize, usize), prog: &Program, spawns: &mut Vec<Spawn>) {
    let (lib, dpd, fx, parent_m, fast_forward) = (cx.lib, cx.dpd, cx.fx, cx.parent_m, cx.fast_forward);
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
        // MoveLoop: Move whose value stands still while the instance is
        // fast-forwarded after a loop (the velocity still takes its keys).
        // SclMoveLoop is the same routine.
        k::MOVE_LOOP | k::SCL_MOVE_LOOP => {
            if inst.computed {
                return;
            }
            if key.frame() == inst.frame {
                let v = inst.work.vec3(w1) + Vec3::from(key.vec3(4));
                inst.work.set_vec3(w1, v);
            }
            if !fast_forward {
                let v = inst.work.vec3(w0) + inst.work.vec3(w1);
                inst.work.set_vec3(w0, v);
            }
        }
        k::ANG_MOVE_LOOP => {
            if inst.computed {
                return;
            }
            let on = key.frame() == inst.frame;
            for c in 0..3 {
                let o = c * 4;
                if on {
                    inst.work.set_i32(w1 + o, inst.work.i32(w1 + o).wrapping_add(key.i32(4 + o as usize)));
                }
                if !fast_forward {
                    inst.work.set_i32(w0 + o, inst.work.i32(w0 + o).wrapping_add(inst.work.i32(w1 + o)));
                }
            }
        }
        // PointLoop, ScaleLoop, AngleLoop: on the first run the value is
        // cleared and the current key remembered (at w0 + 12; here its
        // index + 1); then, as Point, a key tick adds the key. Their reset
        // subtracts the remembered key (see `reset`). AngleLoop brings each
        // angle back below one turn (above 0x10000, minus 0x10000) after a
        // key.
        k::POINT_LOOP | k::SCALE_LOOP | k::ANGLE_LOOP => {
            if inst.computed {
                return;
            }
            if inst.work.i32(w0 + 12) == 0 {
                inst.work.zero(w0, 12);
                inst.work.set_i32(w0 + 12, inst.cursor[pi].min(prog.keys.len().saturating_sub(1)) as i32 + 1);
            }
            if key.frame() == inst.frame {
                for c in 0..3 {
                    let o = w0 + c * 4;
                    if prog.kind == k::ANGLE_LOOP {
                        let mut a = inst.work.i32(o).wrapping_add(key.i32(4 + c as usize * 4));
                        while a > 0x10000 {
                            a -= 0x10000;
                        }
                        inst.work.set_i32(o, a);
                    } else {
                        inst.work.set_f32(o, inst.work.f32(o) + key.f32(4 + c as usize * 4));
                    }
                }
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
        // RandFV / SRandFV / SRandUpFV: RandFloat for three floats. Key @4
        // target offset (-1 = a global), @8 three f32 ranges, @20 mode. The
        // factors are drawn on frame 0 and stored at w0 (one for RandFV,
        // one per axis for the S forms); frame 0 and every later key tick
        // add `range·r − range` (`range·r` for SRandUpFV) to the target.
        // RandDownFV: one factor `−u` (mode 0) or `(−u − u)/2`, and adds
        // `range·r`: each float moves by 0..−range.
        k::RAND_FV | k::RAND_DOWN_FV | k::S_RAND_FV | k::S_RAND_UP_FV => {
            if inst.computed {
                return;
            }
            let mode = key.u8(20);
            if inst.frame == 0 {
                match prog.kind {
                    k::RAND_FV => {
                        let r = if mode == 0 { rng.rand01() * 2.0 } else { rng.rand01() + rng.rand01() };
                        inst.work.set_f32(w0, r);
                    }
                    k::RAND_DOWN_FV => {
                        let u = -rng.rand01();
                        let r = if mode == 0 { u } else { (u - rng.rand01()) * 0.5 };
                        inst.work.set_f32(w0, r);
                    }
                    kind => {
                        for c in 0..3 {
                            let r = s_rand(rng, mode, kind == k::S_RAND_UP_FV);
                            inst.work.set_f32(w0 + c * 4, r);
                        }
                    }
                }
            } else if key.frame() != inst.frame {
                return;
            }
            let target = key.i32(4);
            if target == -1 {
                return;
            }
            for c in 0..3 {
                let one = matches!(prog.kind, k::RAND_FV | k::RAND_DOWN_FV);
                let r = inst.work.f32(if one { w0 } else { w0 + c * 4 });
                let range = key.f32(8 + c as usize * 4);
                let add = if matches!(prog.kind, k::S_RAND_UP_FV | k::RAND_DOWN_FV) { range * r } else { range * r - range };
                let o = target + c * 4;
                inst.work.set_f32(o, add + inst.work.f32(o));
            }
        }
        k::MATRIX_XYZ | k::MATRIX_LOOP | k::MATRIX_YZX | k::MATRIX_YXZ | k::MATRIX_ZXY | k::MATRIX_LOOP_ZXY => {
            let a = [inst.work.i32(w1), inst.work.i32(w1 + 4), inst.work.i32(w1 + 8)];
            let r = match prog.kind {
                k::MATRIX_YZX => math::euler_yzx_i(a),
                k::MATRIX_YXZ => math::euler_yxz_i(a),
                k::MATRIX_ZXY | k::MATRIX_LOOP_ZXY => math::euler_zxy_i(a),
                _ => math::euler_xyz_i(a),
            };
            inst.m = math::trs(inst.work.vec3(w0), r, inst.work.vec3(w2));
        }
        k::MATRIX_SCL => {
            inst.m = math::trs(inst.work.vec3(w0), glam::Mat3::IDENTITY, inst.work.vec3(w1));
        }
        // RyjMatrixNoRot: the rotation is the camera's inverse (the view's
        // rotation transposed), rows scaled by w2, translation w0; w1 is
        // not read. Drawn through `V·Root` the instance keeps facing the
        // camera in 3D.
        k::RYJ_MATRIX_NO_ROT => {
            let r = glam::Mat3::from_mat4(fx.view).transpose();
            inst.m = math::trs(inst.work.vec3(w0), r, inst.work.vec3(w2));
        }
        // KeMatSN: the rotation by (previous angles − angles at w1) goes to
        // w0 + 0x40 (translation row (0, 0, 0, 1)), the angles to w0 + 0x80,
        // and a global pointer to it that nothing reads on PC (like
        // SMatrix). Kept for the work area only.
        k::KE_MAT_SN => {
            let cur = [inst.work.i32(w1), inst.work.i32(w1 + 4), inst.work.i32(w1 + 8)];
            let prev = [inst.work.i32(w0 + 0x80), inst.work.i32(w0 + 0x84), inst.work.i32(w0 + 0x88)];
            for (c, v) in cur.iter().enumerate() {
                inst.work.set_i32(w0 + 0x80 + c as i32 * 4, *v);
            }
            let delta = std::array::from_fn(|c| prev[c].wrapping_sub(cur[c]));
            inst.work.set_mat4(w0 + 0x40, Mat4::from_mat3(math::euler_xyz_i(delta)));
        }
        k::PAR_MATRIX => {
            if let Some(pm) = parent_m {
                let t = pm.transform_point3(inst.m.w_axis.truncate());
                inst.m.w_axis = t.extend(inst.m.w_axis.w);
            }
        }
        // KeParMatR: ParMatrix plus the parent's rotation without its scale:
        // `M = R · M` with `R` the parent's normalised axes, then the
        // translation is the parent's `M` applied to the old one.
        k::KE_PAR_MAT_R => {
            if let Some(pm) = parent_m {
                let t = pm.transform_point3(inst.m.w_axis.truncate());
                let axis = |v: glam::Vec4| v.truncate().normalize_or_zero();
                let r = glam::Mat3::from_cols(axis(pm.x_axis), axis(pm.y_axis), axis(pm.z_axis));
                inst.m = Mat4::from_mat3(r) * inst.m;
                inst.m.w_axis = t.extend(1.0);
            }
        }
        k::DRAW_MATRIX | k::DRAW_MATRIX_LOOP => inst.d = DKind::Full,
        // KeDMat / KeDMatFr: DrawMatrix / DrawMatrixFront that also leave
        // the view-space matrix (`V·Root·M`, or the billboard one) at w1,
        // for the distortion programs.
        k::KE_D_MAT => {
            inst.d = DKind::Full;
            inst.work.set_mat4(w1, fx.vr() * inst.m);
        }
        k::KE_D_MAT_FR => {
            inst.d = DKind::Front;
            inst.work.set_mat4(w1, fx.front(inst.m));
        }
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
                        spawns.push(Spawn::new(key.i32(12) as usize, (_e, inst.uid)).vec3(key.i32(16), carry.map_or(v, |m| m.transform_point3(v))));
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
        // KeBornRnd6: VertexApLc's spawner with an 8-bit countdown (w0 + 2)
        // that only runs while the key's interval (@7) is non-zero, and
        // children that are also turned toward their vertex. Key: @4 i16
        // vertex set, @6 count, @7 interval, @8 order (0 in turn from the
        // cursor at w0, else random), @12 child emitter, @16 child offset
        // for the vertex (x, y, z), @20 must not be -1, @24 child offset for
        // the angles (`born6_angles`).
        k::KE_BORN_RND6 => {
            if fast_forward || inst.computed || key.u8(7) == 0 {
                return;
            }
            let mut count = inst.work.u8(w0 + 2);
            if count == 0 {
                count = key.u8(7);
                inst.work.set_u8(w0 + 2, count);
                if key.i32(12) == -1 || key.i32(20) == -1 {
                    return;
                }
                let d = &lib.dpds[dpd];
                if let Some(vs) = d.vertex_set(key.i16(4) as i32) {
                    let model = d.model(vs.model).map(|m| m.vertices.as_slice()).unwrap_or(&[]);
                    let verts = cx.morph.unwrap_or(model);
                    let n = vs.indices.len();
                    if n > 0 {
                        let random = key.u8(8) != 0;
                        let pick = |rng: &mut Rng| (rng.rand() as usize) % n;
                        let mut idx = if random { pick(rng) } else { inst.work.u16(w0) as usize };
                        for _ in 0..key.u8(6) {
                            let v = vs.indices.get(idx).and_then(|&i| verts.get(i as usize)).copied().unwrap_or([0; 3]);
                            let v = Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32);
                            let mut s = Spawn::new(key.i32(12) as usize, (_e, inst.uid)).vec3(key.i32(16), v);
                            let at = key.i32(24);
                            let (z, x) = born6_angles(v);
                            if let Some(z) = z {
                                s = s.i32(at + 8, z);
                            }
                            if let Some(x) = x {
                                s = s.i32(at, x);
                            }
                            spawns.push(s.i32(at + 4, 0));
                            idx = if random {
                                pick(rng)
                            } else if idx + 1 >= n {
                                0
                            } else {
                                idx + 1
                            };
                        }
                        inst.work.set_u16(w0, idx as u16);
                    }
                }
                count = inst.work.u8(w0 + 2);
            }
            inst.work.set_u8(w0 + 2, count.wrapping_sub(1));
        }
        // S-matrix pointer (unused on PC), Z correction (built at draw time).
        _ => {}
    }
}

/// The angles KeBornRnd6 gives a child for vertex `v` (65536 = one turn):
/// z turns the child's y axis toward the vertex in the x-y plane, x tilts
/// it by the vertex's elevation. Each is `None` when the vector it is
/// built from is zero (the game then leaves that angle as it is). Written
/// as the game's atan cases, quadrant by quadrant, in its precision.
pub(crate) fn born6_angles(v: Vec3) -> (Option<i32>, Option<i32>) {
    use std::f32::consts::{FRAC_PI_2, PI};
    let atan = |r: f32| (r as f64).atan();
    // One of the game's atan2 forms: `a` the numerator side, `b` the
    // reference axis.
    let quad = |a: f32, b: f32, half_b_neg: f32| -> Option<f32> {
        let p = a * b;
        if p > 0.0 {
            let base = if b > 0.0 { 0.0 } else { -PI };
            Some((atan(a / b) + base as f64) as f32)
        } else if p < 0.0 {
            let base = if a > 0.0 { FRAC_PI_2 } else { -FRAC_PI_2 };
            Some((base as f64 - atan(b / a)) as f32)
        } else if b != 0.0 {
            Some(if b > 0.0 { 0.0 } else { half_b_neg })
        } else if a != 0.0 {
            Some(if a > 0.0 { FRAC_PI_2 } else { -FRAC_PI_2 })
        } else {
            None
        }
    };
    let units = |a: f32| (a * 32768.0 / PI) as i32;
    let z = quad(v.x, -v.y, PI).map(units);
    let h = (v.x * v.x + v.y * v.y).sqrt();
    let x = quad(-v.z, h, PI).map(units);
    (z, x)
}

/// The calc part of the draw programs. `shown`: this pass is drawn (the
/// game calls draw slots only then).
#[allow(clippy::too_many_arguments)]
fn draw_calc(lib: &Library, dpd: usize, rng: &mut Rng, fx: &EffectView, inst: &mut Instance, pi: usize, prog: &Program, repeat: bool, shown: bool) {
    let Some(key) = inst.key(prog, pi) else { return };
    match prog.kind {
        // The tails' calc slots: once per tick (the ring and colours, 2X's
        // animation).
        k::KE_SHP_TAIL3X => {
            let o = w(prog, 0);
            if !inst.computed {
                tail::tail3_calc(&mut inst.work, o, key, inst.frame, inst.looped, &inst.m, &fx.root);
            }
            // The draw then uses the time and adds the key's speed (@8),
            // once per tick, only on a drawn pass.
            let time = inst.work.i32(o + tail::TAIL3_TIME) as u32;
            if shown && !inst.computed {
                inst.state[pi] = ProgState::TailTime(time);
                inst.work.set_i32(o + tail::TAIL3_TIME, (time as i32).wrapping_add(key.i32(8)));
            } else if shown {
                inst.state[pi] = ProgState::TailTime(time);
            }
        }
        k::KE_SHP_TAIL2X => {
            if !inst.computed {
                let sh = lib.dpds[dpd].shape(key.i32(4));
                tail::tail2_calc(&mut inst.work, w(prog, 0), key, inst.frame, inst.looped, &inst.m, &fx.root, sh);
            }
        }
        // The distortion programs only integrate their strength (t, rate,
        // acceleration at w0; on a key tick the key's three deltas) and
        // draw nothing here. KeShpDtt once per tick (keys @4); SnoMdlDttNorm
        // on every drawn call, both frames of a tick (keys @8).
        k::KE_SHP_DTT | k::SNO_MDL_DTT_NORM => {
            let dtt = prog.kind == k::KE_SHP_DTT;
            if !shown || (dtt && inst.computed) {
                return;
            }
            let o = w(prog, 0);
            let wk = &mut inst.work;
            let accel = wk.f32(o + 8);
            let rate = accel + wk.f32(o + 4);
            let t = rate + wk.f32(o);
            let on = key.frame() == inst.frame;
            let at = if dtt { 4 } else { 8 };
            let add = |i: usize| if on { key.f32(at + i * 4) } else { 0.0 };
            wk.set_f32(o, t + add(0));
            wk.set_f32(o + 4, rate + add(1));
            wk.set_f32(o + 8, accel + add(2));
        }
        k::DRAW_MDL_LOOP => loop_z(inst, prog, key, false),
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

/// Draw programs' calc while fast-forwarding (no ribbons or particles; no
/// draw slots, so neither the tail's time nor the distortion strength).
fn draw_calc_ff(lib: &Library, dpd: usize, fx: &EffectView, inst: &mut Instance, pi: usize, prog: &Program) {
    let Some(key) = inst.key(prog, pi) else { return };
    match prog.kind {
        k::DRAW_SHAPE | k::DRAW_SHAPE_X => shape_anim(lib, dpd, &mut inst.work, w(prog, 0), key.i32(4), key.i16(8)),
        k::DRAW_MDL_LOOP | k::DRAW_MDL_LOOP_Z => loop_z(inst, prog, key, true),
        k::KE_SHP_TAIL3X => tail::tail3_calc(&mut inst.work, w(prog, 0), key, inst.frame, inst.looped, &inst.m, &fx.root),
        k::KE_SHP_TAIL2X => {
            let sh = lib.dpds[dpd].shape(key.i32(4));
            tail::tail2_calc(&mut inst.work, w(prog, 0), key, inst.frame, inst.looped, &inst.m, &fx.root, sh);
        }
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
        // The spells' programs (Fire, Blizzard, Thunder, Cure, Reflect,
        // Wisdom Form's package).
        for kind in [63, 75, 76, 85, 96, 106, 116, 117, 138, 155, 157, 160, 184, 210, 211, 213, 214, 215, 218, 238] {
            assert!(supported(kind), "{kind}");
        }
        for kind in [k::DRAW_MDL_SEMI3, k::KE_SHP_TAIL2X, k::KE_SHP_TAIL3X, k::KE_SHP_DTT, k::DRAW_MDL_LOOP, k::SNO_MDL_DTT_NORM] {
            assert!(has_draw(kind), "{kind}");
        }
        for kind in [k::POINT_LOOP, k::KE_PAR_MAT_R, k::KE_D_MAT, k::KE_BORN_RND6, k::RYJ_MATRIX_NO_ROT] {
            assert!(!has_draw(kind), "{kind}");
        }
        assert!(draws_nothing(160) && draws_nothing(238) && !draws_nothing(157));
        // Limit Form's package.
        for kind in [65, 89, 212, 234] {
            assert!(supported(kind) && !has_draw(kind), "{kind}");
        }
        // Neighbours that no package uses stay unsupported.
        assert!(!supported(88) && !supported(156) && !supported(64));
    }

    /// A key: frame (ticks), then bytes placed at their offsets.
    fn kb(tick: i32, len: usize, fields: &[(usize, &[u8])]) -> Key {
        let mut data = vec![0u8; len];
        data[..4].copy_from_slice(&(tick * TICK).to_le_bytes());
        for (at, b) in fields {
            data[*at..*at + b.len()].copy_from_slice(b);
        }
        Key { data }
    }

    fn f3b(v: [f32; 3]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    fn i3b(v: [i32; 3]) -> Vec<u8> {
        v.iter().flat_map(|x| x.to_le_bytes()).collect()
    }

    fn prog(kind: u32, keys: Vec<Key>, work: Vec<i32>) -> Program {
        let key_size = keys.first().map_or(0, |k| k.data.len() as u16);
        Program { kind, key_size, keys, work }
    }

    fn inst(n: usize) -> Instance {
        Instance {
            uid: 1,
            parent: None,
            frame: 0,
            computed: false,
            spawned: false,
            reset_pending: false,
            looped: false,
            m: Mat4::IDENTITY,
            d: DKind::None,
            mime: None,
            work: Work::new(0x400),
            cursor: vec![0; n],
            state: vec![ProgState::None; n],
        }
    }

    fn fx() -> EffectView {
        EffectView { root: Mat4::IDENTITY, rs: Vec3::ONE, colour: [1.0; 4], view: Mat4::IDENTITY }
    }

    /// Run program 0 of `progs` at tick `t` (key cursor advanced as the
    /// game does), returning the children it asked for.
    fn step(lib: &Library, fx: &EffectView, rng: &mut Rng, inst: &mut Instance, p: &Program, t: i32, ff: bool) -> Vec<Spawn> {
        inst.frame = t * TICK;
        advance_cursor(inst, 0, p);
        let mut spawns = Vec::new();
        let cx = CalcCtx { lib, dpd: 0, fx, parent_m: None, morph: None, fast_forward: ff };
        calc(&cx, rng, inst, (0, 0), p, &mut spawns);
        spawns
    }

    fn empty_lib() -> Library {
        crate::library::tests::lib_with_dpds(vec![kh2_formats::dpd::Dpd { particles: vec![], textures: vec![], shapes: vec![], models: vec![], vertex_sets: vec![], vram: [0; 4] }])
    }

    fn def(programs: Vec<Program>) -> EmitterDef {
        EmitterDef { start: 0, total: 100 * TICK, loop_start: 0, loop_end: 0, work_size: 0x4a0, priority: 0, programs }
    }

    #[test]
    fn point_loop_runs_on_across_a_loop() {
        let lib = empty_lib();
        let (fx, mut rng) = (fx(), Rng::new(1));
        let p = prog(k::POINT_LOOP, vec![kb(0, 16, &[(4, &f3b([1.0, 2.0, 3.0]))]), kb(2, 16, &[(4, &f3b([10.0, 0.0, 0.0]))])], vec![0]);
        let mut i = inst(1);
        // Whatever was there is cleared on the first run.
        i.work.set_vec3(0, Vec3::splat(99.0));
        for t in 0..3 {
            step(&lib, &fx, &mut rng, &mut i, &p, t, false);
        }
        assert_eq!(i.work.vec3(0), Vec3::new(11.0, 2.0, 3.0));
        assert_eq!(i.work.i32(12), 1);
        // The loop takes back key 0; replaying frame 0 adds it again.
        let d = def(vec![p.clone()]);
        reset(&mut i, &d);
        assert_eq!(i.work.vec3(0), Vec3::new(10.0, 0.0, 0.0));
        step(&lib, &fx, &mut rng, &mut i, &p, 0, true);
        assert_eq!(i.work.vec3(0), Vec3::new(11.0, 2.0, 3.0));
    }

    #[test]
    fn angle_loop_wraps_and_move_loop_holds_in_fast_forward() {
        let lib = empty_lib();
        let (fx, mut rng) = (fx(), Rng::new(1));
        let a = prog(k::ANGLE_LOOP, vec![kb(0, 16, &[(4, &i3b([0x18000, 0x10000, -0x18000]))])], vec![0]);
        let mut i = inst(1);
        step(&lib, &fx, &mut rng, &mut i, &a, 0, false);
        // Above one turn comes back by turns; exactly one turn and negative
        // angles stay.
        assert_eq!([i.work.i32(0), i.work.i32(4), i.work.i32(8)], [0x8000, 0x10000, -0x18000]);

        let m = prog(k::MOVE_LOOP, vec![kb(0, 16, &[(4, &f3b([1.0, 0.0, 0.0]))])], vec![0, 16]);
        let mut i = inst(1);
        step(&lib, &fx, &mut rng, &mut i, &m, 0, false);
        step(&lib, &fx, &mut rng, &mut i, &m, 1, false);
        assert_eq!((i.work.vec3(0).x, i.work.vec3(16).x), (2.0, 1.0));
        // Fast-forwarding: the velocity takes its key, the value holds.
        step(&lib, &fx, &mut rng, &mut i, &m, 0, true);
        assert_eq!((i.work.vec3(0).x, i.work.vec3(16).x), (2.0, 2.0));
        // Its reset clears only the velocity.
        reset(&mut i, &def(vec![m]));
        assert_eq!((i.work.vec3(0).x, i.work.vec3(16).x), (2.0, 0.0));
    }

    #[test]
    fn random_vectors() {
        let lib = empty_lib();
        let fx = fx();
        let ranges = f3b([10.0, 20.0, 40.0]);
        let key0 = |mode: u8| kb(0, 24, &[(4, &64i32.to_le_bytes()), (8, &ranges), (20, &[mode])]);
        let keys = |mode: u8| vec![key0(mode), kb(2, 24, &[(4, &64i32.to_le_bytes()), (8, &ranges), (20, &[mode])])];
        // RandFV: one factor for all three: each component is range·(r − 1).
        let mut rng = Rng::new(5);
        let p = prog(k::RAND_FV, keys(0), vec![0]);
        let mut i = inst(1);
        step(&lib, &fx, &mut rng, &mut i, &p, 0, false);
        let v = i.work.vec3(64);
        let r = i.work.f32(0);
        assert!((0.0..=2.0).contains(&r));
        assert!((v - Vec3::new(10.0, 20.0, 40.0) * (r - 1.0)).length() < 1e-4);
        // Not a key tick: nothing; key tick 2: the same factor again.
        step(&lib, &fx, &mut rng, &mut i, &p, 1, false);
        assert_eq!(i.work.vec3(64), v);
        step(&lib, &fx, &mut rng, &mut i, &p, 2, false);
        assert!((i.work.vec3(64) - v * 2.0).length() < 1e-4);
        // SRandFV: a factor per axis, each in ±range.
        let p = prog(k::S_RAND_FV, keys(1), vec![0]);
        let mut seen = Vec::new();
        for seed in 0..50 {
            let mut rng = Rng::new(seed);
            let mut i = inst(1);
            step(&lib, &fx, &mut rng, &mut i, &p, 0, false);
            let v = i.work.vec3(64);
            assert!(v.x.abs() <= 10.0 && v.y.abs() <= 20.0 && v.z.abs() <= 40.0, "{v}");
            for c in 0..3 {
                let r = i.work.f32(c * 4);
                assert!((0.0..=2.0).contains(&r));
            }
            seen.push(v.x / 10.0 - v.y / 20.0);
        }
        // Independent factors: the normalised components differ.
        assert!(seen.iter().any(|d| d.abs() > 0.05));
        // SRandUpFV: 0..range, (u + u)/2 in mode 1.
        let p = prog(k::S_RAND_UP_FV, keys(1), vec![0]);
        for seed in 0..50 {
            let mut rng = Rng::new(seed);
            let mut i = inst(1);
            step(&lib, &fx, &mut rng, &mut i, &p, 0, false);
            let v = i.work.vec3(64);
            assert!((0.0..=10.0).contains(&v.x) && (0.0..=20.0).contains(&v.y) && (0.0..=40.0).contains(&v.z), "{v}");
        }
        // RandDownFV: one factor in -1..0, each component 0..-range.
        let p = prog(k::RAND_DOWN_FV, keys(0), vec![0]);
        for seed in 0..50 {
            let mut rng = Rng::new(seed);
            let mut i = inst(1);
            step(&lib, &fx, &mut rng, &mut i, &p, 0, false);
            let (v, r) = (i.work.vec3(64), i.work.f32(0));
            assert!((-1.0..=0.0).contains(&r));
            assert!((v - Vec3::new(10.0, 20.0, 40.0) * r).length() < 1e-4, "{v}");
        }
        // Target -1 writes nowhere.
        let p = prog(k::S_RAND_FV, vec![kb(0, 24, &[(4, &(-1i32).to_le_bytes()), (8, &ranges)])], vec![0]);
        let mut i = inst(1);
        step(&lib, &fx, &mut Rng::new(1), &mut i, &p, 0, false);
        assert_eq!(i.work.vec3(64), Vec3::ZERO);
    }

    #[test]
    fn no_rot_matrix_faces_the_camera() {
        let lib = empty_lib();
        let mut fx = fx();
        fx.view = Mat4::from_translation(Vec3::new(5.0, 6.0, 7.0)) * Mat4::from_rotation_y(0.5) * Mat4::from_rotation_x(-0.3);
        let p = prog(k::RYJ_MATRIX_NO_ROT, vec![kb(0, 4, &[])], vec![0, 16, 32]);
        let mut i = inst(1);
        i.work.set_vec3(0, Vec3::new(1.0, 2.0, 3.0));
        i.work.set_vec3(32, Vec3::splat(2.0));
        step(&lib, &fx, &mut Rng::new(1), &mut i, &p, 0, false);
        assert_eq!(i.m.w_axis, glam::Vec4::new(1.0, 2.0, 3.0, 1.0));
        // Through V·Root (root = identity) the rotation cancels: only the scale is left.
        let vm = glam::Mat3::from_mat4(fx.vr() * i.m);
        assert!((vm.x_axis - Vec3::X * 2.0).length() < 1e-5 && (vm.y_axis - Vec3::Y * 2.0).length() < 1e-5 && (vm.z_axis - Vec3::Z * 2.0).length() < 1e-5);
    }

    #[test]
    fn par_mat_r_inherits_rotation_without_scale() {
        let lib = empty_lib();
        let fx = fx();
        let p = prog(k::KE_PAR_MAT_R, vec![kb(0, 12, &[])], vec![96]);
        let parent = Mat4::from_translation(Vec3::new(10.0, 0.0, 0.0)) * Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2) * Mat4::from_scale(Vec3::splat(3.0));
        let mut i = inst(1);
        i.m = Mat4::from_translation(Vec3::new(1.0, 0.0, 0.0)) * Mat4::from_scale(Vec3::splat(2.0));
        let mut spawns = Vec::new();
        let cx = CalcCtx { lib: &lib, dpd: 0, fx: &fx, parent_m: Some(parent), morph: None, fast_forward: false };
        calc(&cx, &mut Rng::new(1), &mut i, (0, 0), &p, &mut spawns);
        // The position goes through the whole parent matrix ...
        assert!((i.m.w_axis.truncate() - Vec3::new(10.0, 3.0, 0.0)).length() < 1e-5);
        // ... the axes through its rotation only: x turned to y, length kept.
        assert!((i.m.x_axis.truncate() - Vec3::new(0.0, 2.0, 0.0)).length() < 1e-5);
        // Without a parent nothing changes.
        let before = i.m;
        step(&lib, &fx, &mut Rng::new(1), &mut i, &p, 0, false);
        assert_eq!(i.m, before);
    }

    #[test]
    fn ke_d_mat_keeps_the_view_matrix() {
        let lib = empty_lib();
        let mut fx = fx();
        fx.view = Mat4::from_translation(Vec3::new(0.0, 0.0, 100.0));
        fx.root = Mat4::from_rotation_y(0.4) * Mat4::from_scale(Vec3::new(2.0, 3.0, 4.0));
        fx.rs = Vec3::new(2.0, 3.0, 4.0);
        let mut i = inst(1);
        i.m = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let p = prog(k::KE_D_MAT, vec![kb(0, 4, &[])], vec![0, 64]);
        step(&lib, &fx, &mut Rng::new(1), &mut i, &p, 0, false);
        assert_eq!(i.d, DKind::Full);
        assert_eq!(i.work.mat4(64), fx.vr() * i.m);
        let p = prog(k::KE_D_MAT_FR, vec![kb(0, 4, &[])], vec![0, 128]);
        step(&lib, &fx, &mut Rng::new(1), &mut i, &p, 0, false);
        assert_eq!(i.d, DKind::Front);
        let f = i.work.mat4(128);
        // Billboard: axes are the root scale lengths, position the view position.
        assert_eq!(f.x_axis.truncate(), Vec3::new(2.0, 0.0, 0.0));
        assert!((f.w_axis.truncate() - fx.vr().transform_point3(Vec3::new(1.0, 2.0, 3.0))).length() < 1e-4);
    }

    #[test]
    fn ke_mat_sn_rotates_by_the_angle_change() {
        let lib = empty_lib();
        let fx = fx();
        let p = prog(k::KE_MAT_SN, vec![kb(0, 4, &[])], vec![0, 0x100]);
        let mut i = inst(1);
        i.work.set_i32(0x100, 0x4000);
        step(&lib, &fx, &mut Rng::new(1), &mut i, &p, 0, false);
        assert_eq!(i.work.i32(0x80), 0x4000);
        // previous (0) − current (a quarter turn about x): y goes to −z.
        let m = i.work.mat4(0x40);
        assert!((m.y_axis.truncate() - Vec3::new(0.0, 0.0, -1.0)).length() < 1e-5);
        assert_eq!(m.w_axis, glam::Vec4::W);
        // No change since: identity.
        step(&lib, &fx, &mut Rng::new(1), &mut i, &p, 1, false);
        assert!((i.work.mat4(0x40) - Mat4::IDENTITY).abs_diff_eq(Mat4::ZERO, 1e-6));
    }

    #[test]
    fn born6_angles_face_the_vertex() {
        // Up (y down in KH2) gives no turn; +x a quarter turn about z.
        assert_eq!(born6_angles(Vec3::new(0.0, -1.0, 0.0)), (Some(0), Some(0)));
        assert_eq!(born6_angles(Vec3::new(1.0, 0.0, 0.0)), (Some(0x4000), Some(0)));
        assert_eq!(born6_angles(Vec3::new(-1.0, 0.0, 0.0)), (Some(-0x4000), Some(0)));
        assert_eq!(born6_angles(Vec3::new(0.0, 1.0, 0.0)), (Some(0x8000), Some(0)));
        assert_eq!(born6_angles(Vec3::new(1.0, -1.0, 0.0)), (Some(0x2000), Some(0)));
        // Elevation: toward −z tilts by +x.
        assert_eq!(born6_angles(Vec3::new(0.0, -1.0, -1.0)), (Some(0), Some(0x2000)));
        assert_eq!(born6_angles(Vec3::new(0.0, 0.0, -1.0)), (None, Some(0x4000)));
        assert_eq!(born6_angles(Vec3::ZERO), (None, None));
    }

    #[test]
    fn born_rnd6_spawns_at_vertices_with_angles() {
        use kh2_formats::dpd::{Dpd, Model, VertexSet};
        let model = Model { number: 3, flags: 0, blend: 0, blend_fix: 0, groups: vec![], vertices: vec![[10, 0, 0], [0, -10, 0], [0, 0, -10]], normals: vec![] };
        let vs = VertexSet { number: 7, model: 3, indices: vec![0, 1, 2] };
        let lib = crate::library::tests::lib_with_dpds(vec![Dpd { particles: vec![], textures: vec![], shapes: vec![], models: vec![model], vertex_sets: vec![vs], vram: [0; 4] }]);
        let fx = fx();
        // Set 7, two per burst every 2 ticks, in order, emitter 1, vertex at
        // child +64, angles at child +80.
        let key = kb(0, 28, &[(4, &7i16.to_le_bytes()), (6, &[2, 2, 0]), (12, &1i32.to_le_bytes()), (16, &64i32.to_le_bytes()), (20, &1i32.to_le_bytes()), (24, &80i32.to_le_bytes())]);
        let p = prog(k::KE_BORN_RND6, vec![key], vec![0, 0]);
        let mut i = inst(1);
        let mut rng = Rng::new(1);
        let words = |s: &Spawn| s.writes.iter().map(|(o, b)| (*o, i32::from_le_bytes(*b))).collect::<Vec<_>>();
        let s0 = step(&lib, &fx, &mut rng, &mut i, &p, 0, false);
        assert_eq!(s0.len(), 2);
        assert_eq!(s0[0].emitter, 1);
        // Vertex (10, 0, 0): position, then z a quarter turn, no x (zero
        // elevation vector... (h = 10, -z = 0) gives 0), y = 0.
        assert_eq!(words(&s0[0]), vec![(64, 10f32.to_bits() as i32), (68, 0), (72, 0), (88, 0x4000), (80, 0), (84, 0)]);
        assert_eq!(words(&s0[1])[0], (64, 0));
        assert_eq!(words(&s0[1])[1], (68, (-10f32).to_bits() as i32));
        assert_eq!((i.work.u16(0), i.work.u8(2)), (2, 1));
        // Countdown: nothing on tick 1, the next burst on tick 2 wraps the cursor.
        assert!(step(&lib, &fx, &mut rng, &mut i, &p, 1, false).is_empty());
        let s2 = step(&lib, &fx, &mut rng, &mut i, &p, 2, false);
        assert_eq!(s2.len(), 2);
        assert_eq!(words(&s2[0])[2], (72, (-10f32).to_bits() as i32));
        assert_eq!(words(&s2[1])[0], (64, 10f32.to_bits() as i32));
        // Never while fast-forwarding; an interval of 0 switches it off.
        assert!(step(&lib, &fx, &mut rng, &mut i, &p, 3, true).is_empty());
        let off = prog(k::KE_BORN_RND6, vec![kb(0, 28, &[(4, &7i16.to_le_bytes()), (6, &[2, 0, 0]), (12, &1i32.to_le_bytes())])], vec![0]);
        let mut j = inst(1);
        assert!(step(&lib, &fx, &mut rng, &mut j, &off, 0, false).is_empty());
        assert_eq!(j.work.u8(2), 0);
        // A -1 at @20: the countdown is set but nothing spawns.
        let none = prog(k::KE_BORN_RND6, vec![kb(0, 28, &[(4, &7i16.to_le_bytes()), (6, &[2, 3, 0]), (12, &1i32.to_le_bytes()), (20, &(-1i32).to_le_bytes())])], vec![0]);
        let mut j = inst(1);
        assert!(step(&lib, &fx, &mut rng, &mut j, &none, 0, false).is_empty());
        assert_eq!(j.work.u8(2), 3);
    }

    #[test]
    fn tail3_draw_time_and_distortion_strength() {
        let lib = empty_lib();
        let fx = fx();
        let mut rng = Rng::new(1);
        let tail = prog(k::KE_SHP_TAIL3X, vec![kb(0, 0x58, &[(8, &0x200i32.to_le_bytes())])], vec![0, 0x300]);
        let mut i = inst(1);
        // First frame of tick 0: drawn with time 0, then the speed is added.
        draw_calc(&lib, 0, &mut rng, &fx, &mut i, 0, &tail, true, true);
        assert!(matches!(i.state[0], ProgState::TailTime(0)));
        assert_eq!(i.work.i32(tail::TAIL3_TIME), 0x200);
        // Second frame (computed): drawn with the new time, no increment.
        i.computed = true;
        draw_calc(&lib, 0, &mut rng, &fx, &mut i, 0, &tail, false, true);
        assert!(matches!(i.state[0], ProgState::TailTime(0x200)));
        assert_eq!(i.work.i32(tail::TAIL3_TIME), 0x200);
        // A catch-up tick (not shown) records but does not advance the time.
        i.computed = false;
        i.frame = TICK;
        draw_calc(&lib, 0, &mut rng, &fx, &mut i, 0, &tail, false, false);
        assert_eq!(i.work.i32(tail::TAIL3_TIME), 0x200);
        assert_eq!(i.work.u8(0x1fa), 26);

        // KeShpDtt: once per tick; SnoMdlDttNorm: on every drawn call.
        // Deltas at @4 for KeShpDtt, at @8 for SnoMdlDttNorm.
        let dtt = prog(k::KE_SHP_DTT, vec![kb(0, 0x20, &[(4, &f3b([1.0, 0.5, 0.25]))])], vec![0]);
        let sno = prog(k::SNO_MDL_DTT_NORM, vec![kb(0, 0x20, &[(8, &f3b([1.0, 0.5, 0.25]))])], vec![0x40, 0x80]);
        let mut i = inst(1);
        for computed in [false, true] {
            i.computed = computed;
            draw_calc(&lib, 0, &mut rng, &fx, &mut i, 0, &dtt, false, true);
            draw_calc(&lib, 0, &mut rng, &fx, &mut i, 0, &sno, false, true);
        }
        // Key tick 0 (both calls for Sno): t, rate, accel = key; then rate += accel, t += rate.
        assert_eq!((i.work.f32(0), i.work.f32(4), i.work.f32(8)), (1.0, 0.5, 0.25));
        assert_eq!((i.work.f32(0x40), i.work.f32(0x44), i.work.f32(0x48)), (2.75, 1.25, 0.5));
    }

    #[test]
    fn loop_resets_keep_the_scroll_position() {
        let p = prog(k::DRAW_MDL_LOOP, vec![kb(0, 40, &[])], vec![0, 0, 32]);
        let mut i = inst(1);
        for (n, v) in [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0].iter().enumerate() {
            i.work.set_f32(32 + n as i32 * 4, *v);
        }
        reset(&mut i, &def(vec![p]));
        let got: Vec<f32> = (0..7).map(|n| i.work.f32(32 + n * 4)).collect();
        assert_eq!(got, vec![1.0, 2.0, 0.0, 0.0, 5.0, 0.0, 0.0]);
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
