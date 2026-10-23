//! The KH2-style HUD: HP/MP gauges (bottom right), the command menu
//! (bottom left: Attack / Magic / Items / Drive, with the Magic list), and
//! the ability menu (Esc / Start). Labels, costs and names come from the
//! install via `kh2_data::kit`; the look is our own.

use bevy::prelude::*;
use kh2_data::kit::{abilities_from, MenuKit};

use crate::Game;

/// Which menu has focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Focus {
    #[default]
    Root,
    Magic,
    Drive,
    Abilities,
}

#[derive(Resource, Default)]
pub struct MenuState {
    pub focus: Focus,
    pub root: usize,
    pub magic: usize,
    pub drive: usize,
    pub ability: usize,
    pub scroll: usize,
    pub shortcuts: Shortcuts,
    /// The shortcut menu is showing (L1 / Ctrl held).
    pub shortcuts_open: bool,
}

/// The magic shortcut slots, in [`SLOTS`] order: positions in the Magic
/// list ([`spell_list`]), or empty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shortcuts(pub [Option<usize>; 4]);

impl Default for Shortcuts {
    /// Fire, Blizzard, Thunder, Cure (our choice).
    fn default() -> Self {
        Shortcuts([Some(0), Some(1), Some(2), Some(3)])
    }
}

/// Shortcut slots: the button's name and its keyboard key.
pub const SLOTS: [(&str, &str); 4] = [("Triangle", "L"), ("Square", "K"), ("Circle", "Space"), ("Cross", "J")];

/// Root entries.
pub const ATTACK: usize = 0;
pub const MAGIC: usize = 1;
pub const DRIVE: usize = 3;

#[derive(Component)]
pub struct CmdMenu;
#[derive(Component)]
pub struct CmdRow(pub usize);
#[derive(Component)]
pub struct SpellRow(pub usize);
#[derive(Component)]
pub struct SpellPanel;
#[derive(Component)]
pub struct HpBack;
#[derive(Component)]
pub struct HpFill;
#[derive(Component)]
pub struct MpBack;
#[derive(Component)]
pub struct MpFill;
#[derive(Component)]
pub struct GaugeText;
#[derive(Component)]
pub struct AbilityPanel;
#[derive(Component)]
pub struct AbilityRow(pub usize);
#[derive(Component)]
pub struct AbilityHeader;

const ROWS: usize = 16;
const NAVY: Color = Color::srgba(0.05, 0.09, 0.22, 0.82);
const NAVY_HI: Color = Color::srgba(0.32, 0.55, 0.95, 0.95);
const DIM: Color = Color::srgba(0.05, 0.07, 0.14, 0.6);
const WHITE: Color = Color::srgb(0.95, 0.96, 1.0);
const GREY: Color = Color::srgb(0.5, 0.52, 0.6);

/// The spell shown for each element: the highest level we let Sora use.
pub fn spell_list(kit: &MenuKit, level: u8) -> Vec<usize> {
    let mut out = Vec::new();
    for e in 0..6u8 {
        let best = kit
            .spells
            .iter()
            .enumerate()
            .filter(|(_, s)| s.element == e && s.level <= level)
            .max_by_key(|(_, s)| s.level)
            .map(|(i, _)| i);
        out.extend(best);
    }
    out
}

