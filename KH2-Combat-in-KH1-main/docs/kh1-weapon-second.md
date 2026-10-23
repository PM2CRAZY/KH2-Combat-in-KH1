# A second Keyblade on KH1 Sora (Valor Form's left hand)

Goal: while Valor Form is active, KH1 Sora holds a second Keyblade in his left hand, as KH2's
Valor does. It is a different Keyblade from the equipped one (the "Valor Keyblade", any
Keyblade he owns). KH1 itself only ever gives a party member one weapon.

Static reading of the Epic exe (image base `0x140000000`, add the ASLR slide), Steam RVAs from
`/opt/re/steam/resolve.py`. Data facts were checked against the user's `btltbl.bin`,
`xa_ex_0010.mdls` and `kh1_first.hed`. Nothing here has been checked at runtime yet; the
checks to make first are in section 6.

Confidence: **[high]** read directly from code or data; **[med]** several pieces agree but
nothing was run; **[low]** a guess.

---

## 1. How KH1 loads a party member's weapon

### 1.1 Which file [high]

- The equipped weapon is the item byte at `save + member * 0x74 + 0x36`. The save pointer is
  at `0x1428695A0`, the same one `pose.rs` reads; Sora is member 0.
- `btltbl.bin` is loaded once, synchronously, at **`0x142D23740`** (`0x14028D840`; Steam
  `0x2D22D40`).
  - Item table: the pointer at `0x142D23738` is `btltbl + 0x1A58`, with 0x14-byte records
    (`0x14028D7E0(id)` returns `table + (id - 1) * 0x14`). Byte `+2` is the owner (member + 2,
    so 2 for Sora). The short at `+6` is the **weapon model number** (1-based).
  - Weapon records: `btltbl + table[member] + (n - 1) * 0x58`, where `table` is the u32 array
    at `0x140528D60`: Sora `0x94F8`, Donald `0x9C88`, Goofy `0xA208`, then `0xA788`, `0`,
    `0xA7E0` (guests). The record starts with the file stem (16 bytes). The rest holds trail
    and effect parameters (not decoded).
- File names are `"%s%s"` (`0x1403B2E0C`) of the stem plus `".se"` (pointer at `0x140528D90`),
  then plus `".wpn"` (pointer at `0x140528D88`). Sora's 22 weapons are items `0x51..0x66`,
  model numbers 1..22, in order:

| item | file | item | file | item | file |
|---|---|---|---|---|---|
| 0x51 | xw_ex_5010 (Kingdom Key) | 0x59 | xw_ex_5050 | 0x61 | xw_ex_5130 |
| 0x52 | xw_dh_5000 (dream sword) | 0x5A | xw_ex_5060 | 0x62 | xw_ex_5140 |
| 0x53 | xw_dh_5010 (dream shield) | 0x5B | xw_ex_5070 | 0x63 | xw_ex_5150 |
| 0x54 | xw_dh_5020 (dream rod) | 0x5C | xw_ex_5080 | 0x64 | xw_ex_5160 |
| 0x55 | xw_di_5000 (wooden sword) | 0x5D | xw_ex_5090 | 0x65 | xw_ex_5190 |
| 0x56 | xw_ex_5020 | 0x5E | xw_ex_5100 | 0x66 | xw_ex_5200 |
| 0x57 | xw_ex_5030 | 0x5F | xw_ex_5110 | | |
| 0x58 | xw_ex_5040 | 0x60 | xw_ex_5120 | | |

  (`0x52..0x55` being the dream weapons and the wooden sword matches `pose.rs`'s existing
  exclusion.)

### 1.2 The weapon slots and the loader [high]

Three **slots** of 0x30 bytes at **`0x14296BF40`** (Steam `0x296B540`). Slot 0 is always
Sora's. Slots 1 and 2 are for any other member (Donald, Goofy, guests).

| slot + | what |
|---|---|
| `0x00` | flags: bit 0 in use; bits 1-2 state (2 = an object uses it, 4 = loading) |
| `0x04` | member |
| `0x08` | weapon model number |
| `0x0C` | buffer size: `0x40000` for Sora, `0x20000` for the others (from `0x140528D98..`) |
| `0x10` | buffer: a static pool at `0x14286BBE0` (Sora first, then the two others) |
| `0x18` | the `.wpn`'s model container (file + `[file+8]`), after setup |
| `0x20` | the `.wpn`'s effect section (file + `[file+4]`), after setup |
| `0x28` | completion callback (gets the member) |

Functions (Epic → Steam):

| Epic | Steam | what |
|---|---|---|
| `0x1402849C0` | `0x286B40` | reset the slots (area start) |
| `0x140284A30(cb)` | `0x286BB0` | load every party member's weapon (members from `save+0x48E..0x490`), chained through `0x140284EE0` |
| `0x140284A60(member, cb)` | `0x286BE0` | load one member's weapon into its slot; returns 1 if a load started, 0 if it was already there or there is none |
| `0x140285080` | `0x287200` | `.se` loaded: register it as sound bank `-3 - slot`, then load the `.wpn` (`0x140285270`) into the same buffer |
| `0x140285120` | `0x2872A0` | `.wpn` loaded: container → `0x1401D45F0` (a `jmp` to the model setup **`0x1401D5AD0`**); effects → `0x1401DDF20`; then the slot's callback |
| `0x140284BE0(actor)` / `0x140284D40(member)` | `0x286D60` / `0x286EC0` | create the **weapon object** for that member's slot; the second also stores it at `actor+0x14C` and runs the pending equip callback |
| `0x140284760(actor)` | `0x2868E0` | free the actor's weapon object |
| `0x1402845A0(item, cb)` | `0x286720` | equip an item (menu: `0x140302AF0` / `0x140302BA0`): free the object, write the save byte, reload, recreate |


