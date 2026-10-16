//! Chests open in battle (the user). KH1's chest check (`0x1402B38D0`'s step) disables the
//! Triangle prompt when the battle flag (`0x142867CD8` bit 0) is set: `test [flag], 1; jz` at
//! `0x1402B3904`; the jump becomes a `jnc` (always taken after `test`), as the KH1 Final Mix
//! Archipelago companion's interaction script does (`1fmRandoInteraction.lua`, by KSX and Gicu,
//! KH-1FM-AP-LUA, MIT). Ini: `chests_in_battle`.

use super::{hook, log};

const JUMP_RVA: usize = 0x2B3904;

pub unsafe fn install() {
    if !super::settings::get().chests_in_battle {
        log("chests: closed in battle, as KH1 has them (chests_in_battle = off)");
        return;
    }
    let base = super::sys::GetModuleHandleA(std::ptr::null()) as usize;
    // `test byte [rip+...], 1` then `jz`.
    let at = base + JUMP_RVA - 6;
    if hook::patch(at, &[0x05, 0xD4, 0x43, 0x5B, 0x02, 0x01, 0x74], &[0x05, 0xD4, 0x43, 0x5B, 0x02, 0x01, 0x73], "chests in battle") {
        log("chests: open in battle too");
    }
}
