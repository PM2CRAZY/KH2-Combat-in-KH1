# KH1 rendering: camera, D3D12 objects, frame structure

Reverse-engineered from the user's KH1 executable (Epic 1.5+2.5, x64, image base
`0x140000000`; add `actual_base - 0x140000000` to every address below, the game uses ASLR).
Ghidra project `/opt/re/kh1proj`; raw decompiler output stays in `/opt/re/out/fn`. Matrices
everywhere use KH1's own convention: **4x4 f32, row-major in memory, row vectors**
(`v' = v * M`, translation in row 3 at byte `+0x30`), which is the same memory layout as glam's
column-major `Mat4` used with `M * v`.

Confidence tags: **[high]** read directly from code or shader bytecode; **[med]** derived from
several pieces that agree but not checked at runtime; **[low]** a guess that needs a runtime check.

---

## 0. The renderer is Direct3D 12, not Direct3D 11 [high]

- The scene, HUD and present all go through **D3D12**. `0x1400F0150` enumerates adapters and
  calls `D3D12CreateDevice` (d3d12.dll ordinal 101, feature level 11_0) for the device stored at
  renderer `+0x2E0` (renderer = the `Axa::GX::GXDevice::GXDeviceImpl` singleton, see section 2).
  The swap chain is created with `IDXGIFactory2::CreateSwapChainForHwnd` on the D3D12 **command
  queue** (`0x1400F9840`); present is `IDXGISwapChain1::Present1` (`0x140104A50`). Every draw is
  recorded into an `ID3D12GraphicsCommandList` (`SetGraphicsRootConstantBufferView`,
  `ResourceBarrier`, `ExecuteBundle`, ... in `0x1400F3350`, `0x140103260`, `0x1400F7290`).
- The only `D3D11CreateDevice` call (`0x14011D940`) makes a device with
  `D3D11_CREATE_DEVICE_VIDEO_SUPPORT | BGRA_SUPPORT` for **movie playback only**: its caller
  `0x14011C780` opens `zmovie/...mp4` with Media Foundation (`MFCreateDXGIDeviceManager`,
  `MFCreateSourceReaderFromURL`), class `Axa::CCipherMovie`. Decoded frames are copied into D3D12
  textures (`MovieTexuture`, `LumaTexuture`, `ChromaTexture` debug names). That D3D11 device never
  sees the 3D scene.
- `d3d11.dll` exports `D3D11On12CreateDevice`, but KH1 does not import it. A D3D11 drawing path
  in the mod therefore needs its own 11-on-12 device built on KH1's D3D12 device and queue (section 5,
  option B). Recording D3D12 commands directly is simpler and keeps KH1's ordering (option A).

Engine name from RTTI: `Axa::GX::GXDevice::{GXDeviceImpl, MainFrameBuffer, ResolveFrameBuffer,
PrevFrameBuffer, FrameBufferManager, RenderTargetManager, DepthStencilManager, CommandBuffer,
Mesh, VertexResource, Texture, Surface}`. The game logic is the PS2 code ported to x64. It
still drives a PS2 GS emulation (GS `TEST`/`ALPHA`/`XYOFFSET` register writes, GIF tags), and the
`Axa` layer translates that into D3D12 state.

---

## 1. Camera

### 1.1 What the GPU uses this frame (read these) [high]

Written once per frame by `0x1401EB5C0 (view*, projStruct*, eye*, target*, zParams*)`, called from
the task `0x1401DEDE0` (task priority 140000, section 3). They stay valid from that point until the
same task on the next frame, so they hold during the whole world draw (task 160000) and the HUD:

| Address | What |
|---|---|
| `0x142E9DE60` | **View** (world -> view), 64 bytes. Uploaded as `cbViewProjection.WorldView`. |
| `0x142E9DFE0` | **Screen** = `P * S` (view -> PS2 GS screen), 64 bytes; first matrix of the active projection struct. Uploaded as `cbConstantVS.ViewProjection` for the "GS" shader family. |
| `0x142E9E120` | `View * Screen` (world -> GS screen). |
| `0x142E9DEE0` | inverse(View) = camera -> world (row 3 = eye position). |
| `0x142E9DEA0` | previous frame's View. |
| `0x142E9DF20` / `0x142E9DF30` | eye / look-at target (vec4), as drawn. `0x142E9DF40` / `0x142E9DF50` = previous. |
| `0x142E9DE18` | field of view in degrees (float, from `0x14299C500`). |
| `0x140550BB0` | `{0, 0, zmin - zmax, zmax}` = projection struct `+0x100` (PS2 24-bit Z remap, CPU sort keys only). |

