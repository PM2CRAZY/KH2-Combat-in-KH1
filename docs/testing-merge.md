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

| # | Feature (main) | Inventory row | Needs |
|---|---|---|---|
| 1 | Bonus AP | 32 | code (main's rule read again) |
| 2 | Growth abilities as main's (one level above base Sora) | 16 | code |
| 3 | Anti Form: the secret code, its look, Anti Glide / Side Spin, foot ledge hang, its rules | 14 | KH1 + KH2 decomp, KH2 files |
| 4 | Reflect levels by the story, "New Magic: Reflect" box | 9 | code (save bytes) |
| 5 | Page Down hides the HUD | 26 | KH1 decomp (Steam draw address) |
| 6 | Swings meet walls as KH1's (spark, sound, no bounce) | 33 | code |
| 7 | KH2 camera follows KH1's up / down invert option | 34 | KH1 decomp (the option's address) |
| 8 | Combo lunges don't carry Sora through room exits or into crate pushes | 35 | code, in-game check |
| 9 | KH2 effects on Counterguard, Flash Step, Retaliating Slash | 36 | code / KH2 files |
| 10 | Dream Shield stretching fix | 49 | in-game check first |
| 11 | Epic Games Store support | 61 | the Epic exe and decomp |
| 12 | README, install notes, `mod.yml` for players | 62 | docs |
