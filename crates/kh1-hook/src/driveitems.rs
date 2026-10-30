//! KH2's Drive Recovery and High Drive Recovery as KH1 items (the owner, the main-branch port's
//! round 5, row 4: answers 1 (a), 2 (b), 3 (a), "go ahead"). Notes: `docs/kh1-drive-forms-native.md`
//! §16.
//!
//! - **KH2** (Checked (files) and (decomp)): items `0x112` / `0x113`, value 3 / 9: using one adds
//!   value × 100 points to the Drive gauge (`0x1403C4B80` → `0x1403C0AD0`), only while the bars are
//!   below the maximum (`0x1403C3DA0`); menu only (no battle command); never sold, sold for 20 / 30.
//!   KH2's names and help: "Drive Recovery", "Restores up to three full Drive Gauges. Usable only
//!   from the Main Menu."; "High Drive Recovery", "Restores all Drive Gauges. Usable only from the
//!   Main Menu."
//! - **KH1** (Checked (files), the player's `btltbl.bin`): ids `0x91` / `0x92` are unused records
//!   (kind 12, no amount, no price, the shared dummy help text, `+0x10` = 10000), so they take KH2's
//!   two items (answer 3 (a): main's ids, a main save's counts carry over). A record (`0x14` bytes):
//!   `+0` the name's word, `+2` the kind (0 an item KH1's battle item slots take, 12 one used only
//!   from the menu, as the tents and stat-ups are), `+3` a byte (kept), `+4` flags (bit 0 the whole
//!   party), `+6` the amount, `+8` the price, `+0xA` what it sells for, `+0xC` the help text, `+0x10`
//!   a word (kept).
//!
//! Pure Rust (no Windows calls), unit tested on any host; the game side is `win::driveitems`.

pub const DRIVE_RECOVERY: u8 = 0x91;
pub const HIGH_DRIVE_RECOVERY: u8 = 0x92;
pub const IDS: [u8; 2] = [DRIVE_RECOVERY, HIGH_DRIVE_RECOVERY];
/// KH2's values (bars: × 100 points).
pub const BARS: [u8; 2] = [3, 9];
/// What a shop asks (main's design, answer 1 (a)) and KH2's sell price.
pub const PRICE: [u16; 2] = [80, 120];
pub const SELLS_FOR: [u16; 2] = [20, 30];
pub const NAMES: [&str; 2] = ["Drive Recovery", "High Drive Recovery"];
/// KH2's help (the menu-only line goes when they are usable in battle).
pub const HELP: [&str; 2] = ["Restores up to three full Drive Gauges.", "Restores all Drive Gauges."];
pub const MENU_ONLY: &str = "\nUsable only from the Main Menu.";
pub const RECORD: usize = 0x14;
/// KH1's item kinds.
pub const KIND_ITEM: u8 = 0;
pub const KIND_MENU: u8 = 12;
/// What KH1's unused records hold at `+0x10` (with no amount at `+6`).
const UNUSED_WORD: u16 = 10000;

/// Which of ours `id` is (0 or 1).
pub fn which(id: u32) -> Option<usize> {
    IDS.iter().position(|&i| i as u32 == id)
}

/// Whether KH1's record (as `btltbl.bin` has it) is one of its unused ones, free to take.
pub fn unused(rec: &[u8]) -> bool {
    rec.len() >= RECORD && rec[2] == KIND_MENU && u16::from_le_bytes([rec[6], rec[7]]) == 0 && u16::from_le_bytes([rec[8], rec[9]]) == 0 && u16::from_le_bytes([rec[0x10], rec[0x11]]) == UNUSED_WORD
}

/// Whether the record is already ours (`set`).
pub fn ours(rec: &[u8], k: usize) -> bool {
    rec.len() >= RECORD && matches!(rec[2], KIND_ITEM | KIND_MENU) && u16::from_le_bytes([rec[6], rec[7]]) == BARS[k] as u16 && u16::from_le_bytes([rec[8], rec[9]]) == PRICE[k] && u16::from_le_bytes([rec[0x10], rec[0x11]]) == UNUSED_WORD
}

