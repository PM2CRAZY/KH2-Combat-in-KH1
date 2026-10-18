# Magic model

Spells are not hard-coded anywhere in this project. Each spell file on your install
(`magic/FIRE_1.mag` … `magic/REFLECT_3.mag`) carries two things:
- a **script** (BDX bytecode)
- an **effect package** (PAX)

The script decides everything the spell does: where its attacks are and how big, when they
start and stop, how projectiles move and home in, and when to heal. The code is split in
two:
- `kh2-bdx` runs the script. It is a virtual machine, written from notes on the game's
  interpreter.
- `kh2-magic` implements the game functions ("traps") the scripts call.

The playground answers the scripts' questions about the arena, Sora and the Shadows, and
applies their attacks with the normal hit rules.

Confidence labels: **Checked (exe)** means read from the executable's code. **Data** means
read from your files. **Ours** means our own choice where the game's behaviour is not known.

## Casting (Checked, exe)

1. **Picking a spell.** The command menu picks a MAGC row: element, level, cast motions
   and the spell file. 03system `cmd` gives the MP cost.
2. **Choosing the motion.**
   - the air motion when airborne
   - the combo-finisher motion when the next swing would be the combo finisher
   - otherwise the ground motion
3. **The sub.** The game computes `sub = level × 2 − 2`, plus 1 for the finisher motion.
   Wisdom, Master and Final Form add 10, 20 and 30 (`drive-model.md`). The sub picks two things:
   - the spell's `magi` record (03system `pref`)
   - its ATKP rows
4. **Targeting.** For Fire, Blizzard, Thunder and Magnet, the caster's target is picked
   first if there isn't one. Cure and Reflect don't pick one.
5. **Trigger 20** starts the script with four arguments: the caster, the spell's resources,
   the sub and a pointer to its `magi` record.
6. **Trigger 21.** When the cast motion reaches its magic-cast event, trigger 21 restarts
   the same script at its release code.
   - The script's trigger-20 code typically waits while the cast motion plays and stops when
     the motion changes, so an interrupted cast does nothing.
   - **Ours:** if the release comes on the motion's first frame, the trigger-20 code runs
     once first. Reflect releases on frame 0.
7. **Every frame,** each running script and its threads run once, after the player.

The game's PC build runs scripts with a frame time of 1. A few timings written for 30 fps
are patched while the frame time is below 2. Fire is one example: its 44-frame duration
becomes 42 when popped. `kh2-bdx` applies the same patches.

## The script machine (`kh2-bdx`, Checked, exe)

**The file:**
- a 16-byte name
- work, stack and temp sizes
- a table of (trigger key, entry) pairs

Code addresses count 16-bit words from byte 16.

**Memory:**
- Each thread has its own stack and its own evaluation stack. Evaluation-stack cells are a
  value plus a type tag (int, float, address).
- Threads started from a script share its work memory.
- `kh2-bdx` keeps everything in one flat byte arena (program, stacks, work, and whatever
  the traps hand out), so pointer arithmetic in scripts behaves as in the game.

**Instructions.** The names follow OpenKH's Bdxio tool.
- **Push:**
  - an immediate int or float
  - an address: `sp+imm`, `wp+imm`, `*(sp)+imm`, or code
  - the value at one of those addresses
- **Pop** to an address. A pop into code is discarded.
- **Memory:** `memcpy` to an address; a store or copy through a popped address;
  `push *(pop + imm)`.
- **Arithmetic:**
  - unary int/float ops (convert, negate, not, abs, compare with zero)
  - binary int/float ops (add, sub, mul, div — division by zero leaves the left operand —
    mod, and, or, xor, shifts, logical and/or)
- **Branches** are relative to the next instruction.
- **Calls:** `jal` and `jal32` push a 2-word frame header (frame size and return address)
  above the caller's frame. `ret` pops it. Returning to address 0 ends the thread.
- **Other:** `halt` (yield until next frame), `exit`, `drop`, `dup`.
- **Trig:** `sin` and `cos` wrap into [−π, π) first. There are also degrees→radians and
  radians→degrees.
- **`syscall`** calls a trap. Its argument count and whether it returns come from the trap
  table. The 828 entries of OpenKH's list match the executable's tables exactly.

**Starting code** at a trigger (or at an address, for threads):
- sets the pc
- pushes four zero words on the stack, so the code's last `ret` ends the thread
- pushes the arguments

Threads run after their parent; a finished thread is dropped.

## What the traps do (`kh2-magic`)

Objects are handed to scripts as 16-byte records whose second word is the object's handle,
the way the game does it. Scripts copy the caster's header into work memory and pass the
copy's address around.

Vectors a trap returns live in static slots, one per trap (some traps share one), as in the
game.

The game's y axis points down; the playground's points up. `kh2-magic` flips y where values
cross. **Checked from the data:** scripts move a point "up 80" by subtracting 80 from y, and
effect offsets of −93 put Fire's ring at chest height.

