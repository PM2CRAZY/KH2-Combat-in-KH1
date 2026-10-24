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
- [8. Mr Bugs: 1. (a) if it ever happens in game I'll just let you know and](#^m8)
- [9. Assistant: t417 to t425 (one piece each; the build at the top of the br](#^m9)
- [10. Mr Bugs: can we have the files generated at run time so we don't take](#^m10)
- [11. Assistant: t427 to t430 (one each)](#^m11)
- [12. Mr Bugs: 1. the older player is to be deprecated](#^m12)
- [13. Assistant: t437 to t444 (one change each).](#^m13)
- [14. Mr Bugs: 1. (b) (I'm unsure the affects of this as I noticed reflect ](#^m14)
- [15. Assistant: t454 to t456.](#^m15)
- [16. Assistant: t466 to t472 (one change each). Your log was from t463, so t](#^m16)
- [17. Assistant: t481, t482 and t483. Your traces and logs hadn't reached the](#^m17)

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

^m8


> [!tip] Assistant
> t417 to t425 (one piece each; the build at the top of the branch has them all)
>
> I've ported every KH2 effect I could onto KH1's own effect system:
> 1. **t417:** a builder that turns any KH2 effects into KH1 effect files (Explosion's comes out byte for byte the same as before).
> 2. **t418:** all of Sora's own effect package, with Magnet Burst switched on.
> 3. **t419:** KH2's version of the morph program, so Vicinity Break and Aerial Finish play right in KH1 (KH1's own version would ruin their last keyframe).
> 4. **t420:** effects that follow a bone by position only, which covers the rest of Sora's moves: Guard Break, Finishing Leap, Aerial Recovery and the ones under `kh2_effects = all`.
> 5. **t421:** the Drive and Revert flashes.
> 6. **t422:** one effect file per package, the Keyblade vanish flash, and effects that don't follow anything.
> 7. **t423:** Valor's and Final's own effects (their body and root ones).
> 8. **t424:** two more KH2 programs the spells need (an angle loop and the flame trails).
> 9. **t425:** Reflect's, Wisdom's and the KH2 magic effects.
>
> Anything KH1 can't do yet stays on the old player, and the log says which effect and why. That's mainly: the Drive sphere's distortion, the effects on Valor's and Final's Keyblade tips, spell effects on KH2's "body centre" point, and anything the log lists when it loads.
>
> **Check:** the decomps for every program (compared slot by slot between the two games) and KH1's binding calls in the exe; Sora's own files for his package. Not checked in game. I only have Sora's effect file here, so the forms', spells' and Keyblade's files couldn't be checked offline; the first log will tell us what got through. If a package needs more texture slots than KH1 has (10 / 6), it's left on the old player and logged. Memory: each package carries its own copy of its textures, so the packages together take some extra memory (built once, in the background).
>
> Questions:
> 1. Valor's and Final's Keyblade-tip effects: (a) use KH1's blade binding, so they turn with the Keyblade the way KH2's own rule does, (b) keep the old player's way (at the tip, facing where Sora faces), or (c) leave them on the old player for now? And for the second Keyblade's tip: (a) I add a "second Keyblade" anchor, or (b) leave those on the old player?
> 2. The Drive sphere uses KH2's screen-distortion program, which the old player doesn't draw either: (a) port the distortion through KH1's renderer, (b) play it through KH1 without the distortion (looks the same as now), or (c) leave it?
> 3. Spell effects on KH2's "body centre" point: (a) I find KH1's matching point and port them, or (b) leave them on the old player?

^m9


> [!note] Tester Mr Bugs
> can we have the files generated at run time so we don't take up extra memory?
> 1. (a)
> 2. (a)
> 3. (a)
>    
>    do the check
>    
>    go ahead 

^m10


 

> [!tip] Assistant
> t427 to t430 (one each)
>
> 1. **t427, memory:** the KH2 effect files are now made at run time and only kept while they can play. Sora's and the Keyblade's are made at load (Sora's only for the moves you have KH2 effects on, plus Drive and Revert). A form's is made when you pick Drive and freed after Revert. Spells and Reflect are made at the levels you have. The common textures are made once and shared by every package, and we no longer keep our own copies of the HD images after KH1 has them. Sora's package went from about 10 MB to about 1.7 MB. There's no hitch on first use: until a file is ready, the old player plays that effect on the same frame, so nothing misses a frame.
> 2. **t428, Keyblade tips:** Valor's and Final's tip effects now turn with the Keyblade, through KH1's own blade binding. The second Keyblade's tip uses the same KH1 binding, on the copy of Sora that the left Keyblade's trail already follows.
> 3. **t429, Drive sphere:** KH2's screen distortion is ported through KH1's own distortion drawing, so the sphere plays through KH1 now.
> 4. **t430, body centre:** KH1 has its own body centre (the centre of Sora's body collision), so spell effects on KH2's body centre use that now. The log prints both heights the first time one plays.
>
> **Check:** the KH1 exe and decomp (texture lookup, file removal, the blade and body-centre anchors) and the KH2 decomp (the distortion program); Sora's effect file offline for the sizes. Not checked in game. Assumptions: two numbers the KH2 decomp leaves out in the distortion (the normal scale and the push strength), that KH1 stops the second Keyblade's effects when it goes, and that freeing a package's texture leaves nothing stale behind. There was no log newer than t417 to check; the first one will list what went native per package (the forms', spells' and Keyblade's files only get checked in game).
>
> Questions:
> 1. The old player still loads its own copy of every package at start, as the safety net for first plays. (a) keep it, (b) drop its copy of each package once ours is ready, and make forms and spells at load so nothing needs the safety net, or (c) drop it and keep everything made when needed (a form's very first effects could be missed if its file isn't ready in time)?
> 2. Body centre: (a) KH1's own body centre (like the bones, KH1's own body), or (b) KH2's exact height (85 above his feet)?

^m11


> [!note] Tester Mr Bugs
> 1. the older player is to be deprecated
> 2. (a)
> 
> do the check
> 
> go ahead

^m12


> [!tip] Assistant
> **t437 to t444** (one change each).
>
> From your t430 log: Sora's and the Keyblade's files and your level 3 spells all built and went through KH1, the programs plugged in fine, and there were no slot or binding errors. Four kinds of spell effect had no KH1 anchor yet, and now they all go through KH1:
> - **t437:** Fire's pieces on Sora's place.
> - **t438:** Fire's and Blizzard's pieces on the second Keyblade (they play when there are two Keyblades, like Final).
> - **t439:** Cure's pieces on KH2's own effect point.
> - **t442:** effects placed once and not attached to anything.
>
> **t440:** only the pieces that actually play are built (the Keyblade's file went from 4 effects to just its vanish flash).
> **t441:** Thunder 3 has two textures with no HD image, so part of it draws untextured. Its file isn't here, so this version logs what's needed to fix it. Please send the log after casting Thunder.
> **t443:** a form's file is built as soon as you could Drive into it, so it's ready before you pick it.
> **t444:** the old player is gone. Everything KH2 plays now goes through KH1's own effect system, and Sora's and the Keyblade's files are ready before KH2 takes him. That frees about 16 MB of textures the old player held, plus its work every frame. If something can't play, the log says which effect and why. The `fx_test` ini key is gone.
>
> Checked against the KH1 exe, the KH2 decomp and the files; nothing is tested in game yet. Still not playing: two KH2 flags no effect in your log uses (scale by bone, two-point), and the second-Keyblade pieces when Sora has one Keyblade.
>
> Questions:
> 1. Cure's pieces on KH2's effect point: (a) KH2's exact height, 105 above his feet on KH1's Sora (what it does now), or (b) KH1's own body centre, like you chose for the spells' body centre?
> 2. Some KH2 effects fade when the Keyblade is hidden (two of Fire's). KH1's Keyblade stays out while casting, so it only shows if it's put away mid-effect: (a) port it, or (b) leave it?
> 3. The debug menu's draw still reserves a 24 MB buffer that was sized for the old effects: (a) shrink it, or (b) leave it?
>
> Also: your request that new threads get their own folder is done.

^m13


> [!note] Tester Mr Bugs
>1. (b) (I'm unsure the affects of this as I noticed reflect is misaligned with sora, should we actually go with a for kh2 magic? )
>2. (a)
>3. (a)
>

^m14


> [!tip] Assistant
> **t454 to t456.**
>
> 1. **t454, Reflect:** it wasn't using either body point. All of its pieces sit on Sora's root with KH2's own numbers, so (a) wouldn't have changed it. The real bug was that KH1's root faces a quarter turn away from KH2's, so every KH2 effect on Sora's root (Reflect, Explosion's burst, Fire's and Cure's root pieces, the forms' root effects) was turned 90° around him. Reflect's side pieces ended up in front and its front pieces at the side. That's fixed now.
> 2. **t455:** KH2 effects now follow Sora's and the Keyblade's visibility like KH2's do. Fire's two pieces go when the Keyblade is put away, and Explosion's flashes also fade with Sora (for example going into first person).
> 3. **t456:** the debug menu's vertex buffer is now 4 MB instead of 24 MB.
>
> **Check:** KH1's root frame in the exe and decomp, and KH2's alpha rule in the decomp. Not checked in game. Assumption: KH2's object frame is the one the old player used (the one you checked with Guard Break).
>
> No new log yet for Thunder 3's textures. Please send one after casting Thunder.
>
> Questions:
> 1. Now that Reflect is sorted, Cure's pieces on KH2's effect point: (a) KH2's exact 105 above his feet (what it does now), (b) KH1's own body centre, like the other spells (about 20 lower on his body than KH2's point), or (c) KH1's body centre plus KH2's 20 between the two points?
> 2. The debug menu also reserves a 24 MB texture buffer meant for the KH2 player model (which is off) just to hold its small font: (a) give the font its own small buffer and only reserve the 24 MB if the model comes back, or (b) leave it?

^m15


sent log and traces heres what I noticed: 

thunder is squares
blizzards projectile doesn't render
firaga doesn't have all of its flames  
reflect still is misplaced
cure is currently in the center of sora

I also noticed that the kh2 effects took over some of the native magic effects like aero, for those effects can we have a toggle in the debug menu? kh2's casting that has its own animations should stay because the move behaves differently I noticed changes on

stop, 
aero,
cure

if I got anything wrong let me know, I'm trusting you for your decision

please have a toggle for those magics for now

1. try (c)?
 2. if the debug allocates for kh2 model then make sure its gone

go ahead

  


> [!tip] Assistant
> **t466 to t472** (one change each). Your log was from t463, so t454 to t456 were already in it.
>
> 1. **t466, Thunder:** KH2 gives each palette of a texture its own HD image, one key apart, and KH1 only ever used the first. Our effects now do it KH2's way, so Thunder should be textured.
> 2. **t467, Blizzard:** the shard used one program we hadn't checked yet. It turned out to be identical in both games, so it plays now.
> 3. **t468, Firaga:** KH1 has only 10 texture slots for all the effects drawn in a frame, and the way the spells were built, Firaga needed 24 at once (Reflect needed 37). Spells are now packed the way KH2's own files are, so they fit. I think that's what was hiding the flames, but I can't confirm it here.
> 4. **t469, Reflect:** I re-checked the turn fix against the KH2 code, the old player and KH2's own Reflect animation, and it holds: the barrier lines up with his raised hands. So it's something else. The log now writes where each spell piece starts next to KH2's numbers for it, so the next log will show exactly what's off.
> 5. **t470, Cure:** (c) is in: KH1's body centre plus KH2's 20.
> 6. **t471, magic toggle:** Stop and Aero never got KH2 effects. They're KH1's own effects, but drawn on KH2's cast animation and cut at its end, which makes them look different. The debug menu has a new KH1 Spells page (on the main page) where you can set Gravity, Stop and Aero each to "KH2 cast" (as now) or "KH1 cast" (KH1's whole cast, effects and all). Cure with Magic on KH2 is KH2's own Cure, so its effects being KH2's is expected, and I left it alone. Also, Gravity's hit didn't seem to come out before the cast animation ended in your log; that's gameplay, so tell me if you want it looked at.
> 7. **t472, debug menu memory:** the font has its own small buffer now, and the 24 MB model buffer is only made when the KH2 model is on.
>
> **Check:** the KH2 decomp against your log for Thunder, the exes for Blizzard's program and KH1's texture slots, the KH2 files for Cure and the casts. Not checked in game.
>
> Questions:
> 1. Reflect: is it (a) too high, (b) off to one side, or (c) turned? A log after casting it would help.
> 2. Reflect's numbers are KH2 Sora's size, and KH1 Sora is about 11% smaller. Should I (a) scale them to KH1 Sora, or (b) keep KH2's exact numbers?
> 3. Firaga: are all the flames there now?

^m16


thunder works, 
blizzard is sideways,
firaga has one flame still

use the decomp (steam c) to find your answer, I'll send some traces and logs.

do the check

1. (b)
2. (a)
3. it isn't changed

go ahead


> [!tip] Assistant
> **t481, t482 and t483.** Your traces and logs hadn't reached the thread folder, so I worked from the decomps, the KH2 spell files and the t478 log on the PC.
>
> 1. **t481, Blizzard sideways.** I found the cause in KH1's own code. When KH1 sets an effect's direction from a matrix, it reads the angles back in a way that doesn't match how it turns them into a direction again (KH1's apply `0x1402ADB40` and its angle read `0x1402A8EF0`, read in the Steam exe). So anything we handed it came out turned: the front pointed to the side, upside down. Blizzard's shard is aimed by its script every frame, so it flew sideways. We now hand KH1 the matrix pre-turned, so it ends up exactly as KH2's script aims it. Wisdom's shots and Reflect's hit flashes go through the same path, so they change too.
> 2. **t482, Reflect off to the side (1b) and Firaga's one flame.** The same quirk was behind both. The quarter turn I added earlier for effects on Sora's root was based on the wrong frame; with KH1's turn counted in, KH1's root frame already matches KH2's exactly, so that quarter turn is gone and Reflect's pieces should sit around his raised hands again. For Firaga, the pieces placed only by position on his root had their height flipped: the fire ring at his chest and its other pieces were drawn 70 to 220 units under his feet, inside the floor. The one flame you saw was the Keyblade tip's. Firaga's ring is three spinning rings with two flame trails each, and they should all show now. This also changes Explosion's burst, the forms' root effects and Cure's pieces, which were turned the same way.
> 3. **t483, Reflect's size (2a).** Reflect's pieces (their places around him and their size) are scaled to KH1 Sora, about 89% of KH2. The barrier's blocking area is unchanged.
>
> Checked: KH1's exe (the angle code) and decomp; the t478 log (an effect on Sora's root confirms the turn); KH2's exe and decomp for the direction trap; KH2's spell files (I ran the Fire and Blizzard 3 scripts and read Firaga's ring). Not tested in game.
>
> Please test: Blizzard's shard (does it point the way it flies?), Firaga's ring (all the flames, at chest height, spinning), Reflect's place and size, and a quick look at Explosion's burst and the forms' root effects. A log after casting Fire, Blizzard and Reflect would let me check the new "origin" lines against KH2's numbers.
>
> Questions:
> 1. Effects stuck to Sora's bones and the Keyblade go through the same KH1 code, so they may be turned the same way. From your play: (a) they look right, leave them; (b) something on his hands or Keyblade looks turned: send a log and I'll measure it the way I did for the root; (c) apply the same fix to them now (I can't confirm yet whether KH1's bones already make up for it).
> 2. Reflect's blocking area is still KH2's size: (a) leave it (gameplay stays KH2's), or (b) scale it to KH1 Sora like the visuals?

^m17

fire and thunders effects are working can't test blizzard because:

I'm unsure about some of the changes because I can't test them, currently there's now a new bug where kh2s magic doesn't get casted the first time you input said magic. 

I've sent the log and traces in "kh2-effects-in-kh1s-own-effect-system" for this thread thats where it will always be. 

I should also state that reflect on the ground is the one currently not placed correctly, I also sent that in the trace.

1. can't test so unsure
2. if sora is smaller than kh2 sora then yes scale it down

go ahead