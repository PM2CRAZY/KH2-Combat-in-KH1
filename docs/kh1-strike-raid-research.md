# Strike Raid (Limit Form, `limi` 704/705/706) on KH1's own systems: research

Tags: Checked (decomp / exe / files), Assumption. Steam addresses only. Base read: `w987` (t987, Infinity's shots) for
`kh2-magic`; `holder.rs` / `kh1-weapon-second.md` from the uploaded tree (t980). Script disassembly `research/sr/limi.dis`.

Script calling convention used below (Checked (files), same as t980..t987): a trap's args[0] is the first value pushed. An
"obj" argument is a pointer to a script ref whose `+4` holds the object handle (`0x1404AD270(ref)+4` in every handler).
Handlers read args as 8-byte cells (`param_1[0]`, `[2]`, `[4]`… = args[0], [1], [2]…). A returned `@ADR` (0x52444140) is an
address cell, `@INT` 0x544E4940, `@FLT` 0x544C4640.

## 1. The 16 missing traps in KH2 (Checked (decomp) unless marked)

| trap | handler | args | what it does |
|---|---|---|---|
| 0:43 fixrad | `0x14041E5E0` → `0x1403B6F20` | (a) | wraps an angle into [−π, π] (`fmodf(a ± π, 2π) ∓ π`). A non-finite angle gives 0 (`_finite`), which matters: Raid's wobble has period 0, so its phase step is 2π/0 = inf → 0. Returns float. |
| 0:91 vector_inner_prodcut | `0x14041D8B0` | (a, b) | NaN components of both inputs zeroed in place; **copies** normalized (`0x140142080`), then dot (`0x140140200`). So it returns the cosine, not the raw dot. Float. |
| 1:18 obj_attach | `0x14042C550` → `0x1403D9FE0 (child, parent, bone, flags)` | (child, parent, bone, flags, n, ptr) | detaches the child first if attached; child `+0x6A0` = parent handle, `+0x6A4` bone (u16), `+0x6A6` flags; clears its local pos `+0x670..` to 0 (w 1) and, without flag 2, its rot `+0x680` (and sets `+0x9B8` bit 3); puts the child in state 3 (`0x1403B4DA0`); links it in the parent's child list (`+0x8B8/+0x8C0`, next at child `+0x8B4`); copies the draw-group word `+0x9D4/+0x750` and the `+0x7B8` bit 6 (visibility?). With args[4] ≠ 0: `+0x6B0` = a clock (`DAT_142A25048`), `+0x6B8` = n, `+0x6C0` = ptr (a callback thread, run on detach). Strike Raid passes (weapon, Sora, hand bone, 0, 0, 0). |
| 1:22 obj_effect_start_pos | `0x14042D4B0` → `0x1403B66C0 (obj, id, pos, a3, a4)` | (obj, id, pos, a3, a4) | starts effect `id` from the obj's PAX (`obj+0x80`, `0x1402C6B30`) and sets its matrix translation to `pos`. Returns the effect (address). Strike Raid: (Sora, 34, weapon's pos, 1, 0), the catch flash. |
| 1:89 obj_move_request | `0x14042F8D0` → `0x1403B9A70` | (obj, dir, speed) | obj `+0xA58` = normalize(dir), `+0xA64` = speed. Consumed once by the move step `0x1403B89A0`: displacement += dir × speed × `DAT_140717480`, added to the position `+0x670`, then `+0xA48..+0xA64` cleared. `trap_frametime` (`0x14041CA50`) returns that same `DAT_140717480` (Checked (exe)), and the script passes speed = |delta| / frametime, so the object lands exactly on the wanted point that frame. Assumption: the weapon object takes no gravity or wall push in that step (the step has a gravity branch for unattached objects, `+0xBA8`; not traced for a weapon). |
| 1:102 obj_detach | `0x14042C5F0` → `0x1403DA250 (obj)` | (obj) | unlinks it from the parent's child list; puts it back in the state saved at attach (`+0x6A8`); takes the parent's world matrix (`0x1403B5B40`) and, without flag 2, turns it to its world facing; sets its **position to its current world place** (`0x1403B64D0`, `+0x670`); `+0x6A0` = 0; runs the attach callback if one was given. |
| 1:126 obj_motion_sync | `0x14042D2C0` → `0x1403C8E90 (master+0x158, slave+0x158)` | (master, slave) | appends the slave's motion player to the master's synced list (`+0xD8/+0xE0`, next at `+0xE8`): the weapon plays Sora's motion with him. Strike Raid: (Sora, weapon). |
| 1:148 obj_set_pos | `0x14042B800` → `0x1403B64D0` | (obj, pos) | NaN zeroed; `+0x670` = pos (w = 1), then `0x1403CCB90` rebuilds its matrices now. |
| 1:149 obj_effect_start_bind_other | `0x14042D5C0` → `0x1403B6670 (obj, id, a3, a4, other)` | (obj, id, other, a3, a4) | effect `id` from **obj's** PAX bound to **other** (`0x1403B6670` swaps the bind object to `other` when non-null, Checked (exe) `cmovne`). Strike Raid: (Sora, 32, weapon, 0, 0), stored in wp12 once: the blade's trail. |
| 1:161 obj_is_attach | `0x14042B980` → `0x1403DA520` | (obj) | `+0x6A0` ≠ 0. Int. |
| 1:234 party_get_weapon | `0x140431310` | (obj, n) | the obj's weapon object `obj+0xD60 + n×8`, as an address. Strike Raid uses n = 0 (the right Keyblade). |
| 1:235 party_hand_to_bone | `0x140431360` → `0x1403D7620 (obj, hand)` | (obj, hand) | the obj's object-entry byte `+7` (`0x1403B5C10`), looked up in a table (`0x1403EAAE0`, bsearch in `DAT_142AE5798`), `row+4 + hand×2` (i16): the hand bone. Int. |
| 1:236 obj_motion_unsync | `0x14042D310` → `0x1403C8F20` | (master, slave) | removes the slave from the master's synced list. |
| 2:22 attack_set_obj_pax | `0x140432A90` | (attack, obj) | attack `+0x28` = handle of obj `+0x80` (the obj's PAX): the hit's effects come from that object's effect file. |
| 6:68 strike_raid_calc_xyzrot | `0x14053EF90` | (dir, up) | d = normalize(dir), u = normalize(up) (w 0; if u == d, u = (0, −1, 0)). Rows: r0 = normalize(u × d), r2 = −d, r1 = normalize(r2 × r0). **Writes args[1] in place: up = d × r0.** Euler angles of that matrix (`0x1401AB290`) into the static `0x1407883D8`, returned as an address. So the blade's −Z points along dir. |
| 8:2 obj_set_xyzrot | `0x140537E70` → `0x1403B65F0` | (obj, rot) | NaN zeroed; `+0x680` = rot, `+0x68C` = rot.y (its yaw copy), `+0x9B8` bit 3 set (rotation given). |

## 2. The script, end to end (Checked (files) from `limi.dis`)

**Dispatch** (trigger 0x17 / main, pc 1427): 704 sets the light (80, 64, 80) over 16 (pc 4909), the follow-up pair wp28 =
[705 Raid, 706 Judgment], **count wp36 = 3**, then 2790. Each follow-up through trigger 0x17 (pc 1242) does wp36 −= 1 first,
then 705 → 4857, 706 → 4883.

**Offers** (main loop, pc 5644, shared with Sonic Rave): while wp36 > 0 and script range 1 is open, `command_override(1,
wp28[0], Sora, 1)`; when wp36 == 0, range 2 and wp28[1]. Ranges 1/2 are motion ranges 16/17, which only K423 (234, the
catch) has, at frames 0..38 (Checked (files) msetdump; 16/17 = script ranges 1/2 as built in t981). So: throw, catch, Raid
offered during the catch, ×3, then Judgment. After Judgment wp36 = −1: nothing more. Five throws in all (1 + 3 + 1).

**Start (2790 → hook 2815):** motion 231 (K420) started, wait for it (2059), then 2848(Sora, 232, 0).
**Raid (705):** hook 4870 → 2848(Sora, 232, 0). **Judgment (706):** hook 4896 → 2848(Sora, 233, 1).

**Throw step 2848 (Sora, motion, kind):**
1. weapon = `party_get_weapon(Sora, 0)` (2997). If not attached (3022): `motion_sync(Sora, weapon)`,
   `attach(weapon, Sora, hand_to_bone(Sora, 0), 0, 0, 0)`.
2. motion start (232 or 233, blend 0), push 234 (K423, the catch).
3. Each frame while the motion is 232/233: on `motion_check_trigger(Sora, 1)` (frame trigger 9: K421 frame 8, K422
   frame 27) start the flight with Sora's facing: kind 0 → 4072 (Raid flight), kind 1 → 3073 (Judgment flight). `halt`.
4. When the motion is 234: **catch** (4738): the trail effect wp12 unbound (`obj_effect_unbind`) and killed
   (`effect_loop_end_kill`); `obj_effect_start_pos(Sora, 34, pos(weapon), 1, 0)`; re-attach (3022: sync + attach to hand
   0); motion 234 started (blend 0), 0 pushed with blend 8.
5. At limit end (5745): if the start command was 704 and the motion is 234, the catch runs again (5921 → 4738).

**Flight thread** (4040: 128 bytes of thread memory, `trap_magic_start_thread`; +0 Sora, +32 the flight point, +48 target
ref, +64 effect, +68 attack, +80 direction, +96 offset, +112 weapon). Raid (4124) / Judgment (3125):
- detach (3674): if attached, `obj_detach(weapon)`, `obj_motion_unsync(Sora, weapon)`; `obj_set_pos(weapon, pos(weapon))`.
- `sysobj_motion_start(weapon, 235, 0)`: the weapon's own motion 235 (in the weapon's mset, `W_EX010_KH1F`, not in this
  upload: what it shows is an Assumption, presumably the spin).
