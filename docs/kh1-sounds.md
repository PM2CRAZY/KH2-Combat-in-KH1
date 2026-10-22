# KH1's sounds (menu sounds and Sora's voice)

Made with the log: `kh1-kh2.ini`'s `sound_log = on` (the default on the testing branch, stage 326)
hooks KH1's sound functions and writes a line to `kh1-kh2.log` for every call. This page is the table
those lines fill in. Ask for a sound by its id ("Sora says clip 0x23"); the log says which id a thing
in game really plays.

## The functions

Epic RVAs, as the source writes them (`addr::Rva`); the Steam build's are in `steam_map.in`
(Steam: `0x17A040`, `0x2901F0`, `0x290930`).

| Function | Epic RVA | Arguments | What it plays |
|---|---|---|---|
| menu sound | `0x177F60` | `(id)` | the menu's cursor, confirm, back and "can't" sounds |
| voice group | `0x28E060` | `(actor, group, other)` | KH1 picks one clip of the group (`kh1voice.rs`, `docs/kh1-voice.md`) |
| voice clip | `0x28E7A0` | `(actor, clip, priority)` | one clip of the speaker's bank, if nothing of a higher priority is speaking |

A voice group's pick shows in the log as a `voice clip` line right after its `voice group` line,
called from inside the group function (`from game +0x...`).

## The log's lines

```
KH1 sound: menu sound 2 (from game +0xXXXXXX)
KH1 sound: voice group 4 on Sora (from game +0xXXXXXX)
KH1 sound: voice clip 35 (0x23) priority 3 on Sora (from the mod)
```

- `from the mod`: the mod's own call (`kh1voice`, `trace`'s record sound, `config`, `combat`'s
  "can't"). `from game +0x...`: KH1's own code, the RVA in the running build.
- `Sora` is KH1's party slot 0; any other actor is shown by its address.
- At most 6000 lines a session.

## Menu sounds

Ids in use by the mod (from the source and `docs/kh1-config-menu.md`; **not yet checked against the
log**, the user found that 1 is the cursor's move sound, not the confirm):

| Id | Heard as | Where |
|---|---|---|
| 0 | cursor (?) | `config.rs`: scrolling our Config page |
| 1 | cursor's move | `config.rs`: toggling one of our rows |
| 2 | confirm (?) | `trace.rs`: a trace recorded (t325) |
| 3 | back (?) | doc: "back" |
| 4 | "can't" | `combat.rs`: refused commands |

To fill in: press confirm, cancel, move the cursor, open and close the pause menu, equip, use an
item, and write down the order next to the log's ids.

## Sora's voice

Groups known from `docs/kh1-voice.md`: 0 / 1 hurt (light, heavy), 2 knocked out, 4 combo swings,
5 finishers, 6 air finishers, 7..13 the seven spells, 15 / 17 the summons' calls. Clips: 21, 22, 23,
24, 28 the swing grunts, 30 the plain grunt, 37 "Give me strength!".

| Clip | Line | Group it comes from |
|---|---|---|
| 37 (0x25) | "Give me strength!" | 17 |

Add rows as the log shows them (clip id, what he says, what triggered it).

## Bank-level log (Steam build, t328)

- `voice call bank B no N = file clip N`: the voice call that actually plays a clip. Sora's banks: 90 di_sora, 92 po, 93 tw, 94 aw, 95 tz, 96 al, 97 lm, 98 nm, 99 he, 100 pi, 101 pp, 102 pc, 103 ew. Full clip table: `kh1-sora-voice-banks.csv`.
- `effect create/play bank B no N id I`: sound effects; the log dumps the bank list nodes once so the bank-to-file layout can be read.
- Epic build: these bank-level hooks are not installed (Epic RVAs unknown).