/// Item `k`'s record fields set in KH1's record `rec` (its name word, `+3`, help and `+0x10` kept):
/// the kind by whether it is used in battle too, one member, KH2's amount, the prices.
pub fn set(rec: &mut [u8], k: usize, battle: bool) {
    rec[2] = if battle { KIND_ITEM } else { KIND_MENU };
    rec[4..6].copy_from_slice(&0u16.to_le_bytes());
    rec[6..8].copy_from_slice(&(BARS[k] as u16).to_le_bytes());
    rec[8..10].copy_from_slice(&PRICE[k].to_le_bytes());
    rec[0xA..0xC].copy_from_slice(&SELLS_FOR[k].to_le_bytes());
}

/// The help text of item `k`.
pub fn help(k: usize, battle: bool) -> String {
    if battle {
        HELP[k].to_string()
    } else {
        format!("{}{MENU_ONLY}", HELP[k])
    }
}

/// KH1's item shops' lists (Steam `0x1404FEE00`, 0xD4 bytes each: a count, then up to 52 item ids;
/// opened by `0x1403010A0`, Checked (decomp) and (exe)): the ones selling a Potion and a Tent (lists
/// 0..3 and 7; KH1's other shop lists, weapons and accessories, don't) also sell ours (main's design,
/// round 5 answer 1 (a)).
pub const SHOP_MAX: usize = 52;
pub const POTION: i32 = 0x01;
pub const TENT: i32 = 0x8E;

/// `list` with ours after KH1's, if it is an item shop's and hasn't them yet.
pub fn shop_with_ours(list: &[i32]) -> Option<Vec<i32>> {
    let has = |i: i32| list.contains(&i);
    if !has(POTION) || !has(TENT) || IDS.iter().any(|&i| has(i as i32)) || list.len() + IDS.len() > SHOP_MAX {
        return None;
    }
    let mut v = list.to_vec();
    v.extend(IDS.iter().map(|&i| i as i32));
    Some(v)
}

/// The enemy drops (main's design, round 5 answer 1 (a)): the chance of each (Drive Recovery, High Drive
/// Recovery) by the prizes KH1's drop gave (`counts`: KH1's 7 prize types, munny 0..2, HP 3 / 4, MP
/// 5 / 6, each already × KH1's count factor): 20 or more prizes 15 % / 4 %, else MP orbs among them
/// 5 % / 0.75 %, else 3 % / 0.5 %. KH1 rolls its own item drops as `random < Lucky Strike factor ×
/// chance` (`0x1402ABC40`, the chance a float; the factor `0x142D60FA8`, 1 + 0.5 a Lucky Strike, Checked
/// (decomp) and (exe)), and so are these.
pub const DROP_MANY: [f32; 2] = [0.15, 0.04];
pub const DROP_MP: [f32; 2] = [0.05, 0.0075];
pub const DROP_BASE: [f32; 2] = [0.03, 0.005];
pub const MANY_PRIZES: i32 = 20;

pub fn drop_chances(counts: &[i32; 7]) -> [f32; 2] {
    let total: i32 = counts.iter().map(|&c| c.max(0)).sum();
    if total >= MANY_PRIZES {
        DROP_MANY
    } else if counts[5] > 0 || counts[6] > 0 {
        DROP_MP
    } else {
        DROP_BASE
    }
}

/// The chests (main's design, round 5 answer 1 (a)): KH1's chest table (Steam `0x140528D60`, a u16 per
/// chest: bits 1..3 the kind, 0 an item, 7 a reward record; bits 4.. the item; Checked (decomp) and
/// (exe)) with these 17 chests of plain consumables holding ours: (chest, KH1's value, which of ours,
/// world and room for the notes). Every KH1 value here was read from the Steam exe (Checked (exe)).
pub const CHESTS: [(u16, u16, usize, &str); 17] = [
    (99, 0x0060, 0, "Wonderland aw02, Mega-Potion"),
    (109, 0x0060, 0, "Wonderland aw08, Mega-Potion"),
    (125, 0x0060, 0, "Deep Jungle tz06, Mega-Potion"),
    (131, 0x0010, 0, "Deep Jungle tz08, Potion"),
    (200, 0x0060, 0, "Agrabah al04, Mega-Potion"),
    (210, 0x0060, 0, "Agrabah al08, Mega-Potion"),
    (302, 0x0060, 0, "Halloween Town nm01, Mega-Potion"),
    (316, 0x0030, 0, "Halloween Town nm07, Ether"),
    (333, 0x0060, 0, "Olympus Coliseum he01, Mega-Potion"),
    (363, 0x0070, 0, "Monstro pi08, Mega-Ether"),
    (375, 0x0070, 0, "Monstro pi10, Mega-Ether"),
    (242, 0x0080, 1, "Monstro pi11, Megalixir"),
    (412, 0x0080, 1, "Hollow Bastion pc01, Megalixir"),
    (420, 0x0080, 1, "Hollow Bastion pc04, Megalixir"),
    (454, 0x07DE, 0, "End of the World ew02, Mega-Potion (reward row 0x7D)"),
    (456, 0x07FE, 1, "End of the World ew02, Elixir (reward row 0x7F)"),
    (475, 0x0080, 1, "End of the World ew25, Megalixir"),
];
pub const CHEST_COUNT: usize = 512;

