//! Draws into KH1's own Direct3D 12 frame: KH2's player model (`model`, world-space textured
//! triangles depth-tested against KH1's scene) and the debug menu's overlay (`draw_overlay`).
//! (Until t430 also KH2's effects, for the old effect player; they are KH1's own now, `kh2dpx`.)
//!
//! KH1 renders with D3D12 (`docs/kh1-render.md`): one renderer singleton at `0x1422BB580` holds
//! the device (`+0x2E0`) and the frame buffers; every draw is recorded inline by the game
//! thread's tasks into the current command list (`0x1400FD3A0(R)`). The model's draw runs right
//! after the world-model pass (`0x1401EADD0`, task 160000, hooked in `model`): the world is drawn,
//! KH1's own effects and the HUD come after. We record into KH1's list, on its bound frame buffer (its
//! targets inherited when the list is one of KH1's bundles), with our own root signature and
//! pipeline states, then hand KH1 a fresh list the way it does itself when its vertex ring
//! runs out mid-scene (`0x1400F7300(R, 0)`, `0x140109360(R, 1, 0)`), so its next draw sets
//! targets, pipeline and constants again.
//! We touch nothing when nothing of ours is drawing.
//!
//! No descriptor heaps of ours: the root signature is 40 root constants (view, projection,
//! KH1's clip scale and offset, the batch's depth offset and texture) and one root SRV, a raw
//! buffer with the textures (RGBA8, sampled bilinear with wrap in the pixel shader).
//! Vertices go to a 4-slot ring in an upload heap (KH1 keeps at most 3 frames in flight).
//! Shaders are compiled at run time from the HLSL below (`D3DCompile`, `d3dcompiler_47.dll`);
//! the root signature is serialised by `d3d12.dll`'s `D3D12SerializeRootSignature`.
//!
//! The clip transform is KH1's own (`docs/kh1-render.md` §4): `c = (p, 1) · View · P`,
//! `clip = (c.x·s + c.w·ox, −(c.y·s + c.w·oy), c.z, c.w)` with View `0x142E9DE60`, P at the
//! active projection struct `+0x80`, `s, ox, oy` from KH1's constant shadow.
//!
//! Every failure logs once and turns the renderer off for the session.
//!
//! (t354) Also the debug menu's screen-space overlay (`draw_overlay`, from the frame-end hook in
//! `debugmenu`): untextured boxes and text from a font texture in a buffer of its own (the
//! font's size; the model's 24 MB texture arena is made only by a model draw),
//! drawn on top of everything KH1 drew that frame (the HUD included), no depth test.

use super::addr::Rva;
use super::crash::readable;
use super::log;
use super::sys::{GetModuleHandleA, GetProcAddress, LoadLibraryA};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

type P = *mut c_void;

/// Calls COM method `slot` of `obj`: `com!(obj, slot, fn(arg types) -> ret, args...)`.
macro_rules! com {
    ($obj:expr, $slot:expr, fn($($t:ty),*) $(-> $r:ty)? $(, $a:expr)*) => {{
        let o: P = $obj;
        let f: unsafe extern "system" fn(P $(, $t)*) $(-> $r)? = std::mem::transmute::<usize, _>(*(*(o as *const *const usize)).add($slot));
        f(o $(, $a)*)
    }};
}

const RENDERER_RVA: Rva = Rva(0x22BB580);
const VIEW_RVA: Rva = Rva(0x2E9DE60);
const PROJ_PTR_RVA: Rva = Rva(0x2340EE0);
const FIELD_PROJ_RVA: Rva = Rva(0x2341320);
const LIST_RVA: Rva = Rva(0xFD3A0);
const CLOSE_RVA: Rva = Rva(0xF7300);
const REOPEN_RVA: Rva = Rva(0x109360);

// ID3D12Device
const DEV_CREATE_PSO: usize = 10;
const DEV_CREATE_ROOT_SIG: usize = 16;
const DEV_CREATE_COMMITTED: usize = 27;
// ID3D12Resource
const RES_MAP: usize = 8;
const RES_GPU_VA: usize = 11;
// ID3D12GraphicsCommandList
const CL_GET_TYPE: usize = 8;
const CL_DRAW: usize = 12;
const CL_TOPOLOGY: usize = 20;
const CL_SET_PSO: usize = 25;
const CL_SET_ROOT_SIG: usize = 30;
const CL_ROOT_CONSTANTS: usize = 36;
const CL_ROOT_SRV: usize = 40;
const CL_SET_VB: usize = 44;
const CL_SET_RT: usize = 46;

#[repr(C)]
#[derive(Clone, Copy)]
struct Guid(u32, u16, u16, [u8; 8]);
const IID_ROOT_SIG: Guid = Guid(0xc54a_6b66, 0x72df, 0x4ee8, [0x8b, 0xe5, 0xa9, 0x46, 0xa1, 0x42, 0x92, 0x14]);
const IID_PSO: Guid = Guid(0x765a_30f3, 0xf624, 0x4c6f, [0xa8, 0x28, 0xac, 0xe9, 0x48, 0x62, 0x24, 0x45]);
const IID_RESOURCE: Guid = Guid(0x6964_42be, 0xa72e, 0x4059, [0xbc, 0x79, 0x5b, 0x5c, 0x98, 0x04, 0x0f, 0xad]);

#[repr(C)]
struct HeapProps {
    ty: u32,
    cpu_page: u32,
    pool: u32,
    create_mask: u32,
    visible_mask: u32,
}

#[repr(C)]
struct ResourceDesc {
    dim: u32,
    align: u64,
    width: u64,
    height: u32,
    depth: u16,
    mips: u16,
    format: u32,
    samples: u32,
    quality: u32,
    layout: u32,
    flags: u32,
}