All loads go through **`0x140289A80(name, buffer, callback, delay)`** (Steam `0x28BC10`).
It queues a request on KH1's loader task, and the read runs as a job (`0x14028A070` →
`0x14028A200`). The task (`0x14028A140`) calls `callback(size, request id, buffer)` **on the
game thread** once the size is known (≥ 0). A failed load never calls back. Requests are
serialised; the job queue at `0x14234B9A8` has only two entries, so never post jobs directly,
only through `0x140289A80`.

### 1.3 The weapon object [high for the fields named, med for the rest]

A pool of 9 objects of 0x60 bytes at **`0x14296BBE0`** (Steam `0x296B1E0`), with an in-use
bitmask at `0x14296BFD8`. `actor+0x14C` is a handle to one of them (`pose::ptr` resolves it).
Creating one also sets `actor+0x374 |= 0x10`.

| obj + | what |
|---|---|
| `0x00` | handle of the model container (slot `+0x18`) |
| `0x04` | handle of the **mesh model**: `0x1401D4600(container, obj+4, 0, 0)` copies the container's `+0x20` |
| `0x0C..0x0F` | RGBA bytes; draw-weapon (`0x1402A0CA0`) sets `ff ff ff 80` |
| `0x18` | pointer to the `.wpn`'s effect section (slot `+0x20`); the swing effect's source in `combat.rs` |
| `0x20` | pointer to the `btltbl` weapon record |
| `0x28` | trail object (from `combat.rs`) |
| `0x40` / `0x48` | cleared at creation |

The `.wpn` container is a `MENV`, not a `MOBJ`: `0x1401D4600` would write through its null
arguments for a `MOBJ`. The container has no instance and no skeleton of its own.

### 1.4 How it is attached to the hand: Sora's own skeleton [high]

The Keyblade has **no matrix of its own**. KH1 Sora's skeleton (293 bones) includes the weapon
rig: 280 is the grip under the right hand (101), and 281 to 292 are the blade and chain (stage
15). The weapon's mesh model has 13 bones and is skinned onto **the last 13 bones of the body's
matrices**: the queue below gives the weapon record a bone offset of
`body bone count − weapon bone count` = 293 − 13 = 280. KH1's motions animate 280..292, and
`pose.rs` writes them when KH2 has Sora. Swapping Keyblades only swaps the mesh. Donald's and
Goofy's weapons work the same way on their own skeletons.

In Sora's bind, bones 282..291 carry the skeleton flag `0x40000000` (bits 30-31 of the links
word = 1); 280, 281 and 292 don't. The queue only copies matrices for bone 0 and flagged bones,
so the mesh is skinned to 282..291.

---

## 2. How the weapon is drawn each frame [high]

1. **`0x14029F4A0(actor, hidden-mesh mask, p3, colour float[4], p5 float)`** (Steam `0x2A1630`)
   is the party actor's model draw. It is reached from four per-actor draw variants
   (`0x14029EE00`, `0x14029F360`, `0x14029F9E0`, `0x14029FBC0`), which pass `actor+0x38C` as
   the mask and `actor+0x394` as p3. It fills a 0x60-byte parameter block:

   | param + | what |
   |---|---|
   | `0x00` | flags (bit 0: use the remastered body table; 0x100 / 0x200 clip and scissor) |
   | `0x04` | `actor+0x420` (lit) |
   | `0x0C` | body RGBA (from the colour floats) |
   | `0x18` | **weapon mesh model** = `ptr(obj+4)`, only if the object, its model and actor flag `0x4000` are all there; else 0 |
   | `0x20` | body RGBA again; `0x24..0x27` = the weapon object's `+0xC..+0xF` |
   | `0x28` / `0x30` | per-character callback and its argument (the actor for Sora) |
   | `0x38` | **the hidden-mesh mask** (bit n hides mesh group n) |
   | `0x40` / `0x48` | lighting callbacks `0x14029BB70`, `0x14029BAA0` |
   | `0x50` / `0x58` | `0x1000070`, `0x800D0` |

   It then calls `0x1401D43B0` (a `jmp` to **`0x1401D1F10`**) with (instance =
   `ptr(actor+0x15C)`, view struct `0x14299C530`, params, actor).
2. **`0x1401D1F10(instance, view, params, actor)`** (Steam `0x1D4070`, prologue
   `40 55 53 56 57`) is the model queue:
   - It counts the body meshes not hidden by `+0x38`. With a weapon present it goes on even if
     that count is 0.
   - Body model record: 0x68 bytes at `0x1423FE658 + (half * 0x4E + n) * 0x68`, with n from
     `0x1423FE1C4`.
     - Its matrix block comes from the frame scratch allocator `0x14028B890`.
     - It **copies the instance's world matrices** (`instance+0x1B4`, the array `pose.rs`
       writes) for bone 0 and every flagged bone. In lit mode it copies two matrices per bone.
     - One 200-byte mesh record per visible mesh goes to `0x142402600 + (half * 0x1E0 + m) * 200`.
   - **Weapon record**: a second model record whose matrix block is the body's (same pointer),
     with the bone offset above, plus one mesh record per weapon mesh. Its colours come from
     params `+0x20..0x27`.
   - The matrices are copied **at queue time**. A second queue call made after changing the
     world array therefore gets its own copy, and the first call's copy stays as it was.
