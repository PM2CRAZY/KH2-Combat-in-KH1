//! `kh1-kh2.ini` next to the game, read when the game starts and again when saved (written with the defaults
//! the first time; keys it lacks keep their defaults). Each newer, untested-in-game feature has
//! a switch here, so it can be turned off without a new build.

use super::log;
use crate::growth::Mode;
use std::sync::atomic::{AtomicPtr, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

const INI: &str = "kh1-kh2.ini";
const DEFAULT_INI: &str = "; kh1-kh2 settings (read when the game starts, and again whenever this file is saved)\r\n\
;\r\n\
; abilities: kh1 = KH1's vanilla progression, and every KH2 ability in KH1's Ability menu (normal\r\n\
; play); remove = take the KH2 abilities back out of your save (load it, save once, then\r\n\
; uninstall); test = every KH2 movement ability at MAX; all = everything, every modifier\r\n\
abilities = kh1\r\n\
; hurt: kh2 = Sora takes hits KH2's way (dodge-roll invulnerability, KH2 guard, KH2\r\n\
; reactions); kh1 = KH1's own hit reactions\r\n\
hurt = kh2\r\n\
; commands: on = KH1's commands other than Attack (magic, items, specials, summons) take\r\n\
; Sora from KH2 until KH1 has him standing again (in the air: KH1's fall from there)\r\n\
commands = on\r\n\
; hit_fx_test: (test) KH2 hits write these values, in turn, into one byte of KH1's hit\r\n\
;   effect bytes (hit_fx_byte: 0, 1 or 2), spells too; empty = off. E.g. 0,1,2,3,5,10\r\n\
hit_fx_test =\r\n\
hit_fx_byte = 0\r\n\
; hit_effect_test: (test) each landed KH2 hit starts the next of these KH1 effect ids (Sora's\r\n\
;   effects, 0x1402ABB60) on the foe hit (hit_effect_on = target) or on Sora (= sora);\r\n\
;   ranges allowed, e.g. 0-20,30; empty = off\r\n\
hit_effect_test =\r\n\
hit_effect_on = target\r\n\
; kh2_sounds: KH2's own sound effects for these moves, from your KH2 install:\r\n\
; abilities (every KH2 ability move: Upper Slash, Explosion, Aerial Finish...), all (every\r\n\
; move, the plain combo too), off\r\n\
kh2_sounds = abilities\r\n\
kh2_volume = 1.0\r\n\
; damage_scale: times the damage of Sora's KH2 attacks (1.0 = KH1's hit x KH2's power)\r\n\
damage_scale = 1.0\r\n\
; criticals: on = KH1's critical hits on every attack, enemies' too (Critical Plus); off = none,\r\n\
;   as in KH2\r\n\
;   (the Config menu's Combat list switches it)\r\n\
criticals = off\r\n\
; party_criticals: the same for Donald's, Goofy's and the guests' attacks (the Combat list's\r\n\
;   Party Criticals row)\r\n\
party_criticals = off\r\n\
; kh1_ability_chance: on = KH1's 30% chance on Stun Impact and Gravity Break in KH1's combo\r\n\
;   (a failed roll picks the next move that fits); off = they come whenever they fit, as KH2's\r\n\
;   rows, the default (the Config menu's Combat > Tweaks list switches it)\r\n\
kh1_ability_chance = off\r\n\
; glide_cast: on = a KH2 spell chosen in KH2's glide ends the glide and is cast in the air, as\r\n\
;   KH2 does; off = refused with KH1's can't sound (the Config menu's Combat > Tweaks list switches it)\r\n\
glide_cast = on\r\n\
; drop_hop: on = an attack pressed on the ground with the floor giving out ahead hops base Sora\r\n\
;   off the ledge in an air swing, as KH2 (its row 1); off = no hop, the press takes the next\r\n\
;   attack that fits; Drive Forms keep KH2's (the Config menu's Combat > Tweaks > Ledge Hop row)\r\n\
drop_hop = on\r\n\
; kh2_voice: on = Sora's KH2 battle voice on those moves instead of KH1's; off = KH1's voice\r\n\
; (battle_quotes, below, takes its place once set)\r\n\
kh2_voice = off\r\n\
; auto_aim: off = swings turn toward only KH1's target (the R1 lock or the small cursor);\r\n\
; on = KH2's own pick, the nearest enemy in reach\r\n\
auto_aim = off\r\n\
; wall_clink: on = a swing that meets a wall clinks off it and bounces Sora back; off = it passes\r\n\
wall_clink = on\r\n\
; camera: kh2 = KH2's field camera (follows behind, right stick turns and zooms, R3 resets);\r\n\
; kh1 = KH1's own. The Config menu's camera row switches it in game too.\r\n\
camera = kh2\r\n\
; camera_scale: KH2's camera distances and heights times this; 0.89 = KH1 Sora's size against\r\n\
; KH2 Sora's (hips 78.9 / 88.5), so he is framed exactly as in KH2\r\n\
camera_scale = 0.89\r\n\
; kh1_slots: on = KH2's moves that have one play as new slots in KH1's own motion table (KH1's\r\n\
;   clock, events, hit windows and blends; for now KH2's ground combo, 151 / 152 / 154, and air combo 183 / 184 / 181, at Base\r\n\
;   / Air Base Combo KH2); a list of KH2 motions (e.g. 152) = only those; off = KH2's motions drawn over\r\n\
;   KH1's (as before)\r\n\
kh1_slots = on\r\n\
; kh1_bake: on = those slots are KH2's motions baked into KH1 motions at load (KH1's own pass\r\n\
;   shows them, nothing drawn over it; KH1's FK, its own leg IK with kh1_leg_ik); off = the\r\n\
;   slot is a retimed KH1 motion with KH2's drawn over it\r\n\
kh1_bake = on\r\n\
; foot_ik: on = feet placed on slopes and stairs: the baked slots keep KH1's own leg IK and ground\r\n\
;   lift (with kh1_leg_ik), and our foot placement runs on the clips drawn over KH1's pass; off\r\n\
;   (the default, the tester: \"lets actually have the foot iks off\") = plain legs everywhere\r\n\
foot_ik = off\r\n\
; kh1_leg_ik: (with foot_ik = on) on = the baked slots keep KH1's own leg IK and ground lift (KH1's\r\n\
;   pass bends his legs onto slopes and stairs, aimed at KH2's ankles and feet); off = plain legs\r\n\
kh1_leg_ik = on\r\n\
; camera_lower: KH2's camera (look point and eye together) this much lower (0 = as KH2)\r\n\
camera_lower = 0\r\n\
; camera_calls: kh2 = (camera = kh2) KH2's camera runs as KH1's own camera mode: KH1's moves,\r\n\
;   its ledge / climbing / pole cameras, its lock-on switches and its door walks keep KH2's\r\n\
;   camera and field of view (events and cutscenes stay KH1's); kh1 = they hand the view to\r\n\
;   KH1's camera (older builds)\r\n\
camera_calls = kh2\r\n\
; r2_camera_hold: on = while R2 is held the right stick doesn't move KH2's camera and R3\r\n\
;   doesn't reset it (the Config menu's Camera row switches it in game too); off = not\r\n\
r2_camera_hold = off\r\n\
; l2_stick_menu: on = while L2 is held the right stick moves the command menu like the d-pad and\r\n\
;   doesn't move the camera (KH1's or KH2's), as in KH2 (the Config menu's Camera row too); off = not\r\n\
l2_stick_menu = off\r\n\
; drive_page: 1 = Drive on the battle menu's page 1 (Attack, Magic, Items, Drive) and Summon on\r\n\
;   page 2 (Attack, Summon, Party, Limit), as in KH2; 2 = Summon on page 1 (KH1's row), Drive on page\r\n\
;   2 (the Config menu's Command Menu list too)\r\n\
drive_page = 1\r\n\
; dpad_left: kh2 = d-pad left at the battle menu's top flips page 1 / page 2, as in KH2; kh1 = it\r\n\
;   moves the cursor to Attack and there is no page 2, as in KH1\r\n\
dpad_left = kh2\r\n\
; cursor_memory: kh2 = after a command every open list and the top go back to their first row,\r\n\
;   a cursor follows its command when a list changes, as in KH2; kh1 = KH1's (lists keep their row)\r\n\
cursor_memory = kh2\r\n\
; command_lock: kh2 = a spell, item or summon is taken only when Sora can act on it, the list stays\r\n\
;   open until then and nothing greys for what Sora is doing, as in KH2; kh1 = KH1's\r\n\
command_lock = kh2\r\n\
; config_row: on = the Config menu's Camera Style row; off = no row (the ini alone switches)\r\n\
config_row = on\r\n\
; experimental: on = the experimental settings (Enemy Weight, Clash Combo, Knock-back Numbers,\r\n\
;   Medium Hits, KH2 Damage, Recoil, Guard Shape) show in the Config menu and their lines show at\r\n\
;   the bottom of this file; off = they are hidden, their lines taken out, and each keeps its\r\n\
;   default (Enemy Weight on, Clash Combo kh2, Knock-back Numbers scaled, Medium Hits blow, KH2\r\n\
;   Damage off, Recoil kh1, Guard Shape kh1); turning it off again puts the defaults back\r\n\
experimental = off\r\n\
; shortcuts: 4 = KH2's four shortcuts (L1 + the cancel button is the fourth; set it in\r\n\
; Customize); 3 = KH1's three\r\n\
shortcuts = 4\r\n\
; kh2_effects: KH2's own visual effects for these moves, from your KH2 install, played by\r\n\
; KH1's own effect system: magnet_explosion (Magnet Burst, Explosion, Vicinity Break, Aerial Finish, Guard\r\n\
;   Break, Finishing Leap, Aerial Recovery), all (every move), off\r\n\
kh2_effects = magnet_explosion\r\n\
; kh2_hit_sparks: on = a KH2 swing's Keyblade hit on an enemy shows KH2's own hit effect for its\r\n\
;   attack (Kingdom Key, base Sora); off = KH1's Keyblade spark. Needs kh2_effects\r\n\
kh2_hit_sparks = on\r\n\
; flip_weapons: Sora's weapons (KH1 item ids) KH2's grip holds the wrong way round, turned half\r\n\
; round in his hand (0x53: the Dream Shield); none = off\r\n\
flip_weapons = 0x53\r\n\
; reflect: on = KH2's Reflect in the Magic menu and the shortcuts once Sora has Aero (KH2's\r\n\
; cast, barrier, counter, effects and sounds, from your KH2 install; Reflera / Reflega with\r\n\
; Aerora / Aeroga; 1 MP); off = KH1's spells only\r\n\
reflect = on\r\n\
; keyblade_turn: on = every Keyblade held with its teeth up, as KH2 holds it (KH1's grip held\r\n\
; them to the ground under KH2's moves); off = as before\r\n\
keyblade_turn = on\r\n\
; drive: on = D-pad left at the battle menu's top flips to a second page (Attack, Magic, Items,\r\n\
; Drive), Drive listing KH2's forms; off = KH1's menu only\r\n\
drive = on\r\n\
; story_unlocks: on = the Drive Forms open by the story (the owner's table from the main branch:\r\n\
;   Guard Armor, Opposite Armor, the Pegasus Cup by the Dive's choice; Master after Anti-Sora,\r\n\
;   Final after Riku 2); off = Valor and Wisdom from the start (Config: Progression > Story Unlocks)\r\n\
story_unlocks = off\r\n\
; gauge_growth: on = the Drive gauge starts at 3 bars and grows one for each of the owner's six fights\r\n\
;   (Sabor, Riku 1, Chernabog, Sephiroth, Kurt Zisa, the Unknown); off = 9 bars (Config: Progression >\r\n\
;   Drive Gauge Growth)\r\n\
gauge_growth = off\r\n\
; drive_mp_refill: on = entering a Drive Form refills MP as well as HP, as KH2's does; off = HP only\r\n\
;   (Config: Drive > MP Refill)\r\n\
drive_mp_refill = off\r\n\
; drive_cost: gauge = a Drive Form costs KH2's Drive gauge bars and runs on KH2's form gauge; mp = 3 MP,\r\n\
;   then MP drained while in the form (Config: Drive > Form Cost)\r\n\
drive_cost = gauge\r\n\
; drive_refill: kh2 = leaving a world in a Drive Form refills the Drive gauge, HP and MP and reverts (KH2's\r\n\
;   world map); off = no refill (Config: Drive > Gauge Refill)\r\n\
drive_refill = kh2\r\n\
; drive_recovery_battle: on = Drive Recovery and High Drive Recovery go in the item slots and work in\r\n\
;   battle too; off = from the menu only, as in KH2 (Config: Drive > Recovery in Battle)\r\n\
drive_recovery_battle = off\r\n\
; drive_party: kh2 = a Drive Form needs KH2's partners (Valor Goofy, Wisdom Donald, Master / Final / Anti\r\n\
;   both slots filled, Limit no one); off = whoever is in the party (Config: Drive > Party Required)\r\n\
drive_party = kh2\r\n\
; drive_limits: kh2 = no KH1 limit (Sonic Blade, Ars Arcanum, ...) offered or used in any Drive Form, as in\r\n\
;   KH2; kh1 = KH1's offer as it is (Valor still refuses them) (Config: Drive > Limits in Forms)\r\n\
drive_limits = kh2\r\n\
; limit_cost: kh2 = Limit Form costs 4 Drive bars, as in KH2; 3 = the main branch's 3 bars (Config: Drive > Limit > Cost)\r\n\
limit_cost = kh2\r\n\
; limit_trinity: on = KH1's Trinity Limit can be used in Limit Form (on Triangle, as KH1 offers it); off = Limit Form's four (Config: Drive > Limit > Trinity Limit)\r\n\
limit_trinity = off\r\n\
; drive_recovery_sources: on = Drive Recovery and High Drive Recovery are sold in KH1's item shops once a\r\n\
;   Drive Form is open (and later dropped and found); off = nowhere (Config: Progression > Drive Recovery)\r\n\
drive_recovery_sources = on\r\n\
; valor_left_blade: on = in Valor Form, its Keyblade in Sora's left hand (KH1's own model of it);\r\n\
; off = nothing of it is installed (turn it off if it misbehaves)\r\n\
valor_left_blade = on\r\n\
; physics: kh1 or kh2, whose physics move base Sora in every move but air casts (run speeds, jump arc,\r\n\
;   gravity, air drift, attack travel), whatever the move styles below (they choose the\r\n\
;   animations); Drive Forms always KH2's. The Config menu's Physics row switches it in game\r\n\
physics = kh2\r\n\
; ground_combo / air_combo: kh1 or kh2, which game's moves base Sora uses\r\n\
; for each (Drive Forms keep their own); the Config menu switches them in game too\r\n\
ground_combo = kh2\r\n\
air_combo = kh2\r\n\
; magic_fire / magic_blizzard / magic_thunder / magic_cure: kh1 or kh2, which game's spell base\r\n\
;   Sora casts, on the ground and in the air (Drive Forms keep their own); the Config menu's\r\n\
;   Magic list switches them\r\n\
magic_fire = kh1\r\n\
magic_blizzard = kh1\r\n\
magic_thunder = kh1\r\n\
magic_cure = kh1\r\n\
; anim_blizzard / anim_thunder / anim_cure / anim_gravity / anim_stop / anim_aero: kh1 or kh2,\r\n\
;   the cast animation of KH1's spell (a spell cast KH2's way always plays KH2's; Fire's follows\r\n\
;   its spell, Reflect's is KH2's); the Config menu's Magic > Animations list switches them\r\n\
anim_blizzard = kh1\r\n\
anim_thunder = kh1\r\n\
anim_cure = kh1\r\n\
anim_gravity = kh1\r\n\
anim_stop = kh1\r\n\
anim_aero = kh1\r\n\
; magic_air_physics: kh1 or kh2, how base Sora moves while he casts in the air, every spell\r\n\
;   (KH1's 30 hop, half speed and fall; or KH2's 40 hop, speed kept, KH2's gravity and the cast's\r\n\
;   hover), whatever physics says; the Config menu's Magic list switches it\r\n\
magic_air_physics = kh2\r\n\
; magic_combo: kh1 or kh2, whether KH1's grounded casts (base Sora) take a step of the combo\r\n\
;   (Gravity and Stop its finisher) and, on KH1's animation, let an Attack or a spell follow from\r\n\
;   KH2's Fire window (kh2), or play as KH1's; on KH2's animation Gravity and Stop are finishers\r\n\
;   either way\r\n\
;   (kh1); the Config menu's Magic list switches it\r\n\
magic_combo = kh1\r\n\
; hit_reactions: kh1 or kh2, KH1's or KH2's reaction animations when base Sora is hit (KH1\r\n\
;   still runs the reaction: its no-damage time, voices, HUD and MP); the Config menu's Combat\r\n\
;   list switches it\r\n\
hit_reactions = kh1\r\n\
; guard_anim: kh1 or kh2, what Sora's guard press plays (Square standing, Dodge Roll kh2): KH1's own\r\n\
;   guard (its motion and its guard on KH1's Keyblade) or KH2's guard (motion 173, its repel and\r\n\
;   Counterguard)\r\n\
guard_anim = kh1\r\n\
; jump_style / glide_style: kh1 or kh2, KH1's or KH2's jump and glide for base Sora\r\n\
jump_style = kh2\r\n\
glide_style = kh2\r\n\
; roll_style: kh1 or kh2, KH1's or KH2's dodge roll for base Sora\r\n\
roll_style = kh2\r\n\
; misc_anims: kh1 or kh2, KH1's or KH2's animations for base Sora standing, walking, running\r\n\
; and using an item (KH2 still moves him; KH1 runs the item either way)\r\n\
misc_anims = kh2\r\n\
; player_model: kh1 or kh2, draw Sora with KH1's model or KH2's (his KH2 Drive Form models and\r\n\
; KH2's Keyblades where KH2 has the same one)\r\n\
player_model = kh1\r\n\
; battle_quotes: kh1 or kh2, Sora's KH1 or KH2 battle quotes (the Config menu's row; without\r\n\
; this line kh2_voice decides)\r\n\
battle_quotes = kh1\r\n\
; gummi: vanilla = fly the Gummi Ship between worlds; skip = land in the chosen world at once\r\n\
; (the Config menu's Gummi Routes row switches it in game too)\r\n\
gummi = vanilla\r\n\
; trail_always: off = Keyblade trails show on swings, as KH1 does; on = every Keyblade shows its\r\n\
; trail whenever it is out (the Config menu's Cosmetics > Keyblade Trail row switches it in game too)\r\n\
trail_always = off\r\n\
; soft_reset: off = nothing; on = L1 + L2 + R1 + R2 + Start + Select held for a second returns to the\r\n\
; title screen, as the PS2's reset did (the Config menu's QoL > Soft Reset row switches it in game too)\r\n\
soft_reset = off\r\n\
; volume_mute: on = the lowest step of each of KH1's volume sliders (master, music, sound effects, voices)\r\n\
; is silence; off = KH1's lowest step, a tenth of full volume (read at start)\r\n\
volume_mute = on\r\n\
; effects_after_fade: slider = after a room or event fade, the two effect channels KH1's fade sets to full\r\n\
; (buses 5 and 6) follow the sound effects slider; vanilla = they come back\r\n\
; at full volume after every fade, as KH1 does (read at start)\r\n\
effects_after_fade = slider\r\n\
; cutscene_skip: off = nothing; seen = skip a story cutscene (the ones the pause menu lets you skip) only when you have\r\n\
; watched it before; all = skip every one of them, a second after it starts. Talks, shops and the dialog scenes\r\n\
; you confirm line by line are never skipped (the Config menu's QoL > Cutscene Skip row switches it in game too)\r\n\
cutscene_skip = off\r\n\
; cutscene_seen: the scenes already watched, kept by the mod for the seen setting (delete the line to forget them)\r\n\
cutscene_seen =\r\n\
; ingame_skip: off = nothing; on = skip the in-game scenes (the low-poly dialog you confirm line by line, which the\r\n\
; pause menu can't skip) with KH1's own skip: items, abilities and story flags the scene gives are still given.\r\n\
; Talks, shops, scenes with a choice and the pause menu's skippable cutscenes are left alone (QoL > In-game Scene Skip)\r\n\
ingame_skip = off\r\n\
; chest_style: vanilla = KH1's chest opening (Sora stands still until the item window is closed); quick = Sora plays\r\n\
; a chest animation (chest_anim), then moves on; skip = as KH2 (the default): no animation, Sora is free at once.\r\n\
; Quick and skip show the item in the box at the top left, no window to close (QoL > Chest Opening; the old kh1 /\r\n\
; kh2 values read as vanilla / skip)\r\n\
chest_style = skip\r\n\
; chest_anim: quick's animation: kh1 = KH1's open motion; kh2 = KH2's (QoL > Chest Animation, shown on Quick)\r\n\
chest_anim = kh1\r\n\
; item_pickup: vanilla = KH1's (Sora stands still until the item window is gone); skip = an item picked up off the\r\n\
; ground (Destiny Islands' raft things and the like) leaves Sora free: KH1's item window shows while he keeps moving\r\n\
; (QoL > Item Pickup)\r\n\
item_pickup = vanilla\r\n\
; trace: (debug tools: only with debug_menu = on) on = the last trace_seconds of Sora's frames (position, KH1's and KH2's states, the pad)\r\n\
; are kept and written to the kh1-kh2-traces folder on Select + Triangle, or on their own when\r\n\
; something looks wrong; always = every frame is also written to kh1-kh2.trace.csv; off = none\r\n\
trace = on\r\n\
; trace_seconds: how many seconds before the dump the trace clip holds (1 to 120, default 10)\r\n\
trace_seconds = 10\r\n\
; trace_game_frames: on = trace_seconds counts game frames (60 a second, at any frame rate), so\r\n\
; a frame-advanced clip holds as many frames as one played at full speed; off = real time\r\n\
; (the debug menu's Trace Recorder page switches it; turning Frame Advance on turns it on)\r\n\
trace_game_frames = off\r\n\
; trace_*: what each trace row holds (on / off each; the debug menu's Trace Recorder page switches\r\n\
; them): movement (position, floor, collision, velocity), kh1_state, form, draw\r\n\
; (animation draw), feet (feet / IK), kh1_camera, kh2_camera, input, config, log (log lines),\r\n\
; hit_fx (hit effect lines), sounds (sound log lines)\r\n\
trace_movement = on\r\n\
trace_kh1_state = on\r\n\
trace_form = on\r\n\
trace_draw = on\r\n\
trace_feet = on\r\n\
trace_kh1_camera = on\r\n\
trace_kh2_camera = on\r\n\
trace_input = on\r\n\
trace_config = on\r\n\
trace_log = on\r\n\
trace_hit_fx = on\r\n\
trace_sounds = on\r\n\
; debug_menu: on = the debug tools are loaded: the debug menu (L1 + R1 + Triangle), Frame\r\n\
; Advance, the trace recorder and the sound log, and the Config menu's Misc list has a Debug\r\n\
; Menu row (it switches the menu for that session only); off = none of it is loaded or hooked\r\n\
; and the row isn't there. Read once when the game starts: a change needs a restart.\r\n\
debug_menu = off\r\n\
; log_*: what goes into kh1-kh2.log, by system (on / off each; the debug menu's Logs page\r\n\
; switches them in game). All off by default, and none of it (no log file at all) unless\r\n\
; debug_menu = on at start: general, loading (loading screen, cache), takeover (who has Sora,\r\n\
; input), animation, combat, magic, forms, kh2_sounds, kh1_sounds (every KH1 sound call, the\r\n\
; old sound_log), effects, camera, menus, debug (debug menu, frame advance, trace), crash\r\n\
; (crash notes, the version and game build lines), cutscenes (KH1's scene prepares, event\r\n\
; scripts and movies), magnet (every frame of each enemy Magnet Burst holds: what moves it)\r\n\
log_general = off\r\n\
log_loading = off\r\n\
log_takeover = off\r\n\
log_animation = off\r\n\
log_combat = off\r\n\
log_magic = off\r\n\
log_forms = off\r\n\
log_kh2_sounds = off\r\n\
log_kh1_sounds = off\r\n\
log_effects = off\r\n\
log_camera = off\r\n\
log_menus = off\r\n\
log_debug = off\r\n\
log_crash = off\r\n\
log_cutscenes = off\r\n\
log_magnet = off\r\n\
; debug_high_jump, debug_quick_run, debug_dodge_roll, debug_aerial_dodge, debug_glide: the debug\r\n\
; menu's Growth Abilities page (only with debug_menu = on): normal (what Sora's level and story\r\n\
; give), off, lv1, lv2, lv3 or max, given whether or not he learned or equipped it\r\n\
debug_high_jump = normal\r\n\
debug_quick_run = normal\r\n\
debug_dodge_roll = normal\r\n\
debug_aerial_dodge = normal\r\n\
debug_glide = normal\r\n\
; debug_ledge_carry: the debug menu's Ledge Carry row (only with debug_menu = on): on, KH2's\r\n\
; ground moves that hold Sora on the ground (Valor's 154) go on off a ledge as in KH2; off, KH1\r\n\
; cuts them into its fall\r\n\
debug_ledge_carry = on\r\n";

/// The experimental block (the owner, t922: "a setting in the ini that enables experimental settings
/// in the config menu and pop up at the bottom of the ini once enabled"): put at the bottom of the
/// ini, with these defaults, when `experimental = on` and it isn't there; taken out, with any of its
/// keys elsewhere in the file (an ini from before t922), when `experimental = off`.
const EXPERIMENTAL_START: &str = "; ==== experimental settings (experimental = on; taken out again when it is off) ====";
const EXPERIMENTAL_END: &str = "; ==== end of experimental settings ====";
const EXPERIMENTAL_BLOCK: &str = "; enemy_weight: on = KH2's knock-back on enemies (its pushes, launches and air flinches) times\r\n\
;   KH1's own weight for each enemy (how far KH1 pushes and launches it); off = KH2's numbers for\r\n\
;   every enemy (the Config menu's Combat > Tweaks list switches it)\r\n\
enemy_weight = on\r\n\
; clash_count: kh1 or kh2, how a clash or a wall bounce counts for the combo: KH2's (every\r\n\
;   meeting of his attack counts as a connected swing, and every recoil, in the air too, takes one\r\n\
;   swing off the count, so the next press repeats it) or KH1's own (the Config menu's Combat >\r\n\
;   Tweaks list switches it)\r\n\
clash_count = kh2\r\n\
; knockback_numbers: kh2, kh1 or scaled, how far KH2's hit reactions push base Sora (hit_reactions\r\n\
;   = kh2): KH2's 200 for every hit, KH1's own number for the hit (750 x the damage's share of\r\n\
;   his max HP, at most 150) on KH2's curve, or KH2's 200 scaled by KH1's share of its cap\r\n\
knockback_numbers = scaled\r\n\
; medium_hits: blow or flinch, with KH2's hit reactions: KH1's middle hits (20..29% of his max HP)\r\n\
;   knock base Sora away (blow) or only flinch him (flinch; 30% and up still knocks him away)\r\n\
medium_hits = blow\r\n\
; kh2_damage: on = KH2's power goes into KH1's own damage function (KH1's curve x KH2's power,\r\n\
;   rounded up per hit as KH2); off = KH1's damage exactly (the Combat > Tweaks list's KH2 Damage row)\r\n\
kh2_damage = off\r\n\
; recoil: kh1 or kh2, the rules when base Sora's attack meets an enemy's attack or guard: KH1's\r\n\
;   (its priorities, its recoil animations and push, 8 frames without damage) or KH2's (KH2's\r\n\
;   attack kinds, its recoil animations, push and hop, no protection); every move, KH2's too;\r\n\
;   also what Sora's KH2 guard does when it blocks: KH1's own block (KH1) or KH2's guard repel\r\n\
;   (KH2: motion 42, pushed back 200, 80 frames without damage, Counterguard)\r\n\
recoil = kh1\r\n\
; guard_shape: kh1 or kh2, the shape of KH2's guard on the Keyblade: KH1's own blade (kh1) or KH2's\r\n\
;   guard shapes (kh2)\r\n\
guard_shape = kh1\r\n";

/// The experimental settings' ini keys and the value each keeps with `experimental = off` (the
/// owner, t922: Enemy Weight on, Clash Combo on (KH2), Knock-back Numbers scaled, Medium Hits blow,
/// KH2 Damage off, Recoil KH1, Guard Shape KH1); the block's defaults are the same.
pub const EXPERIMENTAL_KEYS: [(&str, &str); 7] =
    [("enemy_weight", "on"), ("clash_count", "kh2"), ("knockback_numbers", "scaled"), ("medium_hits", "blow"), ("kh2_damage", "off"), ("recoil", "kh1"), ("guard_shape", "kh1")];

/// A setting line's key (`None` for a comment or a line without `=`).
fn line_key(l: &str) -> Option<&str> {
    let t = l.trim_start();
    if t.starts_with(';') {
        return None;
    }
    t.split_once('=').map(|(k, _)| k.trim())
}

/// The ini as `experimental` wants it (`None`: already so). On: the block at the bottom (kept as it
/// is when it's there; any of its keys elsewhere taken out). Off: the block and its keys taken out.
/// A key outside the block goes with the comment lines above it that start at `; <key>:`.
fn experimental_ini(text: &str, on: bool) -> Option<String> {
    let mut lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().position(|l| l.trim_start().starts_with(EXPERIMENTAL_START.split(" (").next().unwrap_or(EXPERIMENTAL_START)));
    let mut changed = false;
    // (The block's range: to its end line, or to the end of the file if that was deleted.)
    let block = start.map(|s| {
        let e = lines[s..].iter().position(|l| l.trim() == EXPERIMENTAL_END).map_or(lines.len(), |e| s + e + 1);
        (s, e)
    });
    let mut kept_block: Vec<&str> = Vec::new();
    if let Some((s, e)) = block {
        kept_block = lines.drain(s..e).collect();
        if !on {
            changed = true;
        }
    }
    // Its keys elsewhere.
    let is_exp = |k: &str| EXPERIMENTAL_KEYS.iter().any(|(e, _)| e.eq_ignore_ascii_case(k));
    let mut i = 0;
    while i < lines.len() {
        match line_key(lines[i]) {
            Some(k) if is_exp(k) => {
                let head = format!("; {}:", k.to_ascii_lowercase());
                let mut from = i;
                for back in 1..=8 {
                    let Some(j) = i.checked_sub(back) else { break };
                    if !lines[j].trim_start().starts_with(';') {
                        break;
                    }
                    if lines[j].trim_start().to_ascii_lowercase().starts_with(&head) {
                        from = j;
                        break;
                    }
                }
                let _ = lines.drain(from..=i);
                i = from;
                changed = true;
            }
            _ => i += 1,
        }
    }
    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    while out.last().is_some_and(|l| l.trim().is_empty()) && (on || changed) {
        out.pop();
    }
    if on {
        if kept_block.is_empty() {
            out.push(EXPERIMENTAL_START.to_string());
            out.extend(EXPERIMENTAL_BLOCK.lines().map(str::to_string));
            out.push(EXPERIMENTAL_END.to_string());
            changed = true;
        } else {
            // (Left at the bottom: anything after it moves above it.)
            if block.is_some_and(|(_, e)| e < text.lines().count()) {
                changed = true;
            }
            out.extend(kept_block.iter().map(|l| l.to_string()));
            if kept_block.last().map(|l| l.trim()) != Some(EXPERIMENTAL_END) {
                out.push(EXPERIMENTAL_END.to_string());
                changed = true;
            }
        }
    }
    changed.then(|| out.join("\r\n") + "\r\n")
}

#[derive(Debug)]
pub struct Settings {
    pub abilities: Mode,
    pub hurt: bool,
    pub commands: bool,
    /// Which motions play KH2's own sounds: `None` off, `Some(None)` all.
    pub kh2_sounds: Option<Option<Vec<u16>>>,
    pub kh2_volume: f32,
    /// Times the damage of Sora's KH2 hits (KH1's damage x KH2's power x this).
    pub damage_scale: f32,
    /// KH2's power inside KH1's damage function (`kh2_damage = on`, the default; `kh2damage`).
    pub kh2_damage: bool,
    /// KH1's critical hits on every attack (`criticals = on`; off by default, as KH2;
    /// `kh2damage::criticals`).
    pub criticals: bool,
    /// KH1's critical hits on Sora's party members' attacks (`party_criticals = on`; off by
    /// default, as KH2; `kh2damage::party_criticals`).
    pub party_criticals: bool,
    /// KH1's 30% chance on Stun Impact and Gravity Break in the KH1 combo's finisher walk
    /// (`kh1_ability_chance = on`; off by default, the tester, combo row selection m22 answer 2 (b);
    /// the Config menu's Combat > Tweaks list, `kh1picker::chance`).
    pub kh1_ability_chance: bool,
    /// KH2's knock-back on enemies times KH1's own weight for each enemy (`enemy_weight = on`, the
    /// default; the tester, kh2-knockback-on-enemies question 1 (b); `knockback::weight`).
    pub enemy_weight: bool,
    /// A KH2 spell in KH2's glide ends the glide into KH1's air cast (`glide_cast = on`, the default;
    /// the tester, what-is-still-missing m8 answer 1 (a); `glide::cast_out`).
    pub glide_cast: bool,
    /// KH2's rule for a clash or a wall bounce in the combo count (`clash_count = kh2`, the default;
    /// the tester, what-is-still-missing m15 answer 1; `recoil::clash_count`).
    pub clash_count: bool,
    /// KH2's line ahead (its row 1, the hop off a ledge) for base Sora's ground press (`drop_hop =
    /// on`, the default; the owner, t914; `combomods::drop_hop`).
    pub drop_hop: bool,
    /// Glide on Square: always off (held off since t319; its ini line `glide` removed, t924). The
    /// Square glide code stays behind it for later.
    pub glide_on_square: bool,
    /// (Read for `battle_quotes`'s default; `styles::Kind::BattleQuotes` is what plays.)
    #[allow(dead_code)]
    pub kh2_voice: bool,
    pub auto_aim: bool,
    pub wall_clink: bool,
    /// A KH2 swing's Keyblade hit shows KH2's hit element (`combat::swing_spark`).
    pub kh2_hit_sparks: bool,
    pub kh2_camera: bool,
    pub camera_scale: f32,
    pub camera_lower: f32,
    /// KH1's gameplay camera calls drive KH2's camera (`camera_calls = kh2`, `camera.rs`).
    pub camera_calls_kh2: bool,
    /// R2 held stops KH2's camera's right stick and R3 (`r2_camera_hold = on`, `camera::R2_HOLD`).
    pub r2_camera_hold: bool,
    /// L2 + the right stick is the d-pad (`l2_stick_menu = on`, `camera::L2_DPAD`).
    pub l2_stick_menu: bool,
    /// Drive on the battle menu's page 2, Summon on page 1 (`drive_page = 2`; `cmdmenu`). KH2's:
    /// Drive on page 1.
    pub drive_page2: bool,
    /// D-pad left at the battle menu's top flips the page (`dpad_left = kh2`, the default; `cmdmenu`).
    pub dpad_left_kh2: bool,
    /// KH2's cursor rules in the battle menu (`cursor_memory = kh2`, the default; `cmdmenu`).
    pub cursor_memory_kh2: bool,
    /// KH2's command lock (`command_lock = kh2`, the default; `cmdmenu`).
    pub command_lock_kh2: bool,
    pub config_row: bool,
    /// The experimental settings show in the Config menu and at the ini's bottom (`experimental = on`;
    /// off by default, the owner, t922). Off: each keeps `EXPERIMENTAL_KEYS`' value.
    pub experimental: bool,
    /// KH2's fourth shortcut (`shortcuts = 4`).
    pub four_shortcuts: bool,
    /// Which motions show KH2's own effects: `None` off, `Some(None)` all.
    pub kh2_effects: Option<Option<Vec<u16>>>,
    /// (Test, t335) Values written in turn into byte `hit_fx_byte` (0..=2) of a landed KH2 hit's
    /// KH1 effect / sound bytes (the attack object's `+0xA8`), with bit `0x400000` set (spells
    /// too); empty: off.
    pub hit_fx_test: Vec<u8>,
    pub hit_fx_byte: usize,
    /// (Test, t337) KH1 effect ids started in turn on each landed KH2 hit (`0x1402ABB60`, from
    /// Sora's effect source); empty: off. `hit_effect_on = sora` puts them on Sora, else the foe.
    pub hit_effect_test: Vec<i32>,
    pub hit_effect_on_sora: bool,
    pub flip_weapons: Vec<u8>,
    /// KH2's Reflect in KH1's magic menu (`reflect = on`).
    pub reflect: bool,
    /// Keyblades turned teeth up in his hand (`keyblade_turn = on`, `pose::keyblade_teeth_up`).
    pub keyblade_turn: bool,
    /// Whose physics move base Sora (`physics = kh1 | kh2`, KH2's by default;
    /// `shared::physics::rule`).
    pub physics_rule: crate::kh1phys::Rule,
    /// The second command page with Drive (`drive = on`).
    pub drive: bool,
    /// The Drive Forms open by the story, the owner's table (`story_unlocks = on`; off by default;
    /// Config: Progression > Story Unlocks, t926; `formown`, `crate::storyforms`).
    pub story_unlocks: bool,
    /// The Drive gauge's maximum by the owner's six fights (`gauge_growth = on`; off: 9); Config:
    /// Progression > Drive Gauge Growth, t935; `drivegauge::growth`.
    pub gauge_growth: bool,
    /// Entering a Drive Form refills MP too (`drive_mp_refill = on`; off by default; Config: Drive >
    /// MP Refill, t927; `form::mp_refill`).
    pub drive_mp_refill: bool,
    /// A Drive Form costs the Drive gauge (`drive_cost = gauge`, the default) or MP (`mp`); Config:
    /// Drive > Form Cost, t933; `form::gauge_cost`.
    pub drive_cost_gauge: bool,
    /// KH2's refill on leaving a world in a form (`drive_refill = kh2`, the default; `off`); Config:
    /// Drive > Gauge Refill, t934; `form::gauge_refill`.
    pub drive_refill: bool,
    /// Drive Recovery items in battle too (`drive_recovery_battle = on`; off by default, KH2's menu-only
    /// rule); Config: Drive > Recovery in Battle, t949; `driveitems::battle`.
    pub drive_recovery_battle: bool,
    /// A form needs KH2's partners (`drive_party = kh2`, the default; `off`); Config: Drive > Party
    /// Required, t958; `driveparty::required`.
    pub drive_party: bool,
    /// No KH1 limit in a form (`drive_limits = kh2`, the default; `kh1`); Config: Drive > Limits in
    /// Forms, t963; `formlimits::kh2`.
    pub drive_limits: bool,
    /// Limit Form for 3 bars (`limit_cost = 3`; `kh2`, 4 bars, by default); Config: Drive > Limit >
    /// Cost, t972; `limitform::three_bars`.
    pub limit_cost_three: bool,
    /// KH1's Trinity Limit in Limit Form (`limit_trinity = on`; off by default); Config: Drive >
    /// Limit > Trinity Limit, t977; `limitform::trinity`.
    pub limit_trinity: bool,
    /// Where Drive Recovery items come from (`drive_recovery_sources = on`, the default; main's design);
    /// Config: Progression > Drive Recovery, t950; `driveitems::sources`.
    pub drive_recovery_sources: bool,
    /// Valor's Keyblade drawn in his left hand (`valor_left_blade = on`, `leftblade`).
    pub valor_left_blade: bool,
    /// KH2's (true) or KH1's moves per `styles::Kind`, in `Kind::EVERY`'s order.
    pub styles: [bool; super::styles::COUNT],
    /// How far KH2's hit reactions push base Sora (`knockback_numbers = kh2 | kh1 | scaled`, KH2's by
    /// default; `hitreact`).
    pub knockback_numbers: super::hitreact::Numbers,
    /// KH1's middle hits as KH2 flinches (`medium_hits = flinch`; blow by default; `hitreact`).
    pub medium_hits_flinch: bool,
    /// Skip the Gummi Ship's flights (`gummi = skip`, `gummi.rs`).
    pub gummi_skip: bool,
    /// Every Keyblade shows its trail whenever it is out (`trail_always = on`, `cosmetics.rs`).
    pub trail_always: bool,
    /// L1 + L2 + R1 + R2 + Start + Select held a second returns to the title screen (`soft_reset = on`, `softreset.rs`).
    pub soft_reset: bool,
    /// The lowest step of every KH1 volume slider is silence (`volume_mute = on`, the default; `volmute.rs`, t915).
    pub volume_mute: bool,
    /// Buses 5 and 6 follow the effects slider after a fade-in (`effects_after_fade = slider`, the default; `volmute.rs`, t916).
    pub fade_follows_effects: bool,
    /// Skips the pause menu's skippable cutscenes (`cutscene_skip = off | seen | all`: 0, 1, 2; `cutsceneskip.rs`).
    pub cutscene_skip: u8,
    /// The scenes already watched, as the mod keeps them (`cutscene_seen`).
    pub cutscene_seen: String,
    /// Skips the in-game (low-poly) event scenes with KH1's own skip (`ingame_skip = on`, `ingameskip.rs`).
    pub ingame_skip: bool,
    /// The chest opening (`chest_style = vanilla | quick | skip`: 0, 1, 2; the old kh1 / kh2 read as 0 / 2;
    /// `chest.rs`).
    pub chest_style: u8,
    /// Quick's animation is KH2's (`chest_anim = kh2`).
    pub chest_anim_kh2: bool,
    /// An item picked up off the ground leaves Sora free (`item_pickup = vanilla | skip`, t795; `chest.rs`).
    pub item_pickup: bool,
    /// KH2's moves as new slots in KH1 Sora's motion table (`kh2slots.rs`): `None` off (the
    /// default), `Some(None)` every move that has one (`on`), else only these KH2 motions.
    pub kh1_slots: Option<Option<Vec<u16>>>,
    /// Those slots baked into KH1 motions at load (`kh1_bake = on`, the default; `kh1_data::bake`):
    /// KH1 shows them with no pose of ours; off: a retimed KH1 motion drawn over (tier A).
    pub kh1_bake: bool,
    /// The baked slots keep KH1's own leg IK and ground lift (`kh1_leg_ik = on` and `foot_ik =
    /// on`; `kh1_data::legs`): KH1's pass places his feet on slopes and stairs in them too.
    pub kh1_leg_ik: bool,
    /// Feet placed on uneven ground at all (`foot_ik = on`; off by default, the tester's m3
    /// ask: "lets actually have the foot iks off we'll work on this later"): KH1's leg IK on the
    /// baked slots (`kh1_leg_ik`) and our own foot placement on the clips drawn over KH1's pass
    /// (`feet::place`). Off: plain legs in both.
    pub foot_ik: bool,
    pub trace: super::trace::Mode,
    /// How many seconds of Sora's frames a trace dump holds (`trace_seconds`, 1..=120, 10 by default).
    pub trace_seconds: u32,
    /// The trace's seconds count game frames, not real time (`trace_game_frames = on`; off by
    /// default; Frame Advance turns it on, `frameadv`).
    pub trace_game_frames: bool,
    /// What each trace row holds (`trace_*` = on / off, `trace::ITEMS`; all on by default).
    pub trace_cats: u32,
    /// The debug tools are loaded (`debug_menu = on`, read once at start: `win::debug_on`), and
    /// L1 + R1 + Triangle opens the debug menu (`debugmenu.rs`).
    pub debug_menu: bool,
    /// The log's categories on (`log_*` = on / off, `win::LogCat`; all off by default): used only
    /// with the debug tools loaded.
    pub log_cats: u32,
    /// The debug menu's Growth Abilities rows (`growth::GROWTH_ROWS`' order; 0 Normal, 1 Off, 2..5
    /// LV1..MAX, `growth::GROWTH_SETTINGS`): used only with the debug tools loaded.
    pub debug_growth: [u8; 5],
    /// The debug menu's Ledge Carry row (`debugmenu::ledge_carry`): used only with the debug tools.
    pub debug_ledge_carry: bool,
}

/// The ini keys of the debug menu's Growth Abilities rows (`growth::GROWTH_ROWS`' order).
pub const DEBUG_GROWTH_KEYS: [&str; 5] = ["debug_high_jump", "debug_quick_run", "debug_dodge_roll", "debug_aerial_dodge", "debug_glide"];

static SETTINGS: AtomicPtr<Settings> = AtomicPtr::new(std::ptr::null_mut());
static RELOAD: Mutex<()> = Mutex::new(());
static MTIME: Mutex<Option<std::time::SystemTime>> = Mutex::new(None);

/// The ini's `key = value`, if set.
fn setting(text: &str, key: &str) -> Option<String> {
    text.lines().filter_map(|l| l.split(';').next()).filter_map(|l| l.split_once('=')).find(|(k, _)| k.trim().eq_ignore_ascii_case(key)).map(|(_, v)| v.trim().to_ascii_lowercase())
}

fn on(text: &str, key: &str, on_word: &str, default: bool) -> bool {
    match setting(text, key) {
        Some(v) => v == on_word || v == "on" || v == "true" || v == "1" || v == "yes",
        None => default,
    }
}

/// The settings as the ini has them now. The ini is read when the game starts and again
/// whenever it is saved (t336: checked at most every half second), so a change takes effect in
/// game; settings only used at start (hooks put in, the abilities given) still need a restart.
pub fn get() -> &'static Settings {
    let p = SETTINGS.load(Ordering::Acquire);
    if p.is_null() {
        let _g = RELOAD.lock().unwrap_or_else(|e| e.into_inner());
        let p = SETTINGS.load(Ordering::Acquire);
        if !p.is_null() {
            return unsafe { &*p };
        }
        let s = Box::into_raw(Box::new(load(true).expect("first load")));
        SETTINGS.store(s, Ordering::Release);
        let s = unsafe { &*s };
        EXPERIMENTAL.store(1 + s.experimental as u8, Ordering::Relaxed);
        return s;
    }
    if due() {
        if let Ok(_g) = RELOAD.try_lock() {
            if let Some(s) = load(false) {
                // (The old one is left as it is: callers may still hold it.)
                let s = Box::into_raw(Box::new(s));
                SETTINGS.store(s, Ordering::Release);
                let s = unsafe { &*s };
                experimental_now(s);
                return s;
            }
        }
    }
    unsafe { &*SETTINGS.load(Ordering::Acquire) }
}

