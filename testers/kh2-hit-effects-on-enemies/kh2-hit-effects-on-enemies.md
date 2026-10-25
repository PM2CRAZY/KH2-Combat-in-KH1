# KH2 hit effects on enemies

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)
- [2. Assistant: Thanks, built with your answers:](#^m2)

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


1. (c) but for now wait on changing any base effects the keyblade calls as we're going to import models from kh2 and have a switch between kh1 and kh2 keyblades when the time comes
2. (a) if a vanilla magic attack calls for its vanilla spark then let it, if its from kh2 let it call its own spark
3. (b)

go ahead

> [!tip] Assistant
> Thanks, built with your answers:
>
> - **t520**: our KH2 effect reader now keeps the word KH2 uses to find hit effects, and writes it into the KH1 effect list we build, as KH1's own kind and group masks. Nothing looks different from this one on its own.
> - **t521**: KH2's spells now show their own hit spark. That covers KH2-style Fire / Blizzard / Thunder (base Sora with Magic on KH2, and the forms' KH2 casts), Wisdom's shots and Reflect's pop. The hit hands KH1 the spell file's effect list and a copy of the attack data with KH2's group in it, and KH1's own contact code picks, places and turns the spark. A row with no hit effect shows none, as in KH2.
> - Unchanged, as you asked: Keyblade swings keep KH1's spark for every Keyblade, and KH1's own spells keep theirs. With `kh2_effects` off, or before a spell's package is built, KH2 spells fall back to KH1's spark as before.
>
> 1:1 checks: KH2's lookup (`0x1403d1b40`) and KH1's (`0x1402b4320` → `0x1401dffe0`) in the Steam decomps, and every spell file's hit elements in your KH2 install. Fire, Blizzard, Thunder, Magnet, Reflect and Wisdom's archive each have their own kind-1 hit elements for effect-on-hit 1 / 2. What I couldn't check: that a KH2 spell attack really carries its spell file's effects. I didn't find the line in KH2 that sets it; the files fit, but it's an assumption.
>
> Smaller notes:
> - KH1 asks for a hit that doesn't count as its kind 2, which KH2 uses for walls. So in this list KH2's "doesn't count" kind is put on KH1's 2, and KH2's wall kind is left out. Spells hit bodies only through this path, so no wall spark is lost.
> - If a spell's hit element uses a particle program KH1 can't run as KH2 does yet, that hit shows KH1's spark instead of nothing. The load line says which.
> - The hit **sound** isn't touched: spells without KH1 data still play the swing's hit sound.
>
> **Question 3 (b), the enemy's own damage effect.** KH1 never looks into the enemy's own effect list when a hit lands. I checked every caller of its lookup. Its closest thing is a "damage effect" taken from the **attacker's** list (kind 0x80, `0x1402bf630`), which KH1 plays on a random body part of the enemy during its reaction. KH2 spells' lists have no such entry, so they play none there. That matches KH2, whose attacker side plays nothing at that point. So (b) needs more digging: where a KH1 enemy keeps its own effect list, and whether those lists hold anything a KH2 "damage effect" number could point at.
>
> Please test:
> - KH2-style Fire / Blizzard / Thunder on enemies, Wisdom's shots and Reflect's pop: is the spark KH2's, placed on the enemy where the hit lands and turned with the hit? The log has "KH2 spell hit spark:" lines.
> - KH1-style magic and Keyblade swings: still KH1's sparks?
>
> Open questions:
> 1. For the enemy damage effect: (a) I keep digging for KH1 enemies' own effect lists and report what's in them before building anything, (b) we use KH1's own attacker-side damage effect (kind 0x80) instead, giving our KH2 lists an entry for KH2's damage_effect number (that's KH1's mechanism, not KH2's, and it would need an element to stand in), or (c) we leave it out for now.
> 2. Hit sounds for KH2 spells: (a) leave them as they are for now, or (b) start a thread to give KH2 spells their ATKP hit sound through KH1's sound path.

^m2

reflect, it still seems off centred, before these changes it wasn't how were you attaching it to sora? can we do the same while keeping it native? same goes for cure as well.
1. from my judgement I don't think an enemy would hold what effect it can receive, what I'm referring to is when you hit an enemy an effect spawns on the enemy (keyblade hit effect, fire hit effect etc.) Please look into it and find an implementation thats native, I give you the go ahead to work it in 
2.(b)

go ahead