3. The world pass (task 160000, `0x1401EADD0`) draws the mesh records through
   `0x1401DC5A0`, the same as the body.

### 2.1 HD (remastered) Keyblades [high mechanism, med that every `.wpn` has them]

- A mesh model may carry a remastered table: `model+0x10 == 0x96969696` and `model+0x14` is a
  handle to a pointer to the table. The mesh count and entry accessors (`0x1401DB570`,
  `0x1401DB4F0`) use that table when it is there. The draw (`0x1401DC5A0`) takes the remastered
  branch (`0x1401DB690` / `0x1401DB730`) by the same per-mesh flag as Sora's body (see
  `kh1-tint.md`). Weapons go through exactly the same records and draw.
- The marker is set at **setup time**: `0x1401D5AD0` → `0x1401DA690` (Steam `0x1DC7F0`).
  1. It finds the file record whose buffer holds the model (`0x1400E2B10`; record list
     `0x1421ABB30`, record `+0x658` buffer, `+0x640` size).
  2. In the record's remastered-asset list (`+0x670`) it looks for the asset tagged with the
     model's offset in the file (asset `+0x54`) whose data (`+0x60`) starts with `0x4126`.
  3. It builds the table with `0x1401D9FA0` and writes the marker and handle.
- So **an HD Keyblade needs its `.wpn` loaded by KH1's own loader** (which creates the file
  record and its remastered assets, textures included) **and set up with `0x1401D5AD0` while
  that record exists**.
- Evidence that `.wpn` entries carry remastered assets: in `kh1_first.hed`,
  `xw_ex_5010.wpn` is 0x1A490 bytes but takes 0x50E10 in the PKG, and `xw_dh_5000.wpn` is
  0x1CC90 against 0xC22B0. The extra bytes are the appended assets, as with
  `xa_ex_0010.mdls` (0xA69C0 against 0xB2AF00). Their kinds (a `.cvbl` mesh plus DDS
  textures, as for Sora) are not yet read.

### 2.2 A second weapon object through KH1's functions? [high: no]

- `0x140284BE0` / `0x140284D40` always build the object from **the member's own slot**
  (found by member id), so they can't make an object for another item.
- There are three slots for three party members, and in a normal party all three are in use.
- The other loader callers (`0x1401AFB00`, an event command; the menu equip; the area-start
  load-all) all reload a member's own slot.
- KH1 has no code path that loads a weapon model by item id into a spare place, and no code
  that draws two weapons on one actor.
- The weapon object pool (9) and the draw would accept a hand-made object; only the loading
  has to be ours.

---

## 3. Design

### 3.1 Loading the Valor Keyblade [med]

Use KH1's own loader into **our own buffer**, outside every range KH1 loads into:

1. **When:** preload once per session, the first frame KH2 has Sora in the field, and again
   only when the configured Valor Keyblade changes.
   - This takes timing out of battle. The load is a small PKG read on KH1's loader, over a few
     frames, queued behind KH1's own loads (more during area changes).
   - Don't issue it while KH1's pause or event state is up; wait for the callback.
2. **What:** item id (ini, default for example Kingdom Key `0x51`, or "same as equipped") →
   model number `*(i16*)(item record + 6)` → stem = the `btltbl` record's first 16 bytes →
   `"<stem>.wpn"`. Skip the `.se`: the left blade needs no sound bank.
3. **Buffer:** `VirtualAlloc` 0x40000 (Sora's slot size; `.wpn` files are about 0x1A000 to
   0x1D000). Keep 0x60 bytes elsewhere, zeroed, for our weapon object.
4. **Load:** call `0x140289A80(name, buffer, our_callback, 0)` from the game thread.
5. **`our_callback(size: i32, id: u32, buffer: usize)`** (Win64), on the game thread:
   1. Check `size > 0`, and that `[buf+8] < [buf+0xC] <= size` and `[buf+4] <= [buf+8]`.
   2. `container = buf + [buf+8]`; check that its magic is `MENV`.
   3. Call `0x1401D45F0(container)` (Steam `0x1D6750`; or `0x1401D5AD0` directly). This
      relocates it and hooks up the HD table, because our file record exists now.
   4. Build our object:
      - `obj+0 = 0x14038B080(container)` (pointer → handle; registers the 32 MB region in the
        64-entry table at `0x142EE4730` if new)
      - `obj+4 = *(u32*)(container + 0x20)`
      - `obj+0xC = ff ff ff 80`
   5. Log the mesh model `m = ptr(obj+4)`: bone count `*(i16*)m` (expect 13 or fewer), and
      whether `m+0x10 == 0x96969696`. Mark it ready.
   6. Don't call `0x1401DDF20` (effects): the left blade gets no KH1 trail or swing effect.
