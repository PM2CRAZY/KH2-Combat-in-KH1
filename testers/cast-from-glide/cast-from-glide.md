# Cast from Glide

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: I started this thread from What is still missing(../what-is-](#^m1)

---


> [!tip] Assistant
> I started this thread from [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md) (m8), where you answered (a) for casting out of the glide and asked for a Tweaks row.
>
> **t656, casting out of the glide.** With Glide at KH2, a KH2 spell (Fire to Cure at Magic KH2, and Reflect) picked while gliding now ends the glide and is cast in the air, the way KH2 does it.
> - KH2's glide ends into KH1's own air state, keeping its speed, the same as letting go of Square. KH1's own air cast then casts the spell, exactly as it does out of a fall: same animation, hop and air physics (KH2's 40 hop and kept speed at Air Physics KH2, KH1's 30 hop and half speed at KH1).
> - KH1 normally resets your sideways speed from the stick on the frame the state changes. The cast's speed is now kept over that, the same fix the glide's release already uses.
> - The glide is only ended if the cast will actually happen (its animation and files built, enough MP), so a refused spell doesn't knock you out of the glide.
> - New row: **Combat > Tweaks > Cast from Glide** (Off | On), ini `glide_cast`. On is KH2's behaviour and is the default. Off keeps t654's refusal (the "can't" sound, nothing cast, you keep gliding). KH1's own glide (Glide at KH1) still casts nothing, as in KH1.
>
> Checked: KH2's magic gate and cast start in the Steam KH2 decomp. When the cast starts airborne it replaces the glide, and the glide's exit doesn't touch the velocity. The glide motions are free all through in `P_EX100.mset`. On the KH1 side I checked the air cast, the glide's enter / exit and the think step's speed reset in the Steam KH1 decomp. Not checked: in game. Two assumptions:
> - KH1's menu takes a command at all while in its glide state. I didn't trace the state table's flags for that.
> - No frame of the fall animation shows before the cast's animation replaces it.
>
> Please test (Glide at KH2, Cast from Glide On):
> - Glide, then pick Fire / Blizzard / Thunder / Cure (Magic KH2) and Reflect, from the menu and a shortcut. The glide should end into the air cast, carrying your glide speed, and you should land normally after. Log: "out of KH2's glide (its free range, Cast from Glide on)" and "KH2's glide velocity … kept into the fall".
> - The same with too little MP: you keep gliding, with the "can't" sound. Log: "refused: KH2's spell … KH1's glide state casts nothing".
> - Cast from Glide Off: the refusal, as in t654. Check the row's text and that the setting survives a restart.
> - If gliding gives no reaction at all and nothing is logged, KH1's menu is refusing the command before it reaches us. Please tell me if that happens.
>
> Question:
> 1. Gravity, Stop and Aero (KH1's spells) still cast nothing in the glide, KH1's rule. Should they (a) stay that way, or (b) also end the glide and cast with Cast from Glide on, the same as the KH2 spells?

^m1


