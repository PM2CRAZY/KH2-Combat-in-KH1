# KH2 effects in KH1's own effect system

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Mr Bugs: from chat-history-log.md (kh2 moves hitbox effect bugs)](#^m1)
- [2. Assistant: t376](#^m2)
- [3. Assistant: Correction to the build note above: there's a build now. kh1](#^m3)
- [4. Mr Bugs: 4. it should be exact to kh2 so for kh2 moves take the 2 fra](#^m4)
- [5. Assistant: t394 to t398 (one part each; the build at the top of the bra](#^m5)
- [6. Mr Bugs: 1. (a) just make sure that kh1s effects still behave the sam](#^m6)
- [7. Assistant: t402 to t404 (one each)](#^m7)

---


  

> [!note] Tester Mr Bugs

> from chat-history-log.md (kh2 moves hitbox effect bugs)
> "# Porting KH2's effects into KH1's effect system
>Goal (systems-to-port: Effects): any move can play vanilla KH1 or KH2 effects, with KH2's effects running inside KH1's own effect system rather than beside it. Chosen: ladder step 1 (native). First target: Explosion. All addresses Steam. Tags: **Checked (exe)** = read in the Steam exe / Ghidra C export; **Checked (files)** = read in the user's OpenKH extraction; **Assumption** = not yet.
>## Decisions
>- 2026-10-22: ladder step 1 (native), Explosion first.
>- 2026-10-22: **bones: option A**. KH2 elements name KH2 skeleton bones; they attach to the matching **KH1** bone through KH1's own bone binding (the effect follows the Sora on screen, as the base attacks now do: KH2's motions on KH1's skeleton). Not KH2's own pose (option B, a parallel skeleton).
> [!NOTE]
> 
> ## >Where we were (ladder step 3)
> 
> `kh1-hook/src/win/kh2fx.rs` + the `kh2-fx` crate + `fx12.rs`: a full second effect player (KH2's particle machine in Rust, its own clock, instances, bones and D3D12 renderer, drawn after KH1's world-model pass, hook `0x1EADD0`).
> 
> ## Check results (2026-10-22)
> 
> ### Same engine: DPX -> DPD -> "ppp" particle programs (Checked (exe), Checked (files))
> 
> - KH1's effect files are **DPX** (strings `worldmap/eft/map_event.dpx`, `always.dpx`, `prize.dpx`, ...; loaded and relocated by `0x1401E0080`), the same format as KH2's DPX: `version | 0 | 0 | count`, 32-byte entries (DPD offset, particle index, effect number, kind mask, group mask, ...). Same version: `0x82` in both games' files. Both games' DPDs start with `0x96`. KH1 has **no PAX layer**: what KH2's PAX element does is done in KH1 by the code that starts an effect (below).
> - DPD particle data relocation: KH1 `0x14031C2F0` and KH2 `0x1404B06D0` decompile **identically**.
> - DPD texture creation (PC port, `TexCommon`): KH1 `0x14031EC10` vs KH2 `0x1404AECD0`, 90% the same; KH2 adds one extra texture case (`0xFFFD`). Helper KH1 `0x14031E090` = KH2 `0x1404AE0A0`, identical.
> - Program tables: KH1 `0x14054FF00`, KH2 `0x140779A00`, 0x50 per entry (name, calc +0x08, draw +0x18, hooks +0x20/0x28/0x30, construct +0x38, reset +0x40, destruct +0x48). Entries 1..235 same names, same indices. KH2-only: 236 VertexApLcLoop, 237 SnoScaleAll, 238 SnoMdlDttNorm, 239/240/242 RyjMegaBirthUserCtrl(1-3), 241 EiDrawShipoly.
> - **Per-program code comparison** (normalised decompiles, every slot of 1..235): **120 identical**, 115 differ. Most differences are trivial (direct call vs thunk, random-number helper, or only the offset of a field in the effect instance, whose layout differs between the games: KH1 `+0xD5` vs KH2 `+0xC1` for the same flag). Real ones:
>     - pppColor (43): KH2 clamps colour to 32767.
>     - pppRyjMegaBirth (201): KH2's draw is rewritten (particle records 0x18 -> 0x30 bytes, new matrix setup, depth handling, a third matrix mode `+0xCC` 1..3 vs KH1 1..2); calc 77%, draw 61% similar.
>     - pppRyjMatrixWorld* (98-104) construct 57%; pppAngleLoop (214) 38%; pppRyjMegaBirthPrize (203) 19%; pppKeLns* (189-193, 221-225): KH2 adds a calc slot. Not used by Explosion.
> 
> ### Clock (Checked (exe))
> 
> KH1's effect update `0x1401ECB50` steps each effect instance once per 2.0 of frame delta (accumulator at instance `+0xBC`, frame delta `DAT_14233FBDC`): 30 Hz, the same as KH2 (`docs/kh2-effects.md` §1). No frame conversion needed.
> 
> ### Element layer: what starts and places an effect (Checked (exe))
> 
> - Effect instances: pool `0x142678280`, 0x100 x 0xE0 (claimed by `0x1401EC690`, started from a DPX by number with `0x1401E0100 (dpx, element, number, priority)`, stopped by `0x1401EC470`).
> - KH1's "element" (0x3C each, 256 at `0x142D678B0`, allocator `0x1402AE5B0`): holds handles to the instance's root matrix, colour and flags; start at a position `0x1402AE650`, on an actor's bone `0x1402ADCF0` (a per-frame binding through the task list `0x142DB34B0`, callback `0x1402AE910`). The particle header's sound (`+0xFC`) is played by KH1 too.
> - KH2 PAX element features with **no KH1 counterpart found**: start delay, fade-out frames, "ends with the motion" flag (0x20), anchors 0x4001 body centre / 0x4003+ weapon points. These need a small element layer of ours that drives KH1's own instances (start, stop, root matrix, colour), not a renderer.
> 
> ### Textures: the main risk (Checked (exe), Checked (files))
> 
> - Both games have their own resident `texcommon.dpd`, with different contents.
> - Texture addressing: KH1 `0x1401EBC60` vs KH2 `0x1402C1800` (same routine, 66% similar), different VRAM layouts:
>     - KH1 common: TBP `0x3600-0x37FF`, CBP `0x3D00-0x3D3F`; KH1 own textures from TBP `0x3800` / CBP `0x3D40`.
>     - KH2 common: TBP `0x29xx-0x2Exx`, CBP `0x2Exx`; KH2 own from TBP `0x3000` / CBP `0x3E00`; KH2 moves addresses by `-0x200` / `-0x400` (TBP) and `+0x1E000` / `-0x1000` (CBP) at load.
> - So a KH2 DPD can't be handed over as is: both its own textures and the KH2 common ones fall in KH1's _common_ window. Their addresses must be moved into KH1's own window, and the KH2 common textures it uses copied into the DPD as its own (shapes' and models' TEX0 rewritten to match).
> 
> ### Handles (Checked (exe))
> 
> KH1's pointer-to-handle `0x14038AD90` registers a new 32 MB region in the 64-entry handle table when needed, so KH2 DPD/DPX buffers we allocate can be reached through KH1 handles (`pose::handle` / `pose::ptr`).
> 
> ## Explosion, from the files (Checked (files))
> 
> Files: OpenKH extraction `new openkh\data\kh2\obj\P_EX100.a.us`, `kh2\effect\texcommon.dpd`, `kh1\texcommon.dpd` (16-byte header, then the DPD), `kh1\always.dpx`.
> 
> - Explosion = PAX elements 44, 45, 46, 47, 48 -> effects 31, 32, 33, 33, 34 -> particle datas 11, 12, 13, 13, 14 of the PAX's 4th DPD (8 textures, 23 shapes, 39 models).
> - **Programs used (26):** 19 identical in KH1 (SclAccele, ColAccele, AngAccele, SclMove, ColMove, AngMove, Point, Angle, Scale, MatrixScl, ParMatrix, SMatrix, DrawMatrix, DrawMatrixFront, DrawMdl3, DrawMdlTs3, DrawShapeX, KeZCrct, KeZCrctShp). MatrixXYZ / MatrixZXY, RandIV, VertexApLc: equivalent. Color: same + KH2's clamp. **pppRyjMegaBirth: KH2's version needed** (elements 44, 45, 48). **pppEiDrawShipoly: KH2-only** (element 48).
> - Elements 46/47 use only KH1-equivalent programs: the first native test.
> - Bones (KH2 skeleton numbers): 44 weapon point 0x4003, 45 bone 0x43 (67), 46 0xAA (170), 47 0x4E (78), 48 object root 0x4000 (fade-out 8 frames, flags 0x243). 44-47 flags 0xA61.
> - Textures: shapes use KH2 common `0x2A00/0x2E34` (256x448 atlas) and `0x2C40/0x2E00`, and own `0x3060/0x3DEC`.
> 
> ### Bone mapping (option A) (Checked in our own retarget data, `kh1-data/src/retarget.rs`)
> 
> `retarget` is part-based, not a bone-to-bone table: KH2 sides +x / -x (+92 on the arms): clavicle 74/166, shoulder 76/168, elbow 79/171, wrist 80/172; KH1 per side clavicle 194/89, upper-arm bones +2..+4, elbow +5, forearm +6..+8, hand +9..+12. So KH2 **78** and **170** are upper-arm bones (between shoulder 76/168 and elbow 79/171) -> KH1's upper arm (196..198 on +x, 91..93 on -x); KH2 **67** lies on the chest/neck line (`Pole::Line(67, 137)`). Each element's offset and rotation are given in KH2's bone frame; to keep it, convert through both bind poses: local on KH1 = inverse(KH1 bone bind) x KH2 bone bind x KH2 local. The exact KH1 bone per KH2 bone (by bind position) is picked from the two bind poses when the loader is written. Weapon point 0x4003 -> the Keyblade's tip.
> 
> ## Plan for Explosion (step 1, native), one system per version
> 
> 1. **Done, t368** (below). Loader: Explosion's DPD and particle datas into a KH1 DPX we build, textures moved to KH1's own window with the KH2 common ones copied in; KH1's own code relocates and textures it (`0x1401E0080`, `0x14031EC10`). Test: elements 46/47 started on Sora's KH1 bones (option A) through KH1's element calls.
> 2. Element layer: start delay, fade-out, end-with-motion, the weapon-point and object-root anchors (element 48).
> 3. KH2's pppRyjMegaBirth as a plugged-in program (KH1 instance layout, drawing through KH1's own shape draw): elements 44, 45.
> 4. pppEiDrawShipoly (KH2-only) as a plugged-in program: element 48's ribbon.
> 5. Retire the parallel player for Explosion.
> 
> ## Step 1 done: t368 (2026-10-22)
> 
> - `kh2_formats::kh1dpx::build(pax, texcommon, effects)` (pure byte mover): a KH1 DPX of some effects of one KH2 DPD. Keeps only their particle datas (renumbered), packs the textures they draw with (DPD's own + KH2 common) from TBP `0x3800` / CBP `0x3D40` (TBP spans page-aligned, textures sharing a TBP get the largest one's space), rewrites shapes' and models' TEX0, rebuilds each particle's texture mask, sets programs > 235 to 1 (dummy). Explosion (31..34): 4 particle datas, 12 textures, vram `0x3800..0x3AC0` / `0x3D40..0x3D70` = **6 of KH1's 10 TBP slots, 2 of 6 CBP**. Offline check: every TEX0 a kept particle draws has its texture, and it's in that particle's mask.
> - Load path (Checked (exe)): `0x1401E0080` turns each entry's DPD offset into a handle and relocates each DPD's particle datas (`0x1401ECA40` -> `0x14031C2F0` with the program table); call once per DPX. Start on a bone `0x1402ADCF0 (dpx, number, actor, bone)` -> element `0x1402AE5B0` (number `& 0x7FF`, priority 20000) -> `0x1401E0100`; number bit `0x800` = end with the motion (binding `+6` = -1, then the motion at start).
> - Handles: a region is 32 MB (`ptr & ~0x1FFFFFF`), 64 regions; the DPX buffer is placed inside one region.
> - `kh2dpx.rs`: builds on a background thread; `kh2fx::event` hands elements 46/47 to it (effect 33 on KH1 bones 93 / 198, `0x2000 | bone`); falls back to `kh2fx` if not built or KH1 has no free element. Ini `kh2_fx_native`.
> - Known gaps (step 2): rotation offsets (half turn about z) not applied by KH1's bone binding; delay / fade-out / end-with-motion not applied yet (Explosion 46/47 are 10 ticks long and end on their own).
> - **Assumption, to check in game**: KH1's draw-time texture slots (`0x1401EDB00`) give an instance needing 6 slots a contiguous run; if other effects hold slots the flash may draw with the wrong textures or not at all.
> 
> ## t368 test result: untextured squares (2026-10-22, Checked (exe))
> 
> - User: Explosion's flashes show as squares at Sora's body; the rest fine. Log: DPX built, relocated, both elements started on KH1 elements. So placement and programs run; textures don't.
> - Cause: KH1's PC port never decodes PS2 texture data. Texture creation for a DPD (`0x14031EC10`, one texture object per TBP; a record's header is 0x20 bytes: TBP, CBP, bw, psm, x, y, w, h, then its own TEX0 at `+0x10` and a size word at `+0x18`) hands the record's data pointer to the remaster texture lookup `0x14010C050`. That finds the loaded _file_ the pointer lies in (`0x1400E2B20`: file list `0x1421AB020`, each file's base `+0x658`, size `+0x640`), turns the offset into an index (`+0x21C` table) and takes that file's HD texture entry (`+0x670` list, matched by `+0x54`; created by `0x14010BAF0` / `0x14010B1B0` as `Axa::EntryTex`). Our buffer isn't a loaded file, so it returns no texture and the sprites draw untextured.
> - KH2 PC works the same way: its effect textures are HD DDS images stored with each file (`remastered/obj/P_EX100.a.us/-0..-8.dds`, `kh2_formats::egs` can read them from the install).
> - Also found: the builder doesn't rewrite a texture record's own TEX0 (`+0x10`); KH1's records keep it equal to `+0`/`+2`. To fix with whichever option is chosen.
> - Options (to decide with the user): A) register the DPX with KH1 as a loaded file carrying KH2's HD textures, so KH1's own loader makes them (native); B) hook the texture lookup for our buffer and make the texture objects ourselves.
> 
> ## t369: option A, HD textures through KH1's loaded-file list (2026-10-22)
> 
> - Decision (user): option A, native.
> - KH1's resource entry (made by `0x1400E1680`, listed by `0x1400E2F60`; 0x688 bytes): `+0x10` data, `+0x18` size, `+0x20` / `+0x1C4` / `+0x620` name, `+0x208` MD5 of the data, `+0x21C` offset table, `+0x640` size, `+0x64C` -1, `+0x658` data, `+0x668` list vtable, `+0x670` / `+0x678` / `+0x680` HD-entry list head / tail / count. List object (global read through `0x1400E2B20`'s first instruction): `+8` head, `+0x10` tail, `+0x18` count.
> - HD entry (`Axa::ReplaceEntry`, 0x78 bytes): `+8` next, `+0x10` name, `+0x50` size, `+0x54` key, `+0x60` data (`_aligned_malloc`; handed to the texture, which frees it), `+0x68` 999, `+0x70` the texture once made. Key: offset of the record's pixels in the file while `0x142F43000` is set (it is, from `axaAppMain`), else an index into `+0x21C`.
> - Texture lifetime (`0x14010CDA0`): a texture is dropped only when its file is gone (found by MD5) or the file is removed (`0x1400E2E00`); ours is never removed.
> - KH2's HD assets: one per TBP (texcommon: 16 records, 12 TBPs, 12 assets), DXT5 DDS like KH1's.
> - t369 used asset header `+0x20`: wrong (it is the asset's own place in the stream; t369 log: 0 HD textures). t370: `+0x24`, **Checked (exe)**: KH1's own loader reads `+0x24` as the HD entry key (`0x1400D53F0`, low 24 bits).
> - 2026-10-22: t370 confirmed in game by the user: Explosion's wrist flashes draw with their KH2 HD textures through KH1's own effect system. Step 1 done."
>   
>   
>   It was currently going to implement a hybrid layer on top of kh1, I want you to go into kh1 steam gihidra c decomp and find a way to implement the missing features from kh2 into kh1 natively then give me a build so I can test it 
>   
>   
>   

^m1


> [!tip] Assistant
> t376
>
> I went through KH1's Steam decomp, and KH1's own effect system already has most of what KH2's element layer does. So this is native, with no layer on top. This covers step 2 (the element layer) for Explosion's wrist flashes:
> - **Start delay:** KH1's effect instance has its own start timer. The flashes now use KH2's wait through it.
> - **Fade-out:** KH1 tints every effect by an instance colour and already fades its own effects through its alpha. The flashes now fade over KH2's fade-out frames when KH2's move changes or Sora's object goes, then KH1's own stop removes them.
> - **Offset and half turn:** KH1 builds every effect's root from the bone plus the effect's own local offset and turn. Each flash now carries KH2's offset and turn in there (its own copy of the effect data).
> - **Keyblade tip and Sora's root:** KH1 has both anchors (its own "point along the Keyblade" and "object root" codes). Explosion's big flare and burst need them, but those also need KH2's spark and ribbon programs (steps 3 and 4), so they aren't switched on yet.
>
> **About the build:** I can't build the Windows DLL from here, so build it from the testing branch as you did for t371.
>
> **Check:** decomp (Steam KH1, and Steam KH2 for KH2's side). Notes are in `docs/kh2-effects.md` §16. Not checked: anything in game. I also couldn't check the flashes' real wait, fade and offset numbers, because I don't have the effect file here; the log now prints them for each flash when it loads. And it's an assumption that KH1's arm bones (93 / 198) face the same way as KH2's (170 / 78).
>
> Steps 3 and 4 can also be native: KH2's spark and ribbon programs would be our own entries in KH1's program table format, and once KH1 loads the effect file, its program records get pointed at them. No hook needed.
>
> Three questions:
> 1. KH1 starts an effect that's stuck to a bone about 2 frames later than KH2 does. (a) keep KH1's timing, or (b) take those 2 frames off to match KH2?
> 2. For the big flare and burst: (a) wait until their sparks and ribbon are ported (steps 3 and 4), or (b) play them through KH1 now without those, just to check where they sit?
> 3. Can you check the wrist flashes' turn looks right? Compare a run with `kh2_fx_native = on` against one with it off (the old player). If they differ, I'll convert the turn through the two skeletons' bind poses.

^m2


> [!tip] Assistant
> Correction to the build note above: there's a build now. `kh1-kh2.dll` at the top of the testing branch is t376, with everything up to now in it (frame advance t372, camera t373, combo rows t374, sleeves t375, effects t376). From now on, GitHub builds it on Windows and commits it every time the code on the testing branch changes, so pulling the testing branch always gets you the latest build. Install it through OpenKH as usual (`mod.yml`). The log's first line shows which version loaded.

^m3


> [!note] Tester Mr Bugs
> 4. it should be exact to kh2 so for kh2 moves take the 2 frame delay off
> 5. do option (a) and start porting
> 6. can't check it yet as our frame advance still isn't complete, just do 1. and 2. for now
>    
>    go ahead

^m4


> [!tip] Assistant
> t394 to t398 (one part each; the build at the top of the branch has them all)
>
> Done for 1 and 2:
> - **t394:** KH2 moves' effects now start on KH2's exact frame. KH1's 2 extra frames are gone, at any fps.
> - **t395:** KH2's own spark program (MegaBirth, KH2's rewritten version) now runs inside KH1's effect system as a KH1 program, drawn through KH1's own shape draw.
> - **t396:** Explosion's big flare on the Keyblade tip and the flash at the chest play through KH1 now, anchored with KH1's own "point along the Keyblade" binding.
> - **t397:** KH2's ribbon program (EiDrawShipoly), which KH1 doesn't have, is ported as a KH1 program too, drawn with KH1's own renderer calls.
> - **t398:** Explosion's burst at Sora's root plays through KH1 as well. So on Steam, all of Explosion now runs in KH1's own effect system, with no hooks: KH1 loads the effect, and its program records are pointed at our two programs.
>
> **Check:** decomp (both programs ported function by function from the Steam KH2 decomp and compared with KH1's version), the exe for the binding call, and the game files for Explosion's real data. Not checked in game. Assumptions: the KH2 decomp drops a few operands (in the spark's spin and size, the ribbon's width and colour scale), so I filled them in from KH1's matching code and the old effect player; and that KH1's "object root" frame matches KH2's. From the bind poses, the wrist flashes' bones line up within 0.05, so they shouldn't need a turn conversion (your in-game check can wait for frame advance). Not ported yet: the ribbon's scrolling variant, which Explosion doesn't use. Notes are in `docs/kh2-effects.md` §16–17.
>
> When you test: does the flare sit on the Keyblade tip, and does the burst's ribbon trail draw (texture, brightness, which way it narrows)?
>
> Questions:
> 1. KH1's own colour program updates an effect's colour every other frame at 60 fps; KH2's updates every frame. That makes fades a little steppier than KH2's. (a) port KH2's colour program the same way, or (b) leave it?
> 2. KH1's bone binding places an effect from the bone position it read the frame before. (a) check whether KH2 does the same and match it, or (b) leave it for now?
> 3. Next: (a) Magnet Burst (it uses the same two programs, so it can follow the same path), (b) retire the old effect player for Explosion now that it all runs through KH1, or (c) both?

^m5


> [!note] Tester Mr Bugs
> 1. (a) just make sure that kh1s effects still behave the same way
> 2. (a)
> 3. (b)
>    
>    go ahead

^m6


> [!tip] Assistant
> t402 to t404 (one each)
>
> 1. **t402:** KH2's effects now use KH2's own colour program, so their fades update every frame like KH2's (clamped as KH2 does). Only the effect file we build for KH2's moves points at it; KH1's own effects keep KH1's colour program exactly, untouched.
> 2. **t403:** KH2 does the same as KH1 here: an effect follows the bone as it was the frame before. The difference was the first frame: KH2 reads the bone fresh on that frame (and keeps reading while an effect waits to start), while KH1 read it a frame early. KH2's effects now match KH2 on that first frame too. KH1's own effects aren't touched.
> 3. **t404:** on Steam, Explosion no longer uses the old effect player at all, and the `kh2_fx_native` ini switch is gone. The old player only takes Explosion while the effect file is still building in the first seconds, if it couldn't be built, or on non-Steam builds. It still runs Sora's other moves' effects (Vicinity Break, Magnet Burst, Aerial Finish and the rest), the form, Reflect and Wisdom effects, KH2 magic, and the Keyblade vanish flash. The full list is in `docs/kh2-effects.md` §17.8.
>
> **Check:** decomp (both games' colour programs and bone bindings) and the exe. Not checked in game. Assumptions: KH2's colour scale (1/128, an operand the KH2 decomp drops; KH1's and the old player use the same), and that Sora has one skeleton pass per frame.
>
> Questions:
> 1. If KH1 has no free effect slot when Explosion fires, it now just doesn't show, like KH1's own effects. (a) keep that, or (b) let the old player draw it then?
> 2. Next: (a) Magnet Burst through KH1 (it uses the same two programs), (b) another move's effects, or (c) something else?

^m7


> [!note] Tester Mr Bugs
> 1. (a) if it ever happens in game I'll just let you know and we can rewrite it. but don't worry about it.
> 2. (a) if your ready to start porting over every effect you can do it I'll give you the go ahead to port every effect
> 
> go ahead



