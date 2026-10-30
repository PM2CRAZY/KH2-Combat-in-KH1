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
//! Pure Rust (no Windows calls), unit tested on any host; the game side is `win::combat` (Valor's
//! refusal) and `win::formlimits` (the limit offer).

/// Commands an object offers Sora that KH1 runs on that object (t962).
pub const REACTIONS: [u8; 6] = [4, 5, 0x1C, 0x1D, 0x1E, 0x37];
/// KH1's limits, starts and follow-ups.
pub const LIMITS: [u8; 12] = [0x4B, 0x4D, 0x4E, 0x52, 0x57, 0x58, 0x59, 0x5A, 0x5B, 0x5E, 0x5F, 0x60];
/// The last of KH1's own command records.
pub const KH1_LAST: u8 = 0x7C;
/// Ids KH1's command step (`0x1402A7510`), its dispatcher (`0x1402A3600`) or its ground branch
/// (`0x1402A3DB0`) run themselves; any other of KH1's goes to the target object's handler.
/// (0x62 too: its record's kind is Attack's, 1, so it is kept with the moves; Assumption: an
/// attack of KH1's, not looked up.)
const KH1_RUNS: [u8; 23] = [0, 1, 2, 3, 4, 5, 6, 0x1C, 0x1D, 0x1E, 0x37, 0x44, 0x46, 0x47, 0x4F, 0x51, 0x53, 0x54, 0x5C, 0x5D, 0x62, 99, 100];
/// What Valor let through before t962 besides Attack, Magic and items: the party's and the
/// menu's commands that don't move Sora (the limit ids left out: Valor refuses them, the owner's
/// "dont let sora use triangle moves (sonic blade, strike raid etc) in valor form").
const NOT_MOVES: [u8; 9] = [0x1D, 0x1E, 0x46, 0x51, 0x54, 0x64, 0x4F, 0x5C, 0x5D];
/// KH1's item command.
pub const ITEM: u8 = 3;

/// Whether `id` is one of KH1's limits.
pub fn limit(id: u8) -> bool {
    LIMITS.contains(&id)
}

/// Whether `id` is a command KH1 hands to the object Sora chose (a save point, a lever, ...).
pub fn object_command(id: u8) -> bool {
    id <= KH1_LAST && !KH1_RUNS.contains(&id) && !limit(id)
}

/// Whether Valor (two Keyblades) lets Sora use command `id` (t962): Attack, Magic, items, the
/// commands that don't move him, and every command an object offers (KH2's DisableForm 0);
/// not KH1's moves (its limits, Counterattack 99, the summons, ...).
pub fn valor_allows(id: u8) -> bool {
    matches!(id, 0 | 1 | 2 | ITEM) || NOT_MOVES.contains(&id) || REACTIONS.contains(&id) || object_command(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valor_lets_objects_through() {
        for id in [4, 5, 0x1C, 0x1D, 0x1E, 0x37, 0x65, 0x77] {
            assert!(valor_allows(id), "{id:#x}");
        }
        for id in [1, 2, ITEM, 0x46, 0x64] {
            assert!(valor_allows(id), "{id:#x}");
        }
    }

    #[test]
    fn valor_keeps_kh1s_moves_out() {
        for &id in LIMITS.iter() {
            assert!(!valor_allows(id) && limit(id), "{id:#x}");
        }
        for id in [6, 0x44, 0x47, 0x53, 0x62, 99, 0x7D, 0x90] {
            assert!(!valor_allows(id), "{id:#x}");
        }
        assert!(!object_command(0x4B) && !object_command(0x7D) && object_command(0x65));
    }
}