6. **Never let KH1 release our record through the `.wpn` handler.**
   - KH1 drops file records only by address range: `0x1400E38C0(start, end, flag)` when a
     loader reuses a buffer, `0x1400E3830(addr)`, and a few fixed areas. None of them overlaps
     a buffer of ours.
   - Its release handler for `.wpn` (`0x1402851E0`, row pointer at **`0x1404DAF48`**, Steam
     `0x4DA268`, in writable `.data`) looks the buffer up among the three slots. For ours it
     finds none and then calls `0x1401D49D0` on garbage (slot −1 = bytes `0x14296BF28` /
     `0x14296BF30`).
   - To change or free our Keyblade:
     1. Set our record's `+0x64C` (the handler row) to `-1`.
     2. Call `0x1400E2DF0(record, 1)` (Steam `0xE2E00`). This marks its textures for deferred
        deletion and frees the record without calling the handler. Find the record with
        `0x1400E2B10(buffer, 0)`.
     3. Release the model with `0x1401D49D0(container)`, whose texture frees are also deferred.
   - For safety, also replace the row's pointer with a wrapper that ignores our buffer and
     calls `0x1402851E0` for everything else.
7. **Not ready yet** (first seconds, failed load): no left blade, or the equipped Keyblade's own
   mesh as a stand-in. A stand-in needs no load: use `obj+4` = the right object's `+4`.
   Time out a load that hasn't called back after a few seconds; KH1 never calls back on
   failure.

### 3.2 The left hand's matrices (`pose.rs`) [med]

KH2's Valor rig already has the left weapon: `rig.weapons[1]` (`W_EX010_BTLF_L`, attached to
the `sklt` left bone; `kh2_data::sora::load_rig`). Generalise `kh2_weapon` to a weapon index
and pose `weapons[1]` with the same motion. Then, each frame KH2 poses Sora in Valor:

- Calibrate on the right hand.
  - `C = inverse(KH2 right weapon root, mapped onto KH1's right hand) * pose[280]`, where
    "mapped onto KH1's hand" is `drawn hand * inverse(hand offset) * (inverse(KH2 hand) * KH2
    weapon root)`, with the translation scaled to KH1's size.
  - The hand offset is the fixed bind relation between KH2's and KH1's hand bones (both binds
    are T-poses facing +z with the same axes; the retarget already holds them).
  - KH2's two weapons are the same model, so C (KH2 weapon root → KH1 grip 280, including
    `kh1_wrist`, `flip_weapons` and `keyblade_teeth_up`) is the same for both hands.
- `G_L = drawn left hand (206) * inverse(left hand offset) * (inverse(KH2 left hand) * KH2
  left weapon root) * C`.
- Bones `280 + i` for i in 0..13: `L[i] = G_L * inverse(pose[280]) * pose[280 + i]` (the right
  chain's shape). A later step can pose the chain from `weapons[1]`'s chain bones with
  `secondary`.
- Store `to_world * L[i]` (KH1 world, as written to the world array) with the game frame
  number, for the draw hook.
- Fallback when no second KH2 weapon is posed: none (no left blade).

### 3.3 Drawing it: a second queue call [med-high]

Hook **`0x1401D1F10`** (Steam `0x1D4070`). The prologue `40 55 53 56 57` is push rbp, rbx,
rsi and rdi: 5 bytes, position independent.

```
hook(instance, view, params, actor):
    r = original(instance, view, params, actor)
    if !IN_SECOND && actor == Sora's actor && valor active && ready
       && *(u64*)(params+0x18) != 0          // KH1 drew his right Keyblade (weapon out)
       && left matrices are from this game frame:
        p2 = copy of the 0x60 bytes at params
        *(u32*)(p2+0x38) = 0xFFFFFFFF        // hide every body mesh group
        *(u64*)(p2+0x18) = ptr(obj+4)        // our mesh model
        p2[0x24..0x28] = right object's +0xC..+0xF  // KH1's fade / alpha follow
        w = ptr(*(u32*)(instance+0x1B4)) + 280*0x40
        save 13*0x40 bytes at w; write L; IN_SECOND = true
        original(instance, view, &p2, actor)
        IN_SECOND = false; restore the 13 matrices
    return r
```

- KH1 draws the right Keyblade first, copying the right-hand matrices. Our call copies the
  left-hand ones into its own block and queues only the weapon. The array is restored before
  anything else reads it.
- Everything else comes from KH1's own parameters: lighting callbacks, colours, the HD-body
  flag, and the lit path.
- Cost: one more model record (78 per half frame), 2 to 10 mesh records (480), and 18 or
  37 KB of frame scratch.
- The second call also re-runs the instance's texture-animation bookkeeping (counters reset per
  call) [med: harmless as read]. On the non-HD path (`params` bit 0 clear), it would see
  `actor+0x374`'s bit 31 after the first call cleared bit 30 [low impact].
