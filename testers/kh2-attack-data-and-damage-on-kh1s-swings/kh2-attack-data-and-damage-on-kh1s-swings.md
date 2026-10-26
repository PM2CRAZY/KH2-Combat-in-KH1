# KH2 attack data and damage on KH1's swings

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)
- [2. Assistant: Done, in two versions.](#^m2)
- [3. Assistant: Got it. No code this round, since there's no go-ahead yet; h](#^m3)
- [4. Assistant: Done, in three versions.](#^m4)

---


> [!tip] Assistant
> I started this thread from [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md), as you asked: it's the item about the native swings using KH1's attack data and KH1's damage times KH2's power.
> 
> **What we have now.** The native swings on KH1's rows hit 100% the KH1 way: KH1's damage, reaction, element, hit spark, critical and MP charge. KH2's ATKP isn't read at all there. Our own rows (Square moves, Guard Break) and the borrowed / form moves use a KH1 row's data with KH2's reaction kind, and then we rewrite the hit's damage after KH1 made it: KH1's number × KH2's power / 100 (Magic hits also × max MP), with the leftover fraction carried to the next hit.
> 
> **What KH2 does** (Steam KH2 `0x1403C12F0`, checked in the decomp, same as our combat-core): base = Strength or Magic (by the ATKP's stat byte) minus Defense, clamped by both sides' min / max, then damage = power% × base × the element's and general weakness, rounded up; then difficulty, the element boosts, Combo Boost / Air Combo Boost / Reaction Boost, Anti Form and the low-HP rate. No critical hits and no random factor.
> 
> **What KH1 does** (Steam KH1 `0x1402BFA80`, checked in the decomp; its only caller is the record builder `0x1402BF630`): the attack data's power plus Strength minus Defense is an index into a damage curve (pointer at `0x142DE5DC0`). In your `btltbl.bin` the curve goes 1, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12 ... 270, then 300 flat: about ×1.19 a point. Then the data's min / max, the enemy's floor / cap, its element resistance, Aero halves it, mode 2 doubles damage to the party (I think that's Proud, not checked), Second Chance, at least 1. A random critical can add the data's bonus to the power first. Sora's plain swings have power 0, his finishers 4 or 6 (checked in the file). So KH1's power is extra points on an exponential curve, KH2's is a percentage on a straight line.
> 
> **Plan, one system per version:**
> 1. KH2's ATKP as KH1 attack data records: one KH1 record per ATKP Sora's movesets use, built at load from the matching KH1 row, with KH2's element, reaction and stat source. The native swing's KH1 attack object is pointed at the record of the KH2 hit window that's open, so KH1's own contact, spark, record and scripts see KH2's data. (I still have to check that repointing the object mid-row is safe.)
> 2. KH2's damage inside KH1's damage function: a hook on `0x1402BFA80` for our records only, the formula from question 1, then KH1's own tail kept (floor / cap, resistance, Aero, Second Chance), so nothing is rewritten after the fact any more.
> 3. Borrowed and form moves on the same records (with the threads that make those moves native).
> 
> Things KH1 reads that must keep working: the MP you get per hit, the critical's journal tally, and the enemy scripts that hear every hit record and read its kind and damage.
> 
> **What I couldn't find:** how KH2 picks its hit effect from the ATKP (not read yet), KH1's elements 0, 5, 6, 7 (so KH2's dark and neutral have no KH1 element yet), which KH1 difficulty is "mode 2", and how far KH2's straight-line formula would move damage against KH1's enemy HP (that needs a log).
> 
> The notes are in `docs/kh1-kh2-attack-data.md`.
> 
> Questions:
> 1. Damage: (a) keep KH1's curve × KH2's power, just moved into KH1's damage function; (b) KH2's own formula with KH1's Strength, Defense and resistances and max MP standing in for Magic (closest to KH2, but the numbers against KH1's enemy HP could change a lot); (c) KH2's formula with KH1's curve as its base term: the curve at Strength − Defense (no KH1 row power), times KH2's power%, rounded up as KH2.
> 2. Criticals: KH2 has none. (a) drop them on KH2 hits (Critical Plus then does nothing for them), (b) keep KH1's.
> 3. MP per hit: KH2 gives none, KH1's magic needs it. (a) keep KH1's MP charge on KH2 hits, (b) drop it.

^m1


1. either a or b you pick which one the player is going to feel a difference in it feeling like kh2, we're going to implement kh2s other systems like its mp mechanic and drive as a config toggle so this will effect opposite toggle for now.
2. make it a config selection (criticals on or off for all moves that call it)
3. (a) same as the 1st answer, we'll change core kh1 mechanics like that when we implement kh2s systems that are related to it.

go ahead

> [!tip] Assistant
> Done, in two versions.
> 
> **t538: KH2's power inside KH1's damage function.** I picked (a), KH1's curve × KH2's power. KH1's enemy HP is tuned to its damage curve. The curve grows about ×1.19 a point and tops out at 300. KH2's own formula grows in a straight line with Strength − Defense. On KH1's stats I expect (b) to leave the early fights about the same but make the late ones much longer (an estimate from the two formulas, not measured in a fight): a slog, not KH2. With (a) you still get what makes KH2's moves feel like KH2:
> - every move keeps KH2's power against the others (Explosion's 25% ticks, the finishers);
> - each hit is rounded up, as KH2 does;
> - with Criticals Off, there's no random factor.
> 
> How it works now: when Sora's attack hits, KH1's own damage function runs as normal. For that one call, its damage curve is swapped for a copy scaled by the KH2 hit's power. So KH1 still does everything after the curve: the clamps, the enemy's floor and cap, element resistance, Aero, Second Chance, the MP charge, the critical and the enemy scripts. Nothing gets rewritten after the hit any more, and the leftover fractions we used to carry from hit to hit are gone (KH2 rounds up per hit).
> 
> New in this version: your **native swings on KH1's rows** get KH2's power too. That's the KH2 hit window open in the move at that moment. Before, those swings were 100% KH1's damage. With Base Combo KH1, KH1's own swings keep KH1's own damage. The MP charge stays KH1's, as you said (3a).
> 
> **t539: Config > Combat > Criticals: Off / On.** Off is the default, since KH2 has no critical hits. When it's Off, Sora's attacks never roll a critical (KH1's chance reads as 0 for the hit), so Critical Plus does nothing. When it's On, it's KH1's critical as before, on KH2 hits as well. Enemies and Donald / Goofy aren't affected.
> 
> What I checked: KH1's damage function `0x1402BFA80` and its one caller (decomp). From the Steam exe: the hook's first bytes, and that the function reads the curve pointer `0x142DE5DC0` only once. What I couldn't check: how the damage numbers feel against late-game enemies; that needs a play log. The `KH2 damage:` lines in `kh1-kh2.log` show the source, the power and the result for the first 40 hits.
> 
> Not in these versions: KH2's element and reaction on the native swings (they keep KH1's row's: physical, flinch, launch on finishers, which match KH2's for Sora's plain combo) and KH2's own hit effect. Those are plan steps 1 and 3.
> 
> Please test:
> - your damage numbers in a few fights, early and late, with the plain combo, the Square moves, Explosion and a Drive Form;
> - the Criticals row both ways (with Critical Plus equipped, a critical shows KH1's critical spark);
> - that the Combat list fits on screen (it now has nine entries: this round also added a Recoil row from the recoil thread).
> 
> Questions:
> 1. KH1's finisher rows add 4–6 points to the curve before KH2's finisher power is applied, so finishers get a bit of a double boost. (a) Keep it, since that's what (a) is; (b) drop KH1's row power so only KH2's power sets the finisher (that's the old option (c)).
> 2. Criticals Off: (a) Sora only, as now; (b) everyone's attacks, enemies included.
> 3. A Config row for the damage formula itself (KH1's damage / KH2's power)? It's in the ini now as `kh2_damage = on/off`. (a) yes, in Combat; (b) no, the ini is enough.

^m2


1. unsure have to see how much damage is done with the new system so (a)
2. if party members can also do crits then yeah (b)
3. (a)


> [!tip] Assistant
> Got it. No code this round, since there's no go-ahead yet; here's what I found and what each answer would mean.
>
> **1. (a): stays as it is.** Finishers keep KH1's 4–6 row points on the curve with KH2's power on top. Once you've seen the numbers in a few fights we can drop that if finishers hit too hard.
>
> **2. Can party members crit in KH1? Yes.** I checked this in the KH1 decomp (the damage function `0x1402BFA80` and the attack object's constructor `0x1402B2DC0` / `0x1402B3270`). KH1 rolls a critical on every attack, whoever owns it. The chance and the bonus come from the attack's own data, times the owner's critical factor, times the weapon's.
> - **Donald and Goofy:** in your `btltbl.bin` their plain attack rows have a critical chance of 20 (Sora's finishers have 4) and a bonus of 0. So nearly every hit they land is a "critical" that adds their weapon's critical bonus (I haven't read their weapons' numbers yet). The guests' rows have 4 / 6, like Sora's.
> - **Enemies:** their factor is 1, so they crit only if their own attack data says so. That data comes with each enemy's files, which I haven't checked.
>
> So (b) would make Criticals Off apply to every attack: Sora, Donald, Goofy, the guests and enemies all never crit. On stays KH1's exactly. One side effect to know about: with Off, Donald and Goofy lose their weapon's critical bonus on almost every hit, so they'd deal a bit less damage than in plain KH1.
>
> **3. (a):** a Config > Combat row "KH2 Damage: Off / On" for `kh2_damage`, where Off is KH1's damage exactly. That would make the Combat list ten rows.
>
> Can I go ahead with 2 (b) and 3 (a)?
>
> One question: does the Combat list still fit on screen with nine rows? I'd like to know before I add a tenth.

^m3


1. sounds good
2. lets do (b) and in the config menu make a separate option for party member criticals
3. (a)
go ahead

> [!tip] Assistant
> Done, in three versions.
>
> - **t598: Criticals Off now covers every attack.** I checked in the KH1 decomp that KH1 rolls its critical inside its damage function for every attack, whoever made it. With Criticals Off that roll gets no chance for any attack: Sora's, the enemies', everyone's. On is KH1's own roll, untouched. What I couldn't check: which enemies' attacks can crit at all; that's in each enemy's own files, which I haven't read.
> - **t599: Combat > Party Criticals: Off / On.** The separate row you asked for. It decides for Donald's, Goofy's and the guests' attacks, and Criticals decides for everyone else. Off is the default, as in KH2. Remember from last time: their plain attacks "crit" on nearly every hit to add their weapon's bonus, so with it Off they hit a little softer than in plain KH1. KH1 tells an attack's side from a flag on its owner (checked in the decomp). I'm assuming everyone on Sora's side except Sora counts as party, so a summon's own attacks would probably follow this row too; I haven't confirmed that.
> - **t600: Combat > KH2 Damage: Off / On.** On is what you have now: KH2's power on KH1's damage. Off is KH1's damage exactly: nothing of ours touches the number, and `damage_scale` / `magic_scale` do nothing. It switches live, so you can compare the same fight both ways.
>
> The Combat list is twelve rows now, so it's drawn smaller to stay above the help box (about three quarters of KH1's size).
>
> Please test:
> - Criticals Off with Critical Plus on: no critical sparks from Sora, the Heartless, or Donald / Goofy (with Party Criticals Off);
> - Party Criticals On: Donald and Goofy crit again, while Sora's stay off;
> - KH2 Damage Off vs On on the same enemy: Off should match plain KH1's numbers;
> - whether the Combat list is still easy to read at its smaller size.
>
> Questions:
> 1. Guests and summons: (a) keep them under Party Criticals, as now; (b) Party Criticals only for Donald and Goofy, with guests under Criticals.
> 2. If the Combat list reads too small: (a) leave it; (b) move KH2 Damage, Criticals and Party Criticals into their own "Damage" list inside Combat, like Tweaks.

^m4


