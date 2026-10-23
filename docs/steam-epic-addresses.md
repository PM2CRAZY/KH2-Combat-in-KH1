# Steam and Epic addresses

The mod runs on both PC releases of KINGDOM HEARTS FINAL MIX (1.5+2.5): the Epic Games Store build
(PE link stamp 1721648200) and the Steam build (1721645006). They are the same game compiled the
same day, but each links its own store's code, so functions and globals sit at different places.
This note is how the two are kept in sync. Code: `crates/kh1-hook/src/win/addr.rs`,
`crates/kh1-hook/src/win/steam_map.in`, `crates/kh1-hook/tests/steam_map.rs`.

## One address, both builds

- Every game address in `crates/kh1-hook/src/win` is written once, as the **Epic** build's RVA, in
  an `Rva(0x...)`. `base + rva` gives the address in the running build: on Steam it goes through
  `STEAM` (`addr.rs`), the table in `steam_map.in` (Epic RVA -> Steam RVA, numbers only).
- An address found on **Steam** (the Steam decomp, a Steam log) is not written into the source as
  it is. Find the Epic function or global it is (a function: the same instructions once relative
  offsets are set aside, the same callers, the same strings; a global: the instructions that read
  it, in the matching functions), write that Epic RVA, regenerate the table, and check the table
  maps it back to the Steam address that was found. So on Steam the hook uses exactly the address
  found there, and Epic gets it too.
- The other way round, an address found on Epic gets its Steam counterpart in the table the same way.
- No raw `usize` RVAs, `0x14...` addresses or "this build only" constants in code: the generator
  and the test only see `Rva(0x...)`, and an address outside them works in one build only.
- A hook checks the bytes it moves (`hook::install`), so a wrong address refuses to install
  instead of breaking the game. Most functions start with the same bytes in both builds. Where
  they don't, the code makes the bytes for the running build: `addr::mov_rax_rip` for a first
  instruction that is RIP-relative (`config.rs`'s text lookup, `shortcuts.rs`'s get / set), or
  picks them by `addr::Build::current()` when a build sets a function's frame up its own way.
- Some functions are the same in both builds but read a structure at a different offset. KH1's
  settings block (returned by `0x14012AC50`) keeps its volume steps and the confirm swap 4 bytes
  later on Steam; code that reads the block itself adds 4 on Steam (`music.rs`,
  `kh2sound.rs`). Table reads
  through image-base displacements (`[r14 + i*4 + table]`, jump tables) differ by the table's
  own move, so patches that compute them from `Rva`s work in both.

## Regenerating and checking the table

- From `crates/kh1-hook/src/win`: `python3 gen2.py . steam_map.in`. The generator is kept with the
  reverse-engineering tools, outside the repo. It reads every `Rva(0x...)` in `src/win`, maps each
  through the function-by-function match of the two executables, and stops on an address it
  can't map, naming it. Data addresses that fall between two matched ranges are mapped by hand
  in its override list, each checked by contents and by the neighbours' move.
- Then each entry is checked against both executables:
  - code: the same instruction stream in both builds (opcodes, lengths, the bytes that aren't
    relative), each call, jump and RIP-relative target mapping to the same place; a function
    start in one build is a function start in the other;
  - hooks and patches: the prologue each hook expects, the bytes each patch replaces, and the
    callee each redirected call goes to, in each build;
  - data: the instructions that read it in the Epic build read the mapped address at the same
    place in the matching Steam function, or, with no direct reads, its neighbours move by the
    same amount and the contents match.
- `cargo test -p kh1-hook` checks that every `Rva(0x...)` has an entry and the table is sorted.

## Where the references are

- The Steam decomps (KH1 and KH2, Ghidra's C by address block, with a function list) are in the
  user's `local/decomps/steam/`, never in the repo. Ghidra projects and the executables stay
  outside the repo too; findings go into the docs as plain-language notes only.

Last full audit: 2026-10-22, s334 (397 addresses, every hook prologue, patch site and redirected
call checked in both builds; nothing to fix).
