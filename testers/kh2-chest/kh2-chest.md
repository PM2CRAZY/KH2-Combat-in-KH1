# KH2 Chest

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Mr Bugs: I want to add a config in the tweaks menu that lets use quic](#^m1)
- [2. Assistant: t724 adds a new row in Combat > Tweaks: Chest Opening (KH1  ](#^m2)

---


  

> [!note] Tester Mr Bugs
> I want to add a config in the tweaks menu that lets use quickly open and move from the chest opening animation like in kh2, right now in kh1 when you open a chest it plays an animation then your locked in idle and a window appears telling what you got (player assumption) this version I want it to be like kh2 where you open the chest, move then the pop up for the item plays at the top of the game letting you know what you got. start coding for it
> 
> go ahead

^m1


> [!tip] Assistant
> **t724** adds a new row in Combat > Tweaks: **Chest Opening** (KH1 | KH2). It stays on KH1 until you switch it.
>
> **How KH1 opens a chest** (from its code): the Open command gives you the item right away, before any animation. Then it locks Sora, plays his open motion, and starts a small chest script for the lid and the "obtained" window, which holds you until you close it.
>
> **On KH2,** for a normal chest (an item or munny):
> - KH1's own code still opens it and gives the item, the same way as before.
> - Sora is let go at once, so you can walk off straight away.
> - The chest script still runs (the lid, the sounds), but its "obtained" window doesn't open.
> - "Obtained" plus the item name (or the munny) shows in KH1's own notice box, the one level-ups use, with nothing to press.
>
> Chests that start a scene, reward chests that aren't items, and a full inventory still play KH1's way.
>
> **What I checked:** KH1's chest code in the decomp and exe. What I couldn't check: the chest's own script file (too big to fetch this time), KH2's exact chest timing (KH2's chest code wasn't in the part of the decomp I had), and anything in game.
>
> **Please test:**
> - A few plain chests and a munny chest. Do you walk off right away? Does the lid still open, with its sound? Is the item really in your inventory? Does anything pull Sora back?
> - An event chest, and a chest with an item you already have the most of. Both should be unchanged.
> - Log lines (Menus log): "Chest Opening: KH2 open (kind ...)", "... message function N skipped", "... window time is over".
>
> **Questions:**
> 1. Where should the notice go? (a) KH1's level-up box where it is now, or (b) the top of the screen like KH2's info bar. (b) means teaching KH1's HUD a new notice spot.
> 2. Default: (a) KH1 by default, as now, or (b) KH2 by default.
> 3. The open motion: (a) Sora plays KH1's open motion until you move, as now, or (b) no open motion at all, like KH2.

^m2

