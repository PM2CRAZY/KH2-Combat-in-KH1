//! Sora's menus, read from the install: the command menu (labels and MP
//! costs from `03system.bin` `cmd`), his abilities (names from
//! `msg/us/sys.bar`, AP costs from `03system.bin` `item`), his spells
//! (`00battle.bin` MAGC: cast motions, spell files) and his HP/MP/AP.

use character_core::Abilities;
use kh2_formats::msg::Messages;
use kh2_formats::system::{item_type, Command, ItemTable};

use crate::{Install, Result, Source};

/// What an ability does in this project.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Growth ability (High Jump, ...) at a level 1..=4.
    Growth(Growth, u8),
    /// Unlocks combo rows (PTYA `ability`).
    Action,
    Guard,
    AerialRecovery,
    ComboPlus,
    AirComboPlus,
    FinishingPlus,
    NegativeCombo,
    ComboMaster,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Growth {
    HighJump,
    QuickRun,
    AerialDodge,
    Glide,
    DodgeRoll,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AbilityInfo {
    /// Item id.
    pub id: u16,
    /// Ability number (what PTYA rows and `Abilities::equipped` use).
    pub number: u8,
    pub name: String,
    pub ap: u8,
    pub effect: Effect,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpellInfo {
    /// MAGC element id: 0 Fire, 1 Blizzard, 2 Thunder, 3 Cure, 4 Magnet,
    /// 5 Reflect.
    pub element: u8,
    pub level: u8,
    pub name: String,
    pub command: u16,
    /// MP cost; 255 = all remaining MP (Cure).
    pub mp_cost: u8,
    /// Sora's cast motions (-1 = none).
    pub ground_motion: i16,
    pub finish_motion: i16,
    pub air_motion: i16,
    pub ground_blend: i16,
    pub air_blend: i16,
    /// The spell's own file (effects and its behaviour script).
    pub file: String,
}

/// A Drive command (03system cmd: execute 4, argument = form, byte 0x1c =
/// cost in bars).
#[derive(Clone, Debug, PartialEq)]
pub struct DriveInfo {
    pub form: u8,
    pub command: u16,
    pub name: String,
    pub cost: u8,
}

#[derive(Clone, Debug, Default)]
pub struct MenuKit {
    /// The command menu's top level (Attack, Magic, Items, Drive).
    pub root: Vec<(u16, String)>,
    pub abilities: Vec<AbilityInfo>,
    pub spells: Vec<SpellInfo>,
    /// Starting HP/MP (PLRP).
    pub hp: u32,
    pub mp: u32,
    /// AP at each level (LVUP).
    pub ap_by_level: Vec<u8>,
    /// The Drive submenu, in the game's order (Valor, Wisdom, Limit, Master,
    /// Final), and Anti's row (its cost is charged when Anti triggers).
    pub drives: Vec<DriveInfo>,
    pub anti: Option<DriveInfo>,
    /// The Revert command's name.
    pub revert: String,
    /// Form id → EXP from each level to the next (00battle fmlv).
    pub form_exp: Vec<(u8, Vec<u32>)>,
    /// Every command row by id with its label (reaction commands read
    /// their priority, flags and mark from here).
    pub commands: std::collections::HashMap<u16, (Command, String)>,
}

/// The ability ids this project models, by what they do. Item ids are the
/// game's; names and AP come from the install.
fn effect_of(id: u16) -> Option<Effect> {
    let g = |base: u16, kind: Growth| (base..base + 4).contains(&id).then(|| Effect::Growth(kind, (id - base + 1) as u8));
    g(94, Growth::HighJump)
        .or_else(|| g(98, Growth::QuickRun))
        .or_else(|| g(102, Growth::AerialDodge))
        .or_else(|| g(106, Growth::Glide))
        .or_else(|| g(564, Growth::DodgeRoll))
        .or(match id {
            82 => Some(Effect::Guard),
            158 => Some(Effect::AerialRecovery),
            162 => Some(Effect::ComboPlus),
            163 => Some(Effect::AirComboPlus),
            393 => Some(Effect::FinishingPlus),
            394 => Some(Effect::NegativeCombo),
            539 => Some(Effect::ComboMaster),
            _ => None,
        })
}

pub(crate) fn load_kit_with(read: &dyn Fn(&str) -> Result<Vec<u8>>, battle: &kh2_formats::battle::Battle, action_ids: &[u16]) -> Result<MenuKit> {
    let system = read("03system.bin")?;
    let items: ItemTable = kh2_formats::system::parse_items(&system)?;
    let cmds: Vec<Command> = kh2_formats::system::parse_commands(&system)?;
    let msg = read("msg/us/sys.bar").ok().and_then(|b| Messages::parse(&b).ok()).unwrap_or_default();
    let name = |id: u32, fallback: String| msg.text(id).filter(|s| !s.is_empty()).unwrap_or(fallback);
    let cmd = |id: u16| cmds.iter().find(|c| c.id == id);

    let root = [1u16, 2, 3, 4].iter().filter_map(|&id| cmd(id).map(|c| (id, name(c.message as u32, format!("Command {id}"))))).collect();

    let mut abilities: Vec<AbilityInfo> = items
        .items
        .iter()
        .filter(|i| i.kind == item_type::ABILITY)
        .filter_map(|i| {
            // Action abilities: the ones Sora's combo rows ask for (AP 0
            // entries are other characters' variants).
            let effect = effect_of(i.id).or_else(|| (i.ap > 0 && action_ids.contains(&(i.number as u16))).then_some(Effect::Action))?;
            Some(AbilityInfo { id: i.id, number: i.number, name: name(i.name as u32, format!("Ability {}", i.id)), ap: i.ap, effect })
        })
        .collect();
    abilities.sort_by_key(|a| (group_order(a.effect), a.id));

    let mut spells = Vec::new();
    for m in battle.magc.iter().filter(|m| m.world == 0) {
        let Some(c) = cmd(m.command) else { continue };
        spells.push(SpellInfo {
            element: m.id,
            level: m.level,
            name: name(c.message as u32, m.filename.clone()),
            command: m.command,
            mp_cost: c.mp_cost,
            ground_motion: m.ground_motion,
            finish_motion: m.finish_motion,
            air_motion: m.air_motion,
            ground_blend: m.ground_blend,
            air_blend: m.air_blend,
            file: m.filename.clone(),
        });
    }

    let drive = |id: u16| {
        cmd(id).map(|c| DriveInfo { form: c.argument as u8, command: id, name: name(c.message as u32, format!("Form {}", c.argument)), cost: c.mp_cost })
    };
    let drives = [6u16, 7, 0x2a1, 0xb, 0xc].iter().filter_map(|&id| drive(id)).collect();
    let anti = drive(0xd);
    let revert = cmd(5).map_or("Revert".to_string(), |c| name(c.message as u32, "Revert".into()));
    let form_exp = (1..=6u8)
        .map(|f| {
            let mut rows: Vec<_> = battle.fmlv.iter().filter(|r| r.form == f).collect();
            rows.sort_by_key(|r| r.level);
            (f, rows.iter().map(|r| r.exp.max(0) as u32).collect())
        })
        .collect();

    let sora = battle.plrp.iter().find(|p| p.character == 1);
    let ap_by_level = battle.lvup.first().map(|l| l.iter().map(|x| x.ap).collect()).unwrap_or_default();
    Ok(MenuKit {
        root,
        abilities,
        spells,
        hp: sora.map_or(20, |p| p.hp as u32),
        mp: sora.map_or(100, |p| p.mp as u32),
        ap_by_level,
        drives,
        anti,
        revert,
        form_exp,
        commands: cmds.iter().map(|c| (c.id, (c.clone(), name(c.message as u32, format!("Command {}", c.id))))).collect(),
    })
}

fn group_order(e: Effect) -> u8 {
    match e {
        Effect::Growth(..) => 0,
        Effect::Guard | Effect::Action => 1,
        _ => 2,
    }
}

/// Turns a set of equipped ability ids into the controller's switches.
pub fn abilities_from(equipped: &[u16], kit: &MenuKit) -> Abilities {
    let mut a = Abilities { guard: false, ..Abilities::default() };
    for info in kit.abilities.iter().filter(|i| equipped.contains(&i.id)) {
        a.equipped.push(info.number as u16);
        match info.effect {
            Effect::Growth(g, lv) => {
                let slot = match g {
                    Growth::HighJump => &mut a.high_jump,
                    Growth::QuickRun => &mut a.quick_run,
                    Growth::AerialDodge => &mut a.aerial_dodge,
                    Growth::Glide => &mut a.glide,
                    Growth::DodgeRoll => &mut a.dodge_roll,
                };
                *slot = (*slot).max(lv);
            }
            Effect::Action => {}
            Effect::Guard => a.guard = true,
            Effect::AerialRecovery => a.aerial_recovery = true,
            Effect::ComboPlus => a.combo_plus += 1,
            Effect::AirComboPlus => a.air_combo_plus += 1,
            Effect::FinishingPlus => a.finishing_plus += 1,
            Effect::NegativeCombo => a.negative_combo += 1,
            Effect::ComboMaster => a.combo_master = true,
        }
    }
    a
}

impl Install {
    pub fn load_kit(&self, source: Source, battle: &kh2_formats::battle::Battle, action_ids: &[u16]) -> Result<MenuKit> {
        load_kit_with(&|n| self.read_any(source, n), battle, action_ids)
    }
}

/// From extracted files (base names; `msg/us/sys.bar` as `msg_us_sys.bar`).
pub fn load_kit_dir(dir: &std::path::Path, battle: &kh2_formats::battle::Battle, action_ids: &[u16]) -> Result<MenuKit> {
    load_kit_with(
        &|n| {
            let base = if n.starts_with("msg/") { n.replace('/', "_") } else { n.rsplit('/').next().unwrap_or(n).to_string() };
            Ok(std::fs::read(dir.join(base))?)
        },
        battle,
        action_ids,
    )
}
