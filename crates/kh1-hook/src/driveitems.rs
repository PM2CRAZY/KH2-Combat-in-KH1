//! KH2's Drive Recovery and High Drive Recovery as KH1 items (the user: "have drive recovery items
//! exist and be sold at the item shop in traverse town"): their numbers and KH1 records, kept apart
//! from the runtime (`win::items`) so they are easy to check and test. `docs/kh1-items.md` has the
//! sources.
//!
//! - **What they do** (KH2's exe, its item-use switch on item 274 / 275 and its 03system `item`
//!   rows): add `bars × 100` points to the Drive gauge through the gauge's add (so a part-filled
//!   bar carries over, and a full gauge holds at its maximum); `bars` is the row's byte 4: 3 and 9.
//!   In a form they add to the bars waiting under the form's gauge, not to the form's gauge.
//! - **When** (KH2's usable test for the same two items): while the gauge isn't full (bars below
//!   the maximum). Here also only once the gauge is shown (a Drive Form open): before that KH2's
//!   maximum is 0, so its test fails too.
//! - **Where** (KH2's row: type 1, no command; its description: "Usable only from the Main
//!   Menu"): the pause menu's Items page only, as KH1's Tents are (KH1's item kind 12): not in the
//!   command menu, not in item slots, so the party's AI never has them.
//! - **Price**: KH2's rows have no buy price (0: neither is sold in KH2; no shop lists them), only
//!   sell prices, 20 and 30, at KH2's rate of a quarter of the buy price (Potion 40 / 10, Hi-Potion
//!   100 / 25, Ether 120 / 30). So the buy prices are 80 and 120; KH1 sells back at a fifth (its own
//!   rows: Potion 60 / 12, Tent 250 / 50), 16 and 24.

/// KH1 item ids: two of KH1's unused kind-12 records (after Cottage, 0x90; their names are
/// placeholders, "F04" and "F05"; no prize-table row, shop list or synthesis list has them, and
/// KH1's use switch does nothing for them).
pub const DRIVE_RECOVERY: u8 = 0x91;
pub const HIGH_DRIVE_RECOVERY: u8 = 0x92;

/// KH1's item record: 0x14 bytes, `table + (id - 1) * 0x14` (`docs/kh1-items.md`).
pub const RECORD: usize = 0x14;
/// KH1's kind for items used from the menu only (Tent, Camping Set, Cottage, the stat ups).
pub const KIND_MENU: u8 = 12;
/// KH1's kind for the consumables (Potion, Ether, ...): its icon is the one ours show.
pub const KIND_CONSUMABLE: u8 = 0;
/// KH1's inventory cap (`0x140282CF0`).
pub const MAX_COUNT: u8 = 99;

/// KH2's sell rate (sell = buy / 4) and KH1's (sell = buy / 5).
pub const KH2_SELL_DIVISOR: u16 = 4;
pub const KH1_SELL_DIVISOR: u16 = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Item {
    pub id: u8,
    /// KH2's item id (03system `item`).
    pub kh2_id: u16,
    pub name: &'static str,
    /// KH2's description (its sys.bar text).
    pub help: &'static str,
    /// Bars added (KH2's row, byte 4).
    pub bars: u8,
    /// KH2's sell price (its row, `+0x0E`).
    pub kh2_sell: u16,
    /// KH1's field model for the item (record `+3`; Ether's and Elixir's).
    pub model: u8,
    /// KH1's sort key (record `+0x10`): after Cottage's 103.
    pub sort: u16,
}

pub const ITEMS: [Item; 2] = [
    Item {
        id: DRIVE_RECOVERY,
        kh2_id: 274,
        name: "Drive Recovery",
        help: "Restores up to three full Drive Gauges.\nUsable only from the Main Menu.",
        bars: 3,
        kh2_sell: 20,
        model: 0,
        sort: 104,
    },
    Item {
        id: HIGH_DRIVE_RECOVERY,
        kh2_id: 275,
        name: "High Drive Recovery",
        help: "Restores all Drive Gauges.\nUsable only from the Main Menu.",
        bars: 9,
        kh2_sell: 30,
        model: 2,
        sort: 105,
    },
];

pub fn get(id: u32) -> Option<&'static Item> {
    ITEMS.iter().find(|i| i.id as u32 == id)
}