#[repr(C)]
struct RootParam {
    ty: u32,
    _p0: u32,
    a: u32,
    b: u32,
    c: u32,
    _p1: u32,
    visibility: u32,
    _p2: u32,
}

#[repr(C)]
struct RootSigDesc {
    n: u32,
    params: *const RootParam,
    n_samplers: u32,
    samplers: *const c_void,
    flags: u32,
}

#[repr(C)]
struct Bytecode {
    p: *const c_void,
    n: usize,
}

#[repr(C)]
struct SoDesc {
    decl: *const c_void,
    n: u32,
    strides: *const u32,
    n_strides: u32,
    rasterized: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct RtBlend {
    enable: i32,
    logic_enable: i32,
    src: u32,
    dst: u32,
    op: u32,
    src_a: u32,
    dst_a: u32,
    op_a: u32,
    logic_op: u32,
    mask: u8,
    _p: [u8; 3],
}

#[repr(C)]
struct BlendDesc {
    alpha_to_coverage: i32,
    independent: i32,
    rt: [RtBlend; 8],
}

#[repr(C)]
struct RasterDesc {
    fill: u32,
    cull: u32,
    front_ccw: i32,
    depth_bias: i32,
    depth_bias_clamp: f32,
    slope_bias: f32,
    depth_clip: i32,
    multisample: i32,
    aa_lines: i32,
    forced_samples: u32,
    conservative: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct StencilOp {
    fail: u32,
    depth_fail: u32,
    pass: u32,
    func: u32,
}

#[repr(C)]
struct DepthDesc {
    enable: i32,
    write_mask: u32,
    func: u32,
    stencil: i32,
    read_mask: u8,
    write_mask_s: u8,
    _p: [u8; 2],
    front: StencilOp,
    back: StencilOp,
}

#[repr(C)]
struct InputElem {
    name: *const u8,
    index: u32,
    format: u32,
    slot: u32,
    offset: u32,
    class: u32,
    step: u32,
}

#[repr(C)]
struct InputLayout {
    elems: *const InputElem,
    n: u32,
}

#[repr(C)]
struct PsoDesc {
    root: P,
    vs: Bytecode,
    ps: Bytecode,
    ds: Bytecode,
    hs: Bytecode,
    gs: Bytecode,
    so: SoDesc,
    blend: BlendDesc,
    sample_mask: u32,
    raster: RasterDesc,
    depth: DepthDesc,
    input: InputLayout,
    strip_cut: u32,
    topology: u32,
    n_rt: u32,
    rtv: [u32; 8],
    dsv: u32,
    samples: u32,
    quality: u32,
    node_mask: u32,
    cached: Bytecode,
    flags: u32,
}

const _: () = {
    assert!(std::mem::size_of::<PsoDesc>() == 656);
    assert!(std::mem::size_of::<ResourceDesc>() == 56);
    assert!(std::mem::size_of::<RootParam>() == 32);
    assert!(std::mem::size_of::<RtBlend>() == 40);
    assert!(std::mem::size_of::<RasterDesc>() == 44);
    assert!(std::mem::size_of::<DepthDesc>() == 52);
    assert!(std::mem::size_of::<InputElem>() == 32);
    assert!(std::mem::size_of::<kh2_fx::Vertex>() == 36);
};

/// The blend equation (the model's and the overlay's: `lerp(dst, src, As)`; the old effect
/// player's PS2 additive, subtract and darken modes went with it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Mode {
    Alpha,
}

const HLSL: &str = r#"
cbuffer K : register(b0) {
    float4 V0; float4 V1; float4 V2; float4 V3;
    float4 P0; float4 P1; float4 P2; float4 P3;
    float4 clipk;   // KH1's clip scale, x and y offsets; the batch's view-z depth offset
    uint4 tex;      // texel offset, width, height, flags (1 textured, 2 darken, 4 cut out)
};
ByteAddressBuffer T : register(t0);
struct VI { float3 pos : POSITION; float2 uv : TEXCOORD; float4 col : COLOR; };
struct VO { float4 pos : SV_Position; float2 uv : TEXCOORD; float4 col : COLOR; };
VO vs_main(VI i) {
    float4 v = V0 * i.pos.x + V1 * i.pos.y + V2 * i.pos.z + V3;
    float4 c = P0 * v.x + P1 * v.y + P2 * v.z + P3 * v.w;
    float4 vd = v;
    vd.z += clipk.w;
    float4 d = P0 * vd.x + P1 * vd.y + P2 * vd.z + P3 * vd.w;
    float z = abs(d.w) > 1e-6 ? max(d.z / d.w, 0.0) : 0.0;
    VO o;
    o.pos = float4(c.x * clipk.x + c.w * clipk.y, -(c.y * clipk.x + c.w * clipk.z), z * c.w, c.w);
    o.uv = i.uv;
    o.col = i.col;
    return o;
}
float4 texel(int x, int y) {
    int w = (int)tex.y;
    int h = (int)tex.z;
    x = ((x % w) + w) % w;
    y = ((y % h) + h) % h;
    uint p = T.Load((tex.x + (uint)(y * w + x)) * 4);
    return float4(p & 255, (p >> 8) & 255, (p >> 16) & 255, p >> 24) / 255.0;
}
float4 ps_main(VO i) : SV_Target {
    float4 t = float4(1, 1, 1, 1);
    if ((tex.w & 1) != 0 && tex.y > 0 && tex.z > 0) {
        float2 p = i.uv * float2(tex.y, tex.z) - 0.5;
        float2 b = floor(p);
        float2 f = p - b;
        int x = (int)b.x;
        int y = (int)b.y;
        t = lerp(lerp(texel(x, y), texel(x + 1, y), f.x), lerp(texel(x, y + 1), texel(x + 1, y + 1), f.x), f.y);
    }
    float4 s = t * i.col;
    if (s.a <= 0.0) discard;
    // A model's opaque groups: cut out where the texture is mostly clear, else solid.
    if ((tex.w & 4) != 0) {
        if (s.a < 0.5) discard;
        s.a = 1.0;
    }
    float a = saturate(s.a);
    if ((tex.w & 2) != 0) return float4(0, 0, 0, a);
    return float4(s.rgb * s.a, a);
}
"#;

/// One texture's place in the texture buffer.
#[derive(Clone, Copy)]
struct TexSlot {
    offset: u32,
    w: u32,
    h: u32,
}

struct Gpu {
    device: usize,
    root: usize,
    vs: Vec<u8>,
    ps: Vec<u8>,
    /// (mode, depth test, has depth view, samples, quality, depth write) → pipeline state.
    psos: Vec<(PsoKey, usize)>,
    ring_map: usize,
    ring_va: u64,
    /// The vertex ring's write point (bytes).
    frame: u64,
    /// Model textures (`draw_mesh`): an append-only buffer made at the first model draw, and
    /// each texture's place by its caller's key. Only the KH2 player model makes it (24 MB).
    mesh: Option<MeshArena>,
    /// The debug overlay's font (`draw_overlay`): its own buffer, the font's size, made at the
    /// first overlay draw (the tester: "if the debug allocates for kh2 model then make sure its
    /// gone"; until the version after t463 it sat in the model's 24 MB arena).
    font: Option<FontBuf>,
}

/// The overlay font's texture buffer: its place and the texture in it (by key).
struct FontBuf {
    buf: usize,
    map: usize,
    va: u64,
    texels: usize,
    key: Option<u64>,
}

type PsoKey = (Mode, bool, bool, u32, u32, bool);

struct MeshArena {
    buf: usize,
    map: usize,
    va: u64,
    /// Texels used.
    used: usize,
    slots: Vec<(u64, TexSlot)>,
}

/// Room for model textures (texels): Sora, three forms and a dozen Keyblades take about 2.5 M.
const MESH_TEXELS: usize = 6 << 20;

// Raw COM pointers, used only on the game thread.
unsafe impl Send for Gpu {}

/// The vertex ring: 4 MB, room for 4 frames of the debug overlay (KH1 keeps at most 3 in flight),
/// its only user now (until t444 KH2's effects drew through it: 24 MB, 6 MB a slot). One overlay
/// draw is at most 18 lines of text (16 rows, the title and the hint) plus 2 boxes, 6 vertices of
/// 36 bytes each a glyph: a 1 MB slot holds 4854 quads, 269 characters on every line. A bigger
/// draw is cut at a slot (`upload`). (The KH2 player model's draw, off since s259, needs the ring
/// sized for its meshes again, a few MB a draw, if it comes back.)
const RING_SLOTS: usize = 4;
const SLOT_BYTES: usize = 1 << 20;

static GPU: Mutex<Option<Gpu>> = Mutex::new(None);
static OFF: AtomicBool = AtomicBool::new(false);
/// Our draws: how many, and the last one's time (ms since start), for a crash note.
static DRAWS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static LAST_DRAW_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
const DEV_REMOVED_REASON: usize = 37;

/// For a crash note: KH1's device's removed reason (0 = not removed; DXGI_ERROR_DEVICE_HUNG
/// 0x887A0006, _REMOVED 0x887A0005, _RESET 0x887A0007, _DRIVER_INTERNAL_ERROR 0x887A0020,
/// E_OUTOFMEMORY 0x8007000E), our draws so far and how long ago the last one was.
pub unsafe fn crash_note(base: usize) -> String {
    let r = base + RENDERER_RVA;
    let device = if super::crash::readable(r + 0x2E0) { rd::<u64>(r + 0x2E0) as usize } else { 0 };
    let reason = if device > 0x10000 && super::crash::readable(device) { format!("{:#010x}", com!(device as P, DEV_REMOVED_REASON, fn() -> i32) as u32) } else { "?".into() };
    let now = START.get().map_or(0, |t| t.elapsed().as_millis() as u64);
    let n = DRAWS.load(Ordering::Relaxed);
    let last = LAST_DRAW_MS.load(Ordering::Relaxed);
    format!("GPU device removed reason {reason}; our draws (model, debug overlay) {n}{}", if n > 0 { format!(", the last {} ms ago", now.saturating_sub(last)) } else { String::new() })
}

fn fail(what: String) {
    if !OFF.swap(true, Ordering::Relaxed) {
        log(&format!("fx: renderer off: {what}"));
    }
}

type CompileFn = unsafe extern "system" fn(*const c_void, usize, *const u8, *const c_void, *const c_void, *const u8, *const u8, u32, u32, *mut P, *mut P) -> i32;
type SerializeFn = unsafe extern "system" fn(*const RootSigDesc, u32, *mut P, *mut P) -> i32;

unsafe fn blob<'a>(b: P) -> &'a [u8] {
    let p = com!(b, 3, fn() -> *const u8);
    let n = com!(b, 4, fn() -> usize);
    if p.is_null() {
        &[]
    } else {
        std::slice::from_raw_parts(p, n)
    }
}

unsafe fn release(o: P) {
    if !o.is_null() {
        com!(o, 2, fn() -> u32);
    }
}

unsafe fn compile(f: CompileFn, entry: &[u8], target: &[u8]) -> Result<Vec<u8>, String> {
    let (mut code, mut errors) = (std::ptr::null_mut(), std::ptr::null_mut());
    let hr = f(HLSL.as_ptr() as *const c_void, HLSL.len(), b"kh1_kh2_fx.hlsl\0".as_ptr(), std::ptr::null(), std::ptr::null(), entry.as_ptr(), target.as_ptr(), 1 << 15, 0, &mut code, &mut errors);
    let msg = if errors.is_null() { String::new() } else { String::from_utf8_lossy(blob(errors)).into_owned() };
    release(errors);
    if hr < 0 || code.is_null() {
        release(code);
        return Err(format!("D3DCompile {} failed: {:#010x} {msg}", String::from_utf8_lossy(entry).trim_end_matches('\0'), hr as u32));
    }
    let out = blob(code).to_vec();
    release(code);
    Ok(out)
}

unsafe fn buffer(device: usize, bytes: usize) -> Result<(usize, usize, u64), String> {
    let heap = HeapProps { ty: 2, cpu_page: 0, pool: 0, create_mask: 1, visible_mask: 1 };
    let desc = ResourceDesc { dim: 1, align: 0, width: bytes as u64, height: 1, depth: 1, mips: 1, format: 0, samples: 1, quality: 0, layout: 1, flags: 0 };
    let mut res: P = std::ptr::null_mut();
    let hr = com!(device as P, DEV_CREATE_COMMITTED, fn(*const HeapProps, u32, *const ResourceDesc, u32, *const c_void, *const Guid, *mut P) -> i32, &heap, 0, &desc, 0xAC3, std::ptr::null(), &IID_RESOURCE, &mut res);
    if hr < 0 || res.is_null() {
        return Err(format!("CreateCommittedResource({bytes}) failed: {:#010x}", hr as u32));
    }
    let range = [0usize, 0usize];
    let mut map: P = std::ptr::null_mut();
    let hr = com!(res, RES_MAP, fn(u32, *const usize, *mut P) -> i32, 0, range.as_ptr(), &mut map);
    if hr < 0 || map.is_null() {
        release(res);
        return Err(format!("Map failed: {:#010x}", hr as u32));
    }
    let va = com!(res, RES_GPU_VA, fn() -> u64);
    Ok((res as usize, map as usize, va))
}

unsafe fn init(device: usize) -> Result<Gpu, String> {
    let mut lib = std::ptr::null_mut();
    for name in ["d3dcompiler_47.dll\0", "d3dcompiler_46.dll\0", "d3dcompiler_43.dll\0"] {
        lib = LoadLibraryA(name.as_ptr());
        if !lib.is_null() {
            break;
        }
    }
    if lib.is_null() {
        return Err("no d3dcompiler DLL".into());
    }
    let f = GetProcAddress(lib, b"D3DCompile\0".as_ptr());
    if f.is_null() {
        return Err("no D3DCompile".into());
    }
    let f = std::mem::transmute::<P, CompileFn>(f);
    let vs = compile(f, b"vs_main\0", b"vs_5_0\0")?;
    let ps = compile(f, b"ps_main\0", b"ps_5_0\0")?;
    let d3d12 = GetModuleHandleA(b"d3d12.dll\0".as_ptr());
    let ser = if d3d12.is_null() { std::ptr::null_mut() } else { GetProcAddress(d3d12, b"D3D12SerializeRootSignature\0".as_ptr()) };
    if ser.is_null() {
        return Err("no D3D12SerializeRootSignature".into());
    }
    let ser = std::mem::transmute::<P, SerializeFn>(ser);
    let params = [
        // b0: 40 constants, all stages.
        RootParam { ty: 1, _p0: 0, a: 0, b: 0, c: 40, _p1: 0, visibility: 0, _p2: 0 },
        // t0: the texture buffer, pixel shader.
        RootParam { ty: 3, _p0: 0, a: 0, b: 0, c: 0, _p1: 0, visibility: 5, _p2: 0 },
    ];
    let desc = RootSigDesc { n: 2, params: params.as_ptr(), n_samplers: 0, samplers: std::ptr::null(), flags: 1 };
    let (mut sig, mut err) = (std::ptr::null_mut(), std::ptr::null_mut());
    let hr = ser(&desc, 1, &mut sig, &mut err);
    if hr < 0 || sig.is_null() {
        let msg = if err.is_null() { String::new() } else { String::from_utf8_lossy(blob(err)).into_owned() };
        release(err);
        return Err(format!("D3D12SerializeRootSignature failed: {:#010x} {msg}", hr as u32));
    }
    release(err);
    let bytes = blob(sig);
    let mut root: P = std::ptr::null_mut();
    let hr = com!(device as P, DEV_CREATE_ROOT_SIG, fn(u32, *const c_void, usize, *const Guid, *mut P) -> i32, 0, bytes.as_ptr() as *const c_void, bytes.len(), &IID_ROOT_SIG, &mut root);
    release(sig);
    if hr < 0 || root.is_null() {
        return Err(format!("CreateRootSignature failed: {:#010x}", hr as u32));
    }
    // (The ring stays for the session: KH1's frames in flight may name it.)
    let (_ring, ring_map, ring_va) = buffer(device, RING_SLOTS * SLOT_BYTES)?;
    log(&format!("fx: renderer ready (shaders {} + {} bytes)", vs.len(), ps.len()));
    Ok(Gpu { device, root: root as usize, vs, ps, psos: Vec::new(), ring_map, ring_va, frame: 0, mesh: None, font: None })
}

impl Gpu {
    unsafe fn pso(&mut self, key: PsoKey) -> Result<usize, String> {
        if let Some((_, p)) = self.psos.iter().find(|(k, _)| *k == key) {
            return Ok(*p);
        }
        let (mode, depth, has_dsv, samples, quality, write) = key;
        // Premultiplied output (rgb·As, sat(As)); only rgb is written, KH1's alpha stays.
        let (src, dst, op) = match mode {
            Mode::Alpha => (2, 6, 1),
        };
        let rt = RtBlend { enable: 1, logic_enable: 0, src, dst, op, src_a: 1, dst_a: 2, op_a: 1, logic_op: 4, mask: 0x7, _p: [0; 3] };
        let none = RtBlend { enable: 0, logic_enable: 0, src: 2, dst: 1, op: 1, src_a: 2, dst_a: 1, op_a: 1, logic_op: 4, mask: 0, _p: [0; 3] };
        let mut rts = [none; 8];
        rts[0] = rt;
        let keep = StencilOp { fail: 1, depth_fail: 1, pass: 1, func: 8 };
        let elems = [
            InputElem { name: b"POSITION\0".as_ptr(), index: 0, format: 6, slot: 0, offset: 0, class: 0, step: 0 },
            InputElem { name: b"TEXCOORD\0".as_ptr(), index: 0, format: 16, slot: 0, offset: 12, class: 0, step: 0 },
            InputElem { name: b"COLOR\0".as_ptr(), index: 0, format: 2, slot: 0, offset: 20, class: 0, step: 0 },
        ];
        let nob = || Bytecode { p: std::ptr::null(), n: 0 };
        let desc = PsoDesc {
            root: self.root as P,
            vs: Bytecode { p: self.vs.as_ptr() as *const c_void, n: self.vs.len() },
            ps: Bytecode { p: self.ps.as_ptr() as *const c_void, n: self.ps.len() },
            ds: nob(),
            hs: nob(),
            gs: nob(),
            so: SoDesc { decl: std::ptr::null(), n: 0, strides: std::ptr::null(), n_strides: 0, rasterized: 0 },
            blend: BlendDesc { alpha_to_coverage: 0, independent: 0, rt: rts },
            sample_mask: u32::MAX,
            raster: RasterDesc { fill: 3, cull: 1, front_ccw: 0, depth_bias: 0, depth_bias_clamp: 0.0, slope_bias: 0.0, depth_clip: 1, multisample: (samples > 1) as i32, aa_lines: 0, forced_samples: 0, conservative: 0 },
            depth: DepthDesc { enable: (depth && has_dsv) as i32, write_mask: (write && depth && has_dsv) as u32, func: 4, stencil: 0, read_mask: 0xFF, write_mask_s: 0xFF, _p: [0; 2], front: keep, back: keep },
            input: InputLayout { elems: elems.as_ptr(), n: 3 },
            strip_cut: 0,
            topology: 3,
            n_rt: 1,
            rtv: [87, 0, 0, 0, 0, 0, 0, 0],
            dsv: if has_dsv { 20 } else { 0 },
            samples,
            quality,
            node_mask: 0,
            cached: nob(),
            flags: 0,
        };
        let mut pso: P = std::ptr::null_mut();
        let hr = com!(self.device as P, DEV_CREATE_PSO, fn(*const PsoDesc, *const Guid, *mut P) -> i32, &desc, &IID_PSO, &mut pso);
        if hr < 0 || pso.is_null() {
            return Err(format!("CreateGraphicsPipelineState {key:?} failed: {:#010x}", hr as u32));
        }
        self.psos.push((key, pso as usize));
        Ok(pso as usize)
    }
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}

/// KH1's camera this frame: View and the plain projection P (glam layout = KH1's memory).
pub unsafe fn camera(base: usize) -> Option<(glam::Mat4, glam::Mat4)> {
    let view = base + VIEW_RVA;
    let mut proj = rd::<u64>(base + PROJ_PTR_RVA) as usize;
    proj = if proj > 0x10000 && readable(proj + 0x80) && readable(proj + 0xBF) { proj + 0x80 } else { base + FIELD_PROJ_RVA };
    let v = glam::Mat4::from_cols_array(&rd::<[f32; 16]>(view));
    let p = glam::Mat4::from_cols_array(&rd::<[f32; 16]>(proj));
    (v.is_finite() && p.is_finite() && v.determinant().abs() > 1e-6 && p.determinant().abs() > 1e-12).then_some((v, p))
}

/// KH1's bound frame buffer.
struct Target {
    fbi: i32,
    fb: usize,
    rtv: u64,
    dsv: u64,
    has_dsv: bool,
    samples: u32,
    quality: u32,
}

/// KH1's bound frame buffer: 0 main, 5 resolve, 1..4 the others.
unsafe fn target(r: usize) -> Option<Target> {
    let fbi = rd::<i32>(r + 0x6C);
    let fb = match fbi {
        0 => r + 0x540,
        5 => r + 0x660,
        1..=4 => r + 0x688 + fbi as usize * 0x120,
        _ => return None,
    };
    let (rtv_id, dsv_id) = (rd::<i32>(fb + 0x20), rd::<i32>(fb + 0x24));
    let (rtv_base, dsv_base) = (rd::<u64>(fb + 0x38), rd::<u64>(fb + 0x40));
    if rtv_id < 0 || rtv_base == 0 {
        return None;
    }
    let rtv = rtv_base + rd::<u32>(fb + 0x50) as u64 * rtv_id as u64;
    let has_dsv = dsv_id >= 0 && dsv_base != 0;
    let dsv = if has_dsv { dsv_base + rd::<u32>(fb + 0x58) as u64 * dsv_id as u64 } else { 0 };
    let (samples, quality) = (rd::<u32>(fb + 0xA0).max(1), rd::<u32>(fb + 0xA4));
    matches!(samples, 1 | 2 | 4 | 8 | 16).then_some(Target { fbi, fb, rtv, dsv, has_dsv, samples, quality })
}

/// The GPU state for KH1's device, made on the first draw; `None` while off or not ready.
unsafe fn ready<'a>(base: usize, g: &'a mut Option<Gpu>) -> Option<&'a mut Gpu> {
    if OFF.load(Ordering::Relaxed) {
        return None;
    }
    let device = rd::<u64>(base + RENDERER_RVA + 0x2E0) as usize;
    if device < 0x10000 {
        return None;
    }
    if g.as_ref().is_some_and(|g| g.device != device) {
        fail("KH1 made a new device".into());
        return None;
    }
    if g.is_none() {
        match init(device) {
            Ok(x) => *g = Some(x),
            Err(e) => {
                fail(e);
                return None;
            }
        }
    }
    g.as_mut()
}

