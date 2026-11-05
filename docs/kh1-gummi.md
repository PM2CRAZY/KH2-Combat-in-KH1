# Gummi routes: the skip (`win/gummi.rs`)

Found in the game executable (Epic addresses; Steam's through `steam_map.in`). Not derived
from any other mod.

## The Gummi side scene

`0x14020F6D0` runs the scene between the field and the worlds. A mode at `0x14268E3C4` says
what comes next:

| mode | set by | runs |
|---|---|---|
| 1 | `0x14020EF60` | the world map's task `0x140219490` |
| 2 | `0x14020EF10 (kind, stage)`, `0x14020EEB0 (route)` | a flight, `0x14023D8C0` (stage `0x14050B1B0`, kind `0x14268CE6C`) |
| 3 | `0x14020ED90 (landing)`, `0x14020EE00 (world, room)` | back to the field in the landing's world / room (`0x142340EF0` / `EF4`) |
| 4 | `0x14020EE60` | back to the field where Sora was (the map backed out of) |

## World map to flight

- `0x140218E30 (from, ...)` runs the map (its module table at `0x140507BF0`: init
  `0x1401EC0C0`, step `0x1401EC2A0`, end `0x1401EC530`) and returns the chosen world. The map's
  step writes it to `0x140507C94` (and `0x140507C98`, `0x140507C9C`) as it closes.
- `0x140219490` takes it: a negative choice backs out (`0x14020EE60`); else the route between
  the two worlds from the route table (`0x14021D030 (from, to)`: triples at `0x14050F900`, each
  naming a (kind, stage) pair at `0x14050E400`; 99 as "from" for the warp routes), then
  `0x14020EF10 (kind, stage)`, which never returns (the scene changes).
- `0x14050B1B4` is the map's current world (set from the field's world as the scene starts).

## Flight to landing

The flight's task (`0x14023D8C0` → `0x14023F670`) ends cleared in `0x14023DA40`: the camera's
arrival, the route's best results (`0x14020C420`, the "GUMI" save block, a popup when beaten),
then `0x140213B20 (world)`: KH1's landing list for the world (`0x14021CCA0` builds it,
`0x1401EFEC0` runs the menu). Its answer:

- a landing index: `0x140266920 (4)`, then `0x14020ED90 (index)` (mode 3, the landing's world
  and room from its table, the field loads it, with its own arrival events);
- negative (no landing, or backed out): `0x14050B1B4` = the destination, `0x14020EF60` (the map
  again, there).

## The skip

`gummi = skip` (the Config menu's Gummi Routes row, Vanilla | Skip, in the QoL list since t704): `0x14020EF10` is hooked; for
a world the map chose (`0x140507C94`, 0..0x1B) it makes the cleared flight's calls at once
(landing list, landing), skipping the flight. The route's results aren't recorded (no flight
flown). Flights the story starts itself (`0x14020EEB0`) stay as they are.

Unverified in game: the landing list drawn without the flight's scene behind it; story worlds
whose first arrival is tied to a flight.

## The route's event scripts (t1068)

The owner: with the skip on, the story step that follows a world's first arrival didn't happen (answer
(b)); answer (a): do what a cleared flight does but the flight itself.

- Steam KH1, Checked (decomp, exe): each route has a record at `0x14050AF00 + route * 0x58` (`route_tw_1`,
  ...; the route start's first argument is its index). It names two event scripts in `worldmap/ev/`: a
  departure one (`+0x34`, `wld_out_*`, event 0x40) run as the flight starts, and an arrival one (`+0x14`,
  `wld_in_*`, event 0x41) run after the cleared flight's landing list, with flag word `0x142867364` bit 8
  set just before. Routes 3..15 have both, 16..18 only the departure one, the rest none. The skip ran
  neither.
- Now the skip, after the landing list, loads both into the scene's script buffer (`0x14268C438`, its first
  two 0x1000 slots) with KH1's loader `0x140214780`, sets bit 8, and runs them with KH1's runner
  `0x140213150` in the flight's order, then lands (or goes back to the map) as before. The route's scores
  are still not written (no flight flown).
- Assumptions: the world map reloads its own two scripts in those slots when it next starts; the
  departure script runs safely after the landing list instead of before it; the scripts don't need the
  flight's actors. Not checked in game.

## The departure script before the landing list; a save-block diagnostic (after t1085)

The owner (chat): "gummi skip completely breaks story flags".

- What differed from a flown route (Checked (code, against the t1068 decomp notes above)): a flight runs the
  departure script (event 0x40) at its start, before the landing list; t1068's skip ran it after the landing
  list. So the landing list (`0x140215C80` Steam) was built before anything the departure script sets.
  Assumption (not checked: no decomp in that session): the landing list or the landing reads a story
  variable or flag the departure script sets, which would land Sora at a landing the story isn't at.
- Now the skip runs, in the flight's order: event 0x40, the landing list, `0x142867364 |= 8`, event 0x41,
  then the landing (or the map).
- Diagnostic: with the Menus & Config log on, the skip copies KH1's save block (Steam `0x142DE9360`,
  `0x16C00` bytes, script variables and bit flags in it, `docs/kh1-softreset.md`) and logs, after each
  step, which bytes changed (`Gummi skip: save block after ...: +0xOFF old->new`). Nothing runs with
  the log off or with the skip off.
- Still not done by the skip (needs the Steam decomp): the route's results (`0x14020C420` Epic, the "GUMI"
  save block; its Steam address and arguments aren't in the repo), anything else the flight's start
  (`0x14023DF30`) or setup (`0x140241310`) writes, and the mode argument the flight passes to the script
  runner `0x140213150` (the skip passes 0, Assumption). Also unchecked: whether a route script can change
  the scene itself (a story landing) that the skip's own landing then overrides.
