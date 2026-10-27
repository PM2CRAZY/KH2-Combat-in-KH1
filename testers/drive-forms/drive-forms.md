# Drive Forms

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: This is the Drive Forms thread, split out of "What is still ](#^m1)
- [2. Assistant: Thanks for the traces, they made this quick. Nine versions; ](#^m2)
- [3. Assistant: Five versions this round, and you were right about the air f](#^m3)
- [4. Assistant: t762, Wisdom lands with his air speed. Your traces showed th](#^m4)

---


> [!tip] Assistant
> This is the Drive Forms thread, split out of "What is still missing for native integration into KH1" as you asked. The older "Drive Forms native" thread is still there for its own history; new form work goes here. Where things stand:
>
> - **On KH1's own states:** Valor and Wisdom, with form levels (t700), the stance after the change (t694), and the left Keyblade (t697 to t702, then t713's blend). Base Sora's growth from form levels comes from t703.
> - **On the old KH2 controller:** Master, Final, Limit and Anti. That controller is deprecated, and Final stays as it is unless you say otherwise.
> - **Waiting on other systems:**
>   - The Auto forms, to be built natively later.
>   - Final's EXP: you said to wait.
>   - Form Boost, and the form gauge growing with form levels: both wait for KH2's Drive gauge.
>   - MP Haste becoming MP Charge's: waits for a KH2 MP setting.
>   - Dodge Roll and Aerial Dodge growth: waits for Limit and Master to be native.
>
> Nothing changed in this message. Tell me here which form work you want next.

^m1


valor and wisdom are missing a lot of functionality

valor:

missing square ground finisher effect
2nd keyblade moves around when changing animations then goes back to soras hand
final finisher on ground doesn't put you in the air
you drop a little bit before the hitboxes come out on final air finisher

wisdom:

you dodge roll instead of quick run
sora doesn't spin his keyblade on the idle animation
fires physics are missing
gravity doesn't have a finisher
reflect is playing the base animation
air fires physics are wrong
all the air magics movement and physics aren't proper
air reflect is base soras reflect 
air attacks movements seem off
you can't move and attack on the ground

look into the steam c decomp to confirm what I'm saying and apply all fixes, I've traced everything for you to get a head start

go ahead

> [!tip] Assistant
> Thanks for the traces, they made this quick. Nine versions; two items still need something from you.
>
> **Fixed**
> - **t732, Wisdom's Quick Run.** Your trace has Square starting KH1's own dodge roll: Quick Run was still blocked in every Drive Form, an old rule from when forms ran on the KH2 controller. Wisdom's Square is its Quick Run now.
> - **t733, Gravity's finisher.** Gravity played the plain Thunder cast every time. The t721 commit was made from an older copy of combat.rs and dropped t719's finisher pick. It's back. (I checked the other commits since t719: nothing else was lost.)
> - **t734, Wisdom's Keyblade spin in the idle.** KH2's Wisdom twirls the Keyblade with its own weapon animation (its root turns 75 to 246 degrees from base Sora's hold); our form moves kept KH1's plain grip. Form moves now carry the form's own Keyblade hold, so Valor's grip changes show too.
> - **t735, Wisdom's Reflect, ground and air.** In Wisdom, Reflect still fell back to the old KH2 controller, which drew base Sora's animation. It now runs on KH1's cast state with Wisdom's own ground, finisher and air animations.
> - **t736, moving while attacking.** Your trace shows no movement through the shot with the stick held: Wisdom's ground momentum never ran during its swings. It does now, inside each swing's move window from KH2's files.
> - **t737, Fire's movement.** KH2's Wisdom Fire carries Sora: 384 units on the ground, 836 for the finisher cast, 383 in the air. We dropped that.
> - **t738, air attacks and air magic.** KH1 didn't let the stick steer them, and air Fire only moved during its hover. They steer like a jump in each move's window now, and take their whole movement, as KH2 does.
> - **t739, Valor's air finisher drop.** The second air finisher lost its little hop: the first finisher's hover ending restarted his fall from rest. The hop is kept now.
> - **t740, Valor's ground finisher.** Its KH2 animation lifts Sora 125 units, but KH1 kept him on the floor. He goes up with it now, then falls.
>
> **Checked:** your traces for each cause, KH2's files (Wisdom's and Valor's motions, windows and movement), the Steam KH1 decomp for the roll and Quick Run. **Not checked:** anything in game. Assumptions (in AGENTS.md): KH1's rest grip stands for base Sora's KH2 hold (t734); the air steering follows the OG rewrite, not re-traced in the KH2 decomp (t738); the hop's start speed (t739); KH1's air state runs the finisher on to its end (t740).
>
> **Not done yet**
> - **Valor's Square finisher effect.** KH2's data does start effects for it, but your log had the Effects category off, so I can't tell why KH1 didn't play them. Could you turn on Effects in the debug menu's Logs page, do the Square finisher a few times, and send the log?
> - **Valor's second Keyblade moving between animations.** The traces have nothing on the left Keyblade. A short video plus a log of one animation change would show where it jumps.
>
> **Please test**
> - Wisdom: Quick Run from standing and moving; the idle twirl; walking while shooting; Fire's dash on the ground and in the air; steering air shots and air casts; Reflect on the ground and in the air; Gravity at the end of a combo (Magic Combo on KH2).
> - Valor: the ground combo finisher (should rise); the Finishing Plus air finisher (should hop, not drop).
> - Base Sora: should be unchanged.
>
> **Questions**
> 1. The form Keyblade hold only applies to Drive Forms. KH2's base Sora also shifts the Keyblade slightly in his hand (up to about 8 degrees). Should base Sora get it too: (a) yes, (b) no?
> 2. Wisdom's Gravity, Stop and Aero still follow the Magic Combo setting (as t716/t719 built them). Should forms always use KH2's rules for them: (a) yes, (b) keep following the setting?

^m2


valor:
1. sora doesn't properly grip the keyblades
2. the 2nd keyblade is still disappearing when you move from one animation to the next. 
3. I don't think the problem with the last finisher is because of how you coded valor but more because of sora' main physics/animation slots (guess) please look outside of valor to find the issue. 
4. valors last ground finisher still doesn't bring you off the ground, it seems like a similar issue to how we fixed sora's square moves that send him off the ground so look at that.
5. soras square finisher where he hits both keyblades on the ground are missing its effect that has a launch hitbox on it


wisdom
2. drive forms magic shouldn't be affected by the magic combo setting


don't ask me for questions, just come back until you think its complete 

go ahead

> [!tip] Assistant
> Five versions this round, and you were right about the air finisher: it wasn't Valor's code.
>
> - **t757, form magic ignores Magic Combo.** Wisdom's Gravity, Stop and Aero always follow KH2's combo rules now (Thunder's, Magnet's and Cure's parts, a combo step in the air). The Magic Combo row only changes base Sora.
> - **t758, Valor's ground finisher lifts off.** The switch to the air now happens where Finishing Leap's does, before KH1 moves him, so his rise starts the same frame.
> - **t759, a hop out of a held move keeps its hop.** This is the air finisher drop, and it's the same kind of bug as the air-physics thread's fall. The new move's hop was written while KH1 still played the old move's hold, so that frame held him, and the next frame "the hold just ended" restarted his fall from rest and wiped the hop. It hit every move that hops out of a held one, base Sora's too.
> - **t760, Valor's ground slam effect plays.** The slam's floor effect uses a particle program KH1 wasn't cleared to run yet. KH1's copy of it is the same as KH2's, so it plays now.
> - **t761, the second Keyblade stays in his left hand.** Any frame without a fresh left-hand pose drew no left Keyblade. Now his left hand keeps the last hold until the next pose comes.
>
> **How I checked it**
> - t757: the Steam KH2 decomp. KH2's magic command counts every cast with no setting.
> - t758: the Steam KH1 decomp for the order of a frame (control step, then the state's step, then the physics). That the old switch landed him on the same frame is an assumption: there was no trace of t740.
> - t759: your trace f14640, frame by frame. The hop of 14 is written at f14526, held that frame by 183's hold, then gone at f14527, and he's 52 down by 184's own hold at frame 16. Then the Steam KH2 decomp: KH2 starts the new move and its hop in the same frame.
> - t760: the exes (both games' program tables) and both decomps (the two functions match). I also went through all of Valor's effects: only the arrival's light still needs programs KH1 doesn't have yet. The slam's launch is its two Keyblade hitboxes (a blow, reaction 11), which already land as KH1's airborne knock-back.
> - t761: the code path, plus your t749 log (the holder poses and blends were running). Which case made it vanish for you is an assumption. A new log line now names the cause whenever the kept hold is used.
> - Not checked in game: all five.
>
> **The grip (no change).** KH2's files show Valor holds the right Keyblade about 21 degrees tilted compared with base Sora, in every move. He reverses it in 181's first 30 frames, and the left one flips in 153 and 183. I measured this two ways (by the hand bone, and by the finger joints the retarget uses), and both blades are drawn that way through KH1's grip, so I couldn't find what's off.
>
> **Please test:**
> 1. Wisdom with Magic Combo on KH1 and on KH2: Gravity, Stop and Aero should combo the same both ways (Gravity as a finisher at the combo's end, Stop going straight to the finisher).
> 2. Base Sora with Magic Combo on KH1: unchanged.
> 3. Valor's ground combo finisher: he should leave the ground with it, then fall.
> 4. Valor's air combo into its last finisher, with and without Finishing Plus: a small hop, no drop before the hits.
> 5. Base Sora's air finisher out of a Finishing Plus window, and an air cast's hover cut by an Attack: the hop should show.
> 6. Valor's Square slam (both Keyblades on the ground): the floor effect at the hit, and enemies launched.
> 7. The second Keyblade through animation changes (combo into idle, landing, jump, getting hit): it shouldn't vanish. If you turn on the Animation log, a line "no fresh holder pose" names the cause.
>
> **Still open on my side (no answer needed)**
> - The grip: the data and the maths match KH2, so a screenshot next to KH2's would show me what's wrong.
> - Valor's arrival light (elements 5 and 6) needs KH2's KeTh programs in KH1.
> - Base Sora's Keyblade flip in his 153 swing (102 degrees at frame 20) isn't drawn, because his moves keep KH1's grip.
>
> Notes are in `docs/kh1-drive-forms-native.md` §11.1.

^m3


i've recorded a trace on wisdom, he's not following the stick direction when he lands

trace_1793133757_f9084_select_triangle.csv
trace_1793133763_f9407_select_triangle.csv

go ahead

> [!tip] Assistant
> **t762, Wisdom lands with his air speed.** Your traces showed the cause on all eight landings: when he touched down, he slid with the ground speed he'd had before the jump, not the speed the stick gave him in the air. At f8865 he was running at (-1.8, 6.5); in the air the stick took him to (7.6, 4.1); on landing at f8905 he went back to (-1.8, 6.4). The form's ground momentum kept its old value through the whole jump. KH2 uses one speed for the air and the ground, so its landing carries on from the air. Ours does too now.
>
> **How I checked it**
> - Your two traces, frame by frame (f8905, f8975, f9049, f9200, f9267, f9339 all show the old pre-jump speed coming back).
> - The Steam KH2 decomp: one velocity for air and ground. During the landing's first 12 frames KH2 ignores the stick (Wisdom's landing only frees him from frame 12, checked in KH2's files), so he slides on and slows, then turns toward the stick from frame 12. Ours already did that part, so that's unchanged.
> - Not checked in game.
>
> **Please test:**
> 1. Wisdom: jump, steer in the air, land. He should carry on the way the stick took him, not snap back to the way he was running before the jump.
> 2. Hold the stick a new way as he lands: he slides a little and turns after about 12 frames, as in KH2.
> 3. Valor's landings: the same rule applies (it's every Drive Form on KH1's states with momentum; Valor has none, so it should be unchanged).
>
> **Still open on my side (no answer needed)**
> - Everything from my last message here.
>
> Your traces are in `testers/drive-forms/`; notes in AGENTS.md (t762) and `docs/kh1-drive-forms-native.md` §11.1.

^m4