The active **projection struct** (0x110 bytes) is the one `0x142340EE0` points at when task
140000 runs; it is also argument 2 of `0x1401EB5C0`. Hook that function if you want the exact
pointer (prologue `48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 57 48 83 EC 20`, Win64 args
`rcx, rdx, r8, r9, [rsp+0x28]`). Layout, built by `0x1400D4A70` / `0x1400E9040` / `0x1400E8F30`:

| Offset | Matrix |
|---|---|
| `+0x00` | Screen = `P * S` (copied to `0x142E9DFE0`) |
| `+0x40` | identity |
| `+0x80` | **P**, a plain D3D-style left-handed perspective (depth 0..1) |
| `+0xC0` | **S**, a GS screen scale: `diag(sx/2, sy/2, 1, 1)` with row 3 = `(2048, 2048, 0, 1)` |
| `+0x100` | `{0, 0, zmin - zmax, zmax}` with zmin = 1, zmax = 16 777 000 |

The pointer usually targets `0x14299CA80` (field camera). Events can repoint `0x142340ED8`
(view*) / `0x142340EE0` (projection*) through `0x140225B10`, and the event camera task
`0x1402D3040` runs at priority 159999. In the field, P is also available ready-made at
`0x142341320` (and View at `0x1423412A0`); those are the copies the "clip" shader family uploads
(`0x140197E90`, `0x140196A10`).

### 1.2 Projection P (`+0x80`) [high structure, med numbers]

`P00 = k * ax`, `P11 = k * ay`, `P22 = f/(f-n)`, `P23 = 1`, `P32 = -n*f/(f-n)`, everything else 0.

- `k = 2 * scrz * 0.84 / 384`. The PS2 virtual screen is 512x384 with pixel aspect constants
  0.8 / 0.84 (`0x14011F300`, set by `0x14016F240`). `scrz = 256 / tan(fov/2)` lives at
  `0x14299CB90`. `fov` is the PS2 horizontal FOV in degrees at `0x14299C500`, default 53°, which
  gives scrz ≈ 513.5 (setters `0x14028A960` / `0x14028A900` / `0x14028A8B0`).
- Widescreen, with `(W, H)` = render size floats at renderer `+0x18`/`+0x1C`: `ax = H/W` and
  `ay = 1`. When the window is narrower than 16:9 (`9W < 16H`), both `ax` and `ay` are also
  multiplied by `9W/(16H)`. So the horizontal FOV is fixed at its 16:9 value (Hor+ for wider
  windows, Vert+ for narrower ones). Defaults at 16:9: vertical FOV ≈ 48.0°, horizontal ≈ 76.7°.
- `n` = 4.0, or 0.2 in one room (`0x142340E5C == 6 && 0x142340EC4 == 7`). The far plane of the
  matrices the GPU sees is `f = 4.0e8` (`0x1403E3E8C`). A separate per-map far at `0x1423418C8`
  only feeds GL-style culling matrices (below).
- Depth seen by D3D: `z_ndc = f/(f-n) * (1 - n/z_view)`, which is about `1 - 4/z_view`
  (standard LH 0..1).

Not GPU-side, but present: GL-style projections (`(n+f)/(f-n)`, `-2nf/(f-n)`) at `0x142340600`
(far 4e8) and `0x142340640` (map far), and `0x142340700 = View * 0x142340600`, which object
culling uses (`0x14019D270`, `0x1401EA9F0`). Do not use these for drawing.

### 1.3 Conventions [high]

- The look-at is built by `0x1400E63D0`: forward = normalize(target - eye),
  `X = up x forward`, `Y = forward x X`, up = `(0, 1, 0)` rolled by `0x14299C504`. View is the
  inverse of `[X; Y; forward; eye]`.
