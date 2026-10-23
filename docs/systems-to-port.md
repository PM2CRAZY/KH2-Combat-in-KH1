# Systems to build inside KH1, 1:1 with KH2

These are examples, not the full list. The scope is open-ended: look for related systems, hidden states, flags and special cases in the KH2 source and decomp, and in KH1, and report what you find before adding it.

Everything here runs inside KH1's own systems with KH2's values and logic. Status of each decomp check is "not yet looked at" unless noted.

- **Air combos and ground combos**, combo and cancel windows, finishers. Air combos in KH1 should work exactly as in KH2.
- **Growth abilities.** KH2's abilities are custom abilities, and the ability list is swapped depending on the setting. The swap is written to the save (writing to the save slot is allowed, and so can the debug menu). Before building it, define what happens to equipped abilities when the setting changes (for example, KH1 Glide equipped, then the setting flips to KH2: it becomes the KH2 Glide or unequipped, never an ID that no longer exists).
- **Form-specific abilities**, in a new subsection after "shared", running inside KH1's systems and shown only while that form is active.
- **Special states**, for example i-frames: use KH1's system with KH2's frame data, 1:1. Confirm KH1's tick rate in the KH1 decomp first.
- **Glide:** running inside KH1's systems again. It starts from Square, not jump, you can glide out of the Aerial Dodge, and the Physics setting doesn't affect it. KH1 glide rule: in the air the jump press is the Aerial Dodge first, and glide can be requested only after it and only if Glide is equipped; that gating affects KH1 glide logic only. With Glide style KH2, KH1's Glide and Superglide leave the shared abilities list and only KH2's growth Glide appears as its own custom ability; with Glide style KH1 it goes back to vanilla.
- **Drive Forms** (Valor's second Keyblade bake is deferred until Valor is tackled), and **Limit Form** replacing Limit commands. Limits and Summons haven't been looked at in the decomp yet.
- **Hit reactions and stun**, **guard and block**.
- **Effects:** port KH2's effects into KH1's effect system so any move can use either vanilla KH1 or KH2 effects. Custom moves' hitboxes currently only spawn the normal Keyblade swing effect.
- **MP system:** undecided. If ported, it goes through KH1's systems 1:1.
- **Animations:** baked from KH2 into KH1's motion slots raw, behind an ini switch. Keyblade handling comes after in-game testing. Hair, hood and chain movement is baked into the animations, not physics.
- **Camera:** KH2's lock-on and follow camera, compared against traces that include Sora's camera movements.

Rules that apply to every item:
- Each setting's KH1 mode must still behave like vanilla KH1.
- A KH2 setting must not be changed by unrelated Config options.
