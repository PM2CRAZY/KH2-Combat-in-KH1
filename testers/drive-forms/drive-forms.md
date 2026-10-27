# Drive Forms

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: This is the Drive Forms thread, split out of "What is still ](#^m1)
- [2. Assistant: Thanks for the traces, they made this quick. Nine versions; ](#^m2)

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