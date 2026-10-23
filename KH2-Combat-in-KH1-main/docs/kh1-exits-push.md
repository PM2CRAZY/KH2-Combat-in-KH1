# KH1: map-exit triggers and crate pushing, and how to keep KH2's lunges out of them

Research notes (Ghidra headless + disassembly, Epic KH1 Final Mix PC, image base `0x140000000`,
addresses absolute). Read statically, not yet watched in play. Confidence: **[high]** read
plainly from the code, **[med]** inferred from how the code uses it, **[low]** a guess.
"Handle" = KH1's 32-bit compressed pointer (`0x14038B0B0` decodes, `0x14038B080` encodes; the
null pointer encodes to 0). World is y-down.

The two user bugs:

1. A KH2 combo's lunge near a room entrance takes Sora into the next room.
2. A KH2 combo's lunge into a crate starts KH1's push.

Both come from the mod moving Sora with KH1's own mover (`0x140297710`) during KH2 attacks.
That mover is what tells the map and the objects that Sora touched them.

## 1. Map-exit triggers: collision faces that carry an event number

### How KH1 fires them [high]

- Each map collision face (0x14-byte records in the table at `[0x14234F020]`) has an **event
  byte at +0xA** (0 = none). Face flag `0x8000` (record `+0x10`) marks it as an event that also
  fires for actors other than the player.
- The background collision query `0x140193030` (the query block at actor `+0x2A0`) sets
  **`+0x2D8` (u16) = that event byte** (cleared to 0 at the start of every query) in two cases:
  - the swept body touches a face (a wall) with a non-zero event;
  - the body stands on a floor face with one. This happens only when the query is not in the
    air or on-object mode (query flag 1 clear: ground movement states, nothing under him at
    `+0x398`).
  Collision flag 4 (`+0x2D0`) is set too when the face has flag `0x8000`.
- The mover's sub-step loop `0x140299350` (called by `0x140297710` for actors with collision)
  adds each sub-step to the position (`+0x10`) **first**. It then reads `+0x2D8`. When that is
  non-zero, differs from the event already queued in this call, and the actor is Sora (or
  collision flag 4 is set), it queues **`(kind -2, actor, event)`** with `0x14028BBF0`. The
  only kind -2 caller is the call at **`0x1402994F6`**. The loop also stores the last
  sub-step's event in **actor `+0x448`** (0 if none). Nothing else in the game code writes
  `+0x448` outside actor setup.
- `0x14028BBF0 (kind ecx, actor rdx, param r8)` puts the entry in a 32-slot queue at
  `0x142BA2A80` (0x18 bytes each: `+0` kind (-1 = free), `+8` actor, `+0x10` param).
  `0x14028C3C0` drains the queue once a frame, as a task registered with priority 47000
  (after the actors). Kind -2 goes to `0x14019FCE0 (actor id +4, event)`, the others to the
  object-script dispatcher `0x14019F8C0`.
- `0x14019FCE0` does nothing while any of three global flags is set: `0x14237CEE0` bit 1,
  `0x142398860` bit 8, `0x142398864` bit 7. Otherwise:
  - event 1..99: if it differs from the area event already running (`0x1423987F4`), it starts
    the area's event script, entry *event* (`0x1401A1750` on the script at `0x14234F188`), and
    sets `0x142398864 |= 0x40`;
  - event 100 and up: when no secondary event is running (`0x1423987F8 == 0`), it starts entry
    *event - 100* of the second script (`0x1401A2630`, script `0x14232E948`).
- That entry is what plays the exit: the fade, the walk-out and the map change. [med] The
  exact opcodes were not traced, but there is no other walk-in trigger. Kinds 0 / 1 / 3 / 4 /
  5 go to object scripts (action-button commands, touching an object, chests, lock-on). Doors
  are action-button commands to an object's script. No position-in-volume check against Sora
  was found.

### Arming: level-triggered [high]

The event is queued on **every mover call** that touches the face. On a floor face, that
means every call while he stands on it, even a zero move. The only de-duplication is that the
same event is not restarted while it is the running area event, plus the three global
"events off" flags. So:

- Once the lunge has touched the face, the exit starts that frame.
- If the event is only dropped, a lunge that leaves him **on** an exit floor face (or pressed
  into an exit wall) still fires on the first ordinary mover call after the combo. That
  includes KH2's idle frames, because the mod calls the mover every frame.
