//! Which of KH1's commands Sora may use in a Drive Form (the owner, the main-branch port's round 9,
//! rows 18 / 19: answers 1 (a) talking, examining, chests and save points in every form, as KH2;
//! 2 (b) KH1's limits in forms as a Config toggle). Notes: `docs/kh1-drive-forms-native.md` §20.
//!
//! - **KH2** (Checked (files), `03system.bin` `cmd`, DisableForm at +44): Talk, Examine, Open and
//!   Save have DisableForm 0, allowed in every form. KH2's limits are party limits (each with a
//!   partner, 00battle `limt`), and a form takes the partners, so no limit is offered in a form.
//! - **KH1** (Checked (decomp), Steam): the context scan `0x1402948F0` offers Talk (command 4, slot
//!   0) and slot 3's Examine (Assumption: command 5, the decompile drops the argument); the chest
//!   handler `0x1402C5D50` offers 0x1C / 0x1D / 0x1E; the ground branch `0x1402A3DB0` runs 4 / 5
//!   (the object's event, `0x14028DD80`), 0x1C (the open motion by height), 0x37 (a chest's Open, the
//!   chest table `0x140528D60`, `0x1402B58E0`) and hands any id it doesn't know to the object's own
//!   handler (`0x140297CA0`: save points, Assumption, and other objects). KH1's command records end
//!   at 0x7C (Checked (files), `btltbl.bin`); ids from 0x7D are ours (Reflect, Drive, ...).
//! - KH1's limits (`docs/kh1-kh2-limits.md` §3): starts 0x4B Sonic Blade, 0x57 Ars Arcanum, 0x5E
//!   Strike Raid, 0x5A Ragnarok, 0x52 Trinity Limit; follow-ups 0x4D / 0x4E, 0x58 / 0x59, 0x5B,
//!   0x5F / 0x60. The dispatcher `0x1402A3600` sends them to `0x14028D3D0`.
//!
//! - t1023: Valor's own refusal (t962's `valor_allows`: Attack, magic, items, the commands that
//!   don't move Sora and every object's command, not KH1's moves) is gone. The owner (§24.2 answer
//!   6): "valor and master shouldn't refuse commands, we'll hand tune which ones if something
//!   looks off, it should be the same as wisdom form". Every form takes KH1's commands as Wisdom
//!   does; KH1's limits go by Drive > Limits in Forms alone. Talk / Examine / chests / save points
//!   (rows 18 / 19 above) were already Wisdom's.
//!
//! Pure Rust (no Windows calls), unit tested on any host; the game side is `win::formlimits` (the
//! limit offer and refusal).

/// KH1's limits, starts and follow-ups.
pub const LIMITS: [u8; 12] = [0x4B, 0x4D, 0x4E, 0x52, 0x57, 0x58, 0x59, 0x5A, 0x5B, 0x5E, 0x5F, 0x60];

/// Whether `id` is one of KH1's limits.
pub fn limit(id: u8) -> bool {
    LIMITS.contains(&id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_are_kh1s() {
        for &id in LIMITS.iter() {
            assert!(limit(id), "{id:#x}");
        }
        for id in [0, 1, 2, 3, 4, 5, 0x1C, 0x37, 0x4F, 99, 0x7D] {
            assert!(!limit(id), "{id:#x}");
        }
    }
}
