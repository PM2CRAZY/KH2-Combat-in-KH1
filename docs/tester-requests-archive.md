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
