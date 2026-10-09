//! Drive Forms in the playground: the Drive and form gauges
//! (`character_core::drive`), the form objects (each form's own model,
//! motions, weapons, movement and combo table, read from the install), and
//! the change itself: Sora's Drive motion, the swap to the form object and
//! its arrival motion, and the revert.

use std::collections::HashMap;
use std::sync::Arc;

use character_core::drive::{form, growth_at, Drive, FormLevel};
use character_core::Abilities;
use kh2_data::sora::SoraData;

use crate::rig::RigAnim;
use crate::Game;

/// Sora's Drive motion (0x34, 160 frames on P_EX100).
pub const DRIVE_MOTION: u16 = 52;
/// A form's arrival motion (0x35, in each form's motion set).
pub const ARRIVE_MOTION: u16 = 53;
/// The motion normal Sora plays after a revert (4).
pub const REVERT_MOTION: u16 = 4;
/// Screen tint while changing: fade in over 12 frames, out over 15 (exe).
const TINT_IN: f32 = 12.0;
const TINT_OUT: f32 = 15.0;
/// Tint strength (RGB 0x30 grey at alpha 0x80).
pub const TINT_ALPHA: f32 = 0.5;

/// A change in progress: the form it becomes and frames since the command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Change {
    pub to: u8,
    pub frame: u32,
}

pub struct DriveState {
    pub drive: Drive,
    /// Form levels by form id (index 0 unused).
    pub levels: [FormLevel; 7],
    pub change: Option<Change>,
    /// Form objects by form id, and normal Sora.
    pub kits: HashMap<u8, Arc<SoraData>>,
    pub base: Option<Arc<SoraData>>,
    rng: u32,
    /// Current screen tint (0..TINT_ALPHA).
    pub tint: f32,
    /// Bumped whenever the player object changes (the mannequin is rebuilt).
    pub rig_version: u32,
    /// Testing: the next drive's Anti roll always hits.
    pub force_anti: bool,
}

/// Display names (ours; the menu shows the install's).
pub fn form_name(f: u8) -> &'static str {
    match f {
        form::VALOR => "Valor",
        form::WISDOM => "Wisdom",
        form::LIMIT => "Limit",
        form::MASTER => "Master",
        form::FINAL => "Final",
        form::ANTI => "Anti",
        _ => "Sora",
    }
}

/// Magic `sub` offset in a form (exe): Wisdom +10, Master +20, Final +30.
pub fn magic_sub(f: u8) -> i32 {
    match f {
        form::WISDOM => 10,
        form::MASTER => 20,
        form::FINAL => 30,
        _ => 0,
    }
}

/// The Magic command is disabled in Valor, Limit and Anti (cmd mask).
pub fn magic_allowed(f: u8) -> bool {
    !matches!(f, form::VALOR | form::LIMIT | form::ANTI)
}

impl DriveState {
    pub fn new(max_bars: u8, level: u8, base: Option<Arc<SoraData>>, kits: HashMap<u8, Arc<SoraData>>) -> DriveState {
        let mut drive = Drive::new(max_bars);
        // Start full so the forms can be tried at once.
        drive.add_points(100 * max_bars as i32);
        let levels = std::array::from_fn(|_| FormLevel { level: level.clamp(1, 7), exp: 0 });
        DriveState { drive, levels, change: None, kits, base, rng: 0x1234_5678, tint: 0.0, rig_version: 0, force_anti: false }
    }

    pub fn form(&self) -> u8 {
        self.drive.form()
    }

    pub fn busy(&self) -> bool {
        self.change.is_some()
    }

    /// 0..100 (the game's generator is an LCG too).
    fn roll(&mut self) -> u32 {
        self.rng = self.rng.wrapping_mul(0x10dcd).wrapping_add(1);
        (self.rng >> 8) % 100
    }
}