/// The model texture arena, made the first time it's needed; false if it couldn't be.
unsafe fn mesh_arena(gpu: &mut Gpu) -> bool {
    if gpu.mesh.is_none() {
        match buffer(gpu.device, MESH_TEXELS * 4) {
            Ok((buf, map, va)) => gpu.mesh = Some(MeshArena { buf, map, va, used: 0, slots: Vec::new() }),
            Err(e) => {
                fail(format!("model textures: {e}"));
                return false;
            }
        }
    }
    true
}

/// The overlay font in its own buffer (made the first time, the font's size; the font copied in
/// when its key is new): its buffer's address and the texture's place there, `None` if it can't be.
unsafe fn font_buffer(gpu: &mut Gpu, t: &MeshTexture) -> Option<(u64, TexSlot)> {
    let n = (t.width * t.height) as usize;
    if n == 0 || t.rgba.len() < n * 4 {
        return None;
    }
    if gpu.font.is_none() {
        match buffer(gpu.device, n * 4) {
            Ok((buf, map, va)) => gpu.font = Some(FontBuf { buf, map, va, texels: n, key: None }),
            Err(e) => {
                fail(format!("the debug menu's font: {e}"));
                return None;
            }
        }
    }
    let f = gpu.font.as_mut()?;
    if n > f.texels {
        return None;
    }
    if f.key != Some(t.key) {
        std::ptr::copy_nonoverlapping(t.rgba.as_ptr(), f.map as *mut u8, n * 4);
        f.key = Some(t.key);
    }
    let _ = f.buf;
    Some((f.va, TexSlot { offset: 0, w: t.width, h: t.height }))
}

