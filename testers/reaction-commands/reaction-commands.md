# Reaction commands

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)

---


> [!tip] Assistant
> I started this thread from [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md), as you asked. This is research only, no code yet.
>
> **What we have now.** Nothing in the KH1 mod. The old KH2 playground had RCs (offer, choice, the Soldier's Cyclone with its capture motions), but none of it was brought over.
>
> **What KH2 does** (Steam KH2 decomp). There is no RC timer. Every frame `0x1404081C0` rebuilds the one best candidate. `0x140407DE0` walks each enemy's collision shapes; a type-5 shape whose group is switched on (the enemy's motion turns groups on and off) gives its command. The highest priority from 03system `cmd` wins. Ties go to the smaller score from `0x1404088F0`: the distance, ×0.5 for the lock-on target and ×0.8 for the one already shown. The command is greyed when its range or facing check fails, or it is a ground-only command and Sora is in the air. Running it hands it to the enemy's script, which "captures" Sora into a motion kept in the enemy's own motion set (his animation, invulnerability and hitboxes). With nothing offered and HP at a quarter or less, KH2 offers a Drive Form there.
>
> **How KH1 does the same thing** (Steam KH1 decomp). KH1 already has a "context command" system, and it works almost the same way:
> - Four command slots at `0x140528914`; slot 3 is the Triangle box (Open, Examine and KH1's limits), slot 0 is where Talk replaces Attack.
> - Anything can offer one with `0x140293000 (slot, command)`. The first offer of the frame wins.
> - Every frame `0x1402948F0` scores nearby actors by distance and facing and sorts them. Each actor's object record says what it offers: Talk, Open/Examine, or "ask the actor". When asked, the actor's script (event 2) or its own handler answers through `0x140297CA0 (actor, Sora, -1)`. The same call with a real command runs it, which is the same shape as KH2's "the enemy's script runs the RC".
> - KH1's limit offer (`0x14028D0F0`) only fills slot 3 if nothing else took it.
> - The command dispatcher (`0x1402A3600` / `0x1402A3DB0`) sends any command it doesn't know to the target's handler. So a new command record (as Reflect and the Drive list added) chosen from the Triangle box would already reach the enemy natively.
>
> **What's missing.** KH1's enemies offer nothing: they have no type-5 shapes and no RC handler (I think their records simply say "no interaction"; that's an assumption). KH2's 03system has an RC table (`rcct`, 35 enemies), but the game only uses it for the Journal. The real data is in each KH2 enemy's model (its areas) and script (what the RC does).
>
> **Plan, one system per version:**
> 1. KH2's RC commands as new KH1 command records (name, priority, ground-only flag).
> 2. KH2's offer rule through KH1's slot 3, using KH2's areas, priority and score (this replaces KH1's own "locked actor only" rule for RCs).
> 3. The areas: for each KH1 enemy that has a KH2 counterpart, the KH2 model's RC shapes, switched on while the KH1 enemy plays the motion that matches KH2's. The Soldier's Cyclone comes first, since its data is already worked out.
> 4. The run: our handler on `0x140297CA0` for those ids only. Sora plays KH2's capture motion through KH1's motion system, with KH2's invulnerability and hitboxes as KH1 attack objects. The enemy's side goes through KH1's hit records.
> 5. KH2's low-HP Drive offer in the same slot.
>
> **What I couldn't find yet:** which KH2 enemy each `rcct` entry belongs to; which KH1 enemies have a KH2 counterpart with an RC (Soldier for sure; others like Large Body, Darkball, Wyvern or Neoshadow are guesses until I check their KH2 files); and whether KH1's Triangle press goes through the same dispatcher (I only know that from our older Epic notes).
>
> The notes are in `docs/reaction-commands.md`, section "Native research (t480 round)".
>
> **Questions:**
> 1. Enemies with no KH2 counterpart (most of KH1's bosses and world Heartless): (a) no RCs, only enemies KH2 really has; (b) also borrow a KH2 RC from a similar enemy (each one listed for you first).
> 2. KH2's low-HP Drive offer and KH1's limits both want the Triangle box: (a) Drive first, as KH2; (b) KH1's limit first; (c) leave the Drive offer out.
> 3. While an RC is offered, Open / Examine / Talk next to an enemy: (a) the RC wins (KH2's priority); (b) KH1's own command wins.

^m1


