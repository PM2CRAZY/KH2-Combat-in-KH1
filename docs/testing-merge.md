# The testing-merge branch

Made 2026-11-05 from `testing` at t1094 (`bb3b65d`), at the owner's request (chat):

> make another branch on this repo called "testing merge," I want you to use the testing branch as the main
> repo. I want you to implement all of the things main has and make it work natively. I also want the bonus ap
> back and for the time being, the growth abilities to work how it does in the main branch.

(Git branch names can't hold a space, so the branch is `testing-merge`.)

## Rules

- `testing` is the base: everything main has is rebuilt natively inside testing's own systems (the same design
  philosophy and engineering rules as testing: `docs/design-philosophy-instructions.md`,
  `docs/engineering-rules.md`). Main is read for what it did and what the owner decided there, never copied
  (no Epic addresses, no main data).
- Versions here are `tm1`, `tm2`, ... (`VERSION` in `crates/kh1-hook/src/lib.rs`), each with a
  `### <date>: Stage tm<n> (testing-merge branch: <title>)` entry at the top of `AGENTS.md`.
- The Windows build (`.github/workflows/build-testing.yml`, on this branch) builds on every push to
  `testing-merge` and commits `kh1-kh2.dll` here.
- Two of testing's rules are changed here, by the owner's request, "for the time being":
  - **Bonus AP is back** (main's: Sora's max AP grows by each KH2 ability's AP; testing removed it at t718).
  - **Growth abilities work as main's** (s270: a form's growth ability one level above base Sora's; s301:
    from what Sora has learned), instead of t1085's KH2 rule.

## What main has that testing doesn't (the work list)

From `docs/main-port-inventory.md` (main s339 against testing t948), its rows checked again against t1094.
Status as of tm11 (2026-11-05). Nothing is checked in game yet.

| # | Feature (main) | Inventory row | State |
|---|---|---|---|
| 1 | Bonus AP | 32 | tm7 |
| 2 | Growth abilities as main's (by Sora's level, forms one above) | 16 | tm8 (testing's rule kept behind `growth::GROWTH_RULE`) |
| 3 | Anti Form | 14 | tm10 (form, moves, rules), tm11 (secret code). Not built: its look (model / texture readers), counter / roll save bytes, HP orbs, summon bit, its effects, ledge hang, no Keyblade / trails |
| 4 | Reflect levels by the story, "New Magic: Reflect" box | 9 | tm6 (Progression > Reflect by Story, off by default) |
| 5 | Page Down hides the HUD | 26 | tm2: research and a log of the Steam draw bytes; needs the KH1 decomp |
| 6 | Swings meet walls as KH1's | 33 | tm9 (Tweaks > Walls: KH1 / Clink / Pass) |
| 7 | KH2 camera follows KH1's up / down invert | 34 | tm1 |
| 8 | Combo lunges don't carry Sora through room exits / into crates | 35 | tm3 (exits); crates were already refused by KH1 |
| 9 | KH2 effects on Counterguard, Flash Step, Retaliating Slash | 36 | tm4 |
| 10 | Dream Shield stretching | 49 | tm5 (drawn routes fixed; a log for the baked moves) |
| 11 | Epic Games Store support | 61 | not started: needs the Epic exe and decomp |
| 12 | README, install notes, `mod.yml` for players | 62 | not started |