/// Copies `vertices` to the ring's write point, which goes round the whole ring (4 MB): a
/// draw's vertices stay put for many frames, however many draws a frame has (a slot per draw, as
/// before, came round again within KH1's 3 frames in flight with several draws a frame).
/// Returns the vertex buffer view and how many vertices it holds.
unsafe fn upload(gpu: &mut Gpu, vertices: &[kh2_fx::Vertex]) -> ([u64; 2], usize) {
    let max = SLOT_BYTES / 36;
    let n = vertices.len().min(max);
    let bytes = (n * 36 + 255) & !255;
    let total = RING_SLOTS * SLOT_BYTES;
    let mut at = gpu.frame as usize;
    if at + bytes > total {
        at = 0;
    }
    gpu.frame = (at + bytes) as u64;
    std::ptr::copy_nonoverlapping(vertices.as_ptr() as *const u8, (gpu.ring_map + at) as *mut u8, n * 36);
    ([gpu.ring_va + at as u64, ((n * 36) as u64) | (36u64 << 32)], n)
}

/// View, projection and KH1's clip scale and offsets (its cbConstantVS shadow: SafeScale,
/// xyOffset.zw) as root constants 0..35.
unsafe fn constants(r: usize, view: glam::Mat4, proj: glam::Mat4) -> ([u32; 40], f32, f32, f32) {
    let mut s = rd::<f32>(r + 0x5A10);
    if !(s.is_finite() && s > 0.01 && s < 100.0) {
        s = 1.0;
    }
    let (ox, oy) = (rd::<f32>(r + 0x59F8), rd::<f32>(r + 0x59FC));
    let (ox, oy) = if ox.is_finite() && oy.is_finite() { (ox, oy) } else { (0.0, 0.0) };
    let mut k = [0u32; 40];
    for (i, f) in view.to_cols_array().iter().chain(proj.to_cols_array().iter()).enumerate() {
        k[i] = f.to_bits();
    }
    k[32] = s.to_bits();
    k[33] = ox.to_bits();
    k[34] = oy.to_bits();
    (k, s, ox, oy)
}

