//! KH2's Drive Recovery and High Drive Recovery as KH1 items (the user: "have drive recovery items
//! exist and be sold at the item shop in traverse town", then "lets make drive recovery and high
//! drive recoveries more available throughout the game (mob drops, chests, etc)"): their numbers,
//! KH1 records, enemy drop rates (`TIERS`) and changed chests (`CHEST_LIST`), kept apart
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

/// KH1's item shops (every room script's op 0xDE, `docs/kh1-items.md`): the Traverse Town Item
/// Shop (`tw10`, world 3 room 9: KH1's room numbers are the script file's less one) opens lists 0..3
/// by the story, the Accessory Shop (`tw11`) lists 4..6 (accessories only), and Agrabah's (`al19`,
/// world 8 room 18) list 7. The consumables' shops are the ones whose list has a Potion: ours go
/// into those (lists 0..3 and 7), not into the Accessory Shop's.
pub const POTION: u32 = 1;
/// A shop list holds 52 ids (`0xD4` bytes with its count).
pub const SHOP_LIST_MAX: usize = 52;

/// A shop list that sells consumables (it has KH1's Potion).
pub fn sells_consumables(kh1: &[u32]) -> bool {
    kh1.contains(&POTION)
}

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

// --- Enemy drops ---------------------------------------------------------------------------------

/// KH1's enemy drop (`0x1402A9AB0`, its AI's drop op) gets seven prize counts (munny 1 / 5 / 20, HP
/// 1 / 5, MP 2 / 15) and three (chance, item) pairs; each pair drops its item when KH1's random
/// float in [0, 1) is below `chance × luck` (`luck` is KH1's `0x142D619A8`, 1 raised by Lucky
/// Strike). Ours roll the same way after KH1's own, with the chances of the drop's tier below.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropTier {
    pub name: &'static str,
    /// Chances (0..1, before luck) of a Drive Recovery and of a High Drive Recovery.
    pub drive: f32,
    pub high: f32,
}

/// The drop tiers, by what KH1 itself drops for the enemy (its seven counts): the easy table to
/// edit. The user: "findable but not flooding". A few percent per enemy is about one Drive
/// Recovery every 30 enemies; a High one about every 200; bosses and the biggest Heartless (KH1's
/// big drops) are where they are likelier.
pub const TIERS: [DropTier; 3] = [
    // Any enemy (the user's s326 logs: KH1's drops are 3..15 prizes, most of them HP orbs).
    DropTier { name: "an enemy", drive: 0.03, high: 0.005 },
    // An enemy KH1 gives MP orbs too (only some kinds of Heartless have them in their drop): the
    // "drive flavour" tier. KH1's drop doesn't say which enemy it is, so its own contents are
    // the sign (unverified which Heartless these are: the log names the counts).
    DropTier { name: "an enemy with MP orbs", drive: 0.05, high: 0.0075 },
    // A big drop: 20 prizes or more (bosses, the large Heartless).
    DropTier { name: "a big drop", drive: 0.15, high: 0.04 },
];
/// A drop of this many prizes or more (KH1's seven counts together) is a big one.
pub const BIG_DROP: i32 = 20;

/// The tier of a KH1 drop (its seven counts), or none for an empty drop.
pub fn tier(counts: &[i32; 7]) -> Option<&'static DropTier> {
    let total: i32 = counts.iter().map(|&n| n.max(0)).sum();
    if total <= 0 {
        None
    } else if total >= BIG_DROP {
        Some(&TIERS[2])
    } else if counts[5] > 0 || counts[6] > 0 {
        Some(&TIERS[1])
    } else {
        Some(&TIERS[0])
    }
}

/// What a drop gives (at most one of ours): two rolls in [0, 1) and KH1's luck. The High one is
/// rolled first, as the rarer.
pub fn drop_item(t: &DropTier, high_roll: f32, drive_roll: f32, luck: f32) -> Option<u8> {
    let luck = if luck.is_finite() && luck > 0.0 { luck } else { 1.0 };
    if high_roll < t.high * luck {
        Some(HIGH_DRIVE_RECOVERY)
    } else if drive_roll < t.drive * luck {
        Some(DRIVE_RECOVERY)
    } else {
        None
    }
}

// --- Chests ---------------------------------------------------------------------------------------

/// KH1's chests: one u16 per chest at `0x140529A60` in the executable's own data (512 chests; read
/// when a chest is opened, `0x1402B3740`, and at a new game for bit 0, `0x14028342B`). The chest a
/// room's chest object opens is its object record's `+0x48` (records of type 8 in the room's
/// `.ard`). A value: bit 0 starts opened, bits 1..3 the kind, bits 4.. the value: kind 0 an item
/// (given with KH1's inventory add, `0x140282CF0`), 1 a Gummi block, 2 Dalmatians, 3 munny / 10, 7 a
/// row of the prize table (`btltbl` + 0xC6A8), whose item is given.
pub const CHESTS: usize = 512;

/// A chest's value giving item `id` (kind 0).
pub const fn chest_item(id: u8) -> u16 {
    (id as u16) << 4
}

/// One changed chest: its index, where it is (world prefix and room script, KH1's room is the
/// file's number less one), what KH1 put in it (the value, checked before the change) and ours.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chest {
    pub index: u16,
    pub world: &'static str,
    pub room: &'static str,
    pub vanilla: u16,
    pub was: &'static str,
    pub item: u8,
}

const DR: u8 = DRIVE_RECOVERY;
const HDR: u8 = HIGH_DRIVE_RECOVERY;

