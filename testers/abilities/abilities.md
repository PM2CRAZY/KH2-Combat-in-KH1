# Abilities

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: This is the abilities thread, split out of "What is still mi](#^m1)

---


> [!tip] Assistant
> This is the abilities thread, split out of "What is still missing for native integration into KH1" as you asked. Ability work goes here from now on. So far:
>
> - **t718:** vanilla KH1 progression, every KH2 ability given, no extra AP.
> - **t722 and t723:** the two versions below, from your answers 1 and 3 there.
>
> **t722, the KH2 list.** KH1's Abilities menu now has a row "KH2" right after Sora's.
> - It's KH1's own page doing the work: its list, sorting, equipping and AP check. The KH2 abilities cost Sora's AP, and the AP totals on the Status and Equipment pages count both lists.
> - Sora's 48 slots are KH1's alone now, so late-game lists don't push anything out.
> - The list lives in a spare part of the save. Your save moves over by itself on the first load, and anything you had equipped stays equipped.
> - **How it works:** KH1's menu only knows party members, so while the menu itself is drawing or equipping, it sees an extra member "KH2" after Sora. KH1's menu finds that member's list in a part of the save that holds some item counts the page never reads. For each menu call the mod lends the KH2 list into that spot and puts the item counts back right after the call. Outside the menu's own calls the save is exactly KH1's. If anything item-related looks off after using the menu, that's the place to look.
>
> **t723, Superglide:** it counts as Glide LV1 now, and only Final's levels raise Glide. KH2's own Glide (the Glide style's) no longer goes to MAX after Chernabog either.
>
> **What I checked:** KH1's Abilities page, its AP functions and the save block layout in the Steam decomp and exe. Not checked: anything in game, including the menu with a guest in the party (six rows).
>
> **Please test**
> - **The KH2 row:**
>   - Open Abilities and use the KH2 row: equip and unequip, sort, and run AP out.
>   - The AP total should match on the Status page.
>   - After using the menu, check your item counts (Potions, Ethers and so on) are unchanged.
> - **Guest:** try it with a guest in the party.
> - **Saves:** save, reload, then start a new game.
> - **Log lines:**
>   - "KH2 list: installed"
>   - "abilities: N of ours moved from Sora's own list to the KH2 list"
>   - "KH2 list: Abilities menu shows a KH2 row"
> - **Superglide:** with Final below LV3 it should glide as LV1.
>
> **Question:** KH2's own Glide (the one the Glide style uses in place of KH1's Glide and Superglide) still sits in the shared list. Should it (a) stay there, or (b) move to the KH2 row?

^m1