pub fn spawn(mut commands: Commands, game: Res<Game>) {
    let Some(kit) = &game.kit else { return };
    let font = |size: f32| TextFont { font_size: size, ..default() };
    // Command menu (bottom left).
    commands
        .spawn((
            CmdMenu,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(28.0),
                bottom: Val::Px(28.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::FlexEnd,
                column_gap: Val::Px(8.0),
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn(Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(3.0), ..default() }).with_children(|col| {
                col.spawn((Text::new("COMMAND"), font(13.0), TextColor(Color::srgb(0.7, 0.8, 1.0))));
                for (i, (_, label)) in kit.root.iter().enumerate() {
                    col.spawn((
                        CmdRow(i),
                        Node { width: Val::Px(200.0), height: Val::Px(30.0), padding: UiRect::left(Val::Px(14.0)), align_items: AlignItems::Center, ..default() },
                        BackgroundColor(NAVY),
                        BorderRadius::new(Val::Px(4.0), Val::Px(15.0), Val::Px(15.0), Val::Px(4.0)),
                    ))
                    .with_children(|r| {
                        r.spawn((Text::new(label.clone()), font(18.0), TextColor(WHITE)));
                    });
                }
            });
            p.spawn((
                SpellPanel,
                Node { flex_direction: FlexDirection::Column, row_gap: Val::Px(3.0), display: Display::None, ..default() },
            ))
            .with_children(|col| {
                for i in 0..6 {
                    col.spawn((
                        SpellRow(i),
                        Node { width: Val::Px(230.0), height: Val::Px(28.0), padding: UiRect::horizontal(Val::Px(12.0)), align_items: AlignItems::Center, justify_content: JustifyContent::SpaceBetween, ..default() },
                        BackgroundColor(NAVY),
                        BorderRadius::all(Val::Px(4.0)),
                    ))
                    .with_children(|r| {
                        r.spawn((Text::new(""), font(16.0), TextColor(WHITE)));
                        r.spawn((Text::new(""), font(14.0), TextColor(Color::srgb(0.55, 0.75, 1.0))));
                    });
                }
            });
        });

    // HP / MP (bottom right).
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            right: Val::Px(28.0),
            bottom: Val::Px(30.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexEnd,
            row_gap: Val::Px(5.0),
            ..default()
        })
        .with_children(|p| {
            p.spawn((GaugeText, Text::new(""), font(14.0), TextColor(WHITE)));
            p.spawn((HpBack, Node { height: Val::Px(14.0), width: Val::Px(200.0), justify_content: JustifyContent::FlexEnd, ..default() }, BackgroundColor(Color::srgba(0.15, 0.02, 0.02, 0.85)), BorderRadius::all(Val::Px(7.0))))
                .with_children(|b| {
                    b.spawn((HpFill, Node { height: Val::Percent(100.0), width: Val::Percent(100.0), ..default() }, BackgroundColor(Color::srgb(0.25, 0.85, 0.3)), BorderRadius::all(Val::Px(7.0))));
                });
            p.spawn((MpBack, Node { height: Val::Px(10.0), width: Val::Px(200.0), justify_content: JustifyContent::FlexEnd, ..default() }, BackgroundColor(Color::srgba(0.02, 0.04, 0.15, 0.85)), BorderRadius::all(Val::Px(5.0))))
                .with_children(|b| {
                    b.spawn((MpFill, Node { height: Val::Percent(100.0), width: Val::Percent(100.0), ..default() }, BackgroundColor(Color::srgb(0.25, 0.55, 1.0)), BorderRadius::all(Val::Px(5.0))));
                });
        });

    // Ability menu (centre, hidden until opened).
    commands
        .spawn((
            AbilityPanel,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(50.0),
                top: Val::Percent(50.0),
                margin: UiRect { left: Val::Px(-280.0), top: Val::Px(-250.0), ..default() },
                width: Val::Px(560.0),
                padding: UiRect::all(Val::Px(16.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.03, 0.05, 0.12, 0.94)),
            BorderRadius::all(Val::Px(10.0)),
        ))
        .with_children(|p| {
            p.spawn((AbilityHeader, Text::new(""), font(18.0), TextColor(WHITE), Node { margin: UiRect::bottom(Val::Px(8.0)), ..default() }));
            for i in 0..ROWS {
                p.spawn((
                    AbilityRow(i),
                    Node { height: Val::Px(24.0), padding: UiRect::horizontal(Val::Px(10.0)), align_items: AlignItems::Center, justify_content: JustifyContent::SpaceBetween, ..default() },
                    BackgroundColor(DIM),
                    BorderRadius::all(Val::Px(3.0)),
                ))
                .with_children(|r| {
                    r.spawn((Text::new(""), font(15.0), TextColor(WHITE)));
                    r.spawn((Text::new(""), font(14.0), TextColor(Color::srgb(0.95, 0.8, 0.4))));
                });
            }
            p.spawn((
                Text::new("Up/Down choose   J / Enter / A: equip   U: unlimited AP   Esc / Start: close"),
                font(12.0),
                TextColor(GREY),
                Node { margin: UiRect::top(Val::Px(8.0)), ..default() },
            ));
        });
}