/// `experimental` as last loaded: 0 not yet, else 1 + on.
static EXPERIMENTAL: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// After a reload: when `experimental` changed, the experimental settings' live values are read
/// again from the ini as it is now (off: the defaults; on: the block), as at start.
fn experimental_now(s: &Settings) {
    let now = 1 + s.experimental as u8;
    if EXPERIMENTAL.swap(now, Ordering::Relaxed) == now {
        return;
    }
    super::knockback::forget_weight();
    super::recoil::forget_clash_count();
    super::hitreact::forget();
    super::kh2damage::forget_kh2_damage();
    for k in [super::styles::Kind::Recoil, super::styles::Kind::GuardShape] {
        super::styles::reload(k, s.styles[k as usize]);
    }
    log!(Menus, "settings: experimental {}: its settings read again", if s.experimental { "on" } else { "off" });
}

/// Half a second since the ini was last looked at.
fn due() -> bool {
    static START: OnceLock<std::time::Instant> = OnceLock::new();
    static LAST: AtomicU64 = AtomicU64::new(0);
    let now = START.get_or_init(std::time::Instant::now).elapsed().as_millis() as u64 + 1;
    let last = LAST.load(Ordering::Relaxed);
    now.saturating_sub(last) >= 500 && LAST.compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed).is_ok()
}

