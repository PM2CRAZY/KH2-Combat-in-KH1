# Tester requests archive

Closed threads from `tester-requests.md`, moved here word for word by the assistant when a tester wrote `Close`. Newest at the bottom. Nothing here is an open request, and it isn't read for new work unless someone asks.

Each thread keeps its title and ends with a `Closed` line (the commit history shows when).

---

  

## Chat history

  

> [!note] Tester Mr Bugs
> 
>I've logged all my past sessions of looking through the source code with claude, please look through the tester history to be up to date with everything I was working on 
>
>at chat-history-log.md
>
>go ahead

> [!tip] Assistant
> I read all of `docs/chat-history-log.md`. What I'm taking from it:
>
> - The rules in it are already in `design-philosophy-instructions.md` (native always, KH2's numbers through KH1's own systems, open scope, performance, no decompiled code in the repo), and I follow that file.
> - From M004: animations are baked raw from KH2 into KH1's motion slots, and hair, hood and chain movement is baked into them, not physics. I passed that on for the sleeves thread.
> - Effects: Explosion's step 1 is done (t368 to t370). The element layer, KH2's `pppRyjMegaBirth`, `pppEiDrawShipoly` and retiring the parallel player are still open. That's the effects thread below.
> - Hit effects: KH2 spell hits now play the KH1 spell's own spark through KH1's contact code, but deciding that a hit lands is still KH2's code running beside KH1's. That part isn't native yet.
>
> If there's a later chat you want me to have too, add it to the same file and tell me.

> [!note] Tester Mr Bugs
> close

Closed

---

## Testing the new system

> [!note] Boogle
> This is a test on the new system, did you get it?

^m1


> [!tip] Assistant
> Got it, loud and clear. Your message came through the automatic update round: I picked it up from the Requests section, gave it a title ("Testing the new system") and moved it into its own thread file, `testers/testing-the-new-system/testing-the-new-system.md`, which is where I'll answer from now on.
>
> For real requests, the flow is the same: start a new thread at the bottom of `docs/tester-requests.md` (under "Add new threads below this line"), or answer in an existing thread's file under your own `> [!note] Boogle` box. I'll reply in a box like this one, ask anything I need, and only start coding once you write "go ahead". Traces and logs for a thread go in that thread's folder. When you're done with a thread, write "Close" and I'll archive it.
>
> Nothing to test here. If you're happy the system works, write "Close" and I'll archive this one.

^m2


this is another test to see if tester folders are working, did you get it?


> [!tip] Assistant
> Got this one too. The folder side works: this thread lives in `testers/testing-the-new-system/`, and anything you drop in that folder next to this file (Select + Triangle trace CSVs, a `kh1-kh2.log`) gets read along with the thread when I pick it up.
>
> One small thing: your message came through as plain text under my box rather than in a `> [!note] Boogle` box. I still picked it up (plain text after my reply counts as a message), but the box keeps the Messages list tidy, so use it when you can.
>
> Nothing to test here. Write "Close" when you're done with this thread and I'll archive it.

^m3

close

Closed

---

## Invulnerable frames native

> [!tip] Assistant
> This thread is split out of [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md) (your message after message 19), as you asked. Your request, word for word:
>
> > Sora's invulnerable frames (roll, Quick Run, Explosion, Aerial Recovery) [...] are decided in our hit-pass hook, not by KH1's own no-damage timer [...]. have the agents do most native implementation. go ahead
>
> (The Keyblade guard's facing, the other half of that line, has its own thread: [Keyblade guard facing native](../keyblade-guard-facing-native/keyblade-guard-facing-native.md).)
>
> **Where it stands now (from the code and AGENTS.md):**
> - KH2's roll (t523): KH1's own 12-frame no-damage timer is taken off, and our hit-pass hook (`hurt`) throws hits away for KH2's frames by level (10 / 16 / 24 / 32).
> - Quick Run (t504): `hurt` throws hits away for KH2's frames (0 / 8 / 12 / 16 by level).
> - Explosion (32..64) and Aerial Recovery (0..16): their invulnerable ranges are read from KH2's motion and checked in `hurt` each hit.
> - KH1 already has what we need to put them on: its own no-damage timer (`0x140297F30`, actor `+0xC` and flag `0x80`), which its roll and recoils use, and the untouchable flag `0x40` that its hit pass skips. Where KH1's own roll gets `0x40` is still not found.
>
> **The plan:** each move's invulnerable range starts KH1's own timer (or the `0x40` flag) for exactly KH2's frames, from the move's own data. Then KH1's hit pass skips the hits itself, and the `hurt` checks for these moves go away. It's the same for KH1's and KH2's moves, so nothing changes at KH1.
>
> Your go-ahead is noted. The work starts in the next round with the PC connected, because it needs the decomps to check which of KH1's two mechanisms matches KH2's rule. That includes whether KH2's invulnerability also skips the hit effect and the grab, as the `0x40` flag does. This round the PC couldn't be reached. Overlaps: [Dodge Roll, Guard and Counterguard native](../dodge-roll-guard-and-counterguard-native/dodge-roll-guard-and-counterguard-native.md) (t523's roll) and [Quick Run native](../quick-run-native/quick-run-native.md).

^m1

Closed (booglechan, in chat: finished outside the tester file)

---

## Keyblade guard facing native

> [!tip] Assistant
> This thread is split out of [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md) (your message after message 19), as you asked. Your request, word for word:
>
> > Sora's invulnerable frames [...] and the Keyblade guard's facing are decided in our hit-pass hook, not by KH1's own [...] guard objects. have the agents do most native implementation. go ahead
>
> (The invulnerable frames have their own thread: [Invulnerable frames native](../invulnerable-frames-native/invulnerable-frames-native.md).)
>
> **Where it stands now (from the code and AGENTS.md):**
> - The body guards (Aerial Dodge, Aerial Recovery, Explosion, Magnet Burst) are already KH1 guard objects that KH1's own clash meets.
> - The Keyblade guard (the guard press 161 and the repel 162) is still checked in our hit-pass hook (`hurt`). It compares the attacker's direction with Sora's facing against each guard ATKP's own angle (t525: 120° either side for 161, all round for 162).
> - KH1's guard objects can already be directional: type bits `0xE0000` set to 1 or 3, with an arc around the owner's facing (`0x1402B24B0`). Which way KH1 turns the arc's angle is not checked yet.
>
> **The plan:** the Keyblade guard's window puts a KH1 guard object on the Keyblade, with KH1's own directional arc set to the ATKP's angle. KH1's clash then decides the block, and the facing check in `hurt` goes away.
>
> Your go-ahead is noted. The work starts in the next round with the PC connected, because KH1's arc test needs the decomp to match KH2's (`0x1403D1F40`) exactly. This round the PC couldn't be reached. Overlaps: [Dodge Roll, Guard and Counterguard native](../dodge-roll-guard-and-counterguard-native/dodge-roll-guard-and-counterguard-native.md) (t525) and [Body guards on KH2 moves](../body-guards-on-kh2-moves/body-guards-on-kh2-moves.md).

^m1

Closed (booglechan, in chat: finished outside the tester file)

---