/// AP spent on the equipped abilities.
pub fn ap_used(game: &Game) -> u32 {
    let Some(kit) = &game.kit else { return 0 };
    kit.abilities.iter().filter(|a| game.equipped.contains(&a.id)).map(|a| a.ap as u32).sum()
}

/// Re-applies the equipped abilities to the controller.
pub fn apply_abilities(game: &mut Game) {
    let Some(kit) = &game.kit else { return };
    game.sora_abilities = abilities_from(&game.equipped, kit);
    if game.drive.form() == 0 && !game.drive.busy() {
        game.character.abilities = game.sora_abilities.clone();
    }
}

/// The ability menu: open/close, move, equip. Runs while the game is paused.
pub fn ability_menu(keys: Res<ButtonInput<KeyCode>>, mut wheel: EventReader<bevy::input::mouse::MouseWheel>, pads: Query<&Gamepad>, mut menu: ResMut<MenuState>, mut game: ResMut<Game>) {
    let scroll: f32 = wheel.read().map(|w| w.y).sum();
    let Some(n) = game.kit.as_ref().map(|k| k.abilities.len()) else { return };
    let pad = |b: GamepadButton| pads.iter().any(|p| p.just_pressed(b));
    let toggle = keys.just_pressed(KeyCode::Escape) || keys.just_pressed(KeyCode::KeyP) || pad(GamepadButton::Start);
    if toggle {
        menu.focus = if menu.focus == Focus::Abilities { Focus::Root } else { Focus::Abilities };
        return;
    }
    if menu.focus != Focus::Abilities || n == 0 {
        return;
    }
    if keys.just_pressed(KeyCode::ArrowDown) || keys.just_pressed(KeyCode::KeyS) || pad(GamepadButton::DPadDown) || scroll < 0.0 {
        menu.ability = (menu.ability + 1) % n;
    }
    if keys.just_pressed(KeyCode::ArrowUp) || keys.just_pressed(KeyCode::KeyW) || pad(GamepadButton::DPadUp) || scroll > 0.0 {
        menu.ability = (menu.ability + n - 1) % n;
    }
    if keys.just_pressed(KeyCode::KeyU) {
        game.unlimited_ap = !game.unlimited_ap;
    }
    if menu.ability < menu.scroll {
        menu.scroll = menu.ability;
    } else if menu.ability >= menu.scroll + ROWS {
        menu.scroll = menu.ability + 1 - ROWS;
    }
    if keys.just_pressed(KeyCode::KeyJ) || keys.just_pressed(KeyCode::Enter) || pad(GamepadButton::South) {
        let Some(info) = game.kit.as_ref().and_then(|k| k.abilities.get(menu.ability)).cloned() else { return };
        if let Some(i) = game.equipped.iter().position(|&id| id == info.id) {
            game.equipped.remove(i);
        } else if game.unlimited_ap || ap_used(&game) + info.ap as u32 <= game.ap_max {
            game.equipped.push(info.id);
        }
        apply_abilities(&mut game);
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn draw(
    game: Res<Game>,
    menu: Res<MenuState>,
    mut rows: Query<(&CmdRow, &mut BackgroundColor, &Children), (Without<SpellRow>, Without<AbilityRow>)>,
    mut spell_panel: Query<&mut Node, (With<SpellPanel>, Without<AbilityPanel>, Without<HpBack>, Without<MpBack>, Without<HpFill>, Without<MpFill>)>,
    mut spell_rows: Query<(&SpellRow, &mut BackgroundColor, &Children), (Without<CmdRow>, Without<AbilityRow>)>,
    mut ability_panel: Query<&mut Node, (With<AbilityPanel>, Without<SpellPanel>, Without<HpBack>, Without<MpBack>, Without<HpFill>, Without<MpFill>)>,
    mut ability_rows: Query<(&AbilityRow, &mut BackgroundColor, &Children), (Without<CmdRow>, Without<SpellRow>)>,
    mut header: Query<&mut Text, (With<AbilityHeader>, Without<GaugeText>)>,
    mut gauge_text: Query<&mut Text, (With<GaugeText>, Without<AbilityHeader>)>,
    mut hp_back: Query<&mut Node, (With<HpBack>, Without<SpellPanel>, Without<AbilityPanel>, Without<MpBack>, Without<HpFill>, Without<MpFill>)>,
    mut hp_fill: Query<&mut Node, (With<HpFill>, Without<SpellPanel>, Without<AbilityPanel>, Without<HpBack>, Without<MpBack>, Without<MpFill>)>,
    mut mp_back: Query<&mut Node, (With<MpBack>, Without<SpellPanel>, Without<AbilityPanel>, Without<HpBack>, Without<HpFill>, Without<MpFill>)>,
    mut mp_fill: Query<(&mut Node, &mut BackgroundColor), (With<MpFill>, Without<SpellPanel>, Without<AbilityPanel>, Without<HpBack>, Without<HpFill>, Without<MpBack>, Without<CmdRow>, Without<SpellRow>, Without<AbilityRow>)>,
    mut texts: Query<(&mut Text, &mut TextColor), (Without<AbilityHeader>, Without<GaugeText>)>,
) {
    let Some(kit) = &game.kit else { return };
    // Command menu.
    let in_magic = menu.focus == Focus::Magic;
    for (row, mut bg, children) in &mut rows {
        let selected = row.0 == menu.root;
        *bg = BackgroundColor(if selected && !in_magic { NAVY_HI } else if selected { Color::srgba(0.2, 0.32, 0.6, 0.9) } else { NAVY });
        let enabled = row.0 == ATTACK || row.0 == MAGIC;
        for c in children.iter() {
            if let Ok((_, mut col)) = texts.get_mut(*c) {
                *col = TextColor(if enabled { WHITE } else { GREY });
            }
        }
    }
    if let Ok(mut n) = spell_panel.get_single_mut() {
        n.display = if in_magic { Display::Flex } else { Display::None };
    }
    let spells = spell_list(kit, game.magic_level);
    for (row, mut bg, children) in &mut spell_rows {
        let spell = spells.get(row.0).and_then(|&i| kit.spells.get(i));
        *bg = BackgroundColor(if row.0 == menu.magic { NAVY_HI } else { NAVY });
        let affordable = spell.map_or(false, |s| game.can_afford(s.mp_cost));
        for (k, c) in children.iter().enumerate() {
            if let Ok((mut t, mut col)) = texts.get_mut(*c) {
                *t = Text::new(match (spell, k) {
                    (Some(s), 0) => s.name.clone(),
                    (Some(s), _) if s.mp_cost == 255 => "all MP".into(),
                    (Some(s), _) => format!("{} MP", s.mp_cost),
                    _ => String::new(),
                });
                if k == 0 {
                    *col = TextColor(if affordable { WHITE } else { GREY });
                }
            }
        }
    }

    // Gauges: KH2's bars grow with the maximum.
    let (hp, max_hp) = (game.sora.hp as f32, game.sora.max_hp.max(1) as f32);
    if let Ok(mut n) = hp_back.get_single_mut() {
        n.width = Val::Px((max_hp * 3.0).clamp(120.0, 520.0));
    }
    if let Ok(mut n) = hp_fill.get_single_mut() {
        n.width = Val::Percent(100.0 * hp / max_hp);
    }
    if let Ok(mut n) = mp_back.get_single_mut() {
        n.width = Val::Px((game.max_mp * 2.0).clamp(100.0, 400.0));
    }
    if let Ok((mut n, mut bg)) = mp_fill.get_single_mut() {
        match game.mp_charge {
            Some(c) => {
                n.width = Val::Percent(100.0 * c);
                *bg = BackgroundColor(Color::srgb(0.95, 0.45, 0.75));
            }
            None => {
                n.width = Val::Percent(100.0 * game.mp / game.max_mp.max(1.0));
                *bg = BackgroundColor(Color::srgb(0.25, 0.55, 1.0));
            }
        }
    }
    if let Ok(mut t) = gauge_text.get_single_mut() {
        let mp = match game.mp_charge {
            Some(_) => "MP CHARGE".to_string(),
            None => format!("MP {:.0}/{:.0}", game.mp, game.max_mp),
        };
        *t = Text::new(format!("HP {}/{}   {}", game.sora.hp, game.sora.max_hp, mp));
    }

    // Ability menu.
    let open = menu.focus == Focus::Abilities;
    if let Ok(mut n) = ability_panel.get_single_mut() {
        n.display = if open { Display::Flex } else { Display::None };
    }
    if !open {
        return;
    }
    if let Ok(mut t) = header.get_single_mut() {
        let cap = if game.unlimited_ap { "unlimited".to_string() } else { game.ap_max.to_string() };
        *t = Text::new(format!("ABILITIES      AP {} / {}", ap_used(&game), cap));
    }
    for (row, mut bg, children) in &mut ability_rows {
        let i = menu.scroll + row.0;
        let info = kit.abilities.get(i);
        let on = info.map_or(false, |a| game.equipped.contains(&a.id));
        *bg = BackgroundColor(if i == menu.ability { NAVY_HI } else if info.is_some() { DIM } else { Color::NONE });
        for (k, c) in children.iter().enumerate() {
            if let Ok((mut t, mut col)) = texts.get_mut(*c) {
                *t = Text::new(match (info, k) {
                    (Some(a), 0) => format!("{} {}", if on { "[x]" } else { "[ ]" }, a.name),
                    (Some(a), _) => format!("{} AP", a.ap),
                    _ => String::new(),
                });
                if k == 0 {
                    *col = TextColor(if on { WHITE } else { Color::srgb(0.65, 0.68, 0.78) });
                }
            }
        }
    }
}

#[derive(Component)]
pub struct DrivePanel;
#[derive(Component)]
pub struct DriveRow(pub usize);
#[derive(Component)]
pub struct DriveBack;
#[derive(Component)]
pub struct DriveFill;
#[derive(Component)]
pub struct DriveText;
#[derive(Component)]
pub struct Tint;

/// Drive pieces: the Drive submenu (next to the command menu), the Drive /
/// form gauge under MP, and the screen tint of a form change.
pub fn spawn_drive(mut commands: Commands, game: Res<Game>) {
    let Some(kit) = &game.kit else { return };
    let font = |size: f32| TextFont { font_size: size, ..default() };
    commands.spawn((
        Tint,
        Node { position_type: PositionType::Absolute, left: Val::Px(0.0), top: Val::Px(0.0), width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
        BackgroundColor(Color::NONE),
        GlobalZIndex(-1),
    ));
    commands
        .spawn((
            DrivePanel,
            Node { position_type: PositionType::Absolute, left: Val::Px(236.0), bottom: Val::Px(28.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(3.0), display: Display::None, ..default() },
        ))
        .with_children(|col| {
            for i in 0..kit.drives.len() {
                col.spawn((
                    DriveRow(i),
                    Node { width: Val::Px(230.0), height: Val::Px(28.0), padding: UiRect::horizontal(Val::Px(12.0)), align_items: AlignItems::Center, justify_content: JustifyContent::SpaceBetween, ..default() },
                    BackgroundColor(NAVY),
                    BorderRadius::all(Val::Px(4.0)),
                ))
                .with_children(|r| {
                    r.spawn((Text::new(kit.drives[i].name.clone()), font(16.0), TextColor(WHITE)));
                    r.spawn((Text::new(format!("{} bars", kit.drives[i].cost)), font(14.0), TextColor(Color::srgb(0.95, 0.8, 0.4))));
                });
            }
        });
    commands
        .spawn(Node { position_type: PositionType::Absolute, right: Val::Px(28.0), bottom: Val::Px(8.0), flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: Val::Px(8.0), ..default() })
        .with_children(|p| {
            p.spawn((DriveText, Text::new(""), font(13.0), TextColor(Color::srgb(0.95, 0.85, 0.5))));
            p.spawn((DriveBack, Node { height: Val::Px(9.0), width: Val::Px(200.0), justify_content: JustifyContent::FlexStart, ..default() }, BackgroundColor(Color::srgba(0.12, 0.1, 0.02, 0.85)), BorderRadius::all(Val::Px(4.0))))
                .with_children(|b| {
                    b.spawn((DriveFill, Node { height: Val::Percent(100.0), width: Val::Percent(0.0), ..default() }, BackgroundColor(Color::srgb(0.95, 0.8, 0.2)), BorderRadius::all(Val::Px(4.0))));
                });
        });
}

/// Form colours on the HUD (ours).
fn form_color(f: u8) -> Color {
    match f {
        1 => Color::srgb(0.9, 0.2, 0.2),
        2 => Color::srgb(0.3, 0.5, 1.0),
        3 => Color::srgb(0.9, 0.5, 0.3),
        4 => Color::srgb(0.95, 0.85, 0.2),
        5 => Color::srgb(0.9, 0.92, 1.0),
        6 => Color::srgb(0.35, 0.3, 0.45),
        _ => Color::srgb(0.95, 0.8, 0.2),
    }
}

#[allow(clippy::type_complexity)]
pub fn draw_drive(
    game: Res<Game>,
    menu: Res<MenuState>,
    mut panel: Query<&mut Node, (With<DrivePanel>, Without<DriveBack>, Without<DriveFill>)>,
    mut rows: Query<(&DriveRow, &mut BackgroundColor, &Children), (Without<CmdRow>, Without<DriveFill>, Without<Tint>)>,
    cmd_rows: Query<(&CmdRow, &Children)>,
    mut back: Query<&mut Node, (With<DriveBack>, Without<DrivePanel>, Without<DriveFill>)>,
    mut fill: Query<(&mut Node, &mut BackgroundColor), (With<DriveFill>, Without<DrivePanel>, Without<DriveBack>, Without<DriveRow>, Without<Tint>)>,
    mut tint: Query<&mut BackgroundColor, (With<Tint>, Without<DriveRow>, Without<DriveFill>)>,
    mut label: Query<&mut Text, (With<DriveText>, Without<DriveRow>)>,
    mut texts: Query<(&mut Text, &mut TextColor), Without<DriveText>>,
) {
    let Some(kit) = &game.kit else { return };
    let d = &game.drive;
    let form = d.form();
    if let Ok(mut n) = panel.get_single_mut() {
        n.display = if menu.focus == Focus::Drive { Display::Flex } else { Display::None };
    }
    for (row, mut bg, children) in &mut rows {
        *bg = BackgroundColor(if row.0 == menu.drive { NAVY_HI } else { NAVY });
        let ok = kit.drives.get(row.0).is_some_and(|x| crate::drive::available(&game, x.form));
        if let Some(&c) = children.first() {
            if let Ok((_, mut col)) = texts.get_mut(c) {
                *col = TextColor(if ok { WHITE } else { GREY });
            }
        }
    }
    // The command menu's Drive row reads Revert in a form; Magic greys out
    // where forms can't cast.
    for (row, children) in &cmd_rows {
        let Some(&c) = children.first() else { continue };
        let Ok((mut t, mut col)) = texts.get_mut(c) else { continue };
        if row.0 == DRIVE {
            let want = if form != 0 { kit.revert.clone() } else { kit.root.get(DRIVE).map_or("Drive".into(), |r| r.1.clone()) };
            if t.0 != want {
                *t = Text::new(want);
            }
            let any = kit.drives.iter().any(|x| crate::drive::available(&game, x.form));
            *col = TextColor(if (form != 0 && !d.busy()) || any { WHITE } else { GREY });
        } else if row.0 == MAGIC && !crate::drive::magic_allowed(form) {
            *col = TextColor(GREY);
        }
    }
    // Gauge: bars plus the current bar's points; in a form, the form gauge.
    let (frac, text, color, width) = match d.drive.gauge {
        Some(g) => (g.left / g.max.max(1.0), format!("{}  {:.1}", crate::drive::form_name(form).to_uppercase(), g.bars(&d.drive.params)), form_color(form), g.max / d.drive.params.bar_frames * 24.0),
        None => {
            let v = d.drive.bars as f32 + if d.drive.full() { 0.0 } else { d.drive.points as f32 / 100.0 };
            let anti = if d.drive.anti_rate() > 0 { format!("  (Anti {}%)", d.drive.anti_rate()) } else { String::new() };
            ((v / d.drive.max.max(1) as f32).min(1.0), format!("DRIVE {}{}", d.drive.bars, anti), form_color(0), d.drive.max as f32 * 24.0)
        }
    };
    if let Ok(mut n) = back.get_single_mut() {
        n.width = Val::Px(width.clamp(60.0, 360.0));
    }
    if let Ok((mut n, mut bg)) = fill.get_single_mut() {
        n.width = Val::Percent(100.0 * frac);
        *bg = BackgroundColor(color);
    }
    if let Ok(mut t) = label.get_single_mut() {
        if t.0 != text {
            *t = Text::new(text);
        }
    }
    if let Ok(mut bg) = tint.get_single_mut() {
        *bg = BackgroundColor(Color::srgba(0.19, 0.19, 0.19, d.tint));
    }
}

#[derive(Component)]
pub struct RcBox;
#[derive(Component)]
pub struct RcText;

/// The reaction-command slot, above the command menu (KH2's green box).
pub fn spawn_rc(mut commands: Commands) {
    commands
        .spawn((
            RcBox,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(28.0),
                bottom: Val::Px(186.0),
                width: Val::Px(200.0),
                height: Val::Px(32.0),
                padding: UiRect::left(Val::Px(14.0)),
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0.1, 0.55, 0.25, 0.92)),
            BorderRadius::new(Val::Px(4.0), Val::Px(15.0), Val::Px(15.0), Val::Px(4.0)),
        ))
        .with_children(|r| {
            r.spawn((RcText, Text::new(""), TextFont { font_size: 18.0, ..default() }, TextColor(WHITE)));
        });
}

pub fn draw_rc(game: Res<Game>, mut boxes: Query<(&mut Node, &mut BackgroundColor), With<RcBox>>, mut texts: Query<&mut Text, With<RcText>>) {
    let offer = game.rc.offer;
    let name = offer.and_then(|o| game.kit.as_ref().and_then(|k| k.commands.get(&o.cmd)).map(|(_, n)| n.clone()));
    if let Ok((mut n, mut bg)) = boxes.get_single_mut() {
        n.display = if name.is_some() { Display::Flex } else { Display::None };
        let grey = offer.is_some_and(|o| o.greyed);
        *bg = BackgroundColor(if grey { Color::srgba(0.25, 0.3, 0.27, 0.85) } else { Color::srgba(0.1, 0.55, 0.25, 0.92) });
    }
    if let (Ok(mut t), Some(name)) = (texts.get_single_mut(), name) {
        let want = format!("[L]  {name}");
        if t.0 != want {
            *t = Text::new(want);
        }
    }
}

#[derive(Component)]
pub struct ShortcutPanel;
#[derive(Component)]
pub struct ShortcutRow(pub usize);

/// The shortcut menu: while L1 / Ctrl is held it takes the command menu's
/// place, one row per face button (KH2's layout; the look is ours).
pub fn spawn_shortcuts(mut commands: Commands, game: Res<Game>) {
    if game.kit.is_none() {
        return;
    }
    let font = |size: f32| TextFont { font_size: size, ..default() };
    commands
        .spawn((
            ShortcutPanel,
            Node { position_type: PositionType::Absolute, left: Val::Px(28.0), bottom: Val::Px(28.0), flex_direction: FlexDirection::Column, row_gap: Val::Px(3.0), display: Display::None, ..default() },
        ))
        .with_children(|col| {
            col.spawn((Text::new("SHORTCUTS"), font(13.0), TextColor(Color::srgb(0.7, 0.8, 1.0))));
            for (i, (button, key)) in SLOTS.iter().enumerate() {
                col.spawn((
                    ShortcutRow(i),
                    Node { width: Val::Px(300.0), height: Val::Px(30.0), padding: UiRect::horizontal(Val::Px(12.0)), align_items: AlignItems::Center, column_gap: Val::Px(10.0), ..default() },
                    BackgroundColor(NAVY),
                    BorderRadius::new(Val::Px(4.0), Val::Px(15.0), Val::Px(15.0), Val::Px(4.0)),
                ))
                .with_children(|r| {
                    r.spawn((Text::new(format!("{button} [{key}]")), font(13.0), TextColor(Color::srgb(0.95, 0.8, 0.4)), Node { width: Val::Px(110.0), ..default() }));
                    r.spawn((Text::new(""), font(17.0), TextColor(WHITE), Node { flex_grow: 1.0, ..default() }));
                    r.spawn((Text::new(""), font(14.0), TextColor(Color::srgb(0.55, 0.75, 1.0))));
                });
            }
            col.spawn((Text::new("In the Magic list: hold and press to assign"), font(12.0), TextColor(GREY)));
        });
}

#[allow(clippy::type_complexity)]
pub fn draw_shortcuts(
    game: Res<Game>,
    menu: Res<MenuState>,
    mut panel: Query<&mut Node, (With<ShortcutPanel>, Without<CmdMenu>)>,
    mut cmd: Query<&mut Node, (With<CmdMenu>, Without<ShortcutPanel>)>,
    rows: Query<(&ShortcutRow, &Children)>,
    mut texts: Query<(&mut Text, &mut TextColor)>,
) {
    let Some(kit) = &game.kit else { return };
    let open = menu.shortcuts_open && menu.focus != Focus::Abilities;
    if let Ok(mut n) = panel.get_single_mut() {
        n.display = if open { Display::Flex } else { Display::None };
        // Above the command menu and the Magic list while assigning.
        n.bottom = Val::Px(if menu.focus == Focus::Magic { 236.0 } else { 28.0 });
    }
    // In the Magic list the list stays up, so a spell can be assigned.
    if let Ok(mut n) = cmd.get_single_mut() {
        n.display = if open && menu.focus != Focus::Magic { Display::None } else { Display::Flex };
    }
    if !open {
        return;
    }
    let spells = spell_list(kit, game.magic_level);
    let usable = crate::drive::magic_allowed(game.drive.form()) && !game.drive.busy();
    for (row, children) in &rows {
        let spell = menu.shortcuts.0[row.0].and_then(|p| spells.get(p)).and_then(|&i| kit.spells.get(i));
        for (k, c) in children.iter().enumerate().skip(1) {
            if let Ok((mut t, mut col)) = texts.get_mut(*c) {
                *t = Text::new(match (spell, k) {
                    (Some(s), 1) => s.name.clone(),
                    (None, 1) => "-".into(),
                    (Some(s), _) if s.mp_cost == 255 => "all MP".into(),
                    (Some(s), _) => format!("{} MP", s.mp_cost),
                    _ => String::new(),
                });
                if k == 1 {
                    let ok = usable && spell.is_some_and(|s| game.can_afford(s.mp_cost));
                    *col = TextColor(if ok { WHITE } else { GREY });
                }
            }
        }
    }
}
