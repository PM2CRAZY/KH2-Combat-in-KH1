# Keyblade guard facing native

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: This thread is split out of What is still missing(../what-is](#^m1)

---

> [!tip] Assistant
> This thread is split out of [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md) (your message after message 19), as you asked. Your request, word for word:
>
> > Sora's invulnerable frames [...] and the Keyblade guard's facing are decided in our hit-pass hook, not by KH1's own [...] guard objects. have the agents do most native implementation. go ahead
>
> (The invulnerable frames have their own thread: [Invulnerable frames native](../invulnerable-frames-native/invulnerable-frames-native.md).)
>
> **Where it stands now (from the code and AGENTS.md):**
> - The body guards (Aerial Dodge, Aerial Recovery, Explosion, Magnet Burst) are already KH1 guard objects that KH1's own clash meets.
> - The Keyblade guard (the guard press 161 and the repel 162) is still checked in our hit-pass hook (`hurt`). It compares the attacker's direction with Sora's facing against each guard ATKP's own angle (t525: 120° either side for 161, all round for 162).
> - KH1's guard objects can already be directional: type bits `0xE0000` set to 1 or 3, with an arc around the owner's facing (`0x1402B24B0`). Which way KH1 turns the arc's angle is not checked yet.
>
> **The plan:** the Keyblade guard's window puts a KH1 guard object on the Keyblade, with KH1's own directional arc set to the ATKP's angle. KH1's clash then decides the block, and the facing check in `hurt` goes away.
>
> Your go-ahead is noted. The work starts in the next round with the PC connected, because KH1's arc test needs the decomp to match KH2's (`0x1403D1F40`) exactly. This round the PC couldn't be reached. Overlaps: [Dodge Roll, Guard and Counterguard native](../dodge-roll-guard-and-counterguard-native/dodge-roll-guard-and-counterguard-native.md) (t525) and [Body guards on KH2 moves](../body-guards-on-kh2-moves/body-guards-on-kh2-moves.md).

^m1