- A lunge that never touches the face leaves nothing armed. Walking into the face later fires
  it as usual.

### Gate

**Redirect the call at `0x1402994F6`** (`E8 F5 26 FF FF`, calls `0x14028BBF0`; use
`hook::redirect_call`).

- Arguments: `ecx` = -2, `rdx` = actor, `r8` = event (u16 zero-extended). Return: void.
- To skip, return without calling `0x14028BBF0`. The mover does not look at the result.

Use it together with a position rollback in `takeover`, so the lunge never stays inside the
trigger:

1. Set a flag (`LUNGE_MOVE`) **only around takeover's own `mv(...)` calls** on frames where
   KH2's state is `Attack` (and any other KH2 move with a forward root lunge you want covered).
   Clear it right after. KH1's own physics is then never affected.
2. Note `allowed = rd::<u32>(sora + 0x448)` on the last frame **before** the attack began: the
   event face he was already touching, if any (usually 0). This keeps a harmless floor event
   he already stands on from blocking every lunge.
3. In the redirected call: if `LUNGE_MOVE`, the actor is Sora and `event != allowed`, store
   the event in `DROPPED` and return. Otherwise call `0x14028BBF0(kind, actor, event)`.
4. After `mv` returns: if `DROPPED` is set, put Sora's position (`+0x10`) back to where it was
   before this `mv`. He stops at the trigger's edge, losing at most this frame's lunge. If you
   want the floor fields (`+0x320`, `+0x3D4`, `+0x2D0`) to match the restored spot, you can
   also run a zero-length `mv` with `LUNGE_MOVE` still set.

Side effects:

- No face event can start from a KH2 lunge. That includes non-exit faces: an event wall that
  starts a scene, or a tutorial trigger. Walking into them still works.
- KH1's own attacks, run when KH1 has Sora, are untouched.
- Steam: `0x14029B686` (same bytes) calling `0x14028DD80`. `gen.py`'s resolver maps both
  (`code`).

## 2. Pushing crates: the crate decides, Sora goes into movement state 0x1B

### Who starts the push [high]