- **World is right-handed with +y down** (gravity +y; matches the actor notes in AGENTS.md).
  **View space**: +x right, +y down, +z forward (into the screen, depth grows with z). The
  shaders flip y for D3D. Units: KH1 units (Sora's hips at 78.9, so roughly 1 unit ≈ 1 cm).
- The draw order key confirms +z forward: `0x1401EADD0` transforms each object's position by the
  View at `0x142E9DE60` and sorts by view z, far to near.

### 1.4 Camera source and the one-frame lag [high]

- Field camera eye `0x14299C4E0`, target `0x14299C4F0` (vec4), roll `0x14299C504`, FOV
  `0x14299C500`, scrz `0x14299CB90`. The camera task `0x14028ABC0` (priority 110000) builds View
  into the ring `0x14299CBA0 + idx*0x40` and the View·Screen struct into
  `0x14299C640 + idx*0x110`, where `idx = 0x142867CD4`. That index flips after all tasks
  (`0x140282520`).
- Task `0x14028AA80` (priority 135000) copies the **other** slot (`idx ^ 1`) into the "current"
  globals `0x14299C4A0` (View), `0x14299C530` (View·Screen struct) and `0x14299CA80`
  (projection struct). It restores `0x1423412A0`/`0x142341320` the same way (`0x14018A4F0`).
  **The frame is drawn with the previous update's camera.** Use the section 1.1 copies, which are
  exactly what was uploaded.

### 1.5 World -> screen helper (anchor) [high]

`0x1400D4C70 (int out[4], const mat-struct*, const vec4* pos, int flag)` computes
`(pos, 1) * M`, divides by w and multiplies by 16. It returns PS2 GS fixed point 12.4 x/y plus
z/w and w. `0x14016F8F0` (2D marker placement) calls it with `0x14299C530` (View·Screen). It then
takes `(x >> 4) - 0x700` and `(y >> 4) - 0x740`, which gives PS2 screen pixels (GS `XYOFFSET`
1792,1856), and tests visibility against 0..512 x 0..384.

---

## 2. D3D12 objects

### 2.1 Renderer singleton `R = 0x1422BB580` [high]

A function-local static returned by `0x1400FD9C0`. It is constructed on first call, so read it
only after the game has started rendering. Fixed offsets:

| R + | Object |
|---|---|
| `0x2E0` (`0x1422BB860`) | `ID3D12Device*` |
| `0x3F0` | `HWND` |
| `0x468` (`0x1422BB9E8`) | `IDXGISwapChain3*` (QI'd from the swap chain made by `CreateSwapChainForHwnd`) |
| `0x470` (`0x1422BB9F0`) | `ID3D12CommandQueue*`, DIRECT, the queue of the swap chain and of every frame submission |
| `0x488` | a second DIRECT queue (copy/resolve work, `R+0x7B28` wrapper) |
| `0x4A0` / `0x498` | a command allocator / list made at init |
| `0x4D0` | `IDXGIOutput*` (used for `WaitForVBlank`), `0x4D8` the `IDXGIAdapter*` |
| `0x4E0` | RTV descriptor heap (0x140 entries) |
| `0x4E8` | DSV descriptor heap (0x140 entries) |
| `0x4F0` | wrapper of the shader-visible CBV/SRV/UAV heap (16384 entries; the `ID3D12DescriptorHeap*` sits at wrapper `+0x808`) |
| `0x4F8` | sampler heap |
| `0x5988` | current command-buffer wrapper |
| `0x5990` | cached current `ID3D12GraphicsCommandList*` (may be null). **Get it with `0x1400FD3A0(R)`**, which opens or reuses it. |
| `0x540` | `MainFrameBuffer` (fb index 0) |
| `0x660` | `ResolveFrameBuffer` (fb index 5) |
| `0x688 + i*0x120` | frame buffers 1..4 |
| `0x6C` | index of the currently bound frame buffer |
| `0x60` | current shader program id (`0x1401088A0` sets it) |
| `0x15C` / `0x164` | current viewport x,y / w,h (floats, ≤ 0 means 1) |
| `0x7AF0` | current back buffer index (`GetCurrentBackBufferIndex`) |
| `0x8`/`0xC` | swap-chain width/height (ints, used for `CreateSwapChainForHwnd`) |
| `0x18`/`0x1C` | render size (floats; drives P's aspect, section 1.2) |
| `0x454` | highest supported MSAA count for B8G8R8A8 (8/4/2/1, `CheckFeatureSupport`) |

Constant-buffer shadows (exactly what the last draw uploaded):

- `R+0x59E0`, 0x90 bytes: **`cbConstantVS`** (b2, root parameter 0). `+0x00 Projection2D
  {2/vw, 2/vh, vw/2, vh/2}`, `+0x10 xyOffset`, `+0x20 MagicScale`, `+0x30 SafeScale` (f32),
  `+0x34 FogEnable`, `+0x38 __DepthScale` (f32x2), `+0x40 colorScale`, `+0x44 alphaScale`,
  `+0x48 TPage`, `+0x50 ViewProjection` (= `R+0x5A30`, 4x4).
- `R+0x5BA8`, 0xD80 bytes: **`cbShaderParam`** (b3, root parameter 1). `TexScale +0x00`,
  `Ambient +0x10`, `FogParam +0x20`, `ParamVector[5] +0x30`, `LightMatrix +0x80`,
  `ColorMatrix +0xC0`, `TextureMatrix +0x100`, `WorldMatrix +0x140`,
  `LocalProjection[48] +0x180`. Setters: `0x140107830` (type 0..4 and 0x1000), `0x140107770`
  (vec4 slots, 0x1001/0x1002), `0x140107A10` (`LocalProjection[slot]`).
- `R+0x76AC`, 0x40 bytes: **`cbViewProjection.WorldView`** (b4, root parameter 2) = View.
- Root parameter 3 is a 0x50-byte pixel constant buffer (`R+0x5B04`). Root parameter 4 is the
  sampler table, 5/6 the texture SRV tables. The upload code is in `0x1400F3350`; the names come
  from the RDEF chunks of the 47 DXBC blobs embedded in the exe (file offsets `0x3B20F0`..`0x3D04E0`).

### 2.2 Frame buffers [high]

Frame-buffer object `fb` (RTV made in `0x1400FCD60`, DSV made in `0x1400FCB00`):

| fb + | Field |
|---|---|
| `0x08`/`0x0C` | width/height (int) |
| `0x20` | RtvId |
| `0x24` | DsvId |
| `0x38` | RTV heap CPU start (u64) |
| `0x50` | RTV increment (u32) |
| `0x40` | DSV heap CPU start (u64) |
| `0x58` | DSV increment (u32) |
| `0xA0` | MSAA sample count (u32) |
| `0xA4` | sample quality (u32) |
| `0x108` | colour resource table, 0x48-byte entries: `+8 ID3D12Resource*`, `+0x10` tracked state; index RtvId |
| `0x110` | depth resource table, same layout; index DsvId |

- **Colour**: `DXGI_FORMAT_B8G8R8A8_UNORM` (87), placed resource, MSAA (`fb+0xA0` samples),
  created in state `RENDER_TARGET`. RTV handle = `[fb+0x38] + [fb+0x50] * [fb+0x20]`.
- **Depth**: `DXGI_FORMAT_D32_FLOAT_S8X24_UINT` (20), placed, same MSAA, view dimension
  `TEXTURE2DMS`, created in `DEPTH_WRITE`. DSV handle = `[fb+0x40] + [fb+0x58] * [fb+0x24]`.
  This is an ordinary float depth buffer: depth = `z/w` in 0..1, 0 near. Our particles can test
  against it directly with `DepthFunc = LESS_EQUAL` and depth writes off.
- **Depth test mapping** (`0x140383710`, GS `TEST` register): ZTST NEVER/ALWAYS/GEQUAL/GREATER
  becomes `NEVER/ALWAYS/LESS_EQUAL/LESS` (table `0x140401550`). So D3D depth is the standard
  direction (smaller = nearer), not the PS2 "bigger = nearer" convention.
- **Swap chain**: B8G8R8A8_UNORM, 3 buffers, 1 sample, `FLIP_SEQUENTIAL`, flags
  `ALLOW_MODE_SWITCH | FRAME_LATENCY_WAITABLE_OBJECT`, `SetMaximumFrameLatency(2)`.
- **From scene to Present** [med]: the scene renders into the MSAA frame buffer bound at `R+0x6C`
  (normally fb 0 = `R+0x540`). At frame end `0x140103260` binds the "frame texture" (`R+0x78E0`
  `prevFrameTex`, or `R+0xC28`) and draws it to the back buffer with a full-screen pass
  ("fillScreen", program 5/0x2F). That pass scales from the internal size (`fb` width/height) to
  the swap-chain size and runs after MSAA resolve ("resolveTarget", fb 5). There is no other
  post-processing pass. The HUD is drawn into the same frame buffer as the 3D scene, before that
  copy.

---

## 3. Frame structure

### 3.1 Game thread (field mode `0x14016ED10`) [high for order, med for task roles]

1. The task list `0x142867D80` runs in ascending priority (`0x140282520` -> runner
   `0x140288590`; tasks are registered with `0x140282500(priority, fn)`). Rendering is recorded
   **inline** in these tasks. Each draw goes through the state flush `0x1400F3A40` (named
   "applyViewport"), which writes dirty root CBVs, the PSO, RT, viewport and scissor into the
   current command list.
   - 110000 `0x14028ABC0`: camera update (section 1.4).
   - 135000 `0x14028AA80`: copy the previous camera slot into the "current" globals.
   - 140000 `0x1401DEDE0`: GPU camera copies (`0x1401EB5C0`), then `0x1401EA9F0` (pre-pass over
     the model-object table: culling with `0x142340700`, animation).
   - 159998/159999: misc. `0x1402D3040` (159999) is the pause menu's task (registered by its
     opener `0x1402D3450`): it updates and draws the menu's windows (`0x1402E19B0`) before the
     world pass, and keeps the camera repointed (`0x142340ED8` / `0x142340EE0`, put back when
     the menu closes) for the party models the world pass then draws over them. So anything
     drawn after the world pass lands on top of the pause menu (`kh2fx` draws nothing while
     KH1's pause state `0x14232E9B4` is up) [high: read from the code].
   - **160000 `0x1401DEE40` -> `0x1401EADD0`: draws every model object.** The table is at
     `0x142678C80`: 256 entries of 0xE0 bytes, flag word `+0x92`, model `+0x98`, data `+0xA0`.
     Objects are sorted far to near by view z; objects flagged "always first" use key 1e6 (sky),
     "last" ones key 0. Characters, enemies and props are drawn here. Whether every piece of the
     map is drawn here was not proven [med].
   - 165001 / 170001 (many tasks in `0x140266000`..`0x140276000`): at least one is an
     **effect/particle renderer**. `0x140274BD0` sorts triangles into depth buckets with the
     View·Screen struct (`0x1402747E0`) and draws them with KH1's immediate-mode API:
     `0x1400F31E0(count)` returns vertex memory of 0x20-byte vertices (float4 position, uv,
     RGBA8, a u16 "kick"), `0x1400FBB60(count)` flushes state and draws, with shader program 8
     and GS state set from a GIF tag (`0x140383FD0`). That is KH1's own effect path [med].
     The field HUD is sixteen of these 165001 / 170001 tasks, set up by `0x140264C50`
     (`docs/kh1-hud-hide.md`) [high].
   - 180000+ (`0x2BF20`, `0x2BF84`, `0x2BFE7`..`0x2BFE9`, ...): menus, HUD and event tasks
     [low: roles inferred only from their order after all 3D tasks].
2. After the task list: `0x140177100` (screen-space projections for 2D markers), `0x1401790B0`
   (2D/menu work [low]), then `0x1400DAED0` -> `0x140109CC0` -> `0x140109CE0`, the **frame end**
   (under mutex `R+0x7880`):
   - `0x140109AD0`: resolve and "fillScreen" (`0x140103260`), then `ExecuteCommandLists` of
     every list recorded this frame (`0x140102CE0`, which calls `Signal` then
     `ExecuteCommandLists` on `R+0x470`), then `Present1` (`0x140104A50`).
   - Command lists are only **closed and queued** mid-frame (`0x1400F7300(R, 0)`); GPU submission
     happens at frame end (or when something calls `0x1400F7300(R, 1)`). Upload rings are
     triple-buffered (`R+0x5750 % 3`).

### 3.2 Frame pacing / interpolation thread [high existence, low details]

`0x14010A920` -> `0x140109FE0` paces frames and waits for vblank through `IDXGIOutput`. When
interpolation is active (`R+0x59AC != 0` or `R+0x59A4 < threshold`) it replays the recorded
command lambdas ("InterpolateCommandBuffer", `0x140102740`) with a View interpolated between
`R+0xE390` (current) and `R+0xE3D0` (previous). Commands that a mod records straight into the
command list are **not** part of that replay. In an interpolated (high-frame-rate) mode, our
particles would only show on the real game frames. Which user setting turns this on was not
traced.

---

## 4. The shaders and the exact world -> clip transform

All 3D vertex shaders compute `p_view = (pos, 1) * LocalProjection[0] * WorldView` (row
vectors, `cb3[24..27]` then `cb4[0..3]`). Then `c = p_view * ViewProjection` (`cb2[5..8]`).
After that there are two families (SM4 bytecode decoded from the exe):

**Family B, "clip"** (for example the shaders at file offsets `0x3B5E00`, `0x3B9740`,
`0x3BB5E0`, `0x3C1140`: skinned characters and the "KICK" immediate-mode vertices).
ViewProjection is the plain projection P:

```
c        = (p_world, 1) * View * P
clip.x   =  c.x * SafeScale + c.w * xyOffset.z
clip.y   = -(c.y * SafeScale + c.w * xyOffset.w)
clip.z   =  c.z
clip.w   =  c.w
```

**Family A, "PS2 screen"** (for example `0x3B42C0`, `0x3B5010`: meshes drawn through
`0x14031C4E0` / `0x14031C660`). ViewProjection is Screen = `P * S`, so `g = c * S` is in GS
pixels times w, centred on 2048:

```
clip.x =  ((g.x - 2048 g.w) * MagicScale.x * Projection2D.x + g.w * xyOffset.z)
clip.y = -((g.y - 2048 g.w) * MagicScale.y * Projection2D.y + g.w * xyOffset.w)
clip.z =  max(g.z, 0)    (some variants take z from LocalProjection[1]; for ordinary meshes that equals the first matrix)
clip.w =  g.w
```

For these shaders the CPU reconstructs `LocalProjection[0]` (the model matrix) as the PS2
code's model->screen matrix times `inverse(View * Screen)` (`0x14031C660` with `0x142E9DE60` and
`0x142E9DFE0`). Objects with the "screen" flag use identity (`0x1404D7580`). Both families draw
into one depth buffer in the same frame, so they must agree. That requires
`MagicScale.x * Projection2D.x * sx/2 == SafeScale` (and the same for y) [med: values not seen
at runtime]. The z column of S is the identity, so depth is identical in both families.

**What the mod should do** [high for the math, med for the "usually" values]:

```
V = *(mat4*)0x142E9DE60                       // as drawn this frame
P = *(mat4*)(projStruct + 0x80)               // projStruct = *(u8**)0x142340EE0 at task 140000,
                                              // or 0x142341320 in the field
c = (x, y, z, 1) * V * P                      // row vectors
clip = (c.x*s + c.w*ox,  -(c.y*s + c.w*oy),  c.z,  c.w)
  s  = *(f32*)(R+0x5A10)   SafeScale, usually 1
  ox = *(f32*)(R+0x59F8)   xyOffset.z, usually 0 (GS XYOFFSET sub-offset, e.g. screen shake)
  oy = *(f32*)(R+0x59FC)   xyOffset.w, usually 0
```

Use KH1's current viewport (`R+0x15C`..`R+0x168`) with MinDepth 0 and MaxDepth 1, depth
`LESS_EQUAL`, and leave depth writes off for additive particles. To match a family-A mesh
pixel-exactly, the alternative is to run the family-A formula with the live `cbConstantVS`
values at `R+0x59E0` and Screen at `0x142E9DFE0`.

**Q4, "PS2-style pre-transformed vertices?"** Partly. Family A really does go through GS screen
space (4096x4096 GS coordinates, centre 2048, PS2 visible area 512x384 at XYOFFSET 1792,1856).
But depth is a float `z/w` in 0..1 (D32F). The 24-bit integer Z (`zmin = 1`, `zmax = 16 777 000`,
`0x140550BB0`) exists only for CPU-side sort keys (`0x14031BDD0`). Nothing has to be emulated
for depth testing: use the plain P above.

---

## 5. Hook point for world-space particles

### 5.1 Recommended: after the model pass, `0x1401EADD0` [med-high]

`void 0x1401EADD0(void)`, Win64, no arguments, the only caller is the task stub `0x1401DEE40`
at priority 160000. At its return all world objects are drawn and recorded, the camera globals
above are valid, and the HUD has not started. KH1's own effect tasks (165001/170001) come after
it. They are additive and depth-tested, so order against them hardly matters.

- Prologue, 15 bytes, position-independent, safe to relocate into a trampoline:
  `48 8B C4` mov rax,rsp · `48 89 58 08` mov [rax+8],rbx · `48 89 70 10` mov [rax+10h],rsi ·
  `48 89 78 18` mov [rax+18h],rdi. The next bytes are `55 41 54 41 55 41 56 41 57`
  (push rbp/r12..r15).
- Alternative 5-byte patch: the call site `0x1401DEE4D` `E8 7E BF 00 00` (`call 0x1401EADD0`).
  Redirect it to a stub that calls the original and then draws. Do not inline-hook `0x1401DEE40`
  itself: its second instruction is a RIP-relative `cmp`.

### 5.2 Option A, D3D12 recorded into KH1's own list (preferred) [med]

On the game thread, inside the hook after the original returns:

1. `cl = 0x1400FD3A0(R)` gives the current `ID3D12GraphicsCommandList*`.
   `fb` = frame buffer `*(i32*)(R+0x6C)` (0 gives `R+0x540`, 5 gives `R+0x660`, 1..4 give
   `R+0x688+i*0x120`). RTV/DSV handles as in 2.2; sample count from `fb+0xA0`.
2. Record ours: `OMSetRenderTargets(1, &rtv, FALSE, &dsv)`, viewport and scissor, our root
   signature and PSO (RTV format 87, DSV format 20, `SampleDesc.Count = fb+0xA0`, depth
   LESS_EQUAL, write mask 0), our descriptor heap, vertex/constant data in our own upload buffer.
   Keep that buffer alive at least 3 frames, because the list executes at frame end.
3. Give KH1 a clean list again, with the same sequence KH1 itself uses when its vertex ring
   overflows mid-scene (`0x1400FBC72`..`0x1400FBC89` in `0x1400FBBA0`, also in drawVertexBuffer
   `0x1400FBED0`): call `0x1400F7300(R, 0)`, which closes and
   queues the list, then `0x140109360(R, 1, 0)`. The latter opens a new list, sets KH1's root
   signature and descriptor heaps, and marks all cached state dirty (`0x140102980`), so the next
   KH1 draw re-applies RT, viewport, PSO and CBVs. Calling convention: Win64, `rcx = R`,
   `dl`/`r8b` = the flags shown.

### 5.3 Option B, D3D11 through 11-on-12 [med-low]

Create `D3D11On12CreateDevice(R+0x2E0 device, ..., &R+0x470 queue, 1, ...)`. Wrap the fb's
colour and depth resources (`*(ID3D12Resource**)(*(fb+0x108) + RtvId*0x48 + 8)`, and the same
with `+0x110`/DsvId) with in/out states `RENDER_TARGET` / `DEPTH_WRITE`. Views must be
`TEXTURE2DMS` when `fb+0xA0 > 1`. KH1's recorded commands only reach the queue at frame end, so
before drawing you must submit them:

1. `0x1400F7300(R, 0)` then `0x140102CE0(R+0x5928, 0xFFFFFFFF, 0, 0)` (executes the queued lists).
2. 11-on-12 acquire, draw, release, `Flush`.
3. `0x140109360(R, 1, 0)`.

This costs extra mid-frame submissions, but keeps ordering and depth correct.

### 5.4 Not recommended

Drawing at `Present1` (`0x140104A50` / swap chain `R+0x468`) lands after the HUD and the
upscale. The depth buffer there is at internal resolution and MSAA, and the back buffer is
scaled.

---

## 6. Things to verify at runtime (cheap log checks)

- Log `R+0x6C`, `fb+0x08/0x0C/0xA0`, `R+0x15C..0x168` and the 0x90 bytes at `R+0x59E0` inside
  the hook. Expected: fb 0, sample count ≥ 1, SafeScale 1, xyOffset.zw 0, and
  `MagicScale.x * 2/vw * S00 ≈ 1` where `S00` is at `projStruct+0xC0`.
- Project Sora's position (actor `+0x10`) with the formula above and compare it with
  `0x1400D4C70`'s result and with what's on screen. Check that depth `c.z/c.w` sits slightly
  below the value KH1 writes there (a depth-buffer readback, or simply whether the particle
  hides behind walls).