- Alternative: hook `0x14029F4A0` (Steam `0x2A1630`, prologue `40 55 53 56 57`). Call it a
  second time with the mask `0xFFFFFFFF` and `actor+0x14C` briefly pointing at our object
  (that needs our object's handle). This is less direct, because the actor's draw also touches
  shadow and clip state.
- Skip the pause menu (`0x14232E9B4`), whose party models are drawn through the same
  functions.

### 3.4 Lifetime summary

| moment | action |
|---|---|
| first field frame with KH2 in control, or the ini choice changes | load our `.wpn` (3.1) |
| Valor starts | start drawing (3.3) as soon as it is ready |
| every frame in Valor | `pose.rs` stores L; the draw hook queues the left blade while KH1's right one is out |
| revert | stop drawing; keep the model loaded |
| choice changes | drop the old record and model (3.1 step 6), load the new one |

---

## 4. What the left blade does not get (yet)

- No KH1 trail, swing effect or sparkle: those come from the weapon object's effect section
  (`+0x18`) and trail object (`+0x28`), set up by `0x1401DDF20` and driven per actor. Hits are
  ours anyway: KH2's `weapons[1]` shapes.
- No `.se` sound bank: KH2's own sounds play for Valor.

## 5. Address list

| what | Epic | Steam RVA |
|---|---|---|
| Sora's actor pointer | `0x142D37C80` | `0x2D37280` |
| actor's weapon object handle | `actor+0x14C` | same |
| actor's model instance handle | `actor+0x15C` | same |
| instance's world matrices handle | `instance+0x1B4` | same |
| party actor model draw | `0x14029F4A0` | `0x2A1630` |
| model queue | `0x1401D1F10` (via `0x1401D43B0`) | `0x1D4070` (`0x1D6510`) |
| model record table / mesh record table | `0x1423FE658` / `0x142402600` | `0x23FDC58` / `0x2401C00` |
| frame scratch allocator | `0x14028B890` | `0x28DA20` |
| mesh draw | `0x1401DC5A0` | `0x1DE700` |
| HD hookup at setup | `0x1401DA690` | `0x1DC7F0` |
| model setup (relocate) | `0x1401D5AD0` (thunk `0x1401D45F0`) | `0x1D7C30` (`0x1D6750`) |
| object from container | `0x1401D4600` | `0x1D6760` |
| model release | `0x1401D49D0` | `0x1D6B30` |
| pointer → handle / handle → pointer | `0x14038B080` / `0x14038B0B0` | `0x38AD90` / `0x38ADC0` |
| file load (async) | `0x140289A80` | `0x28BC10` |
| file record list | `0x1421ABB30` | `0x21AB020` |
| find / drop a file record | `0x1400E2B10` / `0x1400E2DF0` | `0xE2B20` / `0xE2E00` |
| `.wpn` release handler / its row pointer | `0x1402851E0` / `0x1404DAF48` | `0x287360` / `0x4DA268` |
| weapon slots / object pool / bitmask | `0x14296BF40` / `0x14296BBE0` / `0x14296BFD8` | `0x296B540` / `0x296B1E0` / `0x296B5D8` |
| slot buffers (static) | `0x14286BBE0` | `0x286B1E0` |
| `btltbl.bin` / item table pointer | `0x142D23740` / `0x142D23738` | `0x2D22D40` / `0x2D22D38` |
| per-member weapon table offsets / `.wpn` / `.se` suffix pointers | `0x140528D60` / `0x140528D88` / `0x140528D90` | `0x528060` / `0x528088` / `0x528090` |
| equip item / load member weapon / create object | `0x1402845A0` / `0x140284A60` / `0x140284D40` | `0x286720` / `0x286BE0` / `0x286EC0` |
| draw weapon (flag 0x4000) / put away | `0x1402A0CA0` / `0x1402A10F0` | `0x2A2E30` / `0x2A3280` |

## 6. Runtime checks, in order

1. Log Sora's weapon object (`ptr(actor+0x14C)`) and its mesh model (`ptr(obj+4)`): bone count
   (expect 13), and `+0x10 == 0x96969696` with `+0x14` set (KH1's own Keyblade is HD).
2. Load `xw_ex_5020.wpn` into our buffer and check the same on ours (HD hooked up).
3. First draw test without `pose.rs` changes: L = the right matrices moved 30 units along
   Sora's +x. Expect a second Keyblade beside the first.
4. Change areas with it loaded, and check that it still draws (that no global texture purge
   takes our record's textures).
5. Swap the Valor Keyblade twice, and check that nothing crashes (the record drop of 3.1
   step 6).

---

## 7. What is implemented (`crates/kh1-hook/src/win/leftblade.rs`)

The ini switch is `valor_left_blade` (default on). With it off, nothing is installed: no hook
and no patch, and `show` / `hide` / `note_pose` / `kh2_left` do nothing.

- **Install.** Two steps; if either one's bytes don't match, nothing is installed and the patch
  is put back.
  1. The `.wpn` release handler pointer at `0x1404DAF48` (Steam `0x4DA268`) is checked to hold
     `base + 0x2851E0` and replaced with `wpn_release`. That wrapper calls KH1's handler for
     every record except those on our buffers.
  2. `0x1401D1F10` is hooked, with its 5-byte prologue checked.
- **Load** (`show`, from `valor::started`, on the game thread inside KH2's step).
  - The item's file name comes from `btltbl.bin` (item record `+2 == 2`, row `+6`, Sora's
    table). It is loaded with `0x140289A80` into a fresh `VirtualAlloc` buffer of 0x80000.
  - The callback `loaded(size, id, buffer)` checks:
    - that it is the buffer we wait for;
    - the size;
    - the header offsets;
    - the `MENV` magic.

    Then it calls `0x1401D45F0` (setup) and reads the mesh model from `container+0x20`
    (bones 1..13).
  - It logs whether the remastered table was hooked up (`+0x10 == 0x96969696`).
  - After that the model stays loaded. A different Keyblade at a later Valor is loaded into a
    new buffer, and the old one is released:
    1. its record's row is set to −1;
    2. `0x1401D49D0(container)`;
    3. `0x1400E2DF0(record, 1)`.

    Buffers are never freed, because a late answer from the loader could still land in one.
  - A load with no answer after 15 s is given up for the session. So is one whose model fails a
    check.
- **Pose.** `takeover` hands over the form rig's second weapon each step (`kh2_left`, one line
  after the `POSE` write). `pose`, right after it writes Sora's bones, calls `note_pose` with
  KH1's pose and bind, KH2's body and right Keyblade, and the placement. The grip is the
  calibration of section 3.2:
  - each KH2 hand bone is put on KH1's hand by the bind poses (KH2 translations times
    `hip_ratio`);
  - the turn from KH2's right weapon root to KH1's grip 280 is reused for the left;
  - the chain keeps the right one's shape.

  A grip more than 40 units from the left hand is not drawn that frame.
- **Draw.** In the queue hook, after KH1's own call, all of these must hold:
  - Valor is on, and the model is ready;
  - the actor is Sora and the instance is Sora's;
  - `params+0x18 != 0` (his Keyblade is out);
  - the left matrices are at most 2 game frames old.

  Then bones 280..292 of `instance+0x1B4` are saved and overwritten, and the queue is called
  again on a copy of KH1's 0x60-byte params with `+0x38 = 0xFFFFFFFF` and `+0x18 = our mesh`.
  The bones are put back.
- **Logs** (prefix `left Keyblade:`, each a few times): off or installed, load requested, load
  answered, model set up (HD or not), left grip posed, drawn in the left hand, released, and
  KH1 releasing our record.

Measured from the files while writing this: KH2's weapon motions move the Keyblade's root
against its hand bone a lot (up to 18 units and 3.1 rad in `W_EX010_BTLF_L`; the grip
changes). So the left grip has to come from KH2's posed left weapon. A fixed offset from the
hand would be wrong.

### Runtime checks for the user's log

1. Startup: `hooked model queue (left Keyblade)`, and no `unexpected bytes` lines for the patch
   or the hook.
2. On the first Valor:
   - `load of xw_ex_5xxx.wpn ... requested`;
   - within a second, `load answered`;
   - `model set up: ... 13 bones ... remastered (HD) table hooked up`.

   If it says "no remastered table", the HD hookup did not find our file record (section 2.1).
3. `left grip posed, N units from the left hand` (expect a few units) and `drawn in the left
   hand`. In the game, look at whether the Keyblade sits in the palm with the blade forward,
   and whether its teeth face the right way.
4. Revert and Valor again: no new load, drawn at once. Change Valor's Keyblade and enter Valor:
   `released`, then a new load. An area change while loaded: still drawn.
5. If anything crashes, set `valor_left_blade = off` and send the log. The crash note names the
   last stage reached.

---

## 8. The left Keyblade's trail (KH1's own)

### How KH1 makes its Keyblade trail [high, from the code]

- **`0x1402C0160(actor, id)`** (Steam `0x2C2310`) is called by a move's row (`0x1402A17F0`, the
  row's `+2`) and by `combat` for KH2's swings. It ends the trail the weapon object has at
  `+0x28` (`0x1401DE890`, a `jmp` to `0x1401EA310`). Then it stores the weapon object's effect
  data (`+0x18`) at `+0x30` and `id | 0x40000000` at `+0x38`.
- **`0x1402BFD70(actor)`**, each frame (from the actor step `0x140295E50`):
  - **No trail:**
    - With `+0x38 >= 0`, the 0x40000000 bit is first cleared, one frame.
    - Then, while the actor's `+0x1CC` bit 0x800000 is set and the actor is alive
      (`0x14028FE60`: `+0x374` bit 0 set, bit 1 clear), it makes the trail with
      **`0x1402AC100(actor, effect data, id)`** (Steam `0x2AE290`). It clears the effect's
      `+0x14` bit 0x20, stores the effect at `+0x28` and sets `+0x38 = -1`.
  - **A trail running** (effect `+0x14 & 3 == 1`):
    - while the actor's flags have `0xC001` and the trail bit is set, it is left alone;
    - when the bit goes, the byte at `ptr(effect+0x10)` is set to 1 (the trail ends and fades);
    - otherwise it is ended at once.

    When it isn't running any more, `+0x28` is cleared.
- **The effect** comes from a pool of 0x3C-byte entries (`0x142D682B0`); `ptr(effect+0x18)`'s
  first word is the owner. `0x1402AC100` → `0x1402AC660` / `0x1402AC6F0` registers a follower
  task (`0x1402AC780`). Each frame it places the effect on a bone of the owner:
  - `0x14029C870(actor, bone, out)`: the owner's model instance (`actor+0x15C`), then its world
    matrices (`instance+0x1B4`, `0x1401D40C0`). The bone either comes straight from the effect
    data (flag 0x2000) or is mapped through the instance's skeleton (`0x14029D920`).
  - It also reads the owner's `+0x88`, `+0x160`/`+0x164` (the trail ends when the motion slot
    changes), `+0x370..` (colour) and `+0x374`.
  - So **the trail follows whatever world matrices the owner's instance has when the follower
    runs.**