- speed 30; wobble (amplitude, period, phase): Raid (0, 0, 0), Judgment (30, 20, 0).
- target = lock-on (`btlobj_lockon_target`); the shared block `teamwork_alloc(59, 32)`.
- attack: Raid `attack_new(Sora, 1866, −1, 0)`, radius 160, **`attack_set_obj_pax(attack, weapon)`**; Judgment 1867,
  radius 190, and its `attack_set_obj_pax` is behind `push 0; beqz` (dead code: Judgment's hits use the default PAX).
- the trail: `obj_effect_start_bind_other(Sora, 32, weapon, 0, 0)` into wp12 if wp12 is empty.
- the "up" for the blade: with a target, `strike_raid_calc_xyzrot(target − point, Sora's facing)` is called for its side
  effect only (the facing becomes the perpendicular d × r0), then flipped if `inner_prodcut(−it, (0,−1,0)) > 0`; with no
  target, (0, −1, 0).
- **Loop while `!obj_is_attach(weapon)`:**
  - homing: while the target exists and homing is on, `vector_homing(dir, target − point, 0.08727)` (5° × frametime); homing
    turns off for good once `target_dist < 4 × 30 = 120`.
  - point += normalize(dir) × 30 × frametime (`vector_addf`).
  - floor: `bg_ground_pos(point − 100 up, depth 250 / 220)`: if found, point.y = floor − 150 (Judgment) / − 120 (Raid).
  - dir = disp + (dir − disp) × 0.8 (disp = this frame's real move).
  - offset = up × (A sin(phase) − A/2), phase += 2π/period × frametime, `fixrad`.
  - effect / attack moved to the point (1007: `effect_set_pos`, `attack_set_pos`).
  - weapon moved to point + offset by `obj_move_request(weapon, delta, |delta| / frametime)`.
  - blade up = up + (prev − up) × t, t from 1 down by 0.03 × frametime (to 0); `obj_set_xyzrot(weapon,
    strike_raid_calc_xyzrot(dir, blade up))`. `halt`.
- On exit (the catch attached it): effect killed, attack freed (`attack_free`), target freed, memory freed (1095).

**So KH2 has no return flight**: the Keyblade flies out for the rest of 232/233 and is snapped back at the catch, with
effect 34 where it was. Checked (files). Interrupted throws (Sora hit before 234) never reach the catch: the thread flies on
until something attaches the weapon; what does in KH2 is open (form end? `trap_obj_stop_end_all(17)`?).

**Motions** (Checked (files), `msetdump`): R = range triggers (id, start, end, params), F = frame triggers (frame, id, params).
- 231 K420: R 28 (0..12, 15), 27, 31, 29 (whole), 10 (ATKP 124 guard type: 0..6 with 3, 6.. with 6), 23 (0..24: 1, 72).
  F: 0 14(8), 0 1(42) effect 42, 19 voices at 0/18.
- 232 K421 (Raid throw): R 28 (0..8, 22), 27/29/31, 23 bits through 52. F: 8 14(4), **8 9 = throw**, 8 31(2), 19 at
  0/6/10/42, 2 (sound) at 12 (0,0) and 50 (4,0).
- 233 K422 (Judgment throw): R 28 (0..8, 12..27), 27/29/31, 23 through 86. F: **27 9 = throw**, 0 14(6), 27 31(8), voices
  11/12/34/39/53/75, sounds 12 (9), 40 (10), 86 (4).
- 234 K423 (catch): R 44 (0..38), **26 (704)**, **16, 17 (0..38)**, 23 (0..35, 72). F: 0 29, 0 1(33) effect 33, voices
  0/46, sound 38 (4,0).
- Range 27 (invulnerable, as Sonic Rave) is in 231..233, not 234. Ranges 31, 29, 44 meanings: not read. Motion lengths:
  not read (the last trigger frames, 52 / 86 / 46, are lower bounds). Assumption.

**Attacks** (Checked (files), 00battle `atkp`): 1866 type 1, power 200, reaction 8, sound 500004, drive 8, HP drain 8.
1867 power 500, reaction 11, knockback 50, sound 500009, drive 20, byte +0x2A 75, HP drain 8. One attack object per throw,
freed at the catch, so each throw hits again.

## 3. KH1's own Strike Raid (Checked (decomp) unless marked)

| function | first bytes (Checked (exe)) | role |
|---|---|---|
| `0x14039FFD0` module start | `48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 50 48` | light `0x140397990`; registers state **0x27** (`0x1402B1B80`, enter `0x1403A01D0`, leave `0x1403A01A0`, step `0x1403A0170`) and enters it; requests motion **0xE6**, queues **0xE7** with end callback `0x1403A0290`; starts element 0 of the limit's data (`0x1402AE290`, `DAT_142EFB0E0`); throw count `0x142F114CC` = 0 |
| `0x1403A0290` throw (on 0xE7's end) | `48 89 5C 24 10 48 89 74 24 18 57 48 83 EC 40 48` | turns Sora to the target (`0x1402A3B00`); KH1 attack row **0x2F + count** (`0x1402A3980`: the row's motion, its attack by the row maker `0x1402B3270` on bone **0x30**, trail row+2 `0x1402C2310`, element row+3 bound at 0x30); count += 1; target > 200 above → aim-high motion 0xEA/0xEB and catch 0xEC instead of 0xE9; attack `+0` bit 0x800000; element **0x801** (5th throw **0x806**, its handle in the attack `+8`) bound to Sora at point 0x30 (`0x1402ADCF0` → `0x1402AE7F0`, bind type 2); queues 0xE9/0xEC (end → `0x1403A0440`), 0xEE (→ `0x1403A0200`), 0x40100000 (→ `0x1403A0220`) |
| `0x1403A0440` catch | `48 83 EC 48 48 8B 05 3D 46 1B 00 48 33 C4 48 89` | element 2 at the blade point (`0x14029EB20(Sora, 0x30)`, `0x1402AE650`) |
| `0x1403A0200` / `0x1403A0220` | (tail jumps) | element 3 on Sora; state 0 (end) |
| `0x1403A0230` Triangle | `40 53 48 83 EC 20 33 D2 83 3D 8D 12 B7 02 04 0F` | `0x140293000(0, 0x5F or 0x60 after 3)` while count < 5 and the row's flag `+0x42` bit 0 is up; else `0x140292FB0(0)` |

**The thrown Keyblade is Sora's own weapon bones.** KH1 draws one weapon per actor inside his queue entry, skinned to
**bones 280..292** of his skeleton (`0x1402A1630` → `0x1401D4070`, Checked (decomp), `kh1-weapon-second.md`), and KH1's
motions animate 280..292 (Checked (files) there). Strike Raid's module has **no per-frame flight code**: it only reacts at
motion ends; the attack (row maker on bone 0x30), its trail and the 0x801 element all follow point 0x30, which is the blade
(`0x14029EB20`: for 0x30 the point along the blade from the weapon row's `+0x28` to `+0x24`). The aim-high variant is a
different motion. So the flight is baked into the throw motions (0xE8.. rows) as weapon-bone animation. Assumption on the
last step only: KH1's motion files aren't in the upload to see bone 280 leave the palm. There is no KH1 "detach weapon",
"move weapon" or "return to hand" function for Strike Raid; the "return" is the motion bringing the bones back.

**What KH1 does have for a free Keyblade** (the t830..t848 holder, Checked (decomp + exe)):
| function | first bytes | use |
|---|---|---|
| `0x140290D60 (id)` actor maker | `40 55 53 56 48 8D 6C 24 B9 48 81 EC 90 00 00 00` | the holder's actor from a row copy (type 6) |
| `0x140286D60 (actor)` weapon object | `48 89 5C 24 18 57 48 83 EC 20 48 8B D9 8B 89 30` | one of nine 0x60-byte pool objects (`0x14296B1E0`, bits `0x14296B5D8`): model, effect data `+0x18`, row `+0x20`; actor `+0x14C` |
| `0x1402A06E0 (actor, parent, bone)` attach | `48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 20 41` | `+0x398` = 0, `+0x3A0` = parent, row `+0x50` = bone (0x2000 = raw) |
| `0x1402A0730 (actor)` **detach** | `40 53 48 83 EC 20 48 8B D9 48 8D 91 00 01 00 00` | `+0x10` = world place `+0x100`, `+0x3A0` = 0 (same meaning as KH2's detach) |
| `0x1402A0A40` world step | `48 89 5C 24 20 55 56 57 41 56 41 57 48 8D 6C 24` | with `+0x374` 0x1000 builds its matrix from Euler `+0x30/+0x34/+0x38` (X, then Y, then Z: `0x1400E6AC0` / `E6C80` / `E6E30`), scale `+0x40`, place `+0x10` → `+0x100`; with a live parent, × parent bone matrix (`0x14029EA00`) |
| `0x1402A2E30 (actor, half)` / `0x1402A3280 (actor)` | `48 89 5C 24 08 57 48 83 EC 20 48 8B D9 8B FA 8B` / `81 21 FF BF FF FF C3` | draw weapon (`+0` bit 0x4000, colour) / put away (clear 0x4000) |
| `0x1402B2DC0` / `0x1402B3270` attack ctor / row maker | `48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 48` / `48 89 5C 24 08 57 48 83 EC 20 48 8B F9 E8 3E FB` | KH1 hit object; bone-source actor `+0x90`; for bone 0x30 the weapon row's radius/range, effect data `+8` = weapon object `+0x18` |
| `0x1402ADCF0` / `0x1402AE7F0` / `0x1402AE650` | `48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 57` / `40 53 56 57 48 83 EC 70 48 8B 05 89 62 2A 00 48` / `48 89 5C 24 08 57 48 83 EC 20 49 8B F9 E8 4E FF` | element bound to an actor's point / bind / element at a position |
| `0x14029EB20` / `0x14029EA00` bone point / matrix | `40 53 55 57 48 81 EC 80 00 00 00 0F 29 74 24 60` / `48 89 5C 24 08 48 89 74 24 10 57 48 83 EC 20 49` | |
| `0x140292390` actor end | `48 85 C9 0F 84 C9 00 00 00 53 48 83 EC 20 8B 81` | |

## 4. Native plan

**Design.** Attached = KH1's own Keyblade on Sora's 280..292 (as now). Thrown = a **KH1 actor of the holder kind**, made by
KH1's maker (`0x140290D60`) with a second KH1 weapon object for Sora's loaded `.wpn` (`0x140286D60` path, pool), posed by a
KH1 holder motion (KH2's weapon motion 235 baked onto 280..291: `W_EX010.mdlx`'s 12 bones are KH1's 280..291, Checked (files)
per `kh1-weapon-second.md`), drawn by KH1's actor draw. Detach hides Sora's (`0x1402A3280`) and shows it; attach the reverse.
KH1 steps, draws, hits and trails it; our code only answers the script's traps (which already run once per KH1 step). KH1's
own five-throw Strike Raid (`0x14039FFD0`) stays untouched for the KH1 setting.

| trap | onto KH1 |
|---|---|
| 1:234 party_get_weapon | a new `World` object: "the Keyblade of obj n" (its ref resolves to the thrown actor when detached, else to Sora's weapon point 0x30) |
| 1:235 party_hand_to_bone | KH1's right grip: raw 0x2000 \| 101 (palm; 280 is its child, Checked (files)). The KH2 number is never used elsewhere |
| 1:161 obj_is_attach | thrown actor's `+0x3A0` ≠ 0, or not out |
| 1:18 obj_attach | thrown actor hidden / ended (`0x140292390` or kept and hidden), `0x1402A2E30(Sora, 0)` |
| 1:102 obj_detach | thrown actor placed on Sora's grip 280 world matrix (`0x14029EA00(Sora, 0x2000\|280)`), shown, `0x1402A3280(Sora)`. `0x1402A0730` semantics if attached by `0x1402A06E0` |
| 1:126 / 1:236 motion sync / unsync | nothing: KH1's weapon bones are always posed by Sora's motion; the thrown actor has its own |
| 1:148 obj_set_pos | thrown actor `+0x10` |
| 1:89 obj_move_request | `+0x10` += dir × speed × frametime (KH2's step lands on the same point, Checked) |
| 8:2 obj_set_xyzrot | `+0x30..+0x38`, `+0x374` \|= 0x1000 |
| 6:68, 0:91, 0:43 | math in `kh2-magic` (keep the in-place write of 6:68's args[1], and fixrad's non-finite → 0) |
| 2:22 attack_set_obj_pax | the attack's KH1 effect data `+8` = the thrown weapon object's `+0x18` |
| 1:149 effect_start_bind_other | Sora's KH2 effect 32 bound to the thrown actor (`0x1402AE7F0`-style bind on its blade point) |
| 1:22 obj_effect_start_pos | effect 34 at a point (existing `effect_start` + place) |

**Versions** (numbered from t987; renumber from the folder's VERSION):
1. **t988 Strike Raid's moves**: 231..234 as Limit Form slots of ours with their windows (ranges 16/17, 26, 27, frame
   trigger 9 → `motion_trigger(1)`, effects 42/33, voices, sounds), `FOLLOW_UPS` + Raid / Judgment (two free records),
   `FIRST_MOTION` 231. The throw traps answer "attached" so the limit plays with the Keyblade in hand.
2. **t989 the Keyblade object**: 1:234, 1:235, 1:161, 1:18, 1:102, 1:126, 1:236 on the thrown KH1 actor (maker, weapon
   pool object, hide / show Sora's), plus KH2 motion 235 baked as its holder motion.
3. **t990 the flight**: 1:148, 1:89, 8:2, 6:68, 0:91, 0:43.
4. **t991 the throw's hits and trail**: attacks 1866/1867 on KH1's hit object with the thrown actor as bone source (or the
   script's sphere at the point, as Infinity's shots), 2:22, 1:149, 1:22. HP drain 8 already flows through t982.

**Not 1:1 or not checked:**
- KH2's weapon motion 235 and the lengths of 232/233 aren't in the upload (Assumption until baked from the player's files).
- Euler order / axis signs between KH2's `+0x680` and KH1's `+0x30` (X·Y·Z): Assumption, check with a trace.
- KH2's move step may add gravity to a free weapon object: Assumption (none).
- Interrupted throws: KH2's script never re-attaches unless 234 plays; what KH2 does then is open. KH1 must not leave
  Sora weaponless: re-attach on limit end / state change is a KH1-safety rule, flag it to the owner.
- KH1 pool room for a second weapon object (nine) and the holder's area-change rules (t837) apply. Assumption.
- KH1 time = KH2 frame in our slots (as everywhere), not re-checked here.
