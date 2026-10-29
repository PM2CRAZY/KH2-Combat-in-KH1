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
  - The load profiler (t783, `loadprof`, `kh1-load-profiler.md`): also only with the Loading & Cache log on;
    it samples the game thread around loads and names our functions in slow frames.
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
| Cutscenes & Events | `log_cutscenes` | `cutscenelog` (KH1's scene prepares, event script runs, movie opens; t706) |
| Magnet Debug | `log_magnet` | `magnet` (every frame of each enemy Magnet Burst holds: its state, motion, place, our displacement, KH1's push / `+0x200` / `+0x120`, and how far it really moved; t864) |

Not changed: `kh1-kh2-other.log` (one line when another game in the folder loads the DLL; that
process never reads the ini), the trace's own CSV files (debug tools), and
`kh1-kh2-removed.txt` (the record of abilities taken out of a save: gameplay, not a log).

## Growth Abilities page (the quick-run-native thread, m6)

The tester: "can you make it a setting in the debug where I can adjust the growth abilities", then
"the overrides can always be adjusted in the debug, keep it", "don't worry about drive forms", "(a)"
(given even when not learned or equipped), and "the debug menu should be able to break any rule".

- Main page > **Growth Abilities**: one row each for High Jump, Quick Run, Dodge Roll, Aerial Dodge
  and Glide. Left / right step through Normal, Off, LV1, LV2, LV3, MAX; confirm puts the row back
  to Normal. Each row also shows the level KH2's controller has now ("now LV2").
- Kept in the ini as `debug_high_jump`, `debug_quick_run`, `debug_dodge_roll`,
  `debug_aerial_dodge`, `debug_glide` (`normal` / `off` / `lv1` .. `max`), read at start and
  written at every change. Only with the debug tools (`debug_menu = on` at start): without them the
  keys aren't read and nothing is overridden (`debugmenu::growth_overrides` returns all Normal).
- How (Checked (code)): the level the mod works out from KH1's menu, Sora's level and the story
  (`growth::Kh1Sora::kh2`, or the `abilities = test / all` sets) is what `menu::Growth` gives KH2's
  controller (`character.abilities`); the overrides replace those five levels as they are given
  (`growth::with_growth_override`), and a change on the page is applied at once. Every reader
  goes through `character.abilities`, so they all follow: KH2's own moves and their fmab numbers
  (`Character::growth`), the native Quick Run state and its LV2+ start effect and invulnerable
  frames (`takeover::kh2_quick_run_growth`, `quickrun.rs`), the native dodge roll's invulnerable
  frames (`kh2_dodge_roll_growth`), the Aerial Dodge height / speed (`kh2_air_dodge_growth`) and
  whether it's there (`kh2_air_dodge`), KH2's glide in KH1's glide state (`kh2_glide_growth`, the
  level kept aside `KH2_GLIDE_KEPT` set to the forced one for base Sora), Square's KH2 order
  (`kh2_square`, `with_abilities`), and High Jump's height for KH2's physics on KH1's states
  (`refresh_kh2_numbers`). The menu names "Quick Run LVn", "Glide LVn", "Aerial Dodge LVn" show
  the forced level (Off keeps his own name). The "Ability Up" boxes still follow his real level.
  Nothing is written to the save.
- Not covered (Checked (code)): KH1's own moves at a KH1 style row (Jump KH1, Dodge Roll KH1's own
  roll, Glide KH1) read KH1's own ability list (`0x140282A70`, the bits at `0x142D5F610`), not
  these levels; Off doesn't take those away (Stats > Abilities can take the KH1 ability out).
- Drive Forms (left as they are, the tester): a form gives its own growth ability and none of the
  others (`form::form_abilities`), so an override doesn't show in a form; Sora's own (with the
  overrides) come back on revert. A change made during a form is kept for after it.

## Anim Display row (t763)

The tester: the animation readout Frame Advance shows, as a toggle whether or not Frame Advance
is on.

- Main page > **Anim Display** `< On / Off >`. On draws the Frame Advance info panel (`fapanel`:
  the animation, how it is drawn, the frame within it of its length, loops, the trace frame) in
  the top right corner on every frame, titled ANIMATION; Frame Advance on shows it as before,
  titled FRAME ADVANCE, whatever this row says.
- Not kept in the ini (starts off each launch, like Frame Advance); turned off with the Config
  menu's Debug Menu row. The panel reads what the pose code already records for the trace
  (`pose::drawn`), so it adds no hook; it is drawn only while on.
- (t774) The frame count on KH1's own animations: the panel counted nothing there (the frame stayed 0,
  e.g. the chest open, slot 83), since only KH2's clips recorded a time. KH1's own now reads Sora's motion
  time (`+0x16C`, the time KH1's actor motion code steps each frame, Checked (exe)), so it shows the
  frame within the slot of its length and counts loops. KH2's controller pose still shows no frame.

## Ledge Carry row (the what-is-still-missing thread, m14 answer 4)

The tester: "(a) and can you make it in the debug" (KH2's ledge rule for its ground moves, in the
debug menu).

- Main page > **Ledge Carry** `< On / Off >`. On (the default): a KH2 ground move whose motion
  holds Sora on the ground (no gravity or the edge stop open: Valor's 154, base Sora's 154 / 155 /
  161 / 165 / 169) goes on off a ledge as KH2's does; Off: KH1 cuts it into its fall, as before
  (`shared::physics::ground_next`, `docs/kh1-drive-forms-native.md` §7.4.5).
- Kept in the ini as `debug_ledge_carry = on | off`, read at start and written at every change.
  Only read with the debug tools (`debug_menu = on` at start); without them it is on.