/// The ini parsed; `None` when it wasn't saved since the last read (not on the first).
fn load(first: bool) -> Option<Settings> {
    let path = format!("{}\\{INI}", super::exe_dir());
    // (t336) Read again only when the ini was saved since.
    let mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    {
        let mut m = MTIME.lock().unwrap_or_else(|e| e.into_inner());
        if !first && *m == mtime {
            return None;
        }
        *m = mtime;
    }
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => {
            let _ = std::fs::write(&path, DEFAULT_INI);
            DEFAULT_INI.to_string()
        }
    };
    // (t922) The experimental block put in or taken out as `experimental` says; the file written
    // only when that changed it (and not read again for it).
    let experimental = on(&text, "experimental", "on", false);
    let text = match experimental_ini(&text, experimental) {
        Some(t) => {
            if std::fs::write(&path, &t).is_ok() {
                *MTIME.lock().unwrap_or_else(|e| e.into_inner()) = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
                log!(General, "settings: the experimental settings {} the ini", if experimental { "put at the bottom of" } else { "taken out of" });
            }
            t
        }
        None => text,
    };
    let s = Settings {
        experimental,
        abilities: setting(&text, "abilities").and_then(|v| Mode::parse(&v)).unwrap_or(Mode::Kh1),
        hurt: on(&text, "hurt", "kh2", true),
        commands: on(&text, "commands", "on", true),
        kh2_sounds: match setting(&text, "kh2_sounds").as_deref() {
            Some("off") | Some("0") | Some("no") => None,
            Some("all") => Some(None),
            _ => Some(Some(super::kh2sound::DEFAULT_MOTIONS.to_vec())),
        },
        kh2_volume: setting(&text, "kh2_volume").and_then(|v| v.parse().ok()).unwrap_or(1.0),
        damage_scale: setting(&text, "damage_scale").and_then(|v| v.parse::<f32>().ok()).filter(|v| (0.1..=10.0).contains(v)).unwrap_or(1.0),
        // (Glide on Square held off, t319: a glide is the jump button after the Aerial Dodge. The
        // Square glide code stays, behind this flag, for later; the `glide` ini line is gone, t924.)
        glide_on_square: false,
        // (t922: experimental, off unless `experimental = on`; off by default.)
        kh2_damage: experimental && on(&text, "kh2_damage", "on", false),
        criticals: on(&text, "criticals", "on", false),
        party_criticals: on(&text, "party_criticals", "on", false),
        kh1_ability_chance: on(&text, "kh1_ability_chance", "on", false),
        enemy_weight: !experimental || on(&text, "enemy_weight", "on", true),
        glide_cast: on(&text, "glide_cast", "on", true),
        clash_count: !experimental || on(&text, "clash_count", "kh2", true),
        drop_hop: on(&text, "drop_hop", "on", true),
        kh2_voice: on(&text, "kh2_voice", "on", false),
        auto_aim: on(&text, "auto_aim", "on", false),
        wall_clink: on(&text, "wall_clink", "on", true),
        kh2_hit_sparks: on(&text, "kh2_hit_sparks", "on", true),
        kh2_camera: on(&text, "camera", "kh2", true),
        config_row: on(&text, "config_row", "on", true),
        four_shortcuts: on(&text, "shortcuts", "4", true),
        camera_lower: setting(&text, "camera_lower").and_then(|v| v.parse::<f32>().ok()).filter(|v| (-200.0..200.0).contains(v)).unwrap_or(0.0),
        camera_scale: setting(&text, "camera_scale").and_then(|v| v.parse::<f32>().ok()).filter(|v| (0.3..3.0).contains(v)).unwrap_or(0.89),
        camera_calls_kh2: on(&text, "camera_calls", "kh2", true),
        // (The row was L2 Camera Hold up to t522: an ini that still has `l2_camera_hold` keeps
        // its choice, now on R2.)
        r2_camera_hold: on(&text, "r2_camera_hold", "on", on(&text, "l2_camera_hold", "on", false)),
        l2_stick_menu: on(&text, "l2_stick_menu", "on", false),
        drive_page2: setting(&text, "drive_page").as_deref() == Some("2"),
        dpad_left_kh2: setting(&text, "dpad_left").as_deref() != Some("kh1"),
        cursor_memory_kh2: setting(&text, "cursor_memory").as_deref() != Some("kh1"),
        command_lock_kh2: setting(&text, "command_lock").as_deref() != Some("kh1"),
        kh2_effects: match setting(&text, "kh2_effects").as_deref() {
            Some("off") | Some("0") | Some("no") => None,
            Some("all") => Some(None),
            _ => Some(Some(super::kh2fx::DEFAULT_MOTIONS.to_vec())),
        },
        hit_fx_test: setting(&text, "hit_fx_test")
            .map(|v| {
                v.split([',', ' '])
                    .filter_map(|t| {
                        let t = t.trim();
                        t.strip_prefix("0x").map_or_else(|| t.parse().ok(), |h| u8::from_str_radix(h, 16).ok())
                    })
                    .collect()
            })
            .unwrap_or_default(),
        hit_effect_test: setting(&text, "hit_effect_test")
            .map(|v| {
                v.split([',', ' '])
                    .filter(|t| !t.trim().is_empty())
                    .flat_map(|t| {
                        let t = t.trim();
                        let n = |x: &str| x.trim().strip_prefix("0x").map_or_else(|| x.trim().parse::<i32>().ok(), |h| i32::from_str_radix(h, 16).ok());
                        match t.split_once('-') {
                            Some((a, b)) => match (n(a), n(b)) {
                                (Some(a), Some(b)) if a <= b && b - a < 1000 => (a..=b).collect::<Vec<_>>(),
                                _ => Vec::new(),
                            },
                            None => n(t).into_iter().collect(),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default(),
        hit_effect_on_sora: on(&text, "hit_effect_on", "sora", false),
        hit_fx_byte: setting(&text, "hit_fx_byte").and_then(|v| v.parse::<usize>().ok()).filter(|v| *v <= 2).unwrap_or(0),
        flip_weapons: match setting(&text, "flip_weapons") {
            None => vec![0x53],
            Some(v) => v
                .split([',', ' '])
                .filter_map(|t| {
                    let t = t.trim();
                    t.strip_prefix("0x").map_or_else(|| t.parse().ok(), |h| u8::from_str_radix(h, 16).ok())
                })
                .collect(),
        },
        reflect: on(&text, "reflect", "on", true),
        keyblade_turn: on(&text, "keyblade_turn", "on", true),
        physics_rule: crate::kh1phys::Rule::parse(setting(&text, "physics").as_deref()),
        drive: on(&text, "drive", "on", true),
        story_unlocks: on(&text, "story_unlocks", "on", false),
        gauge_growth: on(&text, "gauge_growth", "on", false),
        drive_mp_refill: on(&text, "drive_mp_refill", "on", false),
        drive_cost_gauge: on(&text, "drive_cost", "gauge", true),
        drive_refill: on(&text, "drive_refill", "kh2", true),
        drive_recovery_battle: on(&text, "drive_recovery_battle", "on", false),
        drive_party: on(&text, "drive_party", "kh2", true),
        drive_limits: on(&text, "drive_limits", "kh2", true),
        limit_cost_three: on(&text, "limit_cost", "3", false),
        limit_trinity: on(&text, "limit_trinity", "on", false),
        drive_recovery_sources: on(&text, "drive_recovery_sources", "on", true),
        valor_left_blade: on(&text, "valor_left_blade", "on", true),
        styles: {
            // (The Magic / Aerial Magic rows before the per-spell rows, m19: the new rows' defaults
            // from them, so an ini from before keeps what it cast.)
            let magic = on(&text, "magic", "kh2", false);
            let kh1_cast = KH1_CAST_KEYS.map(|k| on(&text, k, "on", false));
            [
                on(&text, "ground_combo", "kh2", false),
                on(&text, "air_combo", "kh2", false),
                magic,
                // (`air_magic`, read until t923, set nothing a Config row shows: the owner,
                // "any ini setting that does nothing in the testing branch should be removed".)
                false,
                on(&text, "jump_style", "kh2", false),
                on(&text, "glide_style", "kh2", false),
                on(&text, "roll_style", "kh2", false),
                on(&text, "misc_anims", "kh2", false),
                on(&text, "player_model", "kh2", false),
                on(&text, "battle_quotes", "kh2", on(&text, "kh2_voice", "on", false)),
                on(&text, "magic_cure", "kh2", magic && on(&text, "cure_style", "kh2", true)),
                on(&text, "hit_reactions", "kh2", false),
                experimental && on(&text, "recoil", "kh2", false),
                on(&text, "magic_fire", "kh2", magic),
                on(&text, "magic_blizzard", "kh2", magic),
                on(&text, "magic_thunder", "kh2", magic),
                on(&text, "anim_blizzard", "kh2", false),
                on(&text, "anim_thunder", "kh2", false),
                on(&text, "anim_cure", "kh2", false),
                on(&text, "anim_gravity", "kh2", magic && !kh1_cast[0]),
                on(&text, "anim_stop", "kh2", magic && !kh1_cast[1]),
                on(&text, "anim_aero", "kh2", magic && !kh1_cast[2]),
                // (Air casts moved by the Physics row before: its value, KH2's without it.)
                on(&text, "magic_air_physics", "kh2", crate::kh1phys::Rule::parse(setting(&text, "physics").as_deref()) == crate::kh1phys::Rule::Kh2),
                on(&text, "magic_combo", "kh2", false),
                on(&text, "guard_anim", "kh2", false),
                experimental && on(&text, "guard_shape", "kh2", false),
            ]
        },
        // (t922: experimental; Scaled by default and with `experimental = off`.)
        knockback_numbers: match setting(&text, "knockback_numbers") {
            Some(v) if experimental => super::hitreact::Numbers::parse(Some(&v)),
            _ => super::hitreact::Numbers::Scaled,
        },
        medium_hits_flinch: experimental && on(&text, "medium_hits", "flinch", false),
        gummi_skip: on(&text, "gummi", "skip", false),
        trail_always: on(&text, "trail_always", "on", false),
        soft_reset: on(&text, "soft_reset", "on", false),
        volume_mute: on(&text, "volume_mute", "on", true),
        fade_follows_effects: setting(&text, "effects_after_fade").as_deref() != Some("vanilla"),
        cutscene_skip: match setting(&text, "cutscene_skip").as_deref() {
            Some("seen") => 1,
            Some("all") | Some("on") => 2,
            _ => 0,
        },
        cutscene_seen: setting(&text, "cutscene_seen").unwrap_or_default(),
        ingame_skip: on(&text, "ingame_skip", "on", false),
        chest_style: match setting(&text, "chest_style").as_deref() {
            Some("vanilla") | Some("kh1") | Some("off") => 0,
            Some("quick") => 1,
            _ => 2,
        },
        chest_anim_kh2: on(&text, "chest_anim", "kh2", false),
        item_pickup: on(&text, "item_pickup", "skip", false),
        kh1_slots: match setting(&text, "kh1_slots").as_deref() {
            Some("off") | Some("0") | Some("no") => None,
            None | Some("on") | Some("all") | Some("yes") => Some(None),
            Some(v) => Some(Some(v.split([',', ' ']).filter_map(|t| t.trim().parse().ok()).collect())),
        },
        kh1_bake: on(&text, "kh1_bake", "on", true),
        kh1_leg_ik: on(&text, "kh1_leg_ik", "on", true) && on(&text, "foot_ik", "on", false),
        foot_ik: on(&text, "foot_ik", "on", false),
        trace: super::trace::Mode::parse(setting(&text, "trace").as_deref()),
        trace_seconds: setting(&text, "trace_seconds").and_then(|v| v.trim_end_matches('s').trim().parse::<f32>().ok()).map_or(super::trace::DEFAULT_SECONDS, |v| (v.round() as u32).clamp(1, super::trace::MAX_SECONDS)),
        trace_game_frames: on(&text, "trace_game_frames", "on", false),
        trace_cats: super::trace::ITEMS.iter().filter(|it| on(&text, it.key, "on", true)).fold(0, |m, it| m | it.bit),
        debug_menu: on(&text, "debug_menu", "on", false),
        log_cats: super::LogCat::ALL.iter().filter(|c| on(&text, c.key(), "on", false)).fold(0, |m, c| m | c.bit()),
        debug_growth: DEBUG_GROWTH_KEYS.map(|k| setting(&text, k).map_or(0, |v| crate::growth::growth_setting_parse(&v))),
        debug_ledge_carry: on(&text, "debug_ledge_carry", "on", true),
    };
    log!(General, "settings {} ({path}): {s:?}", if first { "loaded" } else { "reloaded (the ini was saved)" });
    Some(s)
}

/// The ini keys that said Gravity, Stop and Aero cast KH1's whole way at Magic KH2 (before the
/// per-spell rows: read only for `anim_gravity` / `_stop` / `_aero`'s defaults).
pub const KH1_CAST_KEYS: [&str; 3] = ["kh1_cast_gravity", "kh1_cast_stop", "kh1_cast_aero"];

/// Keeps the physics setting in the ini (the Config menu's Physics row).
pub fn save_physics(r: crate::kh1phys::Rule) {
    save("physics", r.key());
}

/// Keeps a Config menu choice in the ini (`key = value`): that line changed, or added, the rest
/// as it was.
pub fn save(key: &str, value: &str) {
    let path = format!("{}\\{INI}", super::exe_dir());
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let mut found = false;
    let mut out: Vec<String> = text
        .lines()
        .map(|l| {
            let t = l.trim_start();
            if !t.starts_with(';') && t.split('=').next().is_some_and(|k| k.trim().eq_ignore_ascii_case(key)) {
                found = true;
                format!("{key} = {value}")
            } else {
                l.to_string()
            }
        })
        .collect();
    if !found {
        out.push(format!("{key} = {value}"));
    }
    let _ = std::fs::write(&path, out.join("\r\n") + "\r\n");
}

