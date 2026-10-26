# Table of contents

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Mr Bugs: from now on can you add a table of contents to the document ](#^m1)
- [2. Assistant: Done. There's a Contents list at the top of the file, under ](#^m2)
- [3. Mr Bugs: can I get sub tables so I can click on the exact message for](#^m3)
- [4. Assistant: Done. Under each thread in the Contents there's now a link t](#^m4)
- [5. Mr Bugs: response to notes](#^m5)
- [6. Mr Bugs: I can't seem to click on them in obsidian, can you fix it fo](#^m6)
- [7. Assistant: Fixed. The old links used HTML anchors, which GitHub underst](#^m7)
- [8. Assistant: Done: it's its own thread now, RAM use and what's being opti](#^m8)

---


  

> [!note] Tester Mr Bugs
> from now on can you add a table of contents to the document  so I can quickly access each thread
> 
> go ahead

^m1


> [!tip] Assistant
> Done. There's a Contents list at the top of the file, under the title, with a link to every open thread. I added a rule to the assistant section so it stays current: each new thread gets a line when it's first answered, and its line comes off when it's closed.

^m2


> [!note] Tester Mr Bugs
> can I get sub tables so I can click on the exact message for each thread
> 
> go ahead

^m3


> [!tip] Assistant
> Done. Under each thread in the Contents there's now a link to every message in it, numbered, with who wrote it and its first words. Each message has a small anchor above it for the link to jump to. I'll keep them current with every reply. I also closed the Chat history thread and moved it to the archive.

^m4


> [!note] Tester Mr Bugs
> response to notes

^m5


> [!note] Tester Mr Bugs
> I can't seem to click on them in obsidian, can you fix it for me?
> go ahead
>

^m6


> [!tip] Assistant
> Fixed. The old links used HTML anchors, which GitHub understands and Obsidian doesn't. Now they use Obsidian's own block links: each message has a block id under it (`^m1`, `^m2`, ...), and the links point at those (`#^m3`).
>
> Also, every thread is now its own file in its folder in `testers` (you asked for that in the frame advance thread). So:
> - `docs/tester-requests.md`'s Contents links to each thread's file, and to each message in it.
> - Each thread file starts with its own Messages list linking to every message in that thread, and a link back to the main file.
>
> One thing to check: the links point to the files by their path from the repo, so open the repo folder as your Obsidian vault (or a folder above it). If a link still doesn't open, tell me which one and how your vault is set up.

^m7


make another thread detailing why ram usage is so high for our mod, what else needs to be optimized and is currently being optimized etc

go ahead

> [!tip] Assistant
> Done: it's its own thread now, [RAM use and what's being optimized](../ram-use-and-whats-being-optimized/ram-use-and-whats-being-optimized.md). In short, the mod keeps roughly 80 to 90 MB of its own, about 30 MB of which is duplicates or things nothing uses (the biggest: the baked animations kept twice). It lists what still runs every frame, what's being optimized now and what's next, plus a few questions for you there.

^m8