/// KH1's current list with our targets (a direct list) or theirs (a bundle), our root signature,
/// triangle lists, the vertices and constants 0..35 bound; and its type (0 direct, 1 bundle).
unsafe fn begin(base: usize, r: usize, gpu: &Gpu, t: &Target, vb: &[u64; 2], k: &[u32; 40], srv: u64) -> Option<(P, u32)> {
    // (t356) No list cached and no command buffer: KH1 isn't recording (its getter would read the
    // null buffer: the debug menu's crash when drawn from the frame end).
    if rd::<u64>(r + 0x5990) == 0 && rd::<u64>(r + 0x5988) == 0 {
        return None;
    }
    let get_list: unsafe extern "system" fn(usize, usize, usize, usize) -> usize = std::mem::transmute(base + LIST_RVA);
    let cl = get_list(r, 0, 0, 0);
    if cl < 0x10000 {
        return None;
    }
    let cl = cl as P;
    let rtv_h = t.rtv as usize;
    let dsv_h = t.dsv as usize;
    // KH1 records its draws into bundles (`0x1400FE6E0` makes them, `0x1400F7290` closes one and
    // runs it in the frame's direct list): a bundle inherits the direct list's targets and may
    // not set them (it would fail to close: the crash of the first try). Only a direct list
    // gets them from us.
    let list_type = com!(cl, CL_GET_TYPE, fn() -> u32);
    if list_type == 0 {
        com!(cl, CL_SET_RT, fn(u32, *const usize, i32, *const usize), 1, &rtv_h, 0, if t.has_dsv { &dsv_h as *const usize } else { std::ptr::null() });
    } else if list_type != 1 {
        return None;
    }
    com!(cl, CL_SET_ROOT_SIG, fn(P), gpu.root as P);
    com!(cl, CL_TOPOLOGY, fn(u32), 4);
    com!(cl, CL_SET_VB, fn(u32, u32, *const u64), 0, 1, vb.as_ptr());
    com!(cl, CL_ROOT_CONSTANTS, fn(u32, u32, *const u32, u32), 0, 35, k.as_ptr(), 0);
    com!(cl, CL_ROOT_SRV, fn(u32, u64), 1, srv);
    Some((cl, list_type))
}

