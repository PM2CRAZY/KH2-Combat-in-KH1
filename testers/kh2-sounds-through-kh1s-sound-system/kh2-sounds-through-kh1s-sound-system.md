# KH2 sounds through KH1's sound system

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)
- [2. Assistant: t509 to t513. KH2's sounds now go through KH1's own sound sy](#^m2)
- [3. Assistant: t514 and t515. I think I found it, though without a log I ca](#^m3)
- [4. Assistant: Yes, we're caching sounds, and part of that cache is older t](#^m4)

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
