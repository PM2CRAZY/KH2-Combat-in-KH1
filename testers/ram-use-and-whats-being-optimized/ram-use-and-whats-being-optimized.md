# RAM use and what's being optimized

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from the Table of contents(../table-of](#^m1)

---

> [!tip] Assistant
> I started this thread from the [Table of contents](../table-of-contents/table-of-contents.md) thread, where you asked why our RAM use is so high, what still needs optimizing and what's being optimized now. Here's where things stand as of t655. I went through everything the DLL loads and keeps, the latest log you sent (t651, the glide thread) and the game files on the owner's PC.
>
> One caveat first: we've never measured the process's memory directly. The sizes below come from our own log lines, from the code, and from parsing the real files with our own readers. I've tagged each one: Checked (code), (log) or (files), or Assumption.
>
> ### 1. Why RAM is high
>
> All together, the mod keeps roughly **80 to 90 MB** of its own after start. About 30 MB of that is duplicates or things nothing is using. On top of that, start-up has a short peak.
>
> - **Baked KH2 animation slots: about 32 MB. The biggest item, and half of it is a duplicate.**
>   - At every start we bake KH2's moves into KH1-format motions: 96 of them, 16.3 MB in total. Checked (log): the "KH2 slots: … baked" lines.
>   - That data is kept twice: once in the build result (`kh2slots::Built.blocks`), and again as the copy KH1 actually plays from (`place`, which never frees it). Checked (code).
>   - Only 65 of the 96 are different KH2 motions (10.8 MB). The rest are the same motion baked again on a different KH1 slot or request (Fire's cast twice, swing 151 twice, and so on). Checked (log).
>   - Shrinking it: free the build copy once it's placed (about −16 MB, easy). Later, share repeat bakes where KH1's format allows (Assumption: it may not, because each copy carries its template's events). With t655 the bake is saved in the mod folder and only loaded after the first start.
> - **Drive Form recolour source pixels: about 10.5 MB.**
>   - At start we read Sora's three HD textures from the KH1 package and keep them decoded forever (2 × 1024×1024 and 1 × 1024×512, 4 bytes a pixel). They're only a fallback for when KH1's own copy can't be used. Checked (code, log: "form tint: … read from the PKG").
>   - The recoloured GPU textures are made once per texture and look, and as far as I can tell never freed when an area change reloads Sora's model. Assumption: that's a slow leak.
>   - Shrinking it: read on first Wisdom/Valor entry and drop it once the GPU texture exists, or bake the recoloured textures to the mod folder.
> - **KH2 sounds: about 9.3 MB, and that's fine.**
>   - 104 cut sounds (9.0 MB) plus Sora's battle voice bank (0.33 MB), kept still compressed. KH1's own sound engine plays straight from them. Checked (log).
>   - We already removed the double copy here after your earlier RAM note: one buffer each. Checked (code, kh1bank.rs).
>   - It could shrink a little if sounds for forms or spells you haven't unlocked were loaded later.
> - **KH2 effects (KH1-format DPX files): about 4 to 12 MB, depending on what's unlocked.**
>   - Sora's package is 2.9 MB, the Keyblade's 0.6 MB, the forms' 0.6 to 3.6 MB each, and the spells' 0.26 to 0.43 MB each. Checked (log: "DPX built").
>   - These live in one reserved 32 MB address range, but only the pages actually used take memory. A form's package is freed when it can't play. Checked (code, log: "DPX freed").
>   - This one's already lean.
> - **KH2's animations as decoded Rust data: about 12 MB.**
>   - Base Sora's 83 motions are about 4.3 MB decoded (the file is 2.6 MB). Final Form's 50 are about 2.5 MB, the Keyblade's about 0.34 MB. Checked (files, parsed).
>   - Wisdom and Valor are about 2 MB each. Assumption: I didn't parse their files.
>   - Why it's kept: the bake is built from it, and some things still use it during play. Drive Forms run on the old controller, and the effects and Reflect read KH2's pose each frame.
>   - Shrinking it: with the bake saved to the mod folder and forms native, most of it doesn't need to be loaded at all.
> - **KH2 player model: about 3 MB, unused.**
>   - The Player Model option has been switched off since s259, but base Sora's model and the three forms' models are still loaded at every start (about 0.8 MB each with textures). Checked (code: `model::load()` always runs; measured on P_EX100.mdlx).
>   - Shrinking it: don't load them while the option is off.
> - **Trace recorder: about 1 to 2 MB.**
>   - It's on by default and keeps the last 10 s of rows (up to 120 rows a second) with every category, in case of a dump. Checked (code).
>   - Assumption: at the ini's maximum of 120 s it could reach 15 to 25 MB.
> - **Smaller items: a few MB in total.** Assumption.
>   - Each form keeps its own parsed copy of KH2's battle tables, plus the movesets, the retarget tables and KH1's own motion file read for the rest poses.
>   - The DLL itself is 3.2 MB.
> - **Start-up peak (temporary, but it adds up).**
>   - Every file we read from either game's package re-reads that game's whole .exe from disk (KH2's is 8.7 MB, KH1's 7.5 MB) and scans it for the package key. It also re-opens the package indexes. There are roughly 100 such reads at start. Checked (code: `key_table()` isn't cached). The count is an Assumption.
>   - Sound loading also holds KH2's three sound banks at once (about 28 MB, measured on the PC) while it cuts the sounds out.
>   - Several loader threads run at the same time (KH2 Sora, model, sounds, effects, recolour). Assumption: that briefly adds tens of MB, then it's freed.
>   - Shrinking it: read the key and the indexes once per start. Once everything is baked to the mod folder, start-up reads almost nothing.
>
> ### 2. What still runs every frame
>
> In total we have about 64 function hooks in KH1, plus some byte patches. Checked (code). Most only act on an event: a hit, a command, a motion change. These are the ones that run every frame:
> - **Sora's physics hook** (`takeover`): reads input and runs the shared-state rules on KH1's states, steps KH2's controller whenever it has Sora (Drive Forms and the few fallbacks left), and fills the trace row.
> - **The skeleton-pass hooks** (`pose`). They're called for every model KH1 poses and return quickly for anything that isn't Sora.
>   - For Sora they pick the route: KH1's own animation, a baked slot, or a KH2 clip drawn over KH1's slot.
>   - Even on a baked slot (KH1 plays it itself), we still work out KH2's pose and Keyblade each frame, only so effects and Reflect can read KH2's bones. Checked (code, `follow_baked`). Your t651 trace shows every frame of that clip on the baked route.
>   - The "drawn over" route also runs our retarget and the hood / coat / shorts rules.
> - **The KH2 camera.** This is by design: it's KH1's own camera mode slot. It reads the sticks and walls each frame.
> - **Small per-frame jobs:** effect fades and bone-binding refreshes (`kh2fx` / `kh2dpx`); checking that our motion tables are still on Sora's actor (`kh2slots`); the body-hit windows (`bodyhits`); re-registering our sound banks after KH1 clears its list (`kh1bank`); the pad hook; Valor's second Keyblade draw; the debug menu and frame-advance checks; a background thread polling a key every 50 ms.
> - **Logging and the trace.** Each log line is written and flushed to disk on the game's own thread. Checked (code). The trace builds a full row every frame. Per-frame memory allocations remain in the pose path (a copied 228-bone pose and the KH2 pose itself).
>
> ### 3. What's being optimized right now
>
> - **Moving KH2's systems into KH1's own (the main performance work).** Each move removes a second simulation running alongside KH1's:
>   - the combo picks inside KH1's picker, with every KH2 combo move on a KH1 row of its own (t622, t623, t631);
>   - no more controller borrows in the combo or for the button fallbacks (t630, t639), nor for KH2 spells in states KH1 doesn't cast in (t654);
>   - KH2's body hits and spell hits as KH1 attack objects in KH1's own collision pass, with our own sweep gone (t638, t640);
>   - idle, landing, fall, jump and item use as baked KH1 slots (t625 to t627, t635, t636);
>   - flight attacks on KH1's flight rows (t648).
> - **Saving the assets to the mod folder.** From your go-ahead in the [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md) thread, t655 saves the baked KH2 motion slots in the mod folder, built once from your own KH1 and KH2 files and rebuilt only when they change. In your t651 log the baking alone took 1.24 s of loader time; that goes away after the first start. The native loading screen is planned, waiting on your answer there.
> - **Already done earlier:** one copy of each KH2 sound; effect packages built only when they can play, and freed when they can't; the draw buffer cut from 24 MB to 4 MB once the effects went native (only the debug menu still uses it).
>
> ### 4. What else needs optimizing (suggested order)
>
> 1. **Measure first.** Add a log line every 30 s: the process's real memory use, our own totals per item above, and how long each of our per-frame sections took. Then every fix below can be checked against real numbers on your machine. Without it, all I have is estimates.
> 2. **Quick RAM wins, one small version each.**
>    - Free the duplicate baked slots (about −16 MB).
>    - Don't load the KH2 player model while it's off (about −3 MB, faster start).
>    - Load the recolour source only when a form needs it, and free it after (about −10 MB).
>    - Read the package key and indexes once per start (shorter start-up peak).
> 3. **More assets to the mod folder.** After the motion slots, the same treatment for the cut sounds, the effect packages and the recoloured textures. Nothing would then be decoded or built at start.
> 4. **Stop working out KH2's pose every frame on baked slots.** The native effects already sit on KH1's bones. Whatever still reads KH2's pose (Reflect's barrier, the second Keyblade's anchors) would take KH1's bones, or points saved with the bake. Assumption: I haven't yet listed every reader.
> 5. **Drive Forms native.** That removes the last per-frame KH2 controller, and with it the forms' decoded animations and tables.
> 6. **Lighter defaults for normal play.** Write log lines from a background thread (the main branch did this in its s277), and make the trace lighter or off unless you turn it on.
> 7. **Share repeat bakes** (65 different motions out of 96 copies), if KH1's motion format allows it.
>
> ### 5. Questions
>
> 1. Measuring: (a) add the memory and timing log line first, so you and I see real numbers before and after each fix; or (b) skip it and go straight to the fixes?
> 2. The quick RAM wins in 4.2: (a) do them now as small separate versions, alongside the asset work; or (b) wait and fold them into the asset work?
> 3. The trace recorder in normal play: (a) keep it on at 10 s as now; (b) off by default, turned on from the debug menu when you want to record; or (c) on, but with only the basic categories by default?
> 4. After the motion slots, what goes to the mod folder next: (a) the sounds; (b) the effect packages; (c) the form recolours; or (d) another order (tell me which)?

^m1