- It is not Sora's ground state. It is the pushable object's own think. Objects whose
  objentry type byte (`+0x54` of the entry at actor `+0x130`) is **9** get the think
  **`0x1402C0ED0`** (chosen in `0x1402957F0`). In the object's state 0 that think calls the
  push check **`0x1402C1450 (object rcx)`**, from **`0x1402C10E4`** (its only caller).
  - Not checked: whether every pushable in the game (Wonderland's bed and rock, for example)
    is a type-9 object, or whether some are pushed by their own scripts. A scripted push
    would show up as "KH1's script moves him" in the log. [med]
  - Hit records make the same object slide along the hit direction (`0x1402C5150`, speed 15)
    for its first 6 hits, and break it on the 7th. That is why swings move crates.
- The "who is touching me" field is **object `+8`** (a handle):
  - Sora's mover writes it. The wall solver `0x140298A70` calls `0x140299600 (mover,
    touched)` for every object collision in the touched mask `+0x90`, and that function links
    `touched+8 = mover` (and the mover's `+8`, by object-id priority).
  - The body-vs-body pass `0x1402990D0` does the same.
  - Every actor's task (`0x140295E50`) clears its own `+8` right after its think.
  - So when the crate thinks, `+8` means "Sora's mover touched me since my last think".
- Push check `0x1402C1450` (the crate's work block is the handle at `+0x144`: `+0` hit count,
  `+4` push timer, `+8` time spent in the object's state 2, `+0xC` a second contact timer).
  The timer counts while **all** of these hold:
  - `+8` is an actor with object id 0 (Sora: entry `+0x4C` = 0);
  - he carries / pushes nothing (`+0x98` null);
  - his movement state `+0x70` is **0**;
  - his actor flag bit `0x4` is clear (KH1's set-motion `0x14029D7F0` sets it for requests
    with bit `0x1000000`, i.e. KH1's own action motions such as its swings, so KH1's attacks
    can never start a push);
  - the height difference (crate origin minus Sora's position) is at most 10;
  - he is lined up with one face (the offset along the other horizontal axis is under 25) and
    faces into it: facing (`+0x30` x or z) beyond ±0.94, i.e. within about 20 degrees, on the
    side where the crate is.

  Each such frame adds the frame step (`0x1423405DC`: 1 at 60 fps, 2 at 30) to `+4`. Any frame
  that fails resets it to 0. **When it passes 12 (0.2 s of steady contact)**:
  - `0x140295AD0` links crate and Sora both ways through `+0x98`;
  - Sora's turn target `+0x150` is set to that axis;
  - **Sora's movement state is set to 0x1B (27)** (`0x1402AF7D0`, which also sets actor flag
    `0x400` and the previous state `+0x378`).
- Then Sora's own think plays the motion. `0x1402A2210` (start of the player think
  `0x1402A1170`) sees flag `0x400` with state 0x1B. It asks for **motion request 0xBC** (push
  start, as a forced action `0x10000BC`), then queues **0xBD** (the push loop).
- State 0x1B handlers (registered by `0x1402C0E70`):
  - update `0x1402C1240` adds Sora's move to the crate's per-frame push `+0x120`;
  - displacement `0x1402C12D0` is zero (Sora moves only by the crate carrying him);
  - transition `0x1402C1170` goes to 2 when airborne, and unlinks if the crate didn't follow;
  - input step `0x1402C12E0` (run from the control step through `0x1402A59A0`) keeps pushing
    while the stick points within about 40 degrees (dot 0.766) of his facing. Otherwise it
    plays **0xBE** (letting go), queues idle and returns to state 0.

### What the mod should recognise [high for the state, med for the slot]

- **KH1 movement state `+0x70 == 0x1B` (27)**, with `+0x98` pointing at the crate. This is the
  reliable signature. `kh1_mode(27)` is false today. The release happens one think later through
  the slot check ("KH1's own action, slot N, KH1 state 27").
- Motion requests 0xBC / 0xBD / 0xBE. The slots they play are Sora's motion table entries:
  `slot = u32 at [handle +0x1D8 decoded] + request*4, & 0xFFF` (`0x14029E050`; entry type bits
  12..14, `0xFFF` = none). They were not resolved offline. Read them once at runtime, or take
  them from the log.

### Gate

**Redirect the call at `0x1402C10E4`** (`call 0x1402C1450`; use `hook::redirect_call`). The
alternative is a hook on the entry **`0x1402C1450`**, prologue
`48 89 74 24 18 57 48 83 EC 70` (instruction ends at 5, 6, 10; the 10 bytes have no RIP-relative
operand; the instruction after is one).

- Calling convention: one argument, the crate actor (`rcx`). Void.
- In ours: if a persistent flag says KH2 has Sora in an attack, and `pose::ptr(rd::<u32>(crate
  + 8))` is Sora, write `u32 0` to `crate + 8`. Then call the original.
  - The flag must be persistent (set on every takeover frame while KH2's state is `Attack`,
    cleared otherwise and on `release`), because the crate thinks in its own task, not inside
    Sora's step.
- The check then sees "nobody touching" and resets the push timer to 0. So contact during a
  combo never counts, and a push after the combo needs the full 12 frames of fresh, facing
  contact.
- Do not just return early: that would keep the timer's count from before the attack.
- Side effects:
  - the second contact timer (`+0xC`, which breaks the crate after 120 frames of contact
    while the toucher is in one class of movement states, `0x1402AF980`) also resets during
    attacks;
  - the crate still takes KH2's hits (slides, breaks), and the body push between the two is
    unchanged;
  - `+8` is cleared by the crate's own task after its think anyway, so nothing else reads the
    zeroed value.
- Steam: call site `0x1402C3294`, function `0x1402C3600` (same prologue). The resolver maps
  both (`code`).

Native alternative, not recommended: keep Sora's actor flag `0x4` set during KH2 attacks, as
KH1's own swing does. It also changes KH1's walk-motion choice (`0x1402C9FF0`), the hit pass
(`0x14029A8D0` / `0x140299A40`) and other places that test the bit.

## Probe to confirm

- In the `0x1402994F6` redirect, log every Sora event (event, world / room `0x142340E5C` /
  `0x142340EC4`, position, `+0x2D0`) for a while. Walking into an exit should log a number just
  before the room changes; a combo lunge at the same spot should log "dropped" and stay in the
  room.
- In the push-check redirect, log the crate's work block `+4` each frame and Sora's `+0x70`.
  Walking into a crate should count to 12 and then give state 27. A combo into it should stay
  at 0.
- Log the slots behind requests 0xBC / 0xBD / 0xBE from Sora's motion table once.