/// KH1 gets a fresh list, exactly as when its vertex ring runs out mid-scene (`0x1400FBBA0`):
/// ours is closed and queued (`0x1400F7300(R, 0)`), a new one opened with KH1's root signature
/// and heaps and all its cached state marked dirty (`0x140109360(R, 1, 0)`), so its next draw
/// sets everything again.
unsafe fn hand_back(base: usize, r: usize) {
    let close: unsafe extern "system" fn(usize, u32) -> u64 = std::mem::transmute(base + CLOSE_RVA);
    let reopen: unsafe extern "system" fn(usize, u32, u32) = std::mem::transmute(base + REOPEN_RVA);
    close(r, 0);
    reopen(r, 1, 0);
}

fn note_draw() {
    DRAWS.fetch_add(1, Ordering::Relaxed);
    LAST_DRAW_MS.store(START.get_or_init(std::time::Instant::now).elapsed().as_millis() as u64, Ordering::Relaxed);
}

/// A texture a model draw uses: `key` names it for good (the same key, the same pixels).
pub struct MeshTexture<'a> {
    pub key: u64,
    pub width: u32,
    pub height: u32,
    /// RGBA8, 255 = opaque.
    pub rgba: &'a [u8],
}

/// A run of a model's triangles: `count` vertices from `first`, with texture `texture` (an
/// index into the draw's textures). Opaque runs are cut out at half alpha and write depth;
/// translucent ones blend and don't.
#[derive(Clone, Copy, Debug)]
pub struct MeshBatch {
    pub first: u32,
    pub count: u32,
    pub texture: usize,
    pub translucent: bool,
}

