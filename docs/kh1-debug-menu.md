# The debug tools (debug menu, Frame Advance, trace, logs)

The debug menu itself (pages, buttons, stats, weapons, abilities) is described at the top of
`crates/kh1-hook/src/win/debugmenu.rs`; Frame Advance in `kh1-frame-advance.md`. This note is
about what loads them.

## Loaded only with `debug_menu = on` (the ram-use thread, m1 answer 3)

The tester: "in the ini I want to make it where if debug menu is off then we don't inject or load
anything related to the debug, if its on then we do. keep debug menu in the config menu but if
its off in the ini then its gone from that menu."

- `kh1-kh2.ini`'s `debug_menu` is read **once, when the game starts**, before any hook goes in
  (`win::debug_on`, `win/mod.rs`). Saving the ini while the game runs doesn't add or remove
  hooks: a change needs a restart.
- **Off** (the default): none of the following is installed or kept.
  - The debug menu's drawing hook (KH1's end of the frame's drawing, Steam `0x1400FBBE0`), so
    the overlay's GPU buffer (`fx12`'s 4 MB vertex ring and the font buffer, made on the first
    overlay draw) and the font (`debugfont`) are never made; the Frame Advance panel
    (`fapanel`) is never drawn.
  - Frame Advance's keyboard hook (`frameadv::install`) and its hold at KH1's frame end.
  - The debug menu's input and stat locks at KH1's frame end.
  - The trace recorder: no row is built, nothing is kept, no dumps, no `kh1-kh2.trace.csv`
    (`trace::refresh` keeps its mode Off; `trace::on` / `wants` are false).
  - The sound log's hooks: KH1's menu sound, voice clip, voice call, sound effect create / play.
  - The heartbeat thread (`monitor`: the "alive" line every 2 minutes, the Insert test key that
    cycled what Sora is drawn with, the event-opcode trace once a second).
  - The probes' debug-only work inside hooks that stay for gameplay (`probe.rs`): the event
    opcode counts, Sora's actor dumps / position search / actor list, the hit-effect notes and
    KH1's own swing record copy and dump.
  - The Config menu's Misc list has no Debug Menu row (Battle Quotes and Gummi Routes only; its
    help line no longer mentions the debug menu).
- **Kept whatever the ini says**, because gameplay runs through them (Checked (code)):
  - KH1's frame end hook (Steam `0x140109B40`): `kh1bank::frame_end` puts KH2's sound banks
    back into KH1's bank list there every frame.
  - The voice group hook (`soundlog.rs`, KH1's `0x14028E060`): KH2's hurt line
    (`hitreact::hurt_line`) and KH2's quotes (`kh2sound::holds_back`) hold KH1's voice back
    there.
  - Every `probe.rs` hook: the set-motion hooks (`SORA_MOTIONS` / `SORA_LAST_SLOT`, read by the
    combat code), the attack-touched-actor hook (`bodyhits`, `combat::scale_record`,
    `formmagic::spell_hit`), the actor update, the pad record hook (KH1's pads merged, the
    debug menu's and Frame Advance's buttons hidden only while they're on) and the event
    opcode hook (opcode 322: the attack tutorial's motion request).
  - `frameadv::clock_ms` / `age` / `holding`: the mod's clock, used everywhere; without Frame
    Advance it is real time.
  - The ini's test keys (`hit_fx_test`, `hit_effect_test`) are gameplay test switches, empty by
    default; untouched.
- **On**: everything as before. The Config menu's Debug Menu row switches the menu (L1 + R1 +
  Triangle) for that session only; it no longer writes `debug_menu` (which would take the row
  and the tools away at the next start). The menu starts switched on.

## The log, by category (the ram-use thread, m1 answer 3)

The tester: "I also want to have our logs attached to the debug menu where you can turn on and
off each log and all should be off, if debug menu is off then none of the logs are injected into
the game".

- Every log line is written with `log!(Category, "...", args)` (`win/mod.rs`). The macro asks
  `log_on(Category)` (one atomic load) before it formats anything, so a category that is off
  costs no string building. The helpers that gather a module's lines (`note`, `cmd_log`) take a
  closure and return at once when their category is off, and the "first N lines" counters only
  count lines actually written (so turning a category on mid-session still shows its first lines).
- **Without the debug tools** (`debug_menu = off` at start): no category can be on, no
  `kh1-kh2.log` is created and the old logs (`kh1-kh2.prev*.log`) aren't rotated, and the crash
  note's exception handler isn't installed (it only wrote to the log). (`CRASH_NOTES_ALWAYS` in
  `win/mod.rs`: true would keep the crash notes and the version lines, and the log file, whatever
  the ini says; false is the tester's rule as written.)
- **With the debug tools**: the log file is opened at start as before (the last five runs kept),
  and the debug menu's **Logs** page switches each category (and All Logs); each is kept in the
  ini as `log_<key> = on / off`, all off by default. KH1 Sound Calls replaces the old
  `sound_log` key and the Trace Recorder's Sound Log row.
- The trace's Log Lines / Sounds categories only see lines whose log category is on.

| Debug menu name | ini key | What logs under it |
|---|---|---|
| General | `log_general` | `mod.rs` (Panacea, heartbeat, Insert key, the settings), `addr`, `hook` (hooks in / refused), `settings` |
| Loading & Cache | `log_loading` | `loadscreen`, the motion slot cache lines in `pose` |
| Takeover & Input | `log_takeover` | `takeover`, `shared`, `probe`, `quickrun`, `glide`, `gates`, `feet`, the style modules |
| Animation & Pose | `log_animation` | `pose`, `kh2slots`, `model`, `tint`, `leftblade` |
| Combat & Hits | `log_combat` | `combat`, `kh1picker`, `combomods`, `bodyhits`, `hurt`, `knockback`, `kh2damage`, `recoil`, `hitreact`, `kh1rows` |
| Magic | `log_magic` | `reflect`, `formmagic`, `mpcost` |
| Drive Forms | `log_forms` | `form`, `drive`, `valor`, `valorstats`, `equip` |
| KH2 Sounds | `log_kh2_sounds` | `kh2sound`, `kh1bank`, `kh1voice` |
| KH1 Sound Calls | `log_kh1_sounds` | `soundlog` (every KH1 menu sound, voice and sound effect call) |
| Effects | `log_effects` | `kh2fx`, `kh2dpx`, `kh2ppp`, `fx12` |
| Camera | `log_camera` | `camera` |
| Menus & Config | `log_menus` | `menu`, `config`, `shortcuts`, `equipmenu`, `gummi`, `styles` |
| Debug Tools | `log_debug` | `debugmenu`, `frameadv`, `trace` |
| Crash & Version | `log_crash` | `crash` (crash notes), panics, the version and game-build lines at start |

Not changed: `kh1-kh2-other.log` (one line when another game in the folder loads the DLL; that
process never reads the ini), the trace's own CSV files (debug tools), and
`kh1-kh2-removed.txt` (the record of abilities taken out of a save: gameplay, not a log).
