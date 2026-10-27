# Invulnerable frames native

[Back to the tester requests](../../docs/tester-requests.md)

## Messages

- [1. Assistant: This thread is split out of What is still missing(../what-is](#^m1)

---

> [!tip] Assistant
> This thread is split out of [What is still missing](../what-is-still-missing-for-native-integration-into-kh1/what-is-still-missing-for-native-integration-into-kh1.md) (your message after message 19), as you asked. Your request, word for word:
>
> > Sora's invulnerable frames (roll, Quick Run, Explosion, Aerial Recovery) [...] are decided in our hit-pass hook, not by KH1's own no-damage timer [...]. have the agents do most native implementation. go ahead
>
> (The Keyblade guard's facing, the other half of that line, has its own thread: [Keyblade guard facing native](../keyblade-guard-facing-native/keyblade-guard-facing-native.md).)
>
> **Where it stands now (from the code and AGENTS.md):**
> - KH2's roll (t523): KH1's own 12-frame no-damage timer is taken off, and our hit-pass hook (`hurt`) throws hits away for KH2's frames by level (10 / 16 / 24 / 32).
> - Quick Run (t504): `hurt` throws hits away for KH2's frames (0 / 8 / 12 / 16 by level).
> - Explosion (32..64) and Aerial Recovery (0..16): their invulnerable ranges are read from KH2's motion and checked in `hurt` each hit.
> - KH1 already has what we need to put them on: its own no-damage timer (`0x140297F30`, actor `+0xC` and flag `0x80`), which its roll and recoils use, and the untouchable flag `0x40` that its hit pass skips. Where KH1's own roll gets `0x40` is still not found.
>
> **The plan:** each move's invulnerable range starts KH1's own timer (or the `0x40` flag) for exactly KH2's frames, from the move's own data. Then KH1's hit pass skips the hits itself, and the `hurt` checks for these moves go away. It's the same for KH1's and KH2's moves, so nothing changes at KH1.
>
> Your go-ahead is noted. The work starts in the next round with the PC connected, because it needs the decomps to check which of KH1's two mechanisms matches KH2's rule. That includes whether KH2's invulnerability also skips the hit effect and the grab, as the `0x40` flag does. This round the PC couldn't be reached. Overlaps: [Dodge Roll, Guard and Counterguard native](../dodge-roll-guard-and-counterguard-native/dodge-roll-guard-and-counterguard-native.md) (t523's roll) and [Quick Run native](../quick-run-native/quick-run-native.md).

^m1

