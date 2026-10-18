# Recolouring Sora in KH1 (Wisdom Form's blue)

Notes from static reading of the Epic exe (addresses are Epic VAs; Steam ones come from
`steam_map.in`). Used by `crates/kh1-hook/src/win/tint.rs`.

## How a model mesh gets its colour

- Sora's per-frame draw (picked by actor kind byte `+6`) works out an RGBA from three things:
  the bytes `actor+0x370..0x373` (128 = neutral, eased each frame toward `+0x350..`), the floats
  `actor+0xA0..0xAC` (KH1's fades) and the scene's light product `actor+0x430`. That colour is
  passed down to the model queue (`0x1401D1F10`) and, per mesh, ends up as the shader's
  **Ambient** constant (RGBA / 128).
- When Sora counts as lit (`actor+0x420 != 0`) the mesh draw forces Ambient to white, so writing
  a tint into the actor fields does nothing in those rooms. Hence the per-draw approach below.
- The mesh draw `0x1401DC5A0` sets the program up and then calls `0x1401DB7F0` with the mesh's
  packet list in rcx (call sites `0x1401DC77D` lit and `0x1401DC797` unlit). Prologue
  `4C 8B DC 49 89 5B 20 55 56 41 54 41 56 41 57` (15 bytes, nothing RIP-relative).
- The pixel is the texture times Ambient, then mixed toward a fog colour by a factor f:
  `out = tex·A·f + F·(1 − f)`. There is no colour matrix: each channel is scaled and offset on
  its own, so a colour can only be pulled toward another, not have its channels swapped.

## The constants

- `0x1400FD9C0()` returns the (per-thread) render state R.
- `0x140107770(id, ptr, 0)` sets a constant and marks the buffer dirty (`R+0x76AB`):
  `0x1001` Ambient (vec4, `R+0x5BB8`), `0x1002` FogParam (vec4, `R+0x5BC8`, f in `.w`),
  `0x2000` FogColor (3 floats 0..1, `R+0x5B34`). Same offsets on Steam.
- Ambient's alpha below 0x80/128 changes sorting and alpha test: left alone.

## Sora's meshes

`xa_ex_0010`: 293 bones, 9 meshes, all in mesh group 0 (so the actor's group mask `+0x38C`
can't pick the clothes), split by texture:

| mesh | content |
|---|---|
| 0 | skin |
| 1, 2 | hair (1 also the shoes) |
| 3, 4 | jacket and hood (4 is a second pass) |
| 5 | red jumpsuit |
| 6, 7 | pockets, belt, chain, buckles (7 a second pass) |
| 8 | face |

At run time his instance (`actor+0x15C`, a handle) points at a slot whose first qword is the
model header: `u16` bone count at `+0`, mesh count at `+0xC`, mesh entries of 0x10 bytes from
`+0x10`, each with its packet list's handle at `+0xC`. (A header whose `+0x10` reads
`0x96969696` uses another table; not Sora's.)

## What the mod does

Hook `0x1401DB7F0`; when the packet list is one of meshes 3..7 and a form's look is on, save
Ambient / FogParam / FogColor, set ours (KH1's own fog folded into the one fog term), call the
original, put KH1's back. The pause menu's copy of Sora shares the packets, so it is tinted too.

The red jumpsuit has almost no green or blue in its texture, so it turns blue mostly through the
fog term (flat, with little shading). A true blue jumpsuit would need a recoloured copy of its HD
texture (the drawn textures are the PC's HD replacements, re-copied to a per-object texture each
draw from the source at image object `+8`), swapped in on entering the form: not done.