### What is implemented (`leftblade.rs`, same `valor_left_blade` switch)

- **The swing's trail id:** `0x1402C0160` is hooked (prologue `48 89 5C 24 08`, checked). Its
  id is noted whenever it's asked for Sora. That is the id `combat` asks for the right trail of
  each KH2 swing, so the left trail has the same look. It is made from the right Keyblade's
  effect data (the weapon object's `+0x18`).
- **The owner:** a block of ours (0x8000 bytes) holds a copy of Sora's actor (0x4B0 bytes) and
  of his model instance (0x200).
  - The actor copy's `+0x15C` is a handle to the instance copy. The instance copy's `+0x1B4` is
    a handle to our 293 matrices: his world matrices, with the left hand's weapon bones in
    280..292.
  - It is refreshed in `note_pose`, every time the left bones are made. Its `+0x1CC` trail bit
    follows the left window.
  - Handles are made with KH1's `0x14038B080` and checked to resolve back.
- **When:**
  - The left weapon's own trail ranges (trigger 12) are read at start, in the background, from
    the KH2 install's `wmst`-named mset for each two-Keyblade form's left hand (`W_EX010_BTLF_L`,
    `_TRIF_L`, `_ULTF_L`; Valor's alone until Master Form, so Final's trail ran on Valor's
    frames), and the current form's are used (`docs/drive-model.md`, "The second Keyblade's
    trail"). The trail is made only in a move (not standing, walking or running).
  - The owner is filled right after the left bones are posed (`note_pose`, Sora's matrices
    just written by the pose pass), and again at the left Keyblade's draw: filled only at the
    draw, it could be a frame behind KH1's own right trail.
  - `takeover` passes the clip's motion, frame and its trail flag (the same `kh2_left` line,
    now with three more arguments). If the mset can't be read, KH2's trail window of either
    hand is used.
  - As the window opens, a trail is made with `0x1402AC100(owner, effect data, id)` and bit 0x20
    cleared, as KH1 does. When the window closes, its end byte is set each step until it stops
    running.
  - Out of Valor it is ended at once (`0x1401DE890`). At most 4 run at once, fading ones
    included.
  - A tracked effect counts as ours only while it is running and its owner word is our block
    (the pool reuses entries).
