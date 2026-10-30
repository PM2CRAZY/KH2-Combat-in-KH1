# Limit, Master, Final and Anti Forms on KH1: research (main-port round 12, rows 12 to 15, 2026-10-30)

Research only; no code. One section per form: what testing has, what KH2 does (tagged Checked (decomp) / (exe) / (files) / (code) or Assumption), how KH1 hosts it natively, the version plan and the questions. Note: the research read a partial copy of the source at t961 with some files from t962 to t968 missing; the branch is at t968, and every plan is re-checked against the current files before code.


---

## Limit Form (KH2 form 3, inventory row 12): research

Tags: Checked (decomp / exe / files / code), Assumption. Steam addresses only. Main = spec only.

## 1. What exists

**Testing (copy at t961)**, Checked (code):
- No Limit Form. `form::Kind` is Valor / Wisdom / Final (`win/form.rs:80`). `native_capable` is Valor | Wisdom (`:126`). `drive::FORMS` has two entries (`win/drive.rs:32`). `kh2slots::FORM_SLOTS` has two (`win/kh2slots.rs:390`). `tint::Look` is Wisdom / Valor (`win/tint.rs:74`).
- Already ready for it: `drivegauge.rs:20` Limit costs 4 bars (KH2's). `storyforms.rs:67` puts Limit in the owner's unlock table. `formown.rs` stores Limit's owned bit and says "forms not built yet (Limit, Master) are stored and wait".
- Form EXP save: `formexp.rs:6` uses `+0x882/+0x884/+0x886` (Valor, Wisdom, Final). Only `+0x887` is left of the checked-free bytes, so Limit needs another free u16.
- Missing from this copy: `formlevel`, `formcmds`, `formlimits` (t962/t963 per docs §20.5, so the branch is newer than this copy), and `kh1picker`, `kh1phys`, `bodyhits`, `hurt`, `quickrun`, `lefthits`, `mpcost`, `kh2table`. All are needed to build.

**Main (spec)**, stages s287–s336:
- Moves: `P_EX100_KH1F`, moveset 0x42, one Keyblade `W_EX010_KH1F`. Zantetsuken, Ripple Drive, Hurricane Period, guard (row 11, motion 173), Zantetsu Counter (row 39), Dodge Roll (209) when moving.
- Look (s293/s296): black jumpsuit and jacket, black hood lining, yellow belt, black shoes. Built from tint ops `Paint`, `Darken` and `Mask::Jacket`, none of which testing has.
- Menu (s291): KH2's four limits from `limi`. s297/s299 replaced them with KH1's five limits in a "Limit" list (no abilities needed, no Triangle offer), plus KH2's shortcut layout.
- Overrides: 3 bars (s331); no revert mid-limit (s314); HP per hit Sonic Blade 5, Strike Raid 8, Ars Arcanum 2, Ragnarok 8 (shots 0), Trinity 1, and limits out of battle (s336).
- All of this ran on [CTRL].

## 2. What KH2 does

- **Command**: `cmd` 0x2A1 is execute 4, argument 3, cost 4 bars (+0x1C), DisableForm 0xFFFE. Row 0x2A2 is Auto Limit's reaction command, also cost 4. Checked (files).
- **Partners**: `0x140400B20` takes nobody for form 3, and `0x140400740(3)` returns 0 (always available). Checked (decomp). So testing's t958/t959 partner-leave code must skip Limit.
- **Enter / gauge / revert**: Limit uses the same path as every form: `0x14040CE60` → `0x1403C17C0` (HP/MP full, gauge `(lv+3)×600`). The revert is issued only from the free-state step `0x140404A40` when the form time runs out. Checked (decomp). I assume a running limit holds Sora in another state, so KH2 itself delays the revert, which matches s314.
- **Moveset 0x42** (`prty`), Checked (files): walk 2, run 8, jump 285, turn 0.262, no accel/brake. Its motions and PTYA rows (0..12, 28, 35, 39, 43, 52..56) come from main's notes. The `.mset`/`.mdlx` files aren't in this upload, so they are an Assumption until baked from the user's install.
- **Growth**, `fmlv` form 3, Checked (files):
  - Growth level 1,1,2,2,3,3,4 at Lv1..7 (Dodge Roll LV1..MAX, items 564..567).
  - Rewards: Lv2 Auto Limit 568, Lv4 Draw 405, Lv6 Lucky Lucky 407.
  - EXP column 3, 6, 12, 19, 23, 27.
  - Dodge Roll invulnerability from `fmab` [1..4] +0x40: 16/24/32/36. Main says 10/16/24/32; recheck which index is which.
- **EXP rule** (`0x1403E8730`, called from `0x1403D8B40`'s follow-up branch), Checked (decomp + files): in form 3, +1 per use of a finishing follow-up command: 686 Giga Impact, 700 Finish, 703 The End, 706 Judgment. That is one per finisher, not one per hit. Docs §9 says "per hit": that needs correcting.
- **Menu**: `0x1403F29E0` (and `0x1403F34A0`, `0x1403F36C0`): with `+0xDE0 == 3`, row 2 becomes `0x2C3`. `0x2C3` is "Limit", execute 707, submenu 12, and its list is built from `0x14074EBE8`. Shortcuts in form 3 come from `DAT_1405C9678[k]` (4 u16). Checked (decomp). Neither table's values can be read without a KH2 exe; none is in the upload.
- **Limits**, Checked (files):

  | Limit | cmd | MP | follow-ups (parent at +0x22) | limt row |
  |---|---|---|---|---|
  | Sonic Rave | 698 | 60 | 699 Rave, 700 Finish | 27 sonic_rave |
  | Last Arcanum | 701 | 75 | 702 Bash, 703 The End | 28 last_alcanam |
  | Strike Raid | 704 | 65 | 705 Raid, 706 Judgment | 29 strike_raid |
  | Infinity | 683 | 80 | 684 Shoot, 685 Impact, 686 Giga Impact | 30 ragnarok |

  - All rows are execute 707. Starts have flags 0x10112, follow-ups 0x150/0x152/0x352.
  - The `limt` character bytes are 1, 0, 0: Sora alone, so these are limits used solo.
- **Grey / pay**, Checked (decomp):
  - `0x1403D88E0` gives state 5 (greyed) while a limit runs (`0x142A24CC0`) or MP Charge runs (`0x1403C1D10`), and state 2 (still usable) when MP < cost.
  - For execute `0x2C3` it also gives 5 if a partner who is present fails `0x1403C3950`. Assumption: that means "partner busy".
  - There is no in-battle check, so KH2's limits work out of battle (same as s336).
  - Paying: `0x1403D8B40` pays through `0x1403D3CF0`. Going below 0 sets MP to 0 and starts MP Charge (`0x1403C20A0`). An area can override the cost (`0x1403A39B0`, area +0x6A4).
- **Follow-ups**: while a limit runs, the command goes to the script as trigger 0x17. Checked (decomp). The `limi` script logic (counts 5/3, chains, the hook) is from main's reading: Assumption.
- **HP drain**, Checked (decomp, `0x1403D1730`): after a hit, a non-zero ATKP +0x2F is passed to the owner's method +0xE8. Main read that as HP, clamped (Assumption).
  - Values, Checked (files): 1857..1861 = 10, 5, 5, 5, 15; 1866/1867 = 8/8; 1878/1879 = 5/5; 1883..1895 = 3, 1×5, 2×6, 5.
  - Only these rows have it. Mapping them to limits is from main (Assumption).

## 3. How KH1 hosts it (the Valor/Wisdom path)

- **Moveset**: add `Kind::Limit` to `native_capable`, `LIMIT_SPECS` to `FORM_SLOTS`, prty 0x42 in `refresh_kh2_numbers` and PTYA 0x42 through `native_data`/`kh1picker::at_kh2`, exactly as §7/§8. `two_blades` false.
- **Square**: base Sora's native guard / counterguard (`combomods::counterguard`) and Dodge Roll (`styles::Kind::DodgeRoll`), with Limit's clips and `fmab` at the form's growth level.
- **Menu**: KH1's top menu (`0x140292090`, menu 0 `0x140529600`, magic list `0x140529638`). The Magic row is renamed "Limit" and its list gets four spell-type records, the way `drive.rs` puts forms in menu 3. Greying follows KH2's `0x1403D88E0`.
- **Limits on KH1's own limit system**, Checked (decomp + exe):
  - The dispatch `0x14028D3D0` starts with `0F B6 02 83 C0 B5` and switches on the command id.
  - The start `0x14028D800` sets bit 16 of `0x142867364`, swaps the motion table (`0x14029E850`), starts the module (`0x140397A50`) and pays (`0x1402A5B60`).
  - Each module registers a KH1 state (`0x1402B1B80`) and enters it (`0x1402B1960`). Strike Raid's `0x14039FFD0` registers state 0x27, requests 0x10000E6 and sets the hit callback `0x1403A0290`.
  - Ours does the same: new command ids caught in `0x14028D3D0`, our start sets bit 16 (so KH1's offer `0x14028D0F0`, data loader, end task `0x14028D620` and event checks all see "limit running"), a KH1 state of ours per limit whose step carries the `limi` logic as plain state code, motions 211..248 as KH1 slots on requests of ours, hits through `kh1rows` with KH2's ATKP, and follow-ups through KH1's Triangle box `0x140293000(3, cmd)` coming back into the same dispatch.
  - Strike Raid's thrown Keyblade: reuse KH1's own Strike Raid object (its module data via `0x1402AE290`) with KH2's flight numbers. Needs research.
  - Infinity's shots have the same open problem as Wisdom's shots (a KH1 script-machine homing shot, §8.2).
- **HP drain**: at KH1's contact `0x1402BF330` (already hooked, `probe::hit`), the hit's KH2 ATKP +0x2F goes into KH1's HP change. Data only, no per-frame code.
- **Look**: a `tint::Look::Limit`. Effects via `kh2fx`/`kh2dpx`.
- **KH1 limits option**: KH1's five run unchanged through `0x14028D800` from the Limit list. The t963 `ret` patch over `0x14028D0F0` must stay in Limit Form either way.

## 4. Plan (one system per version, numbered from the current VERSION)

1 moveset on KH1's states (Kind, Drive entry, slots, physics, PTYA, finishers, arrival 53, no partners); 2 Square (guard, Zantetsu Counter, Dodge Roll by level); 3 form level/EXP (new save u16, +1 per finisher follow-up, growth, rewards); 4 look + effects; 5 Limit list in Magic's row (records, greying, shortcuts); 6 limit start/end on KH1's path + Sonic Rave; 7 Last Arcanum; 8 Strike Raid; 9 Infinity; 10 HP drain per hit; 11 MP-short rule; 12 KH1-limits option if chosen.

## Questions for the owner

1. **Which limits.** (a) KH2's four only (1:1); (b) KH1's five (main s297/s299); (c) a Config row "Limit Form's limits: KH2 | KH1", KH2 by default. Recommend (c): (b) is an owner override with nothing in KH2 to match, and a row keeps both.
2. **Cost.** (a) 4 bars (KH2, testing now); (b) 3 (s331); (c) Balance Changes > Limit 3 bars (inventory step 7). Recommend (c), default 4.
3. **Look.** KH2's Limit Form *is* KH1 Sora's outfit, so 1:1 means no tint. Main's black outfit is the owner's. (a) no tint; (b) main's; (c) a row. Recommend (c) with main's look as default (it is the only way to see the form in KH1).
4. **MP short of cost.** (a) KH2: usable, MP drops to 0 and MP Charge starts (needs MP Charge built first, one version); (b) needs the full cost until MP Charge exists; (c) usable, MP drops to 0, no charge. Recommend (a), with (b) in the meantime.
5. **HP drain for KH1's limits** (only if 1(b)/(c)). (a) main's 5/8/2/8/1; (b) none; (c) KH2's per-hit values on KH1 hits. Recommend (a).
6. **Infinity's shots / Strike Raid's flight.** (a) KH1 script-machine shots, done together with Wisdom's; (b) ship Infinity later; (c) a temporary "running along" flight. Recommend (a), with (b) until then.
7. **Shortcuts.** (a) read `0x1405C9678` from the user's KH2 exe at build; (b) main's order; (c) KH1's own shortcuts. Recommend (a).
8. **Rewards.** KH1 has no Auto Limit; Draw and Lucky Lucky need KH1 counterparts. (a) skip Auto Limit; (b) build Auto Forms; (c) ask per reward. Recommend (a) for now.

Conflicts: 3 bars, KH1 limits, black outfit, HP on KH1 hits are owner overrides; out-of-battle use and no mid-limit revert already match KH2.

---

## Master Form (KH2 form 4, inventory row 13): research

**Base check first.** The source is **t961** (`lib.rs:32`, top AGENTS stage 961), but the docs (§20–§22, inventory row 20) describe t962–t968 as built, and `win/valorslot.rs` isn't in the upload (Checked (code)). Re-read VERSION before coding.

## 1. What testing has now, and what main built

**Testing (Checked (code))**
- No Master anywhere: `form::Kind` = Valor / Wisdom / Final (`form.rs:80`); `native_capable` Valor, Wisdom; `two_blades` Valor, Final; `magic_sub` 10 in Wisdom only (`form.rs:238`), `wisdom_kh2_spell` Wisdom-only (`formmagic.rs:79`); `form_growth` (`form.rs:592`) and the look lack Master.
- Master already has these pieces:
  - Cost 4 bars (`drivegauge::cost(4)`), both partners (`driveparty`, t958), owned bit after Anti-Sora (`storyforms.rs:109`), the level cap (`formexp`), Drive orbs (t944–t948).
  - Base Sora's Aerial Dodge is native (`combomods::aerial_dodge`, `kh1rows.rs:228`, `takeover.rs:606` levels, `shared.rs:327` press).
- Missing: `P_EX100_TRIF` slots; a level record (`formexp` has three u16 at `+0x882..0x887`); a Keyblade slot; its EXP rule (`formexp.rs:28`).
- **Bug risk for Master (Checked (code)):** testing's `form_abilities` gives guard whenever motion 42 exists (`form.rs:562`). Master's 42 is the repel clip with no guard window (main's data). Main fixed this as "a guard window in 42"; testing has not.

**Main (spec only; s300, s301; main `form.rs:69-77`, `docs/drive-model.md` "Master Form in the KH1 port")**
- `P_EX100_TRIF` moveset 4, finishers 183 / 184; Aerial Dodge one above Sora's learned level; Wisdom's magic at sub +20 on casts D370..D374; no guard.
- The left Keyblade `W_EX010_TRIF_L` floats. Main blended between the left hand (held, under about 8 units) and hanging from the right hand (floating). That was a per-frame pose fix in `leftblade.rs:797`, i.e. [RENDER].
- M: row at save `+0x48B`, default Kingdom Key (0x51), copies in use across V / F / M. Yellow look: jumpsuit, collar, hood lining yellow, belt black.
- s301: KH1's spells "don't move Sora". Main moved him along the cast motion's root, but only Gravity (on Thunder's motion, about 1050 units) travels.

## 2. What KH2 does (Steam)

- **Cost and gauge:**
  - 4 bars (Checked (files), 03system `cmd`, §13.3).
  - Drain and revert are the same for every form: no form-id branch in `0x1403A92C0` or the change `0x14040CAA0`, which takes the form's Keyblade by `0x140405190(form)`. Checked (decomp).
- **Partners:** `0x140400B20` takes both slots for forms 4 to 6. Valor takes partner id 3, Wisdom id 2, Limit none. `0x140400740` uses the same test. Checked (decomp).
- **Moveset (Checked (files), 00battle `ptya`):**
  - Set 4 is at ptya +0x1050: 16 rows of 0x44 bytes, each with word +10 = 4.
  - Ground 5 (kind 3, 185), 4 (181), 0/10 (182), 6 (185); finishers 8 (kind 3, 183), 7 (kind 2, 184), 58 (183). Air 21 (kind 3, 181), 9/20/22/23 (185), 24 (182); air finisher 25 (184); row 1 (kind 0, 181).
  - No guard row 11, no Square rows, no ability rows. Matches main's table.
- **Weapons (Checked (files)):** 03system `wmst` rows 8 and 9 (4×2, 4×2+1) are `W_EX010_TRIF_R` / `_L`. The off-hand blade is driven by its own weapon motion set, so the float is data in `_L`'s motions, not code.
  - Which Keyblade it shows: resolver `0x1403DFEB0` case 0x23B is the current form's slot (`0x140405190(DAT_1409acdd4)` = record +0x16 + form×2), or else Sora's own. Only forms 1, 4 and 5 take a Keyblade (`0x1402FC860`). Checked (decomp).
  - Whether the slot Keyblade's stats apply in KH2: Not traced (Assumption: testing's Valor rule, the owner's request).
- **Growth and levels (Checked (files), 00battle `fmlv` rows 28–34):**
  - Aerial Dodge levels by form level: 1, 1, 2, 2, 3, 3, 4.
  - Rewards: Auto Master (387) at Lv2, Air Combo Plus (163) at Lv4 and Lv6; EXP 40, 80, 140, 224, 448, 668.
- **EXP (Checked (decomp)):** a picked-up prize `0x1403DCA50` with an orb value (reward table `0x1405C7420` +6) calls the gauge add `0x1403A9CF0` and then `0x1403E8760(value)`. That adds `value` EXP only in form 4 (`+0xDE0`): small orb 1, big 3.
- **Magic:**
  - Master's ATKP rows exist at subs 20 / 22 / 24 only, e.g. ATKP 502: power 200 / 240 / 300, no odd (finisher) subs; Wisdom 10..15, Final 30..35. Checked (files); reading the record's second byte as the sub is an Assumption.
  - The "+20" rule's code wasn't found in the Steam decomp. Assumption, as testing's §8.3 says for Wisdom.
  - Cast clips, keyblade hits (Fire 1097/1096, etc.) and root travel (Fire about 800, Thunder about 1050, Blizzard out and back, Reflect 92, Cure 0) are main's data. Not re-checked: `P_EX100_TRIF.*` and `W_EX010_TRIF_*` are not in `/mnt/user-data/uploads/data/kh2/obj`.
- **Square:** `0x1403A81F0` finds no Square rows in Master, only its growth path (Checked (decomp)); testing's control reads this from data.

## 3. How KH1 hosts it natively (reuse)

- **Slots and states (Valor's path, §7.5):**
  - Add `Kind::Master` to `native_capable` with `MASTER_SPECS` in `kh2slots::FORM_SLOTS`.
  - Ground 185 / 181 / 182 on 0xC8..0xCA; finisher 183 on 0xCB; air on 0xCC..0xCE.
  - 184 and 58's 183 on our combo rows, arrival 53 on ours, 202 on our Aerial Dodge row. KH1's picker reads PTYA set 4 (`native_data`). Physics from prty 4 (main's data, Assumption until read). Baked once; one rebake.
- **The second blade (Valor's t830+ holder, `kh1-weapon-second.md` §10.4):**
  - `holder_poses` bakes `_L`'s weapon motion relative to KH1's left hand. Assumption (main's data: `_L` sits on the left-hand bone): the float then comes out 1:1 with no per-frame code, and main's right-hand blend isn't needed.
  - Hits `lefthits`, KH1's trail; `two_blades` for Master.
- **Stats:** valorstats' rule generalised to the form's slot Keyblade (it writes the stats every step: per-frame, left as is).
- **Magic (Wisdom's §8.3 path):**
  - Generalise `wisdom_kh2_spell` and `magic_sub` to per-form (Master 20).
  - Master's cast clips on KH1's cast requests; Gravity / Stop / Aero stay KH1's own on them.
  - Travel natively: H10 carries KH2's root travel for requests with KH1's travel flag (`kh1-anim-slots.md` §1.6). Assumption: KH1's cast requests can carry that flag. Check it in the decomp before relying on it; if they can't, set the flag on our cast slots' request rows as data.
- **EXP:** at testing's Drive-orb pickup (`driveorbs`), in Master, add the orb's value through `formexp`.
- **Growth:** `form_growth`: Master sets `aerial_dodge = formexp::growth(Master)`. The existing native Aerial Dodge then plays Master's own 202.
- **Look:** a `tint::Look::Master` built once into the mod folder, as Valor's and Wisdom's are.

## 4. Plan (one system per version, after go-ahead; numbers from the current top)

1. **Slots, combo pick, physics.** `Kind::Master`, `MASTER_SPECS`; one Keyblade, no magic yet; guard only from a guard window in 42 (all forms); growth LV1 until v4.
2. **Second Keyblade.** `W_EX010_TRIF_L` holder bakes, lefthits, trail.
3. **The M: slot.** Save byte (Q1), the free count (t967's hook) and the compare box (t968's), all for the form's slot.
4. **Form level and EXP.** The level record (Q1), EXP from orbs, Aerial Dodge by level, and the fmlv rewards (387, 163 ×2) in KH2's list.
5. **Magic.** Wisdom's path per-form, sub 20, KH2's spells on Master's casts.
6. **Cast travel.** The KH1 travel flag on cast requests (Q4).
7. **Effects / sounds.** Elements 20..32, bank 500, once the files are found.
8. **Look.** Yellow tint.
9. **Drive list.** Master in `Kind::ALL` (KH2's order: Valor, Wisdom, Limit, Master, Final). Possibly folded into v1.

**Questions**

1. **Save bytes.** Master needs one Keyblade byte and a two-byte level record. Main's `+0x48B` is the high byte of testing's layout mark (u16 at `+0x48A`, values 0x4B02..0x4B04).
   - (a) Shrink the mark to a u8 at `+0x48A` (02..04), Master's Keyblade at `+0x48B`, which matches main. Old testing saves read 0x4B there. That is not a Keyblade (Assumption), so the slot reads as empty, as KH2's would. The level record goes after `+0x887`, but the bytes from `+0x888` are outside the checked range (needs an exe scan).
   - (b) Keep the mark. Master's Keyblade goes at `+0x489`, which takes Final's spare byte, and the level record goes in a byte pair found by a new scan.
   - (c) One record per form in KH2's shape (Keyblade + level) in a new scanned block, migrating `+0x488` and `+0x882..`.
   - Recommend (a): reads main saves directly; costs one mark version first.
2. **The float's anchor.**
   - (a) KH2's: from the left hand, by the holder bake (data only).
   - (b) Main's right-hand blend.
   - Recommend (a): 1:1, no per-frame code.
3. **The empty M: slot.**
   - (a) KH2's rule: Sora's own Keyblade (resolver 0x23B), as t966 does for Valor.
   - (b) Main's Kingdom Key.
   - Recommend (a).
4. **Which Master cast clip KH1's Gravity / Stop / Aero play** (that sets their travel: 63 is Thunder's finisher clip, about 1050 units; Cure's clip has none): (a) all on 63 / 64 as Wisdom's mapping, so all travel; (b) main's: only Gravity travels (Stop / Aero on a clip without travel); (c) per spell by its KH1 element. Recommend (a): the same mapping as Wisdom's, travel from data.
5. **Order:** (a) now; (b) after Limit (the inventory's "After Limit"). Recommend (b) if Limit is next anyway.

**Conflicts:** `+0x48B` (Q1); main's Kingdom Key default (Q3); main's floating blend [RENDER] and [CTRL] not carried; testing's guard rule needs main's fix.

**Not checked:** Master's own files (not in the upload: clips, travel, prty, effects are main's data); the "+20" code; KH2's slot-Keyblade stats; KH1's cast travel flag; nothing in game.

---

## Final Form: research for a native port (main = spec, KH2 Steam decomp = check)

Upload: `lib.rs` says t961, but its docs describe t966–t968 (`valorslot.rs`), and the upload lacks `holder.rs`, `lefthits.rs`, `valorslot.rs`. Re-read VERSION before building.

## 1. What testing has, and what main built

**Testing (Checked (code))**
- `Kind::Final` exists: model `P_EX100_ULTF`, growth "Glide", `two_blades()`. It isn't in `Kind::ALL` (`form.rs:89`) or in `native_capable` (`form.rs:125`). `choose` turns it down on Steam (`form.rs:500-504`, t873).
- `probe::pad` still calls `form::pad` (`probe.rs:289`), so the secret code still asks for Final, which then gets turned down.
- The second blade is always the Kingdom Key: `valor::FINAL_KEYBLADE = 0x51` (`valor.rs:50`), and `valorstats.rs:64` reads it there. Final's look is `None` (`form.rs:607`).
- Final's floats still go through the old per-frame retarget: `leftblade::final_float` (`leftblade.rs:765`, called from `pose.rs:2176`).
- Already in place: ULTF loaded (`pose.rs:709`), `kh2fx::FINAL` (`kh2fx.rs:64`), form level/EXP (`+0x886`), owned bit (`+0x881`), Story Unlocks "Riku 2" (`storyforms.rs:12`), gauge cost 5 (`drivegauge.rs:21`), both partners (`driveparty.rs`).
- `glide.rs:139` `built()` excludes every form.

**Main (spec only; Checked (code/notes), never copy its code or its Epic addresses)**
- s231: a secret pad code entered Final.
- s271: Final became the Drive list's third entry (record 0x80, word 0x1FC, argument 0x12) at 3 MP. The code did nothing, and from s272 it gave Anti.
- s262 look: a white outfit and white shoes with a black belt (the owner's words, not sampled from KH2's files).
- s262/s282 floating blades: they follow KH2's blade bone 2, so the glide spins (−12π over 60 frames). Both hang from the right hand.
- s262 hits: the weapon motions' attack windows hit during jump, fall, land and glide.
- s264 magic, Wisdom's way: Fire, Blizzard and Thunder are KH2's at sub +30. Cure, Gravity, Stop and Aero are KH1's, played on Final's motions.
- s266 combo effects: placed on the drawn blades' bone 2.
- s279 level 99, then s307 Riku 2; s283 F: row at save `+0x489` (Kingdom Key fallback); s286 slotted copy in use.

## 2. What KH2 does

- **Id and moveset.** Form 5, `P_EX100_ULTF`, moveset 5, weapons `W_EX010_ULTF_R` / `_L` on bones 178 / 86. Checked (files, as main's notes): this upload doesn't include ULTF or W_EX010.
- **prty record for moveset 5.** Checked (files, `03system.bin` pref/prty, index entry 5 at offset 492): walk 2, run 16, jump 585, field +0xC = 0.19635, accel 0.94, brake 0.99 (ground inertia, like Wisdom), last field 170.
- **Cost and gauge.** Checked (files + decomp): `cmd` 0xC costs 5 bars (paid at `0x1403C2780`). The form gauge is `(level+3)×600` (`0x1403C17C0`).
- **Partners.** Checked (decomp): `0x140400740` and `0x140400B20` count both party slots for form 5, so Final takes both.
- **How Final is first gained.** Checked (decomp `0x140421D60`). A Drive only becomes Final through the Anti roll. When Anti would trigger in battle, and all of these hold:
  - Final (item 0x1D) is not owned;
  - Final's partners are available;
  - story flag 0x4842 is set (`0x14039A980`);
  - the bars cover Final's cost;

  then a second roll at `min(3.0 × 3.0^n, 75)%` decides it (sstm +0x98 / +0x9C / +0xA0; n is Final record +4, which goes up 1 per try). On success:
  - n is reset to 0;
  - item 0x1D is given (`0x1403C3FE0`);
  - the Anti counter drops by 10;
  - the result is form 5.

  Odds 3%, 9%, 27%, then 75% (twin test `0x140422050`). No code path forces Final; an event-script force would be data (Assumption). Choosing Final later lowers the Anti counter by 10.
- **Slot fill on gain.** Checked (decomp `0x1403C3FE0`, `0x1402FC860`): forms 1, 4 and 5 take a Keyblade. An empty record +0 gets the first Keyblade in stock in item-table order.
- **Which Keyblades.** Checked (decomp):
  - The change takes the form's weapon `0x140405190(5)`, from Sora's record +0x16+form×2 (`0x14040CAA0`).
  - The resolver `0x1403DFEB0` case 0x23B returns the form's Keyblade, or Sora's own when that slot is empty.
  - `0x1403D59C0` gives hand 0 Sora's own Keyblade (`0x140405190(0)`) when in a form. So hand 0 = Sora's own and hand 1 = the F: slot. Assumption: partial trace.
- **Magic.** Checked (decomp `0x1403C5170`): sub = level×2−2 (+1 on a finisher), plus 10 / 20 / 30 for forms 2 / 4 / 5, gated on player +0x6C8 bit 7. All of KH2's spells work in Final, Cure included. Main's "sub records 30..35 exist" is Checked (files) by main only.
- **Glide.**
  - The Square handler `0x1403A81F0` reads the Glide level (ability block +0x23, `0x140401680(…,3)`). It glides once the height above ground beats fmab +0x34 for that level (`0x1403E92D0`, indexed by level only). So Final uses the same fmab rows as base Sora at its own level. Checked (decomp).
  - Level 5 is used in world 0x12 room 0x17 or with `0x142A11400` bit 13 set (`0x1403ABCD0`). What that place is: Assumption.
  - Final's level comes from its form level, via the growth table `0x1405C9030` (Glide 106..109). Checked (exe, testing §9).
- **Floating blades and their hits.** Checked (files, main's notes):
  - Each weapon object plays its own motion. Final's glide clips 205..207 spin bone 2.
  - Weapon windows on bone 2 (group 2, three spheres of radius 60):
    - jump: ATKP 1273/1274 (power 20) at frames 2..12, then 1286/1287 (power 10, re-hit 10) from 12;
    - fall: 1275/1276;
    - land: 1277/1278;
    - glide: 1281/1282, power 100, re-hit every 30;
    - hover 208: ATKP 162, a guard.
  - The right and left blades are separate attacks (`0x1403D25D0`, testing §7.2, Checked (decomp)).
- **Damage.** Which Keyblade's Strength a hit uses (per blade, or a sum): not found. Assumption, open.
- **Revert.** Checked (decomp `0x14040CE60(…,0)`, cmd 5): the same as every form, bars untouched.
- **EXP.** Checked (decomp `0x1403E8780`): +1 per defeated enemy with id 3000..4014 (Nobodies).

## 3. How KH1 hosts it natively

- **States and slots.** A Final `FORM_SLOTS` entry (`kh2slots.rs:390`): idle/walk/run, combo rows, jump 3 on takeoffs 0x4/0x9, fall 4, land 5, reactions, arrival, casts, via `native_start`/`arrive`/`revert` (§7.5); prty through `refresh_kh2_numbers` and `ground_inertia`. One rebake.
- **Floating blades (Valor's machinery).**
  - Right blade: KH1's weapon bones 280..292 in each baked Final slot, from `W_EX010_ULTF_R` through `pose::kh2_hold` (bone 2, so the spin is baked).
  - Left blade: holder motions from `_L` (`kh1-weapon-second.md` §10.4 and §12), posed by KH1's pass.
  - `final_float` (per frame) goes for baked slots.
- **Slot (Valor's t966–t968).** F: at save `+0x489`. Checked (exe): no instruction in `/tmp/kh1.asm` uses displacement 0x489, and none addresses `0x142DE97E9`. Assumption: nothing reaches it through a computed pointer.
  - `valorslot`'s free count and compare box, KH2's fill order, `valorstats` read it.
- **Magic (Wisdom's).** `form::magic_sub` becomes per form (Wisdom 10, Final 30). `formmagic`'s Wisdom path is keyed on "casting form", using Final's cast clips (56..73). Gravity, Stop and Aero are KH1's on Final's motions.
- **Glide.** `glide.rs` (state 0x15 wrappers) opens to Final. Final's 205..208 go on the glide requests, and the level comes from `formexp::growth`.
- **Weapon-motion hits.**
  - Windows baked as KH1 motion events; hits through `lefthits` and KH1's contact (`0x1402BF330`) with KH1's re-hit; hover 162 a KH1 guard object.

## 4. Plan (one system per version, after a go-ahead)

1. **tA, Final on KH1's states.** `FORM_SLOTS` Final specs and prty numbers. `native_capable` and `Kind::ALL` gain Final (Drive record 0x80?, a word, argument 0x12; free record: Assumption, check btltbl), listed when owned (`formown`).
2. **tB, the floating blades baked.** Right weapon bones and left holder motions. `final_float` is retired.
3. **tC, the F: slot.** `+0x489`, the equipment row, the free count and compare box, KH2's empty-slot and fill rule. `FINAL_KEYBLADE` goes.
4. **tD, magic.** A per-form sub (+30), Final's cast clips.
5. **tE, glide in Final.**
6. **tF, the weapon windows' hits** (jump, fall, land, glide; the hover guard).
7. **tG, combo effects and the left trail** on blade bone 2, using `_L`'s windows.
8. **tH, the look** (`tint::Look::Final`).
9. **Later, with Anti:** the Final roll (`0x140421D60`). The owner's round-2 answer 11 (a).

## 5. Questions (options, effect, recommendation)

1. **First unlock.**
   - (a) KH2's: only through the Anti roll, so Final waits for Anti, with Riku 2 standing in for flag 0x4842.
   - (b) Now: the Story Unlocks table (Riku 2). With Story Unlocks off, owned like Valor and Wisdom.
   - (c) Main's level 99.

   Recommend (b) now, (a) with Anti.
2. **The secret code.** The owner settled it on Anti (row 14), but testing's code still asks for Final.
   (a) does nothing until Anti; (b) leave it. Recommend (a).
3. **Cost.** Main's 3 MP = testing's MP mode (`COST_MP` 3); gauge mode 5 bars (KH2). (a) keep. Recommend (a).
4. **Look.**
   (a) colours from KH2's ULTF textures, built once (1:1); (b) main's white, black belt. Recommend (a).
5. **Cure in Final.**
   (a) KH2's Cure, as testing's Wisdom; (b) KH1's, as main. Recommend (a): KH2 casts it at sub 30.
6. **Empty F: slot.**
   (a) KH2's: Sora's own in both hands; (b) Kingdom Key (today). Recommend (a), as Valor's 1 (a).
7. **EXP (still waiting on answer 7).**
   (a) none (no Nobodies in KH1); (b) a KH1 stand-in. Recommend (a).
8. **Hover guard (ATKP 162).**
   (a) a KH1 guard object; (b) none. Recommend (a).

**Conflicts:** main's code, 3 MP and look are owner rules, not KH2's; main's KH1 Cure contradicts KH2; testing's §10 hold and t873 refusal end with the owner's new instruction; the two-blade damage rule is open in both.

**Couldn't check:** ULTF/W_EX010 files (not staged), Final's PTYA, btltbl record 0x80, hand order past `0x1403D59C0`, an event-forced Final, anything in game.

---

## Anti Form (KH2 form 6, inventory row 14): research

Tags: Checked (decomp) = Steam KH2 kh2.c / KH1 kh1.c; (files) = 00battle.bin / 03system.bin; (code) = the testing upload; Assumption.

**Before building:** the upload doesn't match itself. `lib.rs` says VERSION `t961` and AGENTS.md's top stage is 961. But `win/form.rs` (newer timestamp) calls `drivedim` and `formlimits`, and docs §20.5–§22.5 describe t962–t968. `hurt.rs`, `formcmds.rs`, `formlimits.rs` and `drivedim.rs` aren't in the upload. Re-read VERSION and the top stage on the real branch first. (Checked (code))

## 1. Testing now and main

**Testing** (Checked (code)):
- `form::Kind` is only Valor, Wisdom and Final (`form.rs:84-137`). `native_capable` is Valor and Wisdom only. There is no Anti anywhere in `win/`.
- The secret code (`form.rs:190-231`) logs "Final Form: the secret code entered" and sets `CODE_DONE`. Nothing reads `CODE_DONE` (its only uses are lines 203 and 227), so the code does nothing today. Final can't be entered anyway: `choose` refuses non-native forms (t873, `form.rs:499-505`).
- Reusable: `drivegauge::cost(6) = 9`, `driveparty` (Anti takes both), `formown` bit 5, the shared change/arrive/revert path, `tint` and its cache, the hit HP hook `0x1402BEEE0` (t954/t957), the limit-offer patch (t963).
- `orb()` (`form.rs:319`) always adds to the form gauge. In Anti it should drain it.

**Main (spec only)**, AGENTS s272–s292, s310, s318, s325, and `form.rs:39-53`:
- Secret code only: free, then half MP (s285), then 9 bars needing 3 (s318). 6-bar gauge drained by orbs; reverts when empty or on a KO.
- ×1.5 damage taken; no guard, even in finishers 183/184 (s283); no growth. Magic, items, summons, KH1 specials, Drive and Revert refused (no revert ever).
- Square: Anti Glide (row 11, motion 173), Side Spin (row 12, motion 161) (s288). Ledge hang by the foot, motions 7/8 with a per-frame pose shift (s273–s280). No Keyblade or trails.
- Look: `texmap` puts KH1 Anti-Sora's textures (`xa_pp_3020.mdls`) on Sora's HD textures, rebuilt every start (~3 s); yellow eyes (253, 230, 95) (s290, s292).

## 2. KH2

**The roll, `0x140421D60 (player, requested)`** (Checked (decomp), its only caller is the Drive command `0x140405440` kind 4):
- Bit 25 of `0x140749804` set: no roll at all (`0x14014EAD0`).
- The counter is Anti's form record `+4` (save `+0xE04 + 5×0x38`), the same field the other forms use for EXP.
- **Final requested:** counter −10 (sstm `+0xA4`), floor 0, and Final is kept.
- **Any other form requested** (Valor, Wisdom, **Limit**, Master): only if `0x140400740(6) == 0`. That needs both party slots filled and able, or a world Drive factor above 1.0.
  - Chance: the fmlv row for `6<<4|1`. Walk down the rows while counter < row `+4`, then chance = row byte `+1` × area byte `+0xE` (`0x1403A3950`, which is 1 with no area).
  - Ability-block `+0x14` bit 15 makes it 100 (what sets the bit: Assumption).
  - The counter goes up by 1 **whether or not in battle**.
  - Anti happens only if rand/0x28F5C29 < chance **and** in battle (`0x142A11404`).
- **Anti fmlv rows** (Checked (files), 00battle `fmlv` rows 42–44):

  | Counter | Chance |
  |---|---|
  | ≥ 10 | 25% |
  | 5–9 | 10% |
  | < 5 | 0% |

  Main's "10% from 5, 25% from 10" is correct.
- **Final instead of Anti:** if Final (item 0x1D) isn't owned, Final is allowed, story flag 0x4842 is set and bars ≥ 5, there is a second roll. Its chance is min(3 × 3^FinalCounter, 75)%. On success: Final is given, Final's counter goes to 0, Anti's counter −10, and the result is Final.
- **Otherwise the result is Anti** and the counter −6 (sstm `+0x90`).
- **Cost:** the Drive command pays the cost of the result (`0x1403C2780`). Anti's cmd 0xD costs 9 and empties the gauge (Checked (files)). The greying uses the requested form's cost.
- **Reset:** gaining a form item resets the counter to 0 (`0x1403C3FE0`, §12).

**Gauge** (Checked (decomp + files)): entry `0x1403C17C0` fills HP and MP; level sstm `+0x8C` (5) − 2 = 3, so (3+3) × 600 = 3600 ticks (60 s in battle, 120 s outside). Hits drain −15 × ATKP `+0x29` as in every form; orbs drain instead of fill (`0x1403C0B50`); Form Boost applies; empty reverts (`0x140404A40`); recovery kind 0xB clears Anti only.

**Damage taken** (Checked (decomp + files)): in `0x1403C12F0` (line 737699), a player target in form 6 takes `(int)(damage × sstm +0x94)`, and sstm `+0x94` is 1.5. This comes before the element resist and before Damage Control.

**Commands** (Checked (decomp + files)):
- **Revert:** greyed in Anti **only in battle** (`0x1403F29E0` line 776498, `0x1403F36C0` line 776900). Out of battle KH2 lets Anti revert.
- **cmd `+44` form mask** (Checked (files); that bit n = form n is an Assumption, but it fits Valor and Wisdom):

  | Command | Off in |
  |---|---|
  | Magic (2), mask 0b1001010 | Valor, Limit, Anti |
  | Items (3), mask 0b1000000 | Anti only |
  | 0x49 (exec 73; Summon: Assumption), mask 0b1110000 | Master, Final, Anti |
  | Limit 0x51 | every form |

- **Reaction commands:** the reaction-command object (`0x140408520`, line 794255) sets `+0x35 = 5` in Anti. Greyed: Assumption. This conflicts with t962's "Talk and Examine in every form".
- **Prizes:** in Anti, Sora doesn't pick up HP orbs (prize kinds 0/1) (`0x1403DBEA0` and `0x1403DC7C0`).
- **Voice:** a callback skips `0x1403D69B0` in Anti (`0x140404D00`). It's a voice lookup: Assumption.

**Moveset** (Checked (files)): `P_EX100_HTLF`, moveset 6 (kh2-data `FORMS[5]`, Checked (code) on main). Its PTYA has 13 rows: ground 154/155, 153/152/151; air 182/181; flag-5 finishers 184/183; row 11 173, row 12 161. No guard row. The model and mset files aren't staged.

**No growth and no EXP:** no form-EXP adder exists for form 6 (§9, Checked (decomp)), and the fmlv Anti rows hold the chance table.

**Model:** `0x140405190(form)`, the objentry at `+0x16 + form×2`.

## 3. KH1 host

- **States:** as Valor and Wisdom: a `FORM_SLOTS` entry (ANTI_SPECS: motions 151–155, 161, 173, 181–184, cycles, reactions, jump/fall/land on KH1's requests, like WISDOM_SPECS, `kh2slots.rs:390-420`), `native_capable`, and Square through `combomods::valor_square`'s exe path (rows 11/12, ability 0).
- **Ledge:** KH1's ledge and bar states 13 and 15 (`combat.rs:299`). Anti's motions 7/8 go onto KH1's hang and climb requests as a form spec. The "by the foot" height is baked as a root offset (data), not main's per-frame shift. Assumption: motion 7's own root already hangs low. Check P_EX100_HTLF when staged.
- **Look:** KH1's own Anti-Sora (`xa_pp_3020.mdls` in `kh1_fifth`, main; not staged, Assumption). Its colours go onto Sora's HD textures through `tint`'s slot swap (`+0x70`), built **once** into `kh1-kh2-cache`. The owner's "every start" conflicts with the build-once rule.
  - With `player_model = kh2`: KH2's HTLF mdlx.
- **Keyblade:** hiding KH1's Keyblade and trail needs a KH1 decomp check of the weapon-draw and trail flags (not located).
- **Refusals:** `formlimits` (t963), `formcmds` (t962) extended per KH2's mask. The prize pickup needs a KH1 hook point (not located).
- **Damage:** the t957 Damage Control spot (hit HP hook `0x1402BEEE0`), ×1.5 truncated first. No new hook.
- **Save:** `+0x881..+0x887` are all taken (owned byte plus three u16 records). The Anti counter, Final's counter, and the Limit and Master records need new free bytes (an exe search).
- **Tick rate:** KH1's tick for 3600 ticks is still not confirmed in the KH1 decomp (Assumption, from `count_steps`).

## 4. Plan (one system each, after go-ahead)

1. **tA:** `Kind::Anti`, HTLF loaded, ANTI_SPECS baked, native_capable. No way in yet.
2. **Secret code → Anti:** `CODE_DONE` taken on the form frame, then `choose(Anti)`. Text "Final" becomes "Anti".
3. **Anti gauge:** 3600 ticks, cost 9, `orb()` drains in Anti, revert when empty.
4. **×1.5 damage taken.**
5. **Commands:** magic, items and summons off by KH2's mask; Revert greyed in battle; reaction commands off (Q3); no HP-orb pickup.
6. **Square:** Anti Glide and Side Spin; no guard.
7. **The Anti and Final roll and counter** (needs save bytes first).
8. **Look:** texmap built once, Look::Anti, eyes.
9. **No Keyblade, no trails.**
10. **Ledge hang.**
11. **Balance toggle:** "Anti free / no revert" (owner).

## Questions

1. **Revert:** (a) KH2's, greyed in battle and allowed outside, never with the toggle on; (b) never. Recommend (a), 1:1 with the toggle off.
2. **Code cost with the toggle off:** (a) KH2's, pays 9 and needs 3 bars; (b) always free; (c) the code works only with the toggle on. Recommend (a).
3. **Reaction commands, Talk and Examine in Anti:** (a) off, as KH2 (overrides t962 for Anti); (b) keep t962. Recommend (a).
4. **Counter storage:** (a) new free KH1 save bytes; (b) a side file in the mod folder. Recommend (a), it travels with the save.
5. **Area multiplier:** (a) 1 everywhere, KH2's no-area default; (b) a per-world table. Recommend (a).
6. **Final's second roll** (flag 0x4842): (a) after Riku 2; (b) none, Story Unlocks only. Recommend (a).
7. **Finishers 183/184** (main removed their guard windows): (a) what KH2's files do, after checking the motion; (b) main's, none. Recommend (a).
8. **Look with the KH2 player model:** (a) HTLF; (b) always KH1 Sora with Anti-Sora's textures. Recommend (a).

**Conflicts:**
- "Free / no revert" against KH2's cost 9, in-battle-only greying and counter. That's settled behind the toggle.
- The code gives Final today in its text only. Final then needs the Drive list once owned.
- "Every start" texmap against the build-once rule.
- The Anti chance needs both partners (`0x140400740(6)`). With Party Required Off it's undecided whether the roll still needs them.
