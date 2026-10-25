# KH2 sounds through KH1's sound system

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)
- [2. Assistant: t509 to t513. KH2's sounds now go through KH1's own sound sy](#^m2)
- [3. Assistant: t514 and t515. I think I found it, though without a log I ca](#^m3)
- [4. Assistant: Yes, we're caching sounds, and part of that cache is older t](#^m4)
- [5. Assistant: t565 to t567. The old player is gone, every KH2 sound now go](#^m5)
- [6. Assistant: 2. Epic: done (docs only, no new version). I added a note to](#^m6)
- [7. Assistant: t568, KH2's quote cut. With Battle Quotes on KH2, a new KH2 ](#^m7)

---


> [!tip] Assistant
> I started this thread from [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md), as you asked: KH2's sounds and battle quotes going through KH1's own sound system.
>
> **What we have now.** We read KH2's sound banks from your KH2 install, decode every sound to raw audio ourselves, and play each one on its own Windows audio device. We read KH1's volume sliders once when a sound starts. So KH1's pause, Frame Advance's held sound, room changes, panning and a slider moved mid-sound don't touch them.
>
> **The good news from the decomps.** KH1 and KH2 on PC use the same Square Enix sound engine. The bank reader is line-for-line the same code in both exes, and KH1's exe has both codecs KH2's banks use (ADPCM and Ogg Vorbis). So KH1 should be able to load a KH2 `.scd` as it is, with no decoding on our side. I haven't tried it yet, so that's still an assumption.
>
> **How KH1 does it.**
> - KH1 keeps almost every effect as its own one-sound file (`se034072.win32.scd` and so on), plus one indexed system bank (`se000`, bank 2, the menu sounds).
> - Game code asks for an effect through a queue (`0x140179AC0`: sound, id, position, volume). Footsteps, hits and objects use it. Each frame `0x1401791E0` creates and plays the queued sounds and pans them from their position.
> - Every playing sound is a record in one list (`0x1404D6618`). KH1's pause (`0x1400DE010`) walks that list, and so does our Frame Advance, which already calls it. Volume goes by bus (1 music, 2 effects, 3 voice), set from the sliders.
> - Banks are entries in a list (`0x1404D65D8`). `0x1400D2C70 (1)` releases all of them (most likely on room loads; I haven't confirmed that), so our banks would have to be registered again after each call.
>
> **Plan, one system per version.**
> 1. Load the KH2 banks we use into KH1's engine with KH1's own loader, again after every bank reset, and log it. Nothing plays through it yet.
> 2. Play KH2's effects through KH1's create and play, so pause, Frame Advance, volume and room stops all work the way they do for KH1's sounds. There are two ways to do this (question 1).
> 3. The Keyblade bank: its sounds are tagged for bus 8 in KH2's file (effects are 2 and voices 3). I don't know yet what bus 8 does in KH1. They might ignore the effects slider or be silent, so I'll check and retag them if needed.
> 4. Battle quotes as KH1 voice records, so KH1's voice priority and the voice slider apply.
> 5. Remove our Windows audio player, keeping only its loader.
>
> **What I couldn't check yet:** KH1's own `.scd` version (I have no KH1 sound file extracted to compare), which buses KH1 sets up, and exactly what calls the bank reset. The notes and all addresses (Steam) are in `docs/kh1-sound-native.md`.
>
> Questions:
> 1. Effects: (a) cut each KH2 sound we use into its own one-sound bank, the way KH1 stores its own, and send it through KH1's effect queue with Sora's position, so it pans like KH1's sounds. This is the most native, but it needs a small `.scd` writer. Or (b) load whole KH2 banks and make KH1's sound record ourselves, the way KH1's create does for its indexed bank. That's less work, but no queue, and the sounds stay centred unless I add the position.
> 2. Battle quotes: (a) through KH1's voice path, so a KH1 line of higher priority isn't talked over and a KH2 quote can be cut the way KH1's are, or (b) as effects on the voice bus (the voice slider still applies, but there's no priority between KH1 and KH2 lines)?
> 3. Positioned sounds: (a) at Sora (KH1's effects pan by where they are), or (b) centred, as now?

^m1


1. a
2. what ever is closet to how kh1s systems handle it
3. same as 2

go ahead


> [!tip] Assistant
> **t509 to t513.** KH2's sounds now go through KH1's own sound system (steps 1 to 4).
>
> - **t509:** each KH2 sound gets cut into its own one-sound bank, in exactly the format KH1 uses for its own. To check that, I cut three of KH1's own sound files with it and got back byte-for-byte identical files.
> - **t510:** those banks are registered with KH1's own loader and put back whenever KH1 clears its banks (on a room change, I believe).
> - **t511:** your 1 (a) and 3. Effects play through KH1's effect queue at Sora's position, so KH1's pause, Frame Advance, the volume sliders, panning and room stops apply to them.
> - **t512:** the Keyblade's sounds were on a bus KH1's effects slider never touches. They're on KH1's effects bus now, which matches KH2, since KH2's effects slider covers that bus too.
> - **t513:** your 2. Battle quotes go through KH1's own voice request on Sora's voice channel: KH1's priority rule, KH1's channel, Sora's mouth movement and the voice slider. The line is picked the way KH2 picks it.
>
> Anything KH1 can't take still plays on the old player, and `kh2_sound_engine = mod` in the ini switches back to it completely. I'll remove the old player (step 5) once you've heard this working.
>
> **Heads-up:** some sounds from KH2's main bank will sound different, including Magnet Burst, Thunder and some of Reflect's. Our old reader was playing the wrong audio for those 35 sounds; KH1's engine now plays the ones KH2's own data points to.
>
> **Check:** the Steam KH1 and KH2 decomps and your sound files. **Not checked:** that KH1's engine actually plays the cut banks and KH2's Vorbis voice clips. That's the big thing to hear.
>
> Please test:
> - Explosion, Magnet Burst, Thunder, Reflect, and a few combos with KH2 quotes on;
> - pausing mid-sound;
> - the effects and voice sliders;
> - sounds panning when the camera turns;
> - sounds after changing rooms.
>
> Please send the log too; the lines starting "KH1 banks:" are the ones I need.
>
> Questions:
> 1. KH2 has one quote at priority 0, and KH1's rule never lets that one through. (a) play it whenever Sora isn't already talking (now), or (b) never?
> 2. Two of Sora's quote lists include a partner's line. (a) Sora says one of his own there (now); (b) the partner says their KH1 line; or (c) nothing?
> 3. Is it OK that those 35 sounds now play KH2's own audio? (a) yes; (b) no, keep the old ones.

^m2


crash when starting the game sent the log

if you find the reason I give you permission to fix it 

go ahead
> [!tip] Assistant
> **t514 and t515.** I think I found it, though without a log I can't be sure.
>
> - **t514, the crash:** when the game starts, the mod hands KH2's battle voice bank to KH1's sound engine. The KH1 function it calls (`0x1400E0110`) takes one more piece of information than I gave it: the bank's name. KH1 uses that name as text right away. Without it, KH1 read whatever garbage was lying around as text and crashed a few seconds after starting, as soon as its sound system was up. It gets a proper name now. Your settings didn't avoid it: the voice bank was loaded even with Battle Quotes on KH1. Checked: KH1's exe, both this function and KH1's own voice loader, which takes the name the same way.
> - **t515:** `kh2_sound_engine = mod` in the ini now really keeps everything out of KH1's sound engine. Before, it only switched which player played the sounds, so it couldn't have worked around the crash.
>
> Please try starting the game and loading a save, then the sound tests from my last message.
>
> One request: there was no log in the folder this time, only the ini. Next time, could you push `kh1-kh2.log` and `kh1-kh2.prev.log` from the game folder (next to the game's .exe) into this thread's folder? After a crash, the line starting with `CRASH` tells me exactly where it happened.

^m3


why did the ram use go up? are we caching a bunch of sounds? if so you need to tell me because this goes against our rules of optimization so I need a solution presented.

> [!tip] Assistant
> Yes, we're caching sounds, and part of that cache is older than t509. Sorry, I should have told you when I added to it. Here are the numbers, measured from a t515 log on your PC (the one in the effects thread: 104 sounds cut, 227 sound slots loaded) and from the KH2 sound files:
>
> **Before t509 (already there):**
> - **~56 MB:** every KH2 effect the mod uses, decoded to raw audio and held for the whole session. That's 227 slots but only 104 different sounds. The same sound gets decoded once per thing that uses it (Reflect's three levels and the forms' spells share many), so about 30 MB of it is duplicates.
> - **~2-3 MB:** Sora's battle quotes, decoded the same way. This one is an estimate: it's 31 mono Vorbis clips, and I didn't measure how long they are.
> - **~28 MB for 2-3 s at startup:** `se000` (17.4 MB), `se999` (7.5 MB) and `se500` (2.9 MB) are read whole to cut the sounds out, then freed.
>
> **What t509-t515 added (about +18.6 MB, which is the rise you saw):**
> - **9.0 MB:** our own copy of the 104 cut one-sound banks (still compressed ADPCM). We keep it so we can register the banks again after KH1 clears its bank list.
> - **9.0 MB:** KH1's copy of those same banks. KH1 frees bank data itself when it clears the list (`_aligned_free` in `0x1400DBAC0`), so each registration has to hand it its own copy.
> - **0.3 MB + 0.3 MB:** the battle voice bank, ours plus KH1's copy.
> - **About 0** inside KH1's sound engine: a bank there takes a record from a pool that already exists and plays straight from the data (`0x14005E0B0`).
>
> So we're at about **77 MB now, against about 58 MB before t509**. Most of it is the old decoded audio, which only exists for the old Windows player that's now just a fallback.
>
> **How KH1 does it (Checked (decomp)):** KH1 keeps its sounds compressed and decodes them as they play. It reads a room's sound files when the room loads, and hands the loaded file straight to its engine: `0x1400E0A50` registers it with `0x1400E0F00`, then clears its own pointer, so only one copy ever exists. At map tear-down it frees them all at once (`0x1400D2C70(1)` → `0x1400DBAC0`). It never keeps a decoded copy, and it never frees one bank on its own.
>
> **Options:**
> - **(a) Finish step 5 (≈18.6 MB, down from 77):** remove the decoded audio and the old Windows player, and keep only the compressed banks (ours plus KH1's). Downside: no fallback, so `kh2_sound_engine = mod` goes away, and a sound KH1's engine won't take just doesn't play (KH1 drops its own the same way). I'd only do this once you've heard the native path working.
> - **(b) (a), but KH1's own lifetime (≈9.3 MB steady):** we keep no copy. After each KH1 bank clear, a background thread reads the sounds again from your KH2 files, cuts them and hands each one to KH1, the way KH1's own file loader does. The cost is about 17 MB for a moment per room load (one KH2 bank at a time) plus some disk and CPU work, and KH2 sounds could be missing for a moment right after a room loads (I haven't measured how long).
> - **(c) (a) or (b), loading only what can play right now (≈2.5-4 MB in KH1):**
>   - base Sora's sounds always (2.2 MB);
>   - Reflect's at your Reflect level (~0.3 MB);
>   - a form's sounds only while you're in that form (0.3-1.5 MB each; all the forms together are 5.9 MB).
>
>   The catch: KH1 never frees a single bank (it only clears them all at once), so dropping a form's sounds on revert means doing by hand what `0x1400DBAC0` does to each entry. That's an assumption until I check it in the exe, so it's the riskiest of the three.
>
> Separately, as a quick fix while the native path is still being tested: I can decode each sound only once. That keeps the fallback but saves ~30 MB (77 → ~47 MB).
>
> I haven't changed any code. **Checked:** the decomp for KH1's bank load, registration and release; your KH2 files for the sizes; your t515 log for the counts. **Not measured:** the voice clips' decoded size, and the Windows heap's own overhead.
>
> Questions:
> 1. Which one? (a) (needs your OK that the native sounds work first), (b), or (c) on top of (a) or (b)? My pick is (a) as soon as you've heard the native sounds, then (b).
> 2. Until then: (a) the quick decode-once fix now (keeps the fallback), or (b) leave it as it is until question 1 is done?

^m4


I want you to gut the old system and transfer everything, research into whats needed to get it working properly, I give you full permission to do what you need to do to get it fully working

go ahead


> [!tip] Assistant
> **t565 to t567.** The old player is gone, every KH2 sound now goes only through KH1's engine, and the RAM is down.
>
> - **t565, the old system out.**
>   - Removed: the Windows audio player, all the decoded audio, its own volume reading and the `kh2_sound_engine` ini switch.
>   - The loader now only cuts the sounds out of KH2's banks, still compressed, hands them to KH1 and frees the banks. KH1 decodes them as it plays them, the way it does its own.
>   - Effects, Keyblade swings, magic, Reflect, the forms and the battle quotes all go through KH1's effect queue or voice channel now. There's no fallback: if KH1 can't take a sound at that moment (for example right after a room load, before our banks are back), it isn't heard, just like KH1's own sounds. Those cases are logged as "KH2 sound not played".
>   - Footsteps were already KH1's own.
>   - Check: your KH2 files (every effect sound we use is ADPCM, which lets me read each sound's length from its header without decoding) and KH1's queue path from t511.
> - **t566, RAM.** There's now one copy of each sound instead of two.
>   - KH1 only frees a bank's data if the entry still points at it. So right after KH1 takes one of ours, I clear that pointer: KH1's room reset unloads the sound from its engine but leaves the data, and we hand the same data back.
>   - Steady use should be about 9-10 MB in total. It was ~77 MB at t515 and ~58 MB before t509.
>   - Startup still reads the three big KH2 banks for a couple of seconds (~28 MB) and frees them.
>   - Check: KH1's decomp (its bank release and everything else that reads the list) and the exe. Not checked: that KH1's engine never writes anything else into a bank while playing it. It does write one byte when it loads one. I check each bank before reusing it, so if something else changed, that sound is left out and logged instead of crashing.
> - **t567, battle quotes.** Your logs from t544 showed that no KH2 quote ever played: every one said "KH1 kept the line playing".
>   - The cause: KH1's attack row starts its own swing grunt a couple of frames before the KH2 motion asks for its quote. KH1's rule then keeps Sora's channel for the grunt.
>   - With Battle Quotes on KH2, KH1's row grunt is now left out on moves that play a KH2 animation, so the KH2 line takes its place, as in KH2. Hurt lines, spells and KH1's own moves keep KH1's voice.
>   - Check: KH1's exe (the call), its decomp and your btltbl (Sora's swing rows carry voice 4 / 5).
>   - Not checked: in game.
>
> **What's left and why:**
> - I didn't do "only load what can play right now" (forms' sounds only in the form). KH1 never frees a single bank by itself, so it would mean doing that by hand, and with t566 the whole set is only ~9 MB.
> - On the Epic build no KH2 sound plays at all now, since the old player was the only thing working there.
>
> Please test:
> - Explosion, Magnet Burst, Thunder, Reflect, Valor / Final swings and a few combos with Battle Quotes on KH2;
> - the quotes in particular: do you hear them now, and no KH1 grunt on top?
> - pausing mid-sound, the sliders, changing rooms a few times (the sounds should still be there after);
> - RAM in Task Manager against t515.
> - Please send `kh1-kh2.log`. The lines I need start with "KH1 banks:", "KH2 sound not played" and "KH2 quotes:".
>
> Questions:
> 1. In KH2, each swing's line cuts off the line Sora is still saying. KH1's voice system can't cut a line, so right now the next quote is skipped while the last one is still playing. That means fewer quotes than in KH2. (a) Leave it like KH1. (b) Add a voice cut to KH1's engine so KH2's rule works 1:1 (more work, and I'd add to KH1's own voice handling).
> 2. Is it fine that the Epic build gets no KH2 sounds? (a) Yes, Steam only. (b) No, I'd need the Epic addresses for the same sound functions.

^m5


1. (b)
2. right now yes update the doc to notify that after we're done with steam we'll update all the missing hooks so that epic works exactly like the steam version


> [!tip] Assistant
> 2. **Epic: done (docs only, no new version).** I added a note to `docs/kh1-sound-native.md` ("Epic build: not yet") and to `docs/engineering-rules.md`. For now the KH2 sounds are Steam-only. Once the Steam work is done, every hook that's Steam-only today gets its Epic address, so the Epic build works exactly like Steam.
> 1. **(b), the voice cut.** Here's what I'd build. I want your go-ahead first.
>    - **What KH2 does (Checked (decomp)).** KH2's quote request (`0x1403CE580`) takes a new line if nothing is pending or playing, or if its priority is at least the last line taken. Its voice step (`0x1403CE260`) then stops the line that's playing and starts the new one. So each swing's line cuts the one before.
>    - **What KH1 lacks (Checked (decomp)).** KH1's voice channel step (`0x140290A50`) starts a new clip without stopping the old one. Its only voice stop is the stop-all (`0x1400DE010 (2)`), and `SslSeStop` (`0x1400DEF50`) only stops effects.
>    - **The plan.** Add a single-voice stop to KH1's engine. It finds the live record of Sora's channel in KH1's live sound list (`0x1404D6618`) and stops just that record, the way the stop-all stops each one. A KH2 quote whose priority is at least that of the line playing then stops it and takes the channel, through KH1's own clip request, mouth motion, bus and pause. KH1's own voice rule stays as it is for everything else.
>    - **Assumption until I check the exe:** that stopping one voice record with the stop-all's per-record step is safe on its own. I'll confirm that in the decomp before writing it.
>
> Question: which lines may a KH2 quote cut? (a) Only Sora's own KH2 quotes, so KH1's own voices (hurt lines from KH1's damage, spells, scripts) are never cut; or (b) any of Sora's lines, the way KH2 cuts whatever he's saying when the new line's priority is at least as high? Either way it only applies with Battle Quotes on KH2. At KH1 nothing changes.
>
> Say go ahead with your pick and I'll build it as its own version.

^m6


do your recommendation

go ahead 


> [!tip] Assistant
> **t568, KH2's quote cut.** With Battle Quotes on KH2, a new KH2 line now cuts the one Sora is still saying when its priority is at least the playing line's. A lower one is dropped. That's KH2's rule (its quote request `0x1403CE580` and voice step `0x1403CE260`).
>
> - **Your pick, (a):** KH2 quotes only cut Sora's own KH2 quotes. KH1's voice channel doesn't store a priority for the line it's playing, only "free, someone else, or the same speaker", so there's nothing to weigh a KH2 line against one of KH1's. KH1's own lines (hurt lines, scripts, events) are never cut, so cutscenes can't break.
> - **A correction while checking:** KH1's "stop-all" I'd planned to borrow only pauses voices (it's the same call as the pause menu's), so it couldn't stop one. The cut uses KH1's real stop instead, the one KH1 itself uses to stop an effect and to clear finished sounds each frame. The new line then goes through KH1's own channel as before, with the mouth motion, the voice slider and pause all applying.
> - **Safety:** we only stop the old line and free the channel the way KH1 does when a line ends. KH1's own per-frame cleanup removes the stopped sound from its list, so we never edit that list ourselves.
>
> **Check:** KH2's decomp (the quote rule), KH1's decomp (the channel, voice records and cleanup) and the exe (the stop's first bytes and how KH1 calls it). Not checked: in game, and that the stop cuts a voice instantly. Notes are in `docs/kh1-sound-native.md`.
>
> Please test, with Battle Quotes on KH2:
> - a few ground and air combos: each swing's line should cut the last one, not overlap it or get skipped;
> - getting hit mid-quote: hurt lines are higher priority, so they should cut a swing line;
> - Explosion: its lines are priority 2, so a priority-1 swing line asked while one plays should be dropped;
> - Reflect's finisher line;
> - talking to someone or a cutscene right after a fight: KH1's own lines should be untouched;
> - pausing mid-line.
>
> Please send `kh1-kh2.log`. The lines I need start with "KH2 quotes:" and "KH1 banks: KH2 quote clip".
>
> Question:
> 1. Does the cut sound abrupt? KH2 cuts with no fade, and I matched that. (a) keep it like KH2, or (b) add a very short fade-out (KH1's stop takes a fade length).

^m7