- **Logs** (`left Keyblade: trail ...`, a few each):
  - the windows read;
  - the owner made;
  - each trail made (motion, frame, id) or not made, and why: no id yet, no effect data, owner
    not ready, Sora not in the field, too many;
  - each trail ended.

### Risks and runtime checks

1. A copy of the actor as an effect's owner: if anything in the effect code writes through
   the owner, or keeps it past Valor, it writes into our block (never freed), not into Sora.
2. The trail's motion-slot check reads the copy's `+0x164`, which is Sora's. A left trail ends
   when Sora's KH1 motion slot changes, as the right one does.
3. Check in the log: `trail windows of W_EX010_BTLF_L (moveset 2) for N motions` (and
   `_TRIF_L`, `_ULTF_L`) at start, then for each
   Valor swing `trail ... made at motion M frame F`, then `ended`. In the game, check that the
   ribbon follows the left blade, not the right one.

## 9. The left trail's length, and the forms' weapon-anchored effects

### The blade's range [high, from the code]

- Sora's weapon object (actor `+0x14C`, one of nine 0x60-byte objects at `0x14296BBE0`, built by
  `0x140284D40`): `+0x00` its model instance, `+0x18` the `.wpn`'s effect data, `+0x20` a pointer
  to the weapon's row in `btltbl.bin`'s weapon table (0x58 bytes a row, the item record's `+6`
  row number; `+0` its `.wpn` stem, `+0x24` / `+0x28` the blade's end / start along bone 0x30,
  `+0x30` Strength, `+0x38` MP).
- A trail point along bone 0x30 (`0x14029C990`) maps `t` into that range, so the trail spans the
  owner's weapon's blade. The left trail's owner (a copy of Sora's actor) copied his weapon
  handle, so it spanned the right Keyblade's blade (the user: "not aligned with the keyblades
  length"). It now has its own weapon object (a copy of his, at the owner block's `+0x700`, its
  handle from `0x14038B080`) with `+0x20` the left Keyblade's row and `+0x18` its effect data.
  Both blades' ranges are logged at each left trail made.

### KH2's weapon anchors [high, KH2's `0x1403B43D0`]

- An effect element's bone code 0x4003 / 0x4004 is the tip of the player's weapon 0 / 1
  (`(code - 1) & 1`), from the weapon model's type-0xF collision point and an offset along its
  x; 0x4005 / 0x4006 the weapon point itself; 0x4007 / 0x4008 half the offset (× 0.5,
  `0x14062399C`).
- Ours (`kh2fx::form_bone`, Valor's and Final's packages only): the point on that Keyblade from
  KH2's weapon bones of the step (the left one's from `leftblade::kh2_left_root`), the tip at
  W_EX010's collision point taken from the weapon skeleton's bind (78 along its root), turned
  with Sora's facing as the object root (the user: the grounded finisher's effect "should spawn
  in front of the keyblades and its direction its sent should be the same as where sora is
  facing"). Every form element's anchor, flags and offset are logged once: if Valor's finisher
  elements (154's 16, 155's 13..15) turn out to use another anchor, the log says which.
