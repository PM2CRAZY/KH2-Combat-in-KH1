# KH2 attack data and damage on KH1's swings

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)

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