impl Item {
    /// The buy price: KH2's sell price at KH2's rate.
    pub fn buy(&self) -> u16 {
        self.kh2_sell * KH2_SELL_DIVISOR
    }
    /// The sell price at KH1's rate.
    pub fn sell(&self) -> u16 {
        self.buy() / KH1_SELL_DIVISOR
    }
    /// Points added to the gauge.
    pub fn points(&self) -> i32 {
        self.bars as i32 * 100
    }

    /// The KH1 record: `old` (the unused record's bytes, which keep their name word `+0`) with
    /// kind 12, the field model, no party flag, the bars (`+6`, KH1's amount field), the prices,
    /// the help text's handle and the sort key.
    pub fn record(&self, old: [u8; RECORD], help: u32) -> [u8; RECORD] {
        let mut r = old;
        r[2] = KIND_MENU;
        r[3] = self.model;
        r[4..6].copy_from_slice(&0u16.to_le_bytes());
        r[6..8].copy_from_slice(&(self.bars as i16).to_le_bytes());
        r[8..10].copy_from_slice(&self.buy().to_le_bytes());
        r[10..12].copy_from_slice(&self.sell().to_le_bytes());
        r[12..16].copy_from_slice(&help.to_le_bytes());
        r[16..18].copy_from_slice(&self.sort.to_le_bytes());
        r[18..20].copy_from_slice(&0u16.to_le_bytes());
        r
    }
}

/// KH2's test (its usable switch for 274 / 275): the gauge isn't full. Ours: and shown.
pub fn usable(shown: bool, bars: u8, max: u8) -> bool {
    shown && bars < max
}

/// KH1's Traverse Town Item Shop (world 3 "tw", room 10: its script opens KH1's item shop with
/// list 0..3 by the story).
pub const SHOP_WORLD: u32 = 3;
pub const SHOP_ROOM: u32 = 10;
/// A shop list holds 52 ids (`0xD4` bytes with its count).
pub const SHOP_LIST_MAX: usize = 52;

/// A shop list with ours after KH1's (each once; nothing past the list's room).
pub fn shop_list(kh1: &[u32]) -> Vec<u32> {
    let mut out: Vec<u32> = kh1.iter().copied().take(SHOP_LIST_MAX).collect();
    for i in ITEMS {
        if !out.contains(&(i.id as u32)) && out.len() < SHOP_LIST_MAX {
            out.push(i.id as u32);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prices() {
        assert_eq!((ITEMS[0].buy(), ITEMS[0].sell()), (80, 16));
        assert_eq!((ITEMS[1].buy(), ITEMS[1].sell()), (120, 24));
    }

    #[test]
    fn record_fields() {
        let mut old = [0u8; RECORD];
        old[0..2].copy_from_slice(&175u16.to_le_bytes());
        old[2] = 12;
        let r = ITEMS[0].record(old, 0x1234_5678);
        assert_eq!(u16::from_le_bytes([r[0], r[1]]), 175);
        assert_eq!(r[2], KIND_MENU);
        assert_eq!(i16::from_le_bytes([r[6], r[7]]), 3);
        assert_eq!(u16::from_le_bytes([r[8], r[9]]), 80);
        assert_eq!(u16::from_le_bytes([r[10], r[11]]), 16);
        assert_eq!(u32::from_le_bytes([r[12], r[13], r[14], r[15]]), 0x1234_5678);
        assert_eq!(u16::from_le_bytes([r[16], r[17]]), 104);
    }

    #[test]
    fn usable_rule() {
        assert!(usable(true, 2, 3));
        assert!(!usable(true, 3, 3));
        assert!(!usable(false, 0, 3));
    }

    #[test]
    fn shop() {
        assert_eq!(shop_list(&[1, 2, 3, 0x8E]), vec![1, 2, 3, 0x8E, 0x91, 0x92]);
        assert_eq!(shop_list(&[1, 0x91]), vec![1, 0x91, 0x92]);
        let full: Vec<u32> = (1..=52).collect();
        assert_eq!(shop_list(&full).len(), SHOP_LIST_MAX);
    }

    #[test]
    fn ids_unused_by_kh1() {
        // Kind-12 placeholders after Cottage (0x90), inside the inventory's 1..=255.
        for i in ITEMS {
            assert!((0x91..=0x94).contains(&i.id));
            assert!(i.points() <= 900);
        }
    }
}
