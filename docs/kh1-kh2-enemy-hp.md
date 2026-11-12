# Enemy HP: KH1's Scan bar and KH2's enemy HP gauge (testing-merge tm45)

The owner (chat 2026-11-11): "Can you add an option in the config to have the choice between KH1 and
KH2's enemy health bars? (Scan)", then "Add this to testing-merge". He picked (b), "B": with the row
on KH2, the gauge still needs KH1's Scan equipped. KH1's rule decides when it shows, and KH2's data
decides how it looks.

Config: Cosmetics > Enemy HP, KH1 | KH2 (default KH2). Ini: `enemy_hp = kh1 | kh2`. The row switches
live. Code: `crates/kh1-hook/src/enemygauge.rs` (KH2's rules, pure, tested) and
`crates/kh1-hook/src/win/enemyhp.rs` (the hook and the draw).

## 1. KH1's bar (Steam KH1)

| Address | What it does | Tag |
|---|---|---|
| `0x140274B20` | KH1's target cursor draw (the lock-on state `0x142DB3510`, the target `0x142DB3518`). Each frame, with a target, it asks `0x1402A4D40` and passes the HP to `0x140274E70`. | Checked (decomp) |
| `0x1402A4D40` | The Scan test. It needs ability 10 (Scan) equipped (`0x140284BE0 (10)` > 0), a target of kind 3 (`+6`, an enemy), and the target's `+0x374 & 0x60` not equal to `0x20`. It then gives the target's stats `+0x3C` (HP) and `+0x40` (max HP). With the target's `+0x374` bit `0x10000` it also signals a linked object (`+0x144`, `0x1402BCD60 (.., 3, 2, ..)`). | Checked (decomp) |
| `0x140274E70` | Stores the HP at `0x14284FE6C` and the max at `0x14284FE70` (from r8 / r9). It sets the frame's signal `0x142850900 = 1`, and moves the bar's state `0x14284FE60` from 0 to 1. | Checked (decomp + exe) |
| `0x1402758B0` | KH1's bar task. State 1 → 2 fades in 0.2 per 60 Hz frame (`0x14284FE64`, using the frame scale `0x14233FBDC`). State 3 holds. State 4 comes once a frame passes without the signal. State 5 fades out, then 0. In any state but 0 it calls the draw. | Checked (decomp) |
| `0x140275000 (hp, max, fade)` | KH1's draw. Set-up: `0x1402775D0` and `0x14027D190` on the register block `0x142851A00` (`+0xE0 = 5`, `+0x60 = 0x30003`), and the packet pointer `0x142851910` set to `0x142EE8F60`. Place: x = the HUD's left edge `0x14232DCC8` + 16. y = 24, or 65 / 80 while `0x140267620` reports another HUD piece, plus 28 while `0x14284DC30` (the lock-on cursor) is set. The frame is max / 2 long, at most 150. The HP is drawn in rows of 300 HP (up to 5 rows), each row hp / 2 long. The colours times the fade. First bytes `48 8B C4 48 89 58 08` (Checked (exe), the same on Epic). | Checked (decomp + exe) |
| `0x1402764B0` | KH1's quad submit. It writes the quad into the packet and processes it at once (`0x14027C9B0`), so a draw's order is the order of its calls. | Checked (decomp) |
| `0x14232DCCC` / `0x14232DCC8` | The HUD's side margin and its negative (`0x1401703C0`, from the screen's shape). The HUD is 512 wide for 4:3, with the margin added on each side. | Checked (decomp) |

So KH1's bar sits top left. KH1 has no second colour per row: its rows of 300 HP stack (Checked
(decomp)).

## 2. KH2's enemy gauge (Steam KH2, `dk::GAUGE_ENEMY`)