/// Can this form be chosen now (affordable, not in a form, loaded)?
pub fn available(game: &Game, f: u8) -> bool {
    let d = &game.drive;
    let cost = cost_of(game, f);
    !d.busy() && d.kits.contains_key(&f) && cost.is_some_and(|c| d.drive.can_afford(c))
}

fn cost_of(game: &Game, f: u8) -> Option<u8> {
    let kit = game.kit.as_ref()?;
    kit.drives.iter().chain(kit.anti.iter()).find(|d| d.form == f).map(|d| d.cost)
}

/// The Drive command: the Anti roll, the cost of the resulting form, then
/// Sora's Drive motion (invulnerable). The swap happens when it ends.
pub fn request(game: &mut Game, f: u8, battle: bool, log: &mut Vec<String>) -> bool {
    if !available(game, f) {
        return false;
    }
    let mut roll = game.drive.roll();
    if std::mem::take(&mut game.drive.force_anti) {
        game.drive.drive.anti_counter = game.drive.drive.anti_counter.max(10);
        roll = 0;
    }
    let to = game.drive.drive.anti_roll(f, battle, roll);
    let to = if game.drive.kits.contains_key(&to) { to } else { f };
    let cost = cost_of(game, to).unwrap_or(0);
    game.drive.drive.spend(cost);
    game.character.play(DRIVE_MOTION, true);
    game.drive.change = Some(Change { to, frame: 0 });
    game.casting = None;
    log.push(if to == f {
        format!("Drive: {} Form (-{cost} bars, Anti chance was {}%)", form_name(to), game.drive.drive.anti_rate())
    } else {
        format!("Drive: {} requested... Antiform! (-{cost} bars)", form_name(f))
    });
    true
}

/// Abilities in a form (exe): Sora's support abilities carry over; his
/// action and growth abilities don't. The form brings its own growth
/// ability at its level. Ours: every combo row of the form's table is
/// unlocked, and it guards if its motion set has the guard motion.
fn form_abilities(game: &Game, data: &SoraData, f: u8) -> Abilities {
    let sora = &game.sora_abilities;
    let g = growth_at(game.drive.levels[f as usize].level);
    let mut a = Abilities {
        guard: data.character.moveset.clips.contains_key(&character_core::motion::GUARD),
        aerial_recovery: false,
        combo_plus: sora.combo_plus,
        air_combo_plus: sora.air_combo_plus,
        finishing_plus: sora.finishing_plus,
        negative_combo: sora.negative_combo,
        combo_master: sora.combo_master,
        high_jump: 0,
        quick_run: 0,
        aerial_dodge: 0,
        glide: 0,
        dodge_roll: 0,
        ..Abilities::default()
    };
    match f {
        form::VALOR => a.high_jump = g,
        form::WISDOM => a.quick_run = g,
        form::LIMIT => a.dodge_roll = g,
        form::MASTER => a.aerial_dodge = g,
        form::FINAL => a.glide = g,
        _ => {}
    }
    a.equipped = data.character.moveset.combo_rows.iter().map(|r| r.ability).filter(|&x| x != 0).collect();
    a
}

/// Replaces the player object (keeps position, facing and lock-on).
fn swap_to(game: &mut Game, data: &Arc<SoraData>, abilities: Abilities) {
    let old = &game.character;
    let (facing, lock) = (old.facing(), old.lock());
    let mut c = data.character.clone();
    c.abilities = abilities;
    c.glide_button = old.glide_button;
    c.set_facing(facing);
    c.set_lock(lock);
    game.character = c;
    if game.anim.is_some() {
        game.anim = data.rig.clone().map(RigAnim::new);
    }
    game.body = crate::world::body_of(data.rig.as_ref().and_then(|r| kh2_data::sora::bg_shape(&r.body_shapes)), true);
    game.weight = data.weight;
    game.volumes.clear();
    game.drive.rig_version += 1;
}