impl MeshArena {
    /// The texture's place, uploaded the first time its key is seen (a texture that doesn't fit
    /// any more is drawn untextured).
    unsafe fn slot(&mut self, t: &MeshTexture) -> Option<TexSlot> {
        if let Some((_, s)) = self.slots.iter().find(|(k, _)| *k == t.key) {
            return Some(*s);
        }
        let n = (t.width * t.height) as usize;
        if n == 0 || t.rgba.len() < n * 4 || self.used + n > MESH_TEXELS {
            return None;
        }
        std::ptr::copy_nonoverlapping(t.rgba.as_ptr(), (self.map + self.used * 4) as *mut u8, n * 4);
        let s = TexSlot { offset: self.used as u32, w: t.width, h: t.height };
        self.used += n;
        self.slots.push((t.key, s));
        Some(s)
    }
}

/// The renderer is up or may still come up (not turned off by a failure).
pub fn usable() -> bool {
    !OFF.load(Ordering::Relaxed)
}

/// Draws a model's triangles (KH1 world space) into KH1's current command list, depth-tested
/// and depth-writing, from the world-pass hook (game thread). Returns the frame buffer it was
/// drawn into, `None` if it wasn't.
pub unsafe fn draw_mesh(base: usize, vertices: &[kh2_fx::Vertex], batches: &[MeshBatch], textures: &[MeshTexture], view: glam::Mat4, proj: glam::Mat4) -> Option<i32> {
    if OFF.load(Ordering::Relaxed) || batches.is_empty() || vertices.is_empty() {
        return None;
    }
    let r = base + RENDERER_RVA;
    let Ok(mut g) = GPU.lock() else { return None };
    let gpu = ready(base, &mut g)?;
    let t = target(r)?;
    if !mesh_arena(gpu) {
        return None;
    }
    let (opaque, blended) = match (gpu.pso((Mode::Alpha, true, t.has_dsv, t.samples, t.quality, true)), gpu.pso((Mode::Alpha, true, t.has_dsv, t.samples, t.quality, false))) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => {
            fail(e);
            return None;
        }
    };
    let arena = gpu.mesh.as_mut().unwrap();
    let slots: Vec<Option<TexSlot>> = textures.iter().map(|t| arena.slot(t)).collect();
    let (arena_va, arena_buf) = (arena.va, arena.buf);
    note_draw();
    let (vb, n) = upload(gpu, vertices);
    let (k, ..) = constants(r, view, proj);
    let (cl, list_type) = begin(base, r, gpu, &t, &vb, &k, arena_va)?;
    // Opaque runs first, then the translucent ones over them.
    let mut last_pso = 0usize;
    for pass in [false, true] {
        for b in batches.iter().filter(|b| b.translucent == pass) {
            let (first, count) = (b.first as usize, b.count as usize);
            if first >= n || count < 3 {
                continue;
            }
            let count = count.min(n - first) / 3 * 3;
            let pso = if pass { blended } else { opaque };
            if pso != last_pso {
                com!(cl, CL_SET_PSO, fn(P), pso as P);
                last_pso = pso;
            }
            let tx = slots.get(b.texture).copied().flatten();
            let flags = if tx.is_some() { 1 } else { 0 } | if pass { 0 } else { 4 };
            let tx = tx.unwrap_or(TexSlot { offset: 0, w: 1, h: 1 });
            let kb = [0f32.to_bits(), tx.offset, tx.w, tx.h, flags];
            com!(cl, CL_ROOT_CONSTANTS, fn(u32, u32, *const u32, u32), 0, 5, kb.as_ptr(), 35);
            com!(cl, CL_DRAW, fn(u32, u32, u32, u32), count as u32, 1, first as u32, 0);
        }
    }
    hand_back(base, r);
    static LOGGED: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    if LOGGED.fetch_add(1, Ordering::Relaxed) < 3 {
        log(&format!(
            "fx: drew a model: {} runs, {n} vertices, {} textures ({} KB of model textures) into a {} list, frame buffer {} ({}x MSAA, depth {})",
            batches.len(),
            textures.len(),
            gpu.mesh.as_ref().map_or(0, |a| a.used * 4 / 1024),
            if list_type == 1 { "bundle" } else { "direct" },
            t.fbi,
            t.samples,
            t.has_dsv
        ));
    }
    let _ = arena_buf;
    Some(t.fbi)
}

