# KH2 hit effects on enemies

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)

---


> [!tip] Assistant
> I started this thread from [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md) at your request. It covers only the spark on the enemy when a hit lands. The move effects themselves stay in the effects thread, and this plan reuses that thread's player.
>
> **What we have now**
> - Every KH2 hit is landed through KH1's own contact code, using a KH1 attack object. So KH1 picks the spark: the equipped Keyblade's KH1 spark for swings, and KH1's spell spark for spells (t345).
> - No KH2 hit element is ever started. Our PAX reader also throws away the one word KH2 uses to find hit effects.
>
> **What KH2 does** (KH2 Steam decomp, `0x1403d1730` → `0x1403d1b40`)
> - An attack carries its own PAX: the Keyblade's file for a swing. That part is an assumption: I didn't trace the line that sets it, but the files fit.
> - When the hit counts, KH2 looks in that PAX for the element tagged with the ATKP row's "effect on hit" number. It starts the element on the enemy's body surface, turned along the hit's direction.
> - An immune hit (0 damage from the element) plays a system element from `00effect.bar` instead. A hit that doesn't count plays nothing.
> - Separately, the ATKP's "damage effect" plays an element from the enemy's own PAX.
> - The Kingdom Key's file (`W_EX010.a.us`) has three hit elements: effect 1, effect 2, and a wall hit.
>
> **How KH1 does it** (KH1 Steam decomp, `0x1402b4320` → `0x1401dffe0`)
> KH1 does the same lookup. It searches the attack's effect list by a kind mask and a group mask, and the group comes from KH1's attack data `+0x28`.
> - KH2's tag word is those two masks packed together: the low half is the kind, the high half the group.
> - KH1's group number is KH2's "effect on hit" minus 1.
> - Even the kinds match: 1 for a hit that counts, 4 for one that doesn't.
>
> So KH2's choice can run through KH1's own lookup, with no layer of our own.
>
> **Plan** (one system per version)
> 1. Keep KH2's tag word when reading a PAX, and write it into the KH1 effect list we build (kind mask and group mask).
> 2. Build the Keyblade package's hit elements into KH1's effect system with the effects thread's loader. That's the same path as the vanish flash, with KH2's HD textures.
> 3. In our hit landing, give the KH1 attack object that list, plus a copy of its attack data with `+0x28` set to KH2's "effect on hit" minus 1. KH1's own code then picks and places the spark. Rows with no hit effect show none, as in KH2.
> 4. Later: KH2's immune-hit spark (needs `00effect.bar` built too), and maybe the enemy's damage effect.
>
> Nothing in KH1's events or scripts reads the spark choice, so this can't break them. The cost is one build on the existing background thread. Each hit only does KH1's own search, with no new per-frame work.
>
> **What I couldn't find:** where KH2 sets which PAX a swing carries, and KH1's version of the immune-hit spark. KH1's Keyblades besides the Kingdom Key have no KH2 file of their own either.
>
> Notes: `docs/kh1-kh2-hit-effects.md`.
>
> Questions:
> 1. KH1 Keyblades with no KH2 counterpart (Lionheart, Oathkeeper as KH1 has it, Ultima Weapon, ...): (a) always the Kingdom Key's KH2 sparks, (b) a KH2 Keyblade picked for each KH1 one, if it has its own hit elements, or (c) KH1's own spark for those.
> 2. Spells: (a) keep KH1's spell sparks (t345) for now, or (b) switch them to KH2's spell hit elements once the spell packages carry their tag words.
> 3. KH2's "damage effect" on the enemy (900 ATKP rows use it): KH1's enemies have no KH2 file, so (a) skip it, or (b) use the KH1 enemy's own effect list if it has a matching entry (more research).

^m1