fn arrive(game: &mut Game, to: u8, log: &mut Vec<String>) {
    let Some(data) = game.drive.kits.get(&to).cloned() else { return };
    let abilities = form_abilities(game, &data, to);
    swap_to(game, &data, abilities);
    // HP and MP to max; MP Charge cancelled (exe).
    game.sora.hp = game.sora.max_hp;
    game.mp_charge = None;
    game.mp = game.max_mp;
    let level = game.drive.levels[to as usize].level;
    game.drive.drive.enter(to, level);
    game.character.play(ARRIVE_MOTION, true);
    let bars = game.drive.drive.gauge.map_or(0.0, |g| g.bars(&game.drive.drive.params));
    if to == form::ANTI {
        log.push(format!("Antiform: {bars:.1} bars"));
    } else {
        log.push(format!("{} Form (Lv{level}): {bars:.1} bars", form_name(to)));
    }
}

/// Back to normal Sora (gauge empty, the Revert command, or a KO).
pub fn revert(game: &mut Game, log: &mut Vec<String>) {
    if game.drive.form() == form::NONE {
        return;
    }
    let Some(base) = game.drive.base.clone() else { return };
    let abilities = game.sora_abilities.clone();
    game.drive.drive.revert();
    swap_to(game, &base, abilities);
    game.character.play(REVERT_MOTION, true);
    log.push("Revert".into());
}

/// After the player's frame: the change in progress, the form gauge.
pub fn frame(game: &mut Game, battle: bool, log: &mut Vec<String>) {
    if let Some(mut c) = game.drive.change {
        c.frame += 1;
        game.drive.change = Some(c);
        game.drive.tint = (game.drive.tint + TINT_ALPHA / TINT_IN).min(TINT_ALPHA);
        // Swap when Sora's Drive motion ends.
        if !game.character.scripted() || c.frame >= 160 {
            game.drive.change = None;
            arrive(game, c.to, log);
        }
        return;
    }
    game.drive.tint = (game.drive.tint - TINT_ALPHA / TINT_OUT).max(0.0);
    if game.drive.drive.tick(battle) {
        log.push("form gauge empty".into());
        revert(game, log);
    }
}

/// One of Sora's hits landed (drive value from the ATKP row). Valor Form
/// earns 1 EXP per hit.
pub fn on_hit(game: &mut Game, d: u8, log: &mut Vec<String>) {
    let charging = game.mp_charge.is_some();
    game.drive.drive.on_hit(d, true, charging);
    if game.drive.form() == form::VALOR {
        gain_exp(game, form::VALOR, 1, log);
    }
}

/// Sora defeated an enemy: Wisdom Form earns 1 EXP per Heartless (status
/// ids 1000..=2011), Final Form per Nobody (3000..=4014).
pub fn on_kill(game: &mut Game, status_id: u16, log: &mut Vec<String>) {
    let f = game.drive.form();
    let heartless = (1000..=2011).contains(&status_id) && !(1047..=1059).contains(&status_id);
    let nobody = (3000..=4014).contains(&status_id);
    if (f == form::WISDOM && heartless) || (f == form::FINAL && nobody) {
        gain_exp(game, f, 1, log);
    }
}

fn gain_exp(game: &mut Game, f: u8, n: u32, log: &mut Vec<String>) {
    let table = game.kit.as_ref().and_then(|k| k.form_exp.iter().find(|(x, _)| *x == f)).map(|(_, t)| t.clone()).unwrap_or_default();
    let before = growth_at(game.drive.levels[f as usize].level);
    let up = game.drive.levels[f as usize].add(n, &table, 7);
    if up > 0 {
        let lv = game.drive.levels[f as usize].level;
        log.push(format!("{} Form reached Lv{lv}", form_name(f)));
        if growth_at(lv) != before {
            if let Some(data) = game.drive.kits.get(&f).cloned() {
                game.character.abilities = form_abilities(game, &data, f);
            }
        }
    }
}