- Final Form's are KH2's own (`docs/kh2-effects.md` §12.3): the weapon point is the Keyblade's
  blade bone (bone 2, which carries its type-0xF collision point), turn included, taken where
  KH1's floated Keyblade is drawn (`leftblade::final_blade`).

## 10. The Dream Sword in the left hand (Valor's default)

Valor's second weapon is the Dream Sword (item `0x52`, `xw_dh_5000.wpn`) until a Keyblade is
picked (`docs/kh1-equipment.md`, "Valor's default: the Dream Sword"). It goes through the same
path as a Keyblade: the file name from its weapon row (row 2), KH1's loader into our buffer, the
model and HD setup, the second queue call. What differs is the hold:

- **The right hand's rule** (`pose::dream_hold`, s226 / s250): the dream weapons and the wooden
  sword (`0x52..=0x55`) are not turned teeth-up and get no keychain swing; every weapon bone is
  held rigidly where KH1's own hold of that weapon puts it from the hand (the grip from the hand,
  each bone from the grip), noted from KH1's pose on frames KH1 posed him. `Retarget::set_grip`
  (s50, KH1's live grip when it is turned more than 1 rad from the battle idle's) is no longer
  called.
- **The Dream Shield stretched across the field** (the user: "having the dream shield
  equipped leads to some pretty bad stretching issues"; video in the Dive, a thin grey bar
  several body lengths long out of his hand in KH2's swings) [high for the cause, from the log]:
  - The shield is the one `flip_weapons` item (`0x53` by default), held by KH1's wrist
    (`pose::kh1_wrist`): the hand takes KH1's forearm-to-hand matrix from KH1's own pose. On
    the frames KH1 blends between its motions its hand and grip can lie thousands of units off
    (every log has such frames: KH1's 280 at about twice Sora's position, 283 with det 0.5,
    0, -0.8). With a Keyblade nothing reads KH1's hand, so they never showed. With the shield,
    that "wrist" moved the drawn hand there and kept it until KH1's next frame: in the s328 log,
    45 of 176 status lines draw 280 (and the hand, 4 from it) 400..120000 units from Sora. In
    the Keyblade sessions before it, none did. The forearm's and the shield's mesh stretch
    out to that hand.
  - Fix: only KH1's turn of the wrist is used, never its translation (the hand stays 11 units
    from the forearm, KH1's bone length in every motion of `xa_ex_0010.mset`), and only from a
    frame whose hand is rigid and a bone's length (±25%) from its forearm. Otherwise the last
    good wrist is kept (logged, the first 10 times).
  - The hold: `dream_hold`'s nearness test now counts only the grip and the skinned bones
    282..291. The shield's hold was refused in every Dive room before room 0x4, "a bone 526
    / 17605 from the grip" (the grip's distance from the world origin), so most likely 281 or
    292, which are never drawn. Those two now always take the bind's place. Until a hold is
    noted, a dream weapon's bones after the grip stay rigid as in Sora's bind (the Keyblade's
    chain swing used to bend the mesh). The log names the weapon, its model number, the hold
    noted (grip offset, turn, scale, reach) or the bone refused.
- **The left hand** (`leftblade::dream_left`, used by `note_pose` when the left item is a dream
  weapon):
  - with KH1's hold noted this session (Sora held the Dream Sword in the Dive): the turn from
    the right Keyblade's grip as drawn to where that hold would put the Dream Sword in the right
    hand (`grip⁻¹ · right hand · hold[0]`) is put on the left grip, and the bones follow the hold;
  - else (the usual case): the Keyblade's left grip, every bone after it as in KH1's bind (the
    mesh undeformed: the Keyblade's keychain swing on bones 283.. would bend it).
- Why the Keyblade's grip is a fair fallback [low-med]: the weapon rows give the trail's blade
  range along the same bone with the same sign for both (`+0x24` / `+0x28`: −4.3 / 0 against
  the Kingdom Key's −3.7 / 0; `+0x20` is 40.0 in both), which suggests the Dream Sword's blade
  lies along its bones the way a Keyblade's does. KH1's motion table
  (`xa_ex_0010.mset`, 118 slots) has no motion with a grip turned more than 1 rad that keeps it in
  the hand (slots 84..90 turn it 1.75..3.1 rad but 20..180 units from the hand: weapon-out
  motions), so the Dive's hold is in another moveset, not read here. Whether its face (the roll
  about the blade) matches KH1's Dive hold is **not checked**: the log names the source used
  ("left Keyblade: item 0x52 isn't a Keyblade: held rigidly, by ...").
- **Stats**: its row's Strength (+1) and MP (0) count in Valor (`valorstats::bonus`).
- **Trail**: its own `.wpn` effect data (the file's second section) and its own row's blade range,
  as any left Keyblade (s300's left-blade windows, owner refreshed after the pose). The owner's
  matrices are moved by the turn from the right grip to the left one as drawn, so with KH1's hold
  they follow the Dream Sword's grip. KH1 makes a trail from the weapon object's effect data only
  (`0x1402C0160` → `0x1402AC100`): a weapon whose data has no trail of that id gets none from KH1;
  ours then takes the right Keyblade's look (logged "the right Keyblade's look").