| Address | What it does | Tag |
|---|---|---|
| `0x14016C3A0` | Builds the field HUD. The enemy gauge is `0x140ABD260` (0x4B30 bytes): 6 pieces at `+0x38` (0x1F8 each) and 16 pip pairs at `+0xC08`. It uses the same sequence as the player's Drive gauge (`0x140153C20 (1)`, the `gaug` SEQD of `field2d/<lang>/zz0field.2dd`, which the mod already loads). | Checked (decomp) |
| `0x140182DD0` | Makes the pieces in this order, each at priority 7000, layer 6, x offset `+0x1DC = 85`: group 38 (the root), 42, 39, 40, 41, 43. Every piece but the root is parented to the root. Pip pair `i`: group 45 then group 44, at x = −i × `0x140192E00` (the group record's `+0x20`), y 0. | Checked (decomp); which piece the step is read from: Assumption (the pair's second, group 44) |
| `0x1403DD440` | Shows the gauge for the player's target (auto-target state 1 or lock-on state 2, `0x142A24E90`). It needs the target to have a status block `+0x5C0`, to pass `0x1403BA720`, and not to have `+0x120` bit 14. It also needs the player's ability bit set `+0x1D0` bit 19 (abilities by number, as `0x140402750` tests). The HP comes from the target's HP rows (`+0x5C0` + part × 0xC: `+0` HP, `+4` max). | Checked (decomp); that bit 19 is KH2's Scan: Assumption (the item table `03system.bin` isn't in this session) |
| `0x140183520` | Show: the max is set, and the HP shown, before and now are all set to the HP (at most the max). The full pips (44) restart for each bar under the HP's. | Checked (decomp) |
| `0x1401833E0` | Hide (the target lost, `0x1403DD510`), at once. KH2 has no fade. | Checked (decomp) |
| `0x1401832A0` | A hit (from `0x1403DCC10`, KH2's HP change with a loss, `0x1403D2C50` / `0x1403D2EB0`). "Before" becomes the HP before the hit, and shown and now become the new HP (at least 0). The damage piece 41 restarts. Pips under "before"'s bar become 44 if they are under the new HP's bar, else 46 (lost). | Checked (decomp) |
| `0x140183220` | A heal (from `0x1403DCBB0`, a gain). Shown, before and now all become the HP (at most the max). The full pips restart. | Checked (decomp) |
| `0x140183640` | Each frame: 200 HP a bar. Pips = (max − 1) / 200, at most 16 drawn. The top bar is max − pips × 200 long, or 200 when the HP is on a lower bar. The HP and "before" are each measured within their own bar ("before" is 200 when it is on another bar). Piece 42 shows only while the HP is on the top bar and there are pips. Piece 39 (the frame) is scaled by frame / 100 and placed at x = −frame, and so is piece 40 (its left end). Piece 41 is drawn at before / 100 and −before, and piece 43 (the fill) at fill / 100 and −fill. Backs (45) show for i < pips, and fulls (44 / 46) for i < "before"'s bar. 1 KH2 unit per HP: the bars grow left from the root's attach point. | Checked (decomp) |
| `0x140193370` | Sets the player's x scale (`+0x17C`) and its scale flag `+0xE8` bit 2 (cleared when every scale is 1). | Checked (decomp) |
| `0x140193880` | The attach point a child is placed from (`0x14016ABF0`). It uses the first animation with flag 2 in the frame: its place and bounce, plus its pivot × its scales, turned by its rotations. | Checked (decomp) |
| `0x14016ABF0` | A piece's place. Root: its own `+0x1DC + +0x1C8 + +0x1C0`. Child: the parent's attach point + the parent's `+0x1C8 / +0x1C0`, plus its own `+0x1DC + +0x1C8`. The root's `+0x1C0 / +0x1C4` is (0, the top-right stack's height, `0x140182C60`'s spring toward `0x140182BA0`'s sum). | Checked (decomp); the stack is 0 with nothing above it: Assumption (KH1 has none of KH2's pieces there) |

- **No colour per bar.** No function above changes a piece's colour, so the colours are the
  sequence's own (Checked (decomp)).
- **The lag after a hit** is the damage piece 41's own animation, restarted on each hit and drawn up
  to "before". Its timing and look are in the `gaug` file. They play 1:1 through `seqplay`, but
  aren't checked here: no KH2 files in this session.
- **Scale in the draw.** In the draw (`0x140192250`, `0x140196570`) the float arguments were
  dropped by the decompiler. That the player's x scale multiplies as in `0x140193880` is an
  Assumption. KH2's exe wasn't in this session.

## 3. How it's built (native: KH1's draw draws it)

- `win/enemyhp.rs` wraps KH1's draw `0x140275000`. On KH1, or while KH2's art and texture aren't
  ready, KH1's own draw runs and nothing of ours does.
- On KH2, KH1's draw's own set-up runs (the same calls and writes), then KH2's pieces are drawn
  through KH1's sprite-to-quad and submit, into KH1's bar packet `0x142851910`, with our `gaug`
  texture bound as the Drive gauge binds it (`gaugehud::Kh1`, `bind_ours`). KH1's TEX0 from its bar
  descriptor `0x1427F1430 + 0x18` is bound back after. KH1's bar isn't drawn.
- **When**: shown while KH1's bar state is 2 or 3. KH1's Scan test, target, events and HUD rules
  are untouched, as the owner's answer (b) asks. KH2 has no fade, so the gauge goes at once in states
  4 / 5. After a target is lost, a new one can take up to KH1's 5 fade-out frames to show, because
  KH1 starts a bar only from state 0 (Assumption that this delay doesn't matter).
- **Values**: KH1's HP and max (the draw's arguments, which KH1 took from the target's stats). A new
  target (`0x142DB3518` changed), a new max, or the bar coming up again counts as KH2's show. Less HP
  counts as a hit, more as a heal. The HP is used as it is: 1 KH2 unit per HP, 200 per bar. KH1's
  enemies have KH1's own HP, so a weak enemy's bar is short, as KH2's is.
- **Where**: KH2's 16:9 screen's top-right corner (512 + 85.33 across, 0 down) on KH1's HUD's top
  right (512 + `0x14232DCCC`, 0), at the Drive gauge's KH2 unit (`gaugehud::KH2_UNIT`, main's
  measured 384 / 448 × 1.34). Assumption: not checked against a KH2 screenshot.
- **The pieces' players** step only while the gauge shows (Assumption: KH2's keep running while
  hidden; it only matters for looping groups).
- The art: `kh2hud::start` now also loads `gaug` when Enemy HP is on KH2 (Drive off), and again when
  the row is switched to KH2.
- **Hooks**: one, `0x140275000` (Steam, first 7 bytes checked; Epic `0x140272E90`, same bytes).
  It calls `0x1402775D0` (`33 D2 C7 44 24 08`) and `0x14027D190` (`48 89 5C 24 10 55 48 8B`)
  (Checked (exe), the same on Epic), plus the Drive gauge's checked calls.
- **Per frame**: one call through the wrapper while KH1's bar task runs (only with a Scan bar up).
  KH2 mode adds the gauge's 38 players' steps and its quads.

## 4. Not checked

- In game (nothing run).
- The `gaug` groups 38–46's contents (place, looks, the damage piece's timing, the pip step):
  KH2's files aren't in this session.
- KH2's draw's float arguments (§2), and the stack height.
