//! `03system.bin` tables beyond `pref`/`sklt`: items (with ability AP costs)
//! and commands (with MP costs). Layouts follow OpenKH's docs and its
//! `OpenKh.Kh2.SystemData` readers (Apache-2.0); the AP byte was identified
//! on the user's install (Combo Plus 1, Guard 2, Aerial Dodge 3, ...).

use crate::bar::Bar;
use crate::reader::Reader;
use crate::{bad, Result};

/// Item types (OpenKH's names).
pub mod item_type {
    pub const CONSUMABLE: u8 = 0;
    pub const KEYBLADE: u8 = 2;
    pub const ARMOR: u8 = 14;
    pub const ACCESSORY: u8 = 15;
    pub const MAGIC: u8 = 18;
    pub const ABILITY: u8 = 19;
    pub const SUMMON: u8 = 20;
    pub const FORM: u8 = 21;
}

/// One `item` row (24 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub id: u16,
    pub kind: u8,
    /// For abilities: the ability number the game's tables use (PTYA rows
    /// name their required ability by it; Guard = 1, Upper Slash = 18).
    pub number: u8,
    /// Index into the stat table (equipment).
    pub stat: u16,
    /// AP cost (abilities).
    pub ap: u8,
    /// Message ids (sys.bar) of the name and description.
    pub name: u16,
    pub description: u16,
    pub command: u16,
}

/// One equipment stat row (16 bytes).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ItemStat {
    pub id: u16,
    /// Ability item id the equipment grants (0 = none).
    pub ability: u16,
    pub strength: u8,
    pub magic: u8,
    pub defense: u8,
    pub ap: u8,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ItemTable {
    pub items: Vec<Item>,
    pub stats: Vec<ItemStat>,
}

impl ItemTable {
    pub fn get(&self, id: u16) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }
}

/// One `cmd` row (48 bytes).
#[derive(Clone, Debug, PartialEq)]
pub struct Command {
    pub id: u16,
    pub execute: u16,
    pub argument: u16,
    /// Menu this command opens (-1 = none).
    pub submenu: i8,
    pub icon: u8,
    /// sys.bar message id of its label.
    pub message: i32,
    pub flags: i32,
    pub range: f32,
    /// MP cost; 255 = all remaining MP. Drive bars for Drive commands.
    pub mp_cost: u8,
    pub group: u8,
    /// Reaction-command fields (checked against the executable's use):
    /// facing checks (cosines; -1 = off), the camera mode, priority (higher
    /// wins), receiver (0 player, 1 target, 2 both), the mark effect index,
    /// the action on execution (1 idle, 2 jump) and a distance limit.
    pub dir: f32,
    pub dir_range: f32,
    pub camera: u8,
    pub priority: u8,
    pub receiver: u8,
    pub mark: u8,
    pub action: u8,
    pub dist: u16,
}

fn entry<'a>(system: &'a [u8], name: &str) -> Result<&'a [u8]> {
    let bar = Bar::parse(system)?;
    match bar.entries.iter().find(|e| e.name == name) {
        Some(e) => Ok(e.data),
        None => bad(format!("03system.bin has no {name}")),
    }
}

pub fn parse_items(system: &[u8]) -> Result<ItemTable> {
    let d = entry(system, "item")?;
    let mut r = Reader::new(d);
    let _version = r.u32()?;
    let count = r.u32()? as usize;
    let mut items = Vec::with_capacity(count);
    for i in 0..count {
        let mut e = Reader::at(d, 8 + i * 24);
        let id = e.u16()?;
        let kind = e.u8()?;
        let _flag0 = e.u8()?;
        let number = e.u8()?;
        let _rank = e.u8()?;
        let stat = e.u8()? as u16;
        let ap = e.u8()?;
        let name = e.u16()?;
        let description = e.u16()?;
        let _buy = e.u16()?;
        let _sell = e.u16()?;
        let command = e.u16()?;
        items.push(Item { id, kind, number, stat, ap, name, description, command });
    }
    let so = 8 + count * 24;
    let mut stats = Vec::new();
    if so + 8 <= d.len() {
        let mut r = Reader::at(d, so);
        let _version = r.u32()?;
        let n = r.u32()? as usize;
        for i in 0..n {
            let mut e = Reader::at(d, so + 8 + i * 16);
            stats.push(ItemStat {
                id: e.u16()?,
                ability: e.u16()?,
                strength: e.u8()?,
                magic: e.u8()?,
                defense: e.u8()?,
                ap: e.u8()?,
            });
        }
    }
    Ok(ItemTable { items, stats })
}

pub fn parse_commands(system: &[u8]) -> Result<Vec<Command>> {
    let d = entry(system, "cmd\0").or_else(|_| entry(system, "cmd"))?;
    let mut r = Reader::new(d);
    let _version = r.u32()?;
    let count = r.u32()? as usize;
    (0..count)
        .map(|i| {
            let mut e = Reader::at(d, 8 + i * 48);
            let id = e.u16()?;
            let execute = e.u16()?;
            let argument = e.u16()?;
            let submenu = e.i8()?;
            let icon = e.u8()?;
            let message = e.i32()?;
            let flags = e.i32()?;
            let range = e.f32()?;
            let dir = e.f32()?;
            let dir_range = e.f32()?;
            let mp_cost = e.u8()?;
            let camera = e.u8()?;
            let priority = e.u8()?;
            let receiver = e.u8()?;
            let _time = e.u16()?;
            let _require = e.u16()?;
            let mark = e.u8()?;
            let action = e.u8()?;
            let _rcount = e.u16()?;
            let dist = e.u16()?;
            e.skip(4)?;
            let group = e.u8()?;
            Ok(Command { id, execute, argument, submenu, icon, message, flags, range, mp_cost, group, dir, dir_range, camera, priority, receiver, mark, action, dist })
        })
        .collect()
}
