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