/// A chest's value giving our item `k` (kind 0).
pub fn chest_value(k: usize) -> u16 {
    (IDS[k] as u16) << 4
}

/// KH2's use test (`0x1403C3DA0`): the bars below the maximum.
pub fn usable(bars: u8, max: u8) -> bool {
    bars < max
}

/// The points item `k` adds (`0x1403C4B80`: value × 100).
pub fn points(k: usize) -> i32 {
    BARS[k] as i32 * 100
}

#[cfg(test)]
mod tests {
    use super::*;

    /// KH1's record `0x91` as the player's `btltbl.bin` has it (word 0xAF, kind 12, `+3` 6, the dummy
    /// help handle, 10000).
    fn kh1_91() -> [u8; RECORD] {
        [0xAF, 0, 0x0C, 0x06, 0, 0, 0, 0, 0, 0, 0, 0, 0x40, 0xCC, 0, 0, 0x10, 0x27, 0, 0]
    }

    #[test]
    fn records_take_kh2s_values() {
        let mut r = kh1_91();
        assert!(unused(&r));
        set(&mut r, 0, false);
        assert!(!unused(&r) && ours(&r, 0) && !ours(&r, 1));
        assert_eq!((r[0], r[2], r[3]), (0xAF, KIND_MENU, 6));
        assert_eq!((u16::from_le_bytes([r[6], r[7]]), u16::from_le_bytes([r[8], r[9]]), u16::from_le_bytes([r[10], r[11]])), (3, 80, 20));
        assert_eq!(&r[0xC..0x12], &kh1_91()[0xC..0x12]);
        set(&mut r, 0, true);
        assert_eq!(r[2], KIND_ITEM);
        assert!(ours(&r, 0));
    }

    #[test]
    fn item_shops_sell_them() {
        // KH1's lists 0 and 1 (Checked (exe)), and a weapon shop's.
        assert_eq!(shop_with_ours(&[1, 2, 3, 0x8E]), Some(vec![1, 2, 3, 0x8E, 0x91, 0x92]));
        assert_eq!(shop_with_ours(&[1, 2, 3, 0x8E, 0x68, 0x6C, 0x7D, 0x7A]).map(|v| v.len()), Some(10));
        assert_eq!(shop_with_ours(&[0x11, 0x14, 0x17, 0x1A, 0x1D, 0x47]), None);
        assert_eq!(shop_with_ours(&[1, 0x8E, 0x91, 0x92]), None);
    }

    #[test]
    fn drop_chances_by_the_prizes() {
        assert_eq!(drop_chances(&[3, 0, 0, 1, 0, 0, 0]), DROP_BASE);
        assert_eq!(drop_chances(&[3, 0, 0, 0, 0, 1, 0]), DROP_MP);
        assert_eq!(drop_chances(&[10, 5, 0, 5, 0, 0, 0]), DROP_MANY);
    }

    #[test]
    fn chests() {
        assert_eq!((chest_value(0), chest_value(1)), (0x0910, 0x0920));
        let mut seen = std::collections::HashSet::new();
        for (i, v, k, _) in CHESTS {
            assert!((i as usize) < CHEST_COUNT && seen.insert(i));
            assert!(v >> 1 & 7 == 0 || v >> 1 & 7 == 7);
            assert!(k < 2);
        }
        assert_eq!(CHESTS.iter().filter(|c| c.2 == 0).count(), 12);
    }

    #[test]
    fn kh2s_rule() {
        assert_eq!((which(0x91), which(0x92), which(0x90)), (Some(0), Some(1), None));
        assert_eq!((points(0), points(1)), (300, 900));
        assert!(usable(2, 9) && !usable(9, 9));
        assert!(help(1, false).ends_with("Main Menu.") && !help(1, true).contains('\n'));
    }
}