/// The changed chests (`docs/kh1-items.md` lists them): only chests of plain consumables (Potion,
/// Ether, Mega-Potion, Mega-Ether, Elixir, Megalixir), never key items, abilities, Keyblades,
/// accessories, stat ups, synthesis items, Dalmatians, Gummi blocks or maps; one or two Drive
/// Recovery per world after Traverse Town (the first Drive Form opens at its end), High Drive
/// Recovery in Hollow Bastion, the End of the World and Monstro's last chamber.
pub const CHEST_LIST: [Chest; 17] = [
    Chest { index: 99, world: "Wonderland", room: "aw02", vanilla: 0x0060, was: "Mega-Potion", item: DR },
    Chest { index: 109, world: "Wonderland", room: "aw08", vanilla: 0x0060, was: "Mega-Potion", item: DR },
    Chest { index: 125, world: "Deep Jungle", room: "tz06", vanilla: 0x0060, was: "Mega-Potion", item: DR },
    Chest { index: 131, world: "Deep Jungle", room: "tz08", vanilla: 0x0010, was: "Potion", item: DR },
    Chest { index: 200, world: "Agrabah", room: "al04", vanilla: 0x0060, was: "Mega-Potion", item: DR },
    Chest { index: 210, world: "Agrabah", room: "al08", vanilla: 0x0060, was: "Mega-Potion", item: DR },
    Chest { index: 302, world: "Halloween Town", room: "nm01", vanilla: 0x0060, was: "Mega-Potion", item: DR },
    Chest { index: 316, world: "Halloween Town", room: "nm07", vanilla: 0x0030, was: "Ether", item: DR },
    Chest { index: 333, world: "Olympus Coliseum", room: "he01", vanilla: 0x0060, was: "Mega-Potion", item: DR },
    Chest { index: 363, world: "Monstro", room: "pi08", vanilla: 0x0070, was: "Mega-Ether", item: DR },
    Chest { index: 375, world: "Monstro", room: "pi10", vanilla: 0x0070, was: "Mega-Ether", item: DR },
    Chest { index: 242, world: "Monstro", room: "pi11", vanilla: 0x0080, was: "Megalixir", item: HDR },
    Chest { index: 412, world: "Hollow Bastion", room: "pc01", vanilla: 0x0080, was: "Megalixir", item: HDR },
    Chest { index: 420, world: "Hollow Bastion", room: "pc04", vanilla: 0x0080, was: "Megalixir", item: HDR },
    Chest { index: 454, world: "End of the World", room: "ew02", vanilla: 0x07DE, was: "Mega-Potion (prize row 0x7D)", item: DR },
    Chest { index: 456, world: "End of the World", room: "ew02", vanilla: 0x07FE, was: "Elixir (prize row 0x7F)", item: HDR },
    Chest { index: 475, world: "End of the World", room: "ew25", vanilla: 0x0080, was: "Megalixir", item: HDR },
];

/// The changed chest of an index.
pub fn chest(index: i32) -> Option<&'static Chest> {
    CHEST_LIST.iter().find(|c| c.index as i32 == index)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drop_tiers() {
        assert_eq!(tier(&[0; 7]), None);
        assert_eq!(tier(&[0, 0, 0, 3, 0, 0, 0]).unwrap().name, "an enemy");
        assert_eq!(tier(&[4, 0, 0, 0, 0, 1, 1]).unwrap().name, "an enemy with MP orbs");
        assert_eq!(tier(&[15, 0, 1, 8, 0, 0, 0]).unwrap().name, "a big drop");
        for t in TIERS {
            assert!(t.high < t.drive && t.drive <= 0.2 && t.high > 0.0);
        }
        // Under 1% for the High one outside big drops.
        assert!(TIERS[0].high < 0.01 && TIERS[1].high < 0.01);
    }

    #[test]
    fn drop_rolls() {
        let t = &TIERS[0];
        assert_eq!(drop_item(t, 0.0, 0.9, 1.0), Some(HIGH_DRIVE_RECOVERY));
        assert_eq!(drop_item(t, 0.5, 0.01, 1.0), Some(DRIVE_RECOVERY));
        assert_eq!(drop_item(t, 0.5, 0.5, 1.0), None);
        // Lucky Strike's luck doubles the chance.
        assert_eq!(drop_item(t, 0.5, 0.05, 2.0), Some(DRIVE_RECOVERY));
        assert_eq!(drop_item(t, 0.5, 0.05, f32::NAN), None);
    }

    #[test]
    fn chests() {
        assert_eq!(chest_item(DRIVE_RECOVERY), 0x0910);
        assert_eq!(chest_item(HIGH_DRIVE_RECOVERY), 0x0920);
        let mut seen = std::collections::HashSet::new();
        for c in CHEST_LIST {
            assert!((c.index as usize) < CHESTS);
            assert!(seen.insert(c.index), "chest {} twice", c.index);
            assert_eq!(c.vanilla & 1, 0);
            let kind = (c.vanilla >> 1) & 7;
            let id = c.vanilla >> 4;
            // Only plain consumables: KH1 items 1..8 directly, or a prize row (kind 7).
            assert!((kind == 0 && (1..=8).contains(&id)) || kind == 7, "chest {}", c.index);
            assert!(c.item == DRIVE_RECOVERY || c.item == HIGH_DRIVE_RECOVERY);
        }
        assert_eq!(chest(412).unwrap().item, HIGH_DRIVE_RECOVERY);
        assert!(chest(0).is_none());
    }

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
        // The item shops' lists (Traverse Town's first, Agrabah's), not the Accessory Shop's.
        assert!(sells_consumables(&[1, 2, 3, 0x8E]));
        assert!(sells_consumables(&[1, 2, 3, 0x8E, 0x8F]));
        assert!(!sells_consumables(&[0x11, 0x14, 0x17, 0x1A, 0x1D, 0x47]));
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
