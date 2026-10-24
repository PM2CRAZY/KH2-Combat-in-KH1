# Config menu rework

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Mr Bugs: the scope of the config menu currently can't scale to what w](#^m1)
- [2. Assistant: Here's the plan. KH1 already has a sub-menu system: it's how](#^m2)
- [3. Mr Bugs: 1. Yes if your going to make magic its own sub catagory put ](#^m3)
- [4. Assistant: Got it, building it now: Config > KH1 in KH2 (above Default ](#^m4)
- [5. Assistant: t448 and t449. Done: Config page 1 now has a "KH1 in KH2" bu](#^m5)

---


  

> [!note] Mr Bugs
> the scope of the config menu currently can't scale to what we want to add in the future so I want to rewrite it, I want to package all of kh1 in kh2's settings that we have currently into one clickable menu in the config menu on the first page, we need to make the other rows smaller so it can fit on page one the button will be called "KH1 in KH2" clicking it will open a submenu that gives you a list of sub menus, categorized e.g camera, combat, misc, etc. make sure physics is in combat all of soras current movement toggles in the menu, as well as his attacks and magic
>

^m1


> [!tip] Assistant
> Here's the plan. KH1 already has a sub-menu system: it's how the Config rows open the button diagram screens, and backing out puts you back on the row you came from. I'd build "KH1 in KH2" the same way. It's one button row on page 1 that looks like KH1's "Default settings" row, and pressing it opens a list: Camera, Combat, Movement, Magic, Misc. Each one opens its own list of KH1/KH2 rows that look like KH1's. The second page goes away.
>
> Where everything goes:
> - Camera: Camera Style
> - Combat: Physics, Base Combo, Aerial Base Combo
> - Movement: Jump, Glide, Dodge Roll, Misc Animations
> - Magic: Magic, Aerial Magic
> - Misc: Battle Quotes, Gummi Routes, Debug Menu
>
> About making the rows smaller: KH1's row boxes are a fixed size, and every menu in the game uses the same ones, so there's no smaller box to switch to. Shrinking just the font doesn't free any room. What KH1 does have is a zoom for its menu windows (it uses it when they open), so I can shrink the whole Config list a bit to fit the 9th row.
>
> A few questions:
> 1. Magic: (a) its own category, or (b) inside Combat with the attacks?
> 2. (a) Just the settings we have in the Config menu now, or (b) also the useful ones that are only in the ini (KH2 hit reactions, auto aim, wall clink, platforms carrying Sora, ceilings, grabs, KH2 Reflect, the Drive page, 4 shortcuts, KH2 sounds/effects)? Some of those would need a restart to take effect.
> 3. Smaller rows: (a) shrink the whole list about 8%, text and boxes together, or (b) keep the text full size and pack the rows tighter (the boxes touch)?
> 4. Put "KH1 in KH2" (a) at the bottom after "Default settings", or (b) above it? The bottom is simpler.
> 5. Debug Menu: (a) in Misc, (b) on the KH1 in KH2 list itself, or (c) just in the ini?
> 6. Should "Default settings" (a) also reset our settings, or (b) leave them alone like it does now?
>
> Also, your other request is done: new requests now get their own folder and file as soon as I see them, which is how this thread got here.
>
> Tell me those and give me the go ahead, and I'll build it.

^m2


> [!note] Mr Bugs
> 1. Yes if your going to make magic its own sub catagory put it inside of combat, so its config > kh2 in kh1 > combat > magic
> 2. (a)
> 3. (a)
> 4. (b)
> 5. (a)
> 6. (b)
>    
>    go ahead

^m3


> [!tip] Assistant
> Got it, building it now: Config > KH1 in KH2 (above Default settings) > Camera / Combat / Movement / Misc, with Magic as its own list inside Combat, the whole Config list shrunk with KH1's zoom, only the current settings, Debug Menu in Misc, and Default settings leaving ours alone. You wrote "kh2 in kh1" in your answer; I'm keeping the button's name as "KH1 in KH2" like your request, tell me if you want it the other way. I'll post the version here when it's done.

^m4


> [!tip] Assistant
> **t448 and t449.** Done: Config page 1 now has a "KH1 in KH2" button above Default settings, and the whole list is shrunk about 8% with KH1's own menu zoom so it fits on one page (t448). The second page is gone. The button opens Camera / Combat / Movement / Misc, and Combat has Magic as its own list (t449). They open and close like KH1's button-diagram screens, with KH1's sounds, and backing out puts you on the row you came from at every level. Default settings still only resets KH1's options. The mod version number is now in the button's help line. Same ini keys as before.
>
> Checked against the KH1 decomp and exe; not tested in game yet. This menu is Steam only (on Epic, the settings are in the ini).
>
> Please check: the whole flow in and out of each list, how the 92% text looks, that the cursor lands back on the button when you back out, and closing the pause menu while you're inside a list.
>
> Questions:
> 1. The sub-lists are KH1's normal size. Should they (a) stay full size like KH1's other sub-screens, or (b) be shrunk to 92% to match page 1?
> 2. Does the 92% text look OK, or should it be (a) a bit bigger, like 95% (the bottom row would then just touch the help box), or (b) left as is?

^m5