| Area | Traps | Notes |
|---|---|---|
| Objects | `obj_pos`, `obj_dir`, `obj_is_air`, `obj_is_jump`, `sysobj_motion_id`, `obj_target_pos`, `obj_apply_bone_matrix`, `obj_hitback`, `obj_cmp`, `obj_is_equip_ability`, `sysobj_party`, `sysobj_is_exist` | Checked (exe). `obj_dir` is (sin, 0, cos, yaw). `obj_target_pos` is the target-type collision shape's centre. Hit-back starts at distance × 0.1 per frame. |
| Targets | `btlobj_target`, `target_pos`, `target_is_exist`, `target_dup`, `target_free` | Checked (exe): the record holds an object, a part and an attack. **Ours:** which Shadow Sora targets is the lock-on, else the nearest in front within 1500, else the nearest. |
| Attacks | `attack_new`, `attack_set_radius`, `attack_set_pos`, `attack_free`, `attack_dup`, `attack_is_hit`, `attack_is_hit_bg`, `attack_is_reflect`, `attack_get_reflect_count`, `attack_strike` | See below. |
| Effects | `pax_start`, `pax_start_bind`, `effect_pos`, `effect_set_pos/dir/scale`, `effect_loop_end`, `effect_kill`, `effect_fadeout`, `effect_is_alive`, `effect_is_active` | See below. |
| Vectors | `vector_add/sub/mul/len/roty/addf/homing` | Checked (exe). `roty` turns a facing of yaw y into y + a. `addf` adds normalize(dir) × speed × frame time. `homing` turns a unit vector toward another by at most max × frame time. NaN inputs are zeroed in place. |
| Threads, memory | `magic_start_thread`, `memory_alloc/free`, `teamwork_alloc`, `frametime` | `magic_start_thread` takes a code address and one argument, and shares work memory. |
| World | `bg_ground_pos`, `bg_get_random_pos_air` | Ground: a ray straight down. **Ours:** the random air point is at a horizontal distance between the first two arguments and a height offset between the last two. |
| Object scripts | `obj_effect_start` | Starts an element from the object's own effect list. Wisdom Form's shot script uses it (`drive-model.md`). |
| Not modelled | `obj_star` (MP orbs), `status_prize_drain_*`, `saveram_increment_friend_recov`, `attack_set_pax`, `attack_set_system_pax`, `sysobj_is_summon` | No gameplay effect here. |

### Spell attacks (Checked, exe)

**Shape.**
- A sphere when radius = height, otherwise an ellipsoid.
- Height defaults to the radius.
- It starts with size 0 and hits nothing until the script sets a radius and a position.

**Sweep.**
- When the position moves more than 2 × frame time, copies are added from the new position
  back toward the old one, one `min(radius, height)` apart.
- They stop once the remaining gap is under half a step.
- Moving also casts a ray from the old position to the new one against walls. Rows with the
  BG flag then report `attack_is_hit_bg`.

**Hitting.** Spell attacks hit through the target's hit record (see `combat-model.md`).
`attack_dup` copies an attack under a new ATKP id with the **same id and group**: Master
Form's double Fire ring doesn't hit twice.

**`attack_strike`** applies an attack straight to an object; Cure uses it. Cure's ATKP rows
are kind 6, which heals `ceil(power × Magic / 100)`.

### Effects (Checked, exe + Data)

**Starting one.** `pax_start(_bind)` starts the PAX **element whose id matches**. The element
names:
- a DPX effect
- a bone
- an offset, rotation and scale
- flags: bind, bind only at start, position only, destroy when the motion changes, …

**Bone codes:**
- 0x4000: the object (feet, facing)
- 0x4001: body centre (the object-type shape)
- 0x4002: the model root
- 0x4003 / 0x4004: weapon tips
- 0x4005 / 0x4006: weapon bases
- 0x4007 / 0x4008: weapon middles
- 0x4009: another collision point
- otherwise a bone index

**Lifetime.** It comes from the DPD particle data's emitter headers:
- a start frame (or "when the loop ends")
- a total length (or "until the loop ends")
- a loop range

A one-shot effect lives until its last emitter ends; Reflect's barrier lasts 27 frames. A
looping one lives until the script ends the loop, kills it or fades it, as Blizzard's shards
do. **Ours:** "alive" and "active" are treated as the same thing. The game's alive flag and
particle pointer differ only in edge cases.

**Drawing.** Particles aren't drawn. The playground shows each live effect as a small
glowing orb at its real position, and each spell attack as its real hit volume, both
coloured by element.

## The six spells, as their scripts play out (your install, level 3)

| Spell | What the script does |
|---|---|
| Firaga | **Ring:** an ellipsoid (radius 240, height 180) on the ring effect at Sora's chest. It lasts 42 frames. **Hits:** ATKP 502, rows alternating 8- and 10-frame re-hits at 50%; the fifth hit is 175% and knocks the target away. |
| Blizzaga | **Recoil:** Sora is pushed back 50. **Shard:** a thread fires one shard from his body centre at speed `magi`[5]. It homes toward the target and stops on a hit or a wall (ATKP 248, 375%). |
| Thundaga | **Bolt:** a thread waits, then strikes at the target with a thin column (ATKP 249) and a burst (ATKP 250, 375%). The finisher and Drive Forms add more bolts. |
| Curaga | **Heal:** heals every party member within `magi`[14] of the target body centre (Sora when he has no target). ATKP 708 on the centre object, 709 on others; 600% of Magic. |
| Magnega | **Pull:** a sphere above and in front catches enemies into an orbit (reaction 5, `combat-model.md`). The spell lasts about 230 frames. |
| Reflega | **Barrier:** ATKP 573 (kind reflect) for its 27-frame effect. Each enemy attack it stops counts. **Counter:** afterwards it fires ATKP 574 every 6 frames, then a 667 burst (550%). Level 1 (REFLECT_1): the counter re-hits every 10 frames, from frame 30 to 57, the burst 58..63. |

Reflect in KH1 (the mod's Magic-menu Reflect, level 1, run from this same script): see
`kh1-reflect-spell.md`.

## Still open

- The MP Charge rate. The playground refills MP over 10 seconds once it runs out
  (**ours**).
- Drive Form variants run from the same scripts with the form's `sub`. They haven't been
  checked one by one.
- Sora's battle-target search when nothing is locked on (**ours**, above).
- Party members (Cure and some counters look for them; the playground has none).
