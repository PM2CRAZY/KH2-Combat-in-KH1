# KH2 sounds through KH1's sound system

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)
- [2. Assistant: t509 to t513. KH2's sounds now go through KH1's own sound sy](#^m2)

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