/// One rectangle of the debug overlay, in pixels of the bound frame buffer (`overlay_size`) from
/// its top left: a plain box (`textured` false) or a piece of the font texture (`uv`: left, top,
/// right, bottom, 0..1), in `color` (RGBA 0..1, alpha blended).
#[derive(Clone, Copy, Debug)]
pub struct OverlayQuad {
    pub rect: [f32; 4],
    pub uv: [f32; 4],
    pub color: [f32; 4],
    pub textured: bool,
}

/// The size (pixels) of the frame buffer the overlay draws into now, `None` before KH1 renders.
pub unsafe fn overlay_size(base: usize) -> Option<(f32, f32)> {
    if OFF.load(Ordering::Relaxed) {
        return None;
    }
    let t = target(base + RENDERER_RVA)?;
    let (w, h) = (rd::<i32>(t.fb + 8), rd::<i32>(t.fb + 0xC));
    (w > 0 && h > 0).then_some((w as f32, h as f32))
}

/// Draws the debug overlay into KH1's current command list: the boxes first, then the text, no
/// depth test, on top of everything drawn so far (from the frame-end hook, game thread). `size`
/// is the pixel space the rectangles are in (`overlay_size`). False if nothing was drawn.
pub unsafe fn draw_overlay(base: usize, quads: &[OverlayQuad], font: &MeshTexture, size: (f32, f32)) -> bool {
    if OFF.load(Ordering::Relaxed) || quads.is_empty() || size.0 < 1.0 || size.1 < 1.0 {
        return false;
    }
    let r = base + RENDERER_RVA;
    let Ok(mut g) = GPU.lock() else { return false };
    let Some(gpu) = ready(base, &mut g) else { return false };
    let Some(t) = target(r) else { return false };
    let pso = match gpu.pso((Mode::Alpha, false, t.has_dsv, t.samples, t.quality, false)) {
        Ok(p) => p,
        Err(e) => {
            fail(e);
            return false;
        }
    };
    // (No font buffer: the boxes alone, drawn with the root SRV at the ring, never sampled.)
    let (font_va, font_slot) = match font_buffer(gpu, font) {
        Some((va, slot)) => (va, Some(slot)),
        None => (gpu.ring_va, None),
    };
    // Boxes, then text: two runs.
    let mut vertices: Vec<kh2_fx::Vertex> = Vec::with_capacity(quads.len() * 6);
    let mut runs: Vec<(u32, u32, bool)> = Vec::with_capacity(2);
    for textured in [false, true] {
        if textured && font_slot.is_none() {
            break;
        }
        let first = vertices.len() as u32;
        for q in quads.iter().filter(|q| q.textured == textured) {
            let [x, y, w, h] = q.rect;
            let [u0, v0, u1, v1] = q.uv;
            let v = |px: f32, py: f32, u: f32, vv: f32| kh2_fx::Vertex { pos: [px, py, 0.0], uv: [u, vv], color: q.color };
            vertices.extend_from_slice(&[v(x, y, u0, v0), v(x + w, y, u1, v0), v(x, y + h, u0, v1), v(x + w, y, u1, v0), v(x + w, y + h, u1, v1), v(x, y + h, u0, v1)]);
        }
        let count = vertices.len() as u32 - first;
        if count > 0 {
            runs.push((first, count, textured));
        }
    }
    if runs.is_empty() {
        return false;
    }
    note_draw();
    let (vb, n) = upload(gpu, &vertices);
    // View = identity, projection = pixels to clip (the shader flips y): x' = 2x/W - 1, y' = 2y/H - 1.
    let proj = glam::Mat4::from_cols(glam::Vec4::new(2.0 / size.0, 0.0, 0.0, 0.0), glam::Vec4::new(0.0, 2.0 / size.1, 0.0, 0.0), glam::Vec4::ZERO, glam::Vec4::new(-1.0, -1.0, 0.0, 1.0));
    let mut k = [0u32; 40];
    for (i, f) in glam::Mat4::IDENTITY.to_cols_array().iter().chain(proj.to_cols_array().iter()).enumerate() {
        k[i] = f.to_bits();
    }
    k[32] = 1f32.to_bits();
    let Some((cl, _)) = begin(base, r, gpu, &t, &vb, &k, font_va) else { return false };
    com!(cl, CL_SET_PSO, fn(P), pso as P);
    for (first, count, textured) in runs {
        if first as usize >= n {
            continue;
        }
        let count = count.min(n as u32 - first) / 3 * 3;
        let tx = if textured { font_slot.unwrap_or(TexSlot { offset: 0, w: 1, h: 1 }) } else { TexSlot { offset: 0, w: 1, h: 1 } };
        let kb = [0f32.to_bits(), tx.offset, tx.w, tx.h, textured as u32];
        com!(cl, CL_ROOT_CONSTANTS, fn(u32, u32, *const u32, u32), 0, 5, kb.as_ptr(), 35);
        com!(cl, CL_DRAW, fn(u32, u32, u32, u32), count, 1, first, 0);
    }
    hand_back(base, r);
    static LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if !LOGGED.swap(true, Ordering::Relaxed) {
        log(&format!("debug menu: overlay drawn ({} quads, {}x{} frame buffer {}, {}x MSAA)", quads.len(), size.0, size.1, t.fbi, t.samples));
    }
    true
}
