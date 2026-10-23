//! KH2FM movement and combat playground.
//!
//! Primitive shapes only: no textures, meshes or sounds are loaded. Sora's
//! numbers (speeds, jump heights, growth abilities, every move's length,
//! root motion, hitbox and combo windows, combo table, attack power) and his
//! skeleton and animations are read from the user's own install at startup
//! through `kh2-data`. He is drawn as a mannequin of capsules on his bones.
//!
//! Run:  kh2-playground --game "D:\epic\KH_1.5_2.5\KINGDOM HEARTS II FINAL MIX.exe"
//! (or set KH2_GAME). See the HUD for controls.

mod drive;
mod foes;
mod hud;
mod magic;
mod rc;
mod rig;
mod synthetic;
mod world;

use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, PI};

use bevy::input::mouse::MouseMotion;
use bevy::math::Isometry3d;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use character_core::{Abilities, Button as Btn, Character, Env, Input as CInput, Output, StateName, TargetInfo};
use combat_core::{first_overlap, resolve_hit, AttackVolume, Combatant, DamageRules, HitRecord, Shape as CShape, ShapeKind, Stats, Team};
use kh2_data::sora::BladeSphere;
use kh2_data::{Install, Source};

use rig::{Paint, Part, Piece, RigAnim, Shape};
use world::Arena;

/// KH2 units (about 1 cm) to Bevy meters.
const SCALE: f32 = 0.01;
const SORA_RADIUS: f32 = 35.0;
const SORA_HEIGHT: f32 = 150.0;
const DUMMY_RADIUS: f32 = 50.0;
const DUMMY_HEIGHT: f32 = 110.0;
/// The Soldier's ENMP id (33).
const DUMMY_ENEMY: u16 = 33;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    // Candidates in order: --game, KH2_GAME, the default Epic install path.
    let mut candidates: Vec<String> = Vec::new();
    candidates.extend(arg("--game"));
    candidates.extend(std::env::var("KH2_GAME").ok());
    candidates.push(r"D:\epic\KH_1.5_2.5\KINGDOM HEARTS II FINAL MIX.exe".to_string());
    let level: usize = arg("--level").and_then(|v| v.parse().ok()).unwrap_or(30).clamp(1, 99);
    let dummy_level: u16 = arg("--dummy-level").and_then(|v| v.parse().ok()).unwrap_or(level as u16).clamp(1, 99);
    let source = if args.iter().any(|a| a == "--backup") { Source::Backup } else { Source::Live };
    let sora_hp: u32 = arg("--hp").and_then(|v| v.parse().ok()).unwrap_or(60);
    let gravity: Option<f32> = arg("--gravity").and_then(|v| v.parse().ok());
    let screenshot_after: Option<u32> = arg("--screenshot").and_then(|v| v.parse().ok());
    let autoplay = args.iter().any(|a| a == "--autoplay");
    // --scene idle|run|combo|jump|roll|glide: scripted input for screenshots.
    let scene = arg("--scene");
    // The game's field camera (row 0); --cam-distance changes its default.
    let mut cam_params = kh2_camera::Params::FIELD;
    if let Some(d) = arg("--cam-distance").and_then(|v| v.parse::<f32>().ok()) {
        cam_params.dist_default = d;
        cam_params.dist_max = cam_params.dist_max.max(d);
    }

    let synthetic = args.iter().any(|a| a == "--synthetic");
    let mut tried = Vec::new();
    let mut used: Option<Install> = None;
    let loaded = if synthetic {
        Ok(synthetic::data())
    } else if let Some(dir) = arg("--data-dir") {
        // Research: files already extracted into a local folder.
        kh2_data::sora::load_sora_dir(std::path::Path::new(&dir))
    } else {
        let mut result = Err(kh2_data::Error::Format("no path given".into()));
        for c in &candidates {
            match Install::locate(c).and_then(|i| i.load_sora(source).map(|d| (i, d))) {
                Ok((i, d)) => {
                    used = Some(i);
                    if c != &candidates[0] {
                        eprintln!("note: loaded KH2 data from fallback path {c}");
                    }
                    result = Ok(d);
                    break;
                }
                Err(e) => {
                    tried.push(format!("  {c}\n    -> {e}"));
                    result = Err(e);
                }
            }
        }
        result
    };
    // The Soldiers: their own files from the same place.
    let enemy = match (&loaded, synthetic) {
        (Ok(d), false) => {
            let r = match (&used, arg("--data-dir")) {
                (Some(i), _) => i.load_enemy(source, kh2_data::enemy::SOLDIER, &d.combat),
                (None, Some(dir)) => kh2_data::enemy::load_enemy_dir(std::path::Path::new(&dir), kh2_data::enemy::SOLDIER, &d.combat),
                _ => Err(kh2_data::Error::Format("no install".into())),
            };
            match r {
                Ok(e) => Some(e),
                Err(e) => {
                    eprintln!("note: the Soldier could not be loaded ({e}); using column dummies");
                    None
                }
            }
        }
        _ => None,
    };
    if let Ok(Some(e)) = loaded.as_ref().map(|d| d.rig_error.as_ref()) {
        eprintln!("note: Sora's skeleton/animations could not be loaded ({e}); using the capsule");
    }
    let loaded = loaded.map(|mut d| {
        if let Some(g) = gravity {
            d.character.params.gravity = g;
        }
        d
    });

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "KH2FM movement & combat (kh2-combat playground)".into(),
            resolution: WindowResolution::new(1280.0, 720.0),
            ..default()
        }),
        ..default()
    }))
    .insert_resource(ClearColor(Color::srgb(0.08, 0.09, 0.12)))
    .insert_resource(AmbientLight { color: Color::WHITE, brightness: 300.0 })
    .insert_resource(Time::<Fixed>::from_hz(60.0))
    .insert_resource(Latch::default())
    .insert_resource(CameraRig { yaw: 0.0, params: cam_params, cam: None, auto: !args.iter().any(|a| a == "--manual-camera") })
    .insert_resource(Debug {
        show_boxes: true,
        show_hurt: args.iter().any(|a| a == "--show-hurt"),
        show_skeleton: args.iter().any(|a| a == "--show-skeleton"),
        show_other: args.iter().any(|a| a == "--show-other"),
        log: Vec::new(), last: None })
    .insert_resource(Shot { after_frames: screenshot_after, frames: 0, pause_at: arg("--pause-at").and_then(|v| v.parse().ok()) })
    .insert_resource(Autoplay { on: autoplay || scene.is_some(), frame: 0, scene: scene.unwrap_or_default(), trace: args.iter().any(|a| a == "--trace") })
    .add_systems(Startup, (world::spawn_arena, spawn_camera_and_light));

    match loaded {
        Ok(data) => {
            let lvup = data.combat.battle.lvup.first().and_then(|c| c.get(level - 1)).copied();
            let stats = match lvup {
                // Max HP depends on story bonuses (not read yet): 60 by default.
                Some(l) => Stats { hp: sora_hp, strength: l.strength as u16, magic: l.magic as u16, defense: l.defense as u16 },
                None => Stats { hp: 100, strength: 10, magic: 10, defense: 5 },
            };
            // The enemy's capture motions (reaction commands) on Sora.
            let mut data = data;
            if let Some(e) = &enemy {
                rc::register_captures(&mut data, e, "001");
            }
            let mut character = data.character.clone();
            character.abilities = Abilities::max_growth();
            // Glide on the action button: Square on a PlayStation pad, K /
            // Left Shift on the keyboard (user's choice; jump still Aerial
            // Dodges).
            character.glide_button = character_core::GlideButton::Action;
            let dummy_template = data.combat.enemy(DUMMY_ENEMY, Some(dummy_level));
            // Menus, abilities and magic from the install.
            let action_ids: Vec<u16> = data.character.moveset.combo_rows.iter().map(|r| r.ability).filter(|a| *a != 0).collect();
            let kit = match (&used, arg("--data-dir"), synthetic) {
                (_, _, true) => None,
                (Some(i), _, _) => i.load_kit(source, &data.combat.battle, &action_ids).map_err(|e| eprintln!("note: menus not loaded ({e})")).ok(),
                (None, Some(dir), _) => kh2_data::kit::load_kit_dir(std::path::Path::new(&dir), &data.combat.battle, &action_ids).map_err(|e| eprintln!("note: menus not loaded ({e})")).ok(),
                _ => None,
            };
            let anim = data.rig.clone().filter(|_| !args.iter().any(|a| a == "--capsule")).map(RigAnim::new);
            // The Drive Forms: each one's own object from the same place.
            let mut kits = HashMap::new();
            if !synthetic {
                for spec in kh2_data::sora::FORMS {
                    let r = match (&used, arg("--data-dir")) {
                        (Some(i), _) => i.load_player(source, spec),
                        (None, Some(d)) => kh2_data::sora::load_player_dir(std::path::Path::new(&d), spec),
                        _ => continue,
                    };
                    match r {
                        Ok(mut d) => {
                            if let Some(e) = &enemy {
                                rc::register_captures(&mut d, e, if spec.form == 3 { "027" } else { "001" });
                            }
                            kits.insert(spec.form, std::sync::Arc::new(d));
                        }
                        Err(e) => eprintln!("note: {} not loaded ({e})", spec.model),
                    }
                }
            }
            let drive_max: u8 = arg("--drive-max").and_then(|v| v.parse().ok()).unwrap_or(9);
            let form_level: u8 = arg("--form-level").and_then(|v| v.parse().ok()).unwrap_or(7);
            let drive_state = drive::DriveState::new(drive_max, form_level, Some(std::sync::Arc::new(data.clone())), kits);
            // Every growth ability at MAX, Guard and Aerial Recovery.
            let equipped: Vec<u16> = kit
                .as_ref()
                .map(|k| {
                    use kh2_data::kit::Effect;
                    k.abilities.iter().filter(|a| matches!(a.effect, Effect::Growth(_, 4) | Effect::Guard | Effect::AerialRecovery)).map(|a| a.id).collect()
                })
                .unwrap_or_default();
            if let Some(k) = &kit {
                character.abilities = kh2_data::kit::abilities_from(&equipped, k);
            }
            // The spells' own files (behaviour scripts and effects).
            let spells = kit.as_ref().map(|k| {
                let dir = arg("--data-dir");
                magic::Spells::load(k, data.magi.clone(), |name| match (&used, &dir) {
                    (Some(i), _) => i.read_any(source, name).ok(),
                    (None, Some(d)) => std::fs::read(std::path::Path::new(d).join(name.rsplit('/').next().unwrap_or(name))).ok(),
                    _ => None,
                })
            });
            let sora_abilities = character.abilities.clone();
            app.insert_resource(Game {
                anim,
                character,
                pos: [0.0, 0.0, -400.0],
                vel_display: [0.0; 3],
                last: None,
                sora: Combatant::new(1, Team::Player, stats),
                level,
                battle: std::sync::Arc::new(data.combat.battle.clone()),
                sora_record: HitRecord::new(),
                live_swings: Default::default(),
                spells,
                cast_now: None,
                blade: data.blade_groups.clone(),
                rules: DamageRules::default(),
                frame: 0,
                grounded: true,
                ground_y: 0.0,
                ground_normal: Vec3::Y,
                body: world::body_of(data.rig.as_ref().and_then(|r| kh2_data::sora::bg_shape(&r.body_shapes)), true),
                dummy_template,
                weight: data.weight,
                enemy_count: arg("--enemies").and_then(|v| v.parse().ok()).unwrap_or(1),
                enemies_away: false,
                dummy_level,
                combo_plus: 0,
                volumes: HashMap::new(),
                live_attacks: Vec::new(),
                bg_flags: data.combat.battle.atkp.iter().filter(|a| a.sub() == 0).map(|a| (a.id, (a.flags & 1 != 0, a.floor_check))).collect(),
                connected: Default::default(),
                clinked: Default::default(),
                sparks: Vec::new(),
                atkp_raw: {
                    let mut m = HashMap::new();
                    for a in data.combat.battle.atkp.iter().filter(|a| a.sub() == 0) {
                        m.entry(a.id).or_insert_with(|| a.clone());
                    }
                    m
                },
                foe_kit: enemy.map(foes::FoeKit::new),
                sora_hits: Vec::new(),
                sora_guards: Vec::new(),
                sora_hit_by: Default::default(),
                ap_max: kit.as_ref().and_then(|k| k.ap_by_level.get(level - 1)).map_or(0, |&a| a as u32),
                equipped,
                unlimited_ap: false,
                mp: kit.as_ref().map_or(100.0, |k| k.mp as f32),
                max_mp: kit.as_ref().map_or(100.0, |k| k.mp as f32),
                mp_charge: None,
                magic_level: arg("--magic-level").and_then(|v| v.parse().ok()).unwrap_or(3),
                sora_abilities,
                drive: drive_state,
                rc: rc::RcState::default(),
                casting: None,
                kit,
            })
            .insert_resource(hud::MenuState::default())
            .add_systems(Startup, (spawn_player, spawn_dummies, hud::spawn, hud::spawn_drive, hud::spawn_shortcuts, hud::spawn_rc, magic::spawn_pool))
            .add_systems(Update, (latch_input, camera_control, sync_visuals, sync_segments, foes::sync_pieces, draw_debug, draw_skeleton, hud, abilities_keys, screenshot))
            .add_systems(Update, (hud::ability_menu, hud::draw, hud::draw_drive.after(hud::draw), hud::draw_shortcuts.after(hud::draw), hud::draw_rc, rc::draw_mark, magic::draw, rebuild_player))
            .add_systems(FixedUpdate, (simulate, camera_step).chain());
        }
        Err(_) => {
            let msg = format!(
                "Could not load KH2 data. Tried:\n{}\n\nRun (one line):\n  cargo run --release -p kh2-bevy -- --game \"D:\\epic\\KH_1.5_2.5\\KINGDOM HEARTS II FINAL MIX.exe\"\nor add --synthetic to try the controls with made-up numbers.\nThis playground reads all of Sora's numbers from your own install; it ships none.",
                tried.join("\n")
            );
            eprintln!("{msg}");
            app.insert_resource(ErrorText(msg)).add_systems(Startup, show_error);
        }
    }
    app.run();
}

#[derive(Resource)]
struct ErrorText(String);

fn show_error(mut commands: Commands, e: Res<ErrorText>) {
    commands.spawn((
        Text::new(e.0.clone()),
        TextFont { font_size: 18.0, ..default() },
        TextColor(Color::srgb(1.0, 0.6, 0.6)),
        Node { position_type: PositionType::Absolute, top: Val::Px(24.0), left: Val::Px(24.0), ..default() },
    ));
}

#[derive(Resource)]
struct Game {
    /// Skeleton animation (absent with --synthetic / --capsule).
    anim: Option<RigAnim>,
    character: Character,
    pos: [f32; 3],
    vel_display: [f32; 3],
    last: Option<Output>,
    sora: Combatant,
    level: usize,
    /// Every ATKP row: an attack's row is picked when it hits, by how many
    /// hits in a row the target has taken from the same attack group.
    battle: std::sync::Arc<kh2_formats::battle::Battle>,
    /// Sora's hit record (what has hit him, and the group timer).
    sora_record: HitRecord,
    /// Attack windows live last frame (Sora's and the Soldiers'), to forget
    /// them in the hit records when they end.
    live_swings: std::collections::HashSet<u64>,
    /// Spells in flight (absent without the install's menus).
    spells: Option<magic::Spells>,
    /// A cast started this frame: kit spell index and whether the
    /// finisher motion was used.
    cast_now: Option<(usize, bool)>,
    blade: HashMap<u8, Vec<BladeSphere>>,
    rules: DamageRules,
    frame: u64,
    grounded: bool,
    ground_y: f32,
    /// The ground's normal under Sora (grounded moves follow it).
    ground_normal: Vec3,
    /// Sora's map-collision shape (P_EX100's type-4 sphere).
    body: map_collision::Body,
    /// Normal Sora's abilities (a form uses its own, plus his support ones).
    sora_abilities: Abilities,
    /// Drive gauge, form gauge, the form objects and a change in progress.
    drive: drive::DriveState,
    /// The reaction command offered, and one in progress.
    rc: rc::RcState,
    dummy_template: Option<Combatant>,
    enemy_count: usize,
    /// F6: the enemies are sent away (out of combat until brought back).
    enemies_away: bool,
    /// Sora's objentry weight.
    weight: f32,
    dummy_level: u16,
    combo_plus: u8,
    /// Live attack windows' shapes from the previous frame (for the sweep),
    /// by window id.
    volumes: HashMap<(u64, u8), AttackVolume>,
    /// This frame's attack shapes including sweep copies (for drawing).
    live_attacks: Vec<CShape>,
    /// ATKP id -> (strikes walls, floor-check ray length).
    bg_flags: HashMap<u16, (bool, u8)>,
    /// Live windows that already connected with a target / bounced off a wall.
    connected: std::collections::HashSet<u64>,
    clinked: std::collections::HashSet<u64>,
    /// Wall-hit sparks: position and frames left.
    sparks: Vec<(Vec3, u32)>,
    /// Every ATKP row (sub 0) by id, for clash and reaction rules.
    atkp_raw: HashMap<u16, kh2_formats::battle::Atkp>,
    /// Soldiers' data (absent without an install or with --synthetic).
    foe_kit: Option<foes::FoeKit>,
    /// Sora's live attack / guard windows and their shapes this frame.
    sora_hits: Vec<(character_core::ActiveHitbox, Vec<CShape>)>,
    sora_guards: Vec<(character_core::ActiveHitbox, Vec<CShape>)>,
    /// (Soldier id, swing) pairs that already hit or clashed with Sora.
    sora_hit_by: std::collections::HashSet<(u32, u64)>,
    /// Menus, abilities and magic (absent with --synthetic).
    kit: Option<kh2_data::kit::MenuKit>,
    /// Equipped ability item ids.
    equipped: Vec<u16>,
    ap_max: u32,
    unlimited_ap: bool,
    mp: f32,
    max_mp: f32,
    /// MP Charge progress 0..1 once MP has run out.
    mp_charge: Option<f32>,
    /// Highest spell level Sora knows (1 = Fire, 3 = Firaga).
    magic_level: u8,
    /// The spell being cast (index into the kit), released by the motion's
    /// magic-cast event.
    casting: Option<usize>,
}

impl Game {
    fn can_afford(&self, cost: u8) -> bool {
        self.mp_charge.is_none() && if cost == 255 { self.mp > 0.0 } else { self.mp >= cost as f32 }
    }
}

#[derive(Resource)]
struct Debug {
    show_boxes: bool,
    show_hurt: bool,
    show_skeleton: bool,
    show_other: bool,
    log: Vec<String>,
    last: Option<String>,
}

#[derive(Resource)]
struct Shot {
    after_frames: Option<u32>,
    frames: u32,
    /// `--pause-at N`: stop the simulation at frame N (for screenshots of
    /// a given moment).
    pause_at: Option<u64>,
}

/// Button presses seen since the last fixed step, so short taps are never lost.
#[derive(Resource, Default)]
struct Latch {
    stick: Vec2,
    walk: bool,
    held: [bool; 4],
    pressed: [bool; 4],
    camera: Vec2,
    /// Camera reset pressed (R3 / F).
    cam_reset: bool,
    /// Lock-on switch pressed: −1 left, +1 right (R2 + right stick / Z, C).
    switch: i8,
    /// R2 + right stick already switched (wait for the stick to return).
    switch_held: bool,
    /// A Drive shortcut (form id; 0 = Revert).
    drive: Option<u8>,
    /// The reaction-command button (L / pad Y) was pressed.
    rc: bool,
    /// Command-menu cursor presses and magic shortcuts since the last step.
    menu_up: bool,
    menu_down: bool,
    shortcut: Option<usize>,
    /// The shortcut menu modifier (L1 / Ctrl) is held, and the slot
    /// (`hud::SLOTS` order) whose button was pressed with it.
    short_held: bool,
    short_slot: Option<usize>,
    /// Testing aid: fill the Drive gauge (F5 / pad Select).
    refill: bool,
    /// F6: send the enemies away / bring them back.
    away: bool,
}

/// `--autoplay`: scripted input for smoke tests and screenshots
/// (walk toward the dummies, then mash attack).
#[derive(Resource)]
struct Autoplay {
    on: bool,
    frame: u32,
    scene: String,
    /// `--trace`: print state and position every frame (stderr).
    trace: bool,
}

#[derive(Resource)]
struct CameraRig {
    /// Horizontal view direction (for camera-relative movement).
    yaw: f32,
    params: kh2_camera::Params,
    /// The game's camera (created on the first frame).
    cam: Option<kh2_camera::Camera>,
    /// Swing behind while running (the game's Camera "Auto" option).
    auto: bool,
}

#[derive(Component)]
struct Player;
#[derive(Component)]
struct PlayerBody;
#[derive(Component)]
struct PlayerBlade;
#[derive(Component)]
struct Hud;

#[derive(Component)]
struct Dummy {
    id: u32,
    home: [f32; 3],
    pos: [f32; 3],
    combatant: Combatant,
    knock: [f32; 2],
    knock_frames: u32,
    respawn: u32,
    flash: u32,
    /// A Soldier on its own skeleton (absent: a plain column dummy).
    foe: Option<foes::FoeState>,
    /// What has hit it (exe rules: once per attack, rows by hits in a row).
    record: HitRecord,
}

fn spawn_camera_and_light(mut commands: Commands) {
    // The game's vertical field of view (1.5 rad horizontal on 4:3) and
    // near plane (1 unit).
    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection { fov: kh2_camera::Params::FIELD.fov_v(), near: SCALE, ..default() }),
        Transform::from_xyz(0.0, 3.0, -8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        DirectionalLight { illuminance: 9000.0, shadows_enabled: true, ..default() },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 0.6, 0.0)),
    ));
}

/// The mannequin's pieces for a rig, coloured for a Drive Form.
fn spawn_pieces(commands: &mut Commands, rig: &std::sync::Arc<kh2_data::sora::Rig>, form: u8, meshes: &mut Assets<Mesh>, mats: &mut Assets<StandardMaterial>) {
    let bind = RigAnim::new(rig.clone());
    let sphere = meshes.add(Sphere::new(0.5).mesh().uv(24, 16));
    let paints: HashMap<Paint, Handle<StandardMaterial>> = Paint::ALL
        .iter()
        .map(|p| {
            let mut m = p.material();
            // Each form's outfit colour (our styling).
            if *p == Paint::Jacket {
                m.base_color = match form {
                    1 => Color::srgb(0.62, 0.08, 0.1),
                    2 => Color::srgb(0.1, 0.22, 0.62),
                    3 => Color::srgb(0.75, 0.15, 0.15),
                    4 => Color::srgb(0.8, 0.62, 0.08),
                    5 => Color::srgb(0.85, 0.86, 0.9),
                    6 => Color::srgb(0.02, 0.02, 0.03),
                    _ => m.base_color,
                };
            }
            (*p, mats.add(m))
        })
        .collect();
    for piece in rig::pieces(rig) {
        // Bones are rigid, so each capsule is sized once from the bind pose.
        let mesh = match piece.shape {
            Shape::Capsule(a, b, r) => {
                let len = match (bind.anchor(piece.part, a), bind.anchor(piece.part, b)) {
                    (Some(a), Some(b)) => a.distance(b),
                    _ => 0.0,
                };
                meshes.add(Capsule3d::new(r * SCALE, len * SCALE).mesh().rings(4).latitudes(12).longitudes(16))
            }
            Shape::Sphere(..) => sphere.clone(),
        };
        let paint = if form == 6 && piece.paint != Paint::Eye { Paint::Shade } else { piece.paint };
        let mat = if form == 6 && paint == Paint::Shade { paints[&Paint::Jacket].clone() } else { paints[&paint].clone() };
        commands.spawn((piece, Mesh3d(mesh), MeshMaterial3d(mat), Transform::default()));
    }
}

/// Rebuilds the mannequin when the player object changes (a Drive or a
/// revert).
fn rebuild_player(mut commands: Commands, game: Res<Game>, pieces: Query<Entity, With<Piece>>, mut seen: Local<u32>, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    if game.drive.rig_version == *seen {
        return;
    }
    *seen = game.drive.rig_version;
    let Some(anim) = &game.anim else { return };
    for e in &pieces {
        commands.entity(e).despawn();
    }
    spawn_pieces(&mut commands, &anim.rig, game.drive.form(), &mut meshes, &mut mats);
}

fn spawn_player(mut commands: Commands, game: Res<Game>, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    if let Some(anim) = &game.anim {
        // Mannequin: rounded pieces on the bones, placed every frame.
        commands.spawn((Player, Transform::default(), Visibility::default()));
        spawn_pieces(&mut commands, &anim.rig, 0, &mut meshes, &mut mats);
        return;
    }
    let body_len = (SORA_HEIGHT - 2.0 * SORA_RADIUS) * SCALE;
    commands
        .spawn((Player, Transform::default(), Visibility::default()))
        .with_children(|p| {
            p.spawn((
                PlayerBody,
                Mesh3d(meshes.add(Capsule3d::new(SORA_RADIUS * SCALE, body_len))),
                MeshMaterial3d(mats.add(StandardMaterial { base_color: Color::srgb(0.85, 0.2, 0.25), ..default() })),
                Transform::from_xyz(0.0, SORA_HEIGHT * 0.5 * SCALE, 0.0),
            ))
            .with_children(|b| {
                // Facing marker ("nose").
                b.spawn((
                    Mesh3d(meshes.add(Cuboid::new(0.12, 0.12, 0.2))),
                    MeshMaterial3d(mats.add(Color::srgb(1.0, 0.85, 0.3))),
                    Transform::from_xyz(0.0, 0.35, SORA_RADIUS * SCALE + 0.05),
                ));
            });
            p.spawn((
                PlayerBlade,
                Mesh3d(meshes.add(Cuboid::new(0.06, 0.06, 1.0))),
                MeshMaterial3d(mats.add(Color::srgb(0.75, 0.75, 0.85))),
                Transform::from_xyz(0.35, 0.9, 0.0),
            ));
        });
}

fn spawn_dummies(mut commands: Commands, game: Res<Game>, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    let Some(template) = &game.dummy_template else { return };
    let mesh = meshes.add(Cylinder::new(DUMMY_RADIUS * SCALE, DUMMY_HEIGHT * SCALE));
    let mat = mats.add(Color::srgb(0.15, 0.15, 0.2));
    // One Soldier (`--enemies N` for more).
    let all = [[0.0, 0.0, 0.0], [200.0, 0.0, 150.0], [-250.0, 0.0, 120.0], [600.0, 0.0, 600.0], [-700.0, 0.0, 500.0]];
    let spots = &all[..game.enemy_count.clamp(1, all.len())];
    let sphere = meshes.add(Sphere::new(0.5).mesh().uv(16, 10));
    let paints: HashMap<Paint, Handle<StandardMaterial>> = Paint::ALL.iter().map(|p| (*p, mats.add(p.material()))).collect();
    for (i, p) in spots.iter().enumerate() {
        let mut c = template.clone();
        c.id = 100 + i as u32;
        let mut dm = Dummy { id: c.id, home: *p, pos: *p, combatant: c, knock: [0.0; 2], knock_frames: 0, respawn: 0, flash: 0, foe: None, record: HitRecord::new() };
        let Some(kit) = &game.foe_kit else {
            commands.spawn((dm, Mesh3d(mesh.clone()), MeshMaterial3d(mat.clone()), Transform::from_xyz(p[0] * SCALE, DUMMY_HEIGHT * 0.5 * SCALE, p[2] * SCALE)));
            continue;
        };
        // Face the arena's centre-ish (toward -z) to start.
        let mut st = foes::FoeState::new(kit, i as u32 * 7919 + 13, PI);
        // Pose once so capsule lengths can be measured.
        let out = st.npc.step(&Default::default(), true);
        st.anim.update_npc(&out, &kit.data.clips, *p);
        let pieces = foes::pieces(kit);
        let lens: Vec<f32> = pieces
            .iter()
            .map(|pc| match pc.shape {
                Shape::Capsule(a, b, _) => match (st.anim.anchor(Part::Body, a), st.anim.anchor(Part::Body, b)) {
                    (Some(a), Some(b)) => a.distance(b),
                    _ => 0.0,
                },
                _ => 0.0,
            })
            .collect();
        dm.foe = Some(st);
        let owner = commands.spawn((dm, Transform::default(), Visibility::default())).id();
        for (pc, len) in pieces.into_iter().zip(lens) {
            let m = match pc.shape {
                Shape::Capsule(_, _, r) => meshes.add(Capsule3d::new(r * SCALE, len * SCALE).mesh().rings(2).latitudes(8).longitudes(10)),
                Shape::Sphere(..) => sphere.clone(),
            };
            commands.spawn((foes::FoePiece { owner, piece: pc }, Mesh3d(m), MeshMaterial3d(paints[&pc.paint].clone()), Transform::default(), Visibility::default()));
        }
    }
}

fn latch_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion: EventReader<MouseMotion>,
    mut wheel: EventReader<bevy::input::mouse::MouseWheel>,
    pads: Query<&Gamepad>,
    mut latch: ResMut<Latch>,
) {
    // The mouse wheel moves the command menu cursor (down = next).
    for w in wheel.read() {
        if w.y < 0.0 {
            latch.menu_down = true;
        } else if w.y > 0.0 {
            latch.menu_up = true;
        }
    }
    let mut stick = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        stick.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        stick.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        stick.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) {
        stick.x += 1.0;
    }
    stick = stick.normalize_or_zero();
    let mut held = [
        keys.pressed(KeyCode::Space),
        keys.pressed(KeyCode::KeyJ) || mouse.pressed(MouseButton::Left),
        keys.pressed(KeyCode::KeyK) || keys.pressed(KeyCode::ShiftLeft),
        keys.pressed(KeyCode::Tab) || mouse.pressed(MouseButton::Middle),
    ];
    let mut cam = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyQ) || keys.pressed(KeyCode::ArrowLeft) {
        cam.x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyE) || keys.pressed(KeyCode::ArrowRight) {
        cam.x += 1.0;
    }
    // Up/down move the command menu cursor (D-pad on a pad); camera pitch
    // is on the mouse / right stick.
    latch.menu_up |= keys.just_pressed(KeyCode::ArrowUp);
    latch.menu_down |= keys.just_pressed(KeyCode::ArrowDown);
    for (i, k) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4, KeyCode::Digit5, KeyCode::Digit6].iter().enumerate() {
        if keys.just_pressed(*k) {
            latch.shortcut = Some(i);
        }
    }
    latch.cam_reset |= keys.just_pressed(KeyCode::KeyF);
    latch.refill |= keys.just_pressed(KeyCode::F5) || pads.iter().any(|p| p.just_pressed(GamepadButton::Select));
    latch.away |= keys.just_pressed(KeyCode::F6);
    // The shortcut menu: hold L1 / Ctrl, then a face button casts its slot
    // instead of jumping, attacking and so on (KH2).
    let modifier = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight) || pads.iter().any(|p| p.pressed(GamepadButton::LeftTrigger));
    if modifier {
        let pad = |b: GamepadButton| pads.iter().any(|p| p.just_pressed(b));
        let slots = [
            keys.just_pressed(KeyCode::KeyL) || pad(GamepadButton::North),
            keys.just_pressed(KeyCode::KeyK) || keys.just_pressed(KeyCode::ShiftLeft) || pad(GamepadButton::West),
            keys.just_pressed(KeyCode::Space) || pad(GamepadButton::East),
            keys.just_pressed(KeyCode::KeyJ) || mouse.just_pressed(MouseButton::Left) || pad(GamepadButton::South),
        ];
        if let Some(i) = slots.iter().position(|&p| p) {
            latch.short_slot = Some(i);
        }
    } else {
        latch.rc |= keys.just_pressed(KeyCode::KeyL) || pads.iter().any(|p| p.just_pressed(GamepadButton::North));
    }
    // Drive shortcuts: 7 Valor, 8 Wisdom, 9 Limit, 0 Master, - Final,
    // Backspace Revert.
    for (k, f) in [(KeyCode::Digit7, 1u8), (KeyCode::Digit8, 2), (KeyCode::Digit9, 3), (KeyCode::Digit0, 4), (KeyCode::Minus, 5), (KeyCode::Backspace, 0)] {
        if keys.just_pressed(k) {
            latch.drive = Some(f);
        }
    }
    if keys.just_pressed(KeyCode::KeyZ) {
        latch.switch = -1;
    }
    if keys.just_pressed(KeyCode::KeyC) {
        latch.switch = 1;
    }
    if mouse.pressed(MouseButton::Right) {
        for m in motion.read() {
            cam += Vec2::new(m.delta.x, -m.delta.y) * 0.05;
        }
    } else {
        motion.clear();
    }
    for pad in &pads {
        let l = pad.left_stick();
        if l.length() > 0.2 {
            stick = l.clamp_length_max(1.0);
        }
        let r = pad.right_stick();
        latch.cam_reset |= pad.just_pressed(GamepadButton::RightThumb);
        if pad.pressed(GamepadButton::RightTrigger2) {
            // R2 + right stick left/right switches the lock-on target.
            if r.x.abs() > 0.5 {
                if !latch.switch_held {
                    latch.switch = if r.x > 0.0 { 1 } else { -1 };
                }
                latch.switch_held = true;
            } else {
                latch.switch_held = false;
            }
        } else if r.length() > 0.2 {
            cam += r;
        }
        latch.menu_up |= pad.just_pressed(GamepadButton::DPadUp);
        latch.menu_down |= pad.just_pressed(GamepadButton::DPadDown);
        // Xbox layout: A attack, B jump, X action (guard/roll), R1 lock-on.
        held[0] |= pad.pressed(GamepadButton::East);
        held[1] |= pad.pressed(GamepadButton::South);
        held[2] |= pad.pressed(GamepadButton::West);
        held[3] |= pad.pressed(GamepadButton::RightTrigger);
    }
    latch.walk = keys.pressed(KeyCode::AltLeft);
    latch.stick = if latch.walk { stick * 0.4 } else { stick };
    let released = latch.short_held && !modifier;
    latch.short_held = modifier;
    for i in 0..4 {
        // With the modifier the face buttons belong to the shortcuts (not
        // lock-on), and letting go of it doesn't turn a button still held
        // into a fresh press.
        if modifier && i < 3 {
            held[i] = false;
        }
        if held[i] && !latch.held[i] && !(released && i < 3) {
            latch.pressed[i] = true;
        }
        latch.held[i] = held[i];
    }
    latch.camera = cam;
}

/// The game's camera, one 60 Hz step after the simulation.
fn camera_step(game: Res<Game>, mut latch: ResMut<Latch>, dummies: Query<&Dummy>, mut rig: ResMut<CameraRig>, shot: Res<Shot>) {
    if shot.pause_at.is_some_and(|p| game.frame >= p) {
        return;
    }
    let subject = kh2_camera::Subject { feet: Vec3::from(game.pos), facing: game.character.facing(), run_speed: game.character.params.run_speed };
    let params = rig.params;
    let auto = rig.auto;
    let cam = rig.cam.get_or_insert_with(|| kh2_camera::Camera::new(params, &subject));
    // The target's camera point (type 7), else its lock-on point (type 3).
    let locked = game.last.as_ref().and_then(|o| o.locked).and_then(|id| dummies.iter().find(|d| d.id == id));
    let lock = locked.map(|d| {
        let from_rig = d.foe.as_ref().and_then(|f| {
            let pick = |k: u8| f.anim.shapes_of_kind(k).first().map(|(s, _)| Vec3::from(s.center));
            pick(7).or_else(|| pick(3))
        });
        from_rig.unwrap_or(Vec3::from(d.pos) + Vec3::Y * DUMMY_HEIGHT * 0.7)
    });
    let ctl = kh2_camera::Controls { move_stick: latch.stick, look_stick: latch.camera.clamp_length_max(1.0), reset: std::mem::take(&mut latch.cam_reset), auto };
    let arena = Arena::get();
    let wall = |a: Vec3, b: Vec3| arena.map.segment(a, b, map_collision::attr::PLAYER_PASS).map(|h| (h.point, arena.map.polys[h.poly].normal));
    cam.update(&subject, &ctl, lock, 1.0, &wall);
    rig.yaw = rig.cam.as_ref().map_or(0.0, |c| c.view_yaw());
}

fn camera_control(rig: Res<CameraRig>, mut cam: Query<&mut Transform, With<Camera3d>>) {
    let Some(c) = &rig.cam else { return };
    if let Ok(mut t) = cam.get_single_mut() {
        *t = Transform::from_translation(c.eye * SCALE).looking_at(c.at * SCALE, Vec3::Y);
    }
}

fn abilities_keys(keys: Res<ButtonInput<KeyCode>>, mut game: ResMut<Game>, mut dbg: ResMut<Debug>) {
    // Without the install's menus (--synthetic), the number keys toggle the
    // growth and combo abilities directly; otherwise they are magic
    // shortcuts and the ability menu (Esc) equips.
    let has_kit = game.kit.is_some();
    let a = &mut game.character.abilities;
    let cycle = |v: &mut u8| *v = (*v + 1) % 5;
    let keys_on = !has_kit;
    if keys_on && keys.just_pressed(KeyCode::Digit1) {
        cycle(&mut a.high_jump);
    }
    if keys_on && keys.just_pressed(KeyCode::Digit2) {
        cycle(&mut a.quick_run);
    }
    if keys_on && keys.just_pressed(KeyCode::Digit3) {
        cycle(&mut a.aerial_dodge);
    }
    if keys_on && keys.just_pressed(KeyCode::Digit4) {
        cycle(&mut a.glide);
    }
    if keys_on && keys.just_pressed(KeyCode::Digit5) {
        cycle(&mut a.dodge_roll);
    }
    if keys_on && keys.just_pressed(KeyCode::Digit6) {
        a.combo_plus = (a.combo_plus + 1) % 4;
    }
    if keys_on && keys.just_pressed(KeyCode::Digit7) {
        a.air_combo_plus = (a.air_combo_plus + 1) % 3;
    }
    if keys_on && keys.just_pressed(KeyCode::Digit8) {
        a.finishing_plus = (a.finishing_plus + 1) % 4;
    }
    if keys.just_pressed(KeyCode::F1) {
        dbg.show_boxes = !dbg.show_boxes;
    }
    if keys.just_pressed(KeyCode::F2) {
        dbg.show_hurt = !dbg.show_hurt;
    }
    if keys.just_pressed(KeyCode::F3) {
        dbg.show_skeleton = !dbg.show_skeleton;
    }
    if keys.just_pressed(KeyCode::F4) {
        dbg.show_other = !dbg.show_other;
    }
    if keys.just_pressed(KeyCode::KeyR) {
        game.pos = [0.0, 0.0, -400.0];
    }
    game.combo_plus = game.character.abilities.combo_plus;
}

/// Where the Keyblade points this frame. The real game animates a skeleton;
/// this playground has no skeleton, so a live hitbox window is drawn as a
/// right-to-left horizontal sweep in front of the character (approximation).
fn blade_pose(pos: [f32; 3], facing: f32, sweep: Option<f32>) -> (Vec3, Vec3) {
    let hand = Vec3::new(pos[0], pos[1] + 95.0, pos[2]);
    let fwd = Vec3::new(facing.sin(), 0.0, facing.cos());
    let ang = match sweep {
        Some(t) => FRAC_PI_2 * 0.8 * (1.0 - 2.0 * t),
        None => -FRAC_PI_2,
    };
    let dir = Quat::from_rotation_y(ang) * fwd;
    (hand + fwd * 25.0, dir)
}

/// Shapes for a live attack or guard window: the group's collision shapes
/// on the real bones when the skeleton is loaded, otherwise spheres along a
/// swept stand-in blade (approximation).
fn attack_shapes(game: &Game, facing: f32, group: u8, weapon: u8) -> Vec<CShape> {
    if let Some(anim) = &game.anim {
        let s = anim.attack_shapes(group, weapon);
        if !s.is_empty() {
            return s;
        }
    }
    let (origin, dir) = blade_pose(game.pos, facing, Some(0.5));
    let spheres: Vec<(Vec3, f32)> = match game.blade.get(&group) {
        Some(s) if !s.is_empty() => s.iter().map(|b| (origin + dir * (40.0 + b.along), b.radius)).collect(),
        // Body/other attack groups (approximation): one sphere in front.
        _ => vec![(origin + dir * 60.0, 80.0)],
    };
    spheres.into_iter().map(|(c, r)| CShape::sphere(c.into(), r)).collect()
}

/// A training dummy's hit shape: a column standing on the ground
/// (stand-in until enemies' own collision data is read in Phase 3).
fn dummy_shape(pos: [f32; 3]) -> CShape {
    CShape::column([pos[0], pos[1] + DUMMY_HEIGHT * 0.5, pos[2]], DUMMY_RADIUS, DUMMY_HEIGHT * 0.5)
}

fn simulate(
    mut game: ResMut<Game>,
    mut latch: ResMut<Latch>,
    mut auto: ResMut<Autoplay>,
    rig: Res<CameraRig>,
    mut dummies: Query<&mut Dummy>,
    mut dbg: ResMut<Debug>,
    shot: Res<Shot>,
    mut menu: ResMut<hud::MenuState>,
) {
    let game = &mut *game;
    if shot.pause_at.is_some_and(|p| game.frame >= p) {
        return;
    }
    if menu.focus == hud::Focus::Abilities {
        // Paused in the ability menu; drop presses made meanwhile.
        latch.pressed = [false; 4];
        latch.menu_up = false;
        latch.menu_down = false;
        latch.shortcut = None;
        latch.short_slot = None;
        menu.shortcuts_open = false;
        return;
    }
    game.frame += 1;
    let log_before = dbg.log.len();
    if auto.on {
        auto.frame += 1;
        let f = auto.frame;
        let press = |i: usize, on: bool, latch: &mut Latch| {
            if on && !latch.held[i] {
                latch.pressed[i] = true;
            }
            latch.held[i] = on;
        };
        match auto.scene.as_str() {
            "idle" => latch.stick = Vec2::ZERO,
            "run" => latch.stick = Vec2::X,
            "jump" => press(0, f % 90 == 5, &mut latch),
            // Open the ability menu (pauses the game).
            "abilities" => {
                if f == 3 {
                    menu.focus = hud::Focus::Abilities;
                    menu.ability = 21;
                }
            }
            // Open the command menu's Magic list and cast, then use shortcuts.
            "magic" => {
                latch.stick = Vec2::ZERO;
                latch.menu_down |= f == 5;
                press(1, f == 8 || f == 12, &mut latch);
                if f == 150 {
                    latch.shortcut = Some(3);
                }
            }
            // Out of combat: enemies sent away, then stand (field idle),
            // walk, run, and attack (the Keyblade comes out for the swing).
            "field" => {
                latch.away |= f == 1;
                latch.walk = (100..160).contains(&f);
                latch.stick = if (100..160).contains(&f) { Vec2::Y * 0.4 } else if (160..220).contains(&f) { Vec2::Y } else { Vec2::ZERO };
                press(1, f == 240, &mut latch);
                if auto.trace && f % 10 == 0 {
                    let a = game.anim.as_ref();
                    eprintln!("f{f} field {} weapons_out {}", a.map_or(false, |a| a.field), a.map_or(true, |a| a.weapons_out()));
                }
            }
            // The shortcut menu: hold it open and cast Square (Blizzard);
            // then in the Magic list put Thunder on Triangle and cast it
            // from the shortcuts; then refill the Drive gauge.
            "shortcuts" => {
                latch.stick = Vec2::ZERO;
                latch.short_held = (5..60).contains(&f) || (85..90).contains(&f) || (130..140).contains(&f);
                if f == 20 {
                    latch.short_slot = Some(1);
                }
                latch.menu_down |= f == 70 || f == 76 || f == 79;
                press(1, f == 73, &mut latch);
                if f == 86 || f == 132 {
                    latch.short_slot = Some(0);
                }
                press(0, f == 95, &mut latch);
                if f == 150 {
                    game.drive.drive.bars = 0;
                    game.drive.drive.points = 0;
                }
                latch.refill |= f == 155;
                if auto.trace && f % 10 == 0 {
                    eprintln!("f{f} focus {:?} cursor {} shortcuts {:?} drive {}+{}", menu.focus, menu.magic, menu.shortcuts.0, game.drive.drive.bars, game.drive.drive.points);
                }
            }
            // One spell, cast twice from a few steps away: fire, blizzard,
            // thunder, cure, magnet, reflect (the Magic list's order).
            name @ ("fire" | "blizzard" | "thunder" | "cure" | "magnet" | "reflect") => {
                let i = ["fire", "blizzard", "thunder", "cure", "magnet", "reflect"].iter().position(|n| *n == name).unwrap_or(0);
                latch.stick = if f < 12 { Vec2::Y } else { Vec2::ZERO };
                // Hurt first, so Cure has something to heal.
                if name == "cure" && f == 1 {
                    game.sora.hp = game.sora.max_hp / 3;
                }
                // Reflect is short: cast it as a Soldier starts a swipe.
                let hit_at = game
                    .foe_kit
                    .as_ref()
                    .and_then(|k| k.data.clips.get(&foes::soldier::SWIPE))
                    .and_then(|c| c.windows.iter().filter(|w| matches!(w.kind, character_core::WindowKind::Hitbox { .. })).map(|w| w.start).min())
                    .unwrap_or(0);
                let swiping = dummies.iter().any(|d| d.foe.as_ref().and_then(|s| s.out.as_ref()).is_some_and(|o| o.motion == foes::soldier::SWIPE && o.clip_frame == hit_at.saturating_sub(12)));
                let again = if name == "reflect" { swiping } else { f == 20 || f == 160 };
                if again {
                    latch.shortcut = Some(i);
                }
            }
            // Stand still and guard every 40 frames: Soldiers' swipes bounce off.
            "guard" => {
                latch.stick = Vec2::ZERO;
                press(2, f % 40 == 1, &mut latch);
            }
            // Swing at a wall: the Keyblade clinks off.
            "wall" => {
                // The far side of the 300-high block, away from the dummies'
                // auto-target range; face -x.
                if f == 1 {
                    game.pos = [1290.0, 0.0, 0.0];
                }
                let d = Vec2::new(rig.yaw.sin(), rig.yaw.cos());
                let right = Vec2::new(-d.y, d.x);
                let w = Vec2::new(-1.0, 0.0);
                latch.stick = if f < 20 { Vec2::new(w.dot(right), w.dot(d)) * 0.3 } else { Vec2::ZERO };
                press(1, f >= 20 && f % 8 < 4, &mut latch);
            }
            // Jump at the 300-high ledge from the floor: grab, then climb.
            "hang" => {
                if f == 1 {
                    game.pos = [700.0, 0.0, -60.0];
                    game.character.abilities.high_jump = 0;
                }
                // World direction -> camera-relative stick.
                let d = Vec2::new(rig.yaw.sin(), rig.yaw.cos());
                let right = Vec2::new(-d.y, d.x);
                let to_stick = |w: Vec2| Vec2::new(w.dot(right), w.dot(d));
                // Toward the wall (+x) and mostly sideways
                // (grab and hang) until frame 150, then push in (climb).
                latch.stick = if f < 150 { to_stick(Vec2::new(0.05, 0.3)) } else { to_stick(Vec2::new(1.0, 0.0)) };
                press(0, f == 5, &mut latch);
                latch.held[0] = f >= 5 && f < 60;
            }
            // Map collision: walk up the stairs / the 20° ramp / into the 45°
            // ramp, or jump under the floating slab.
            name @ ("stairs" | "ramp" | "steep" | "ceiling") => {
                let (start, dir) = match name {
                    "stairs" => ([-1125.0, 0.0, 0.0], Vec2::new(0.0, -1.0)),
                    "ramp" => ([-1125.0, 0.0, -150.0], Vec2::new(0.0, 1.0)),
                    "steep" => ([-1625.0, 0.0, -150.0], Vec2::new(0.0, 1.0)),
                    _ => ([1600.0, 0.0, 600.0], Vec2::new(-1.0, 0.0)),
                };
                if f == 1 {
                    game.pos = start;
                    if name == "ceiling" {
                        game.character.abilities.high_jump = 0;
                    }
                }
                let d = Vec2::new(rig.yaw.sin(), rig.yaw.cos());
                let right = Vec2::new(-d.y, d.x);
                let to_stick = |w: Vec2| Vec2::new(w.dot(right), w.dot(d));
                if name == "ceiling" {
                    latch.stick = if (30..40).contains(&(f % 90)) { to_stick(dir) * 0.3 } else { Vec2::ZERO };
                    press(0, f % 90 == 5, &mut latch);
                    latch.held[0] = (5..40).contains(&(f % 90));
                    // Mash attack while bonked (refused for 16 frames).
                    press(1, f % 90 > 20 && f % 90 < 50 && f % 4 < 2, &mut latch);
                } else {
                    latch.stick = if f > 5 && f < 240 { to_stick(dir) } else { Vec2::ZERO };
                }
                if auto.trace && f % 5 == 0 {
                    eprintln!("f{f} pos {:.1} {:.1} {:.1} grounded {} n {:.2}", game.pos[0], game.pos[1], game.pos[2], game.grounded, game.ground_normal);
                }
            }
            // Lock on to the Soldier and circle it; the camera keeps both in
            // view.
            "lock" => {
                press(3, f == 5, &mut latch);
                latch.stick = if f > 10 { Vec2::X } else { Vec2::ZERO };
                if auto.trace && f % 10 == 0 {
                    if let Some(c) = &rig.cam {
                        eprintln!("f{f} cam yaw {:.1} dist {:.0} locked {} pos {:?}", c.yaw.to_degrees(), c.dist, c.locked, game.pos);
                    }
                }
            }
            // Drive into a form, then fight: valor, wisdom, limit, master,
            // final, anti (Anti forced through the roll).
            name @ ("valor" | "wisdom" | "limit" | "master" | "final" | "anti") => {
                let f_ = f;
                let f = ["valor", "wisdom", "limit", "master", "final", "anti"].iter().position(|n| *n == name).unwrap_or(0) as u8 + 1;
                if f_ == 3 {
                    if f == 6 {
                        game.drive.force_anti = true;
                    }
                    latch.drive = Some(if f == 6 { 1 } else { f });
                }
                latch.stick = if (190..215).contains(&f_) { Vec2::Y } else { Vec2::ZERO };
                press(1, f_ >= 215 && f_ % 8 < 4, &mut latch);
                if auto.trace && f_ % 20 == 0 {
                    eprintln!("f{f_} form {} gauge {:?} drive {}+{} motion {:?}", game.drive.form(), game.drive.drive.gauge.map(|g| g.left), game.drive.drive.bars, game.drive.drive.points, game.last.as_ref().map(|o| o.motion));
                }
            }
            // The Soldier's Cyclone: stand off, make it do its spinning
            // attack (which opens the RC area), press the reaction button.
            "rc" => {
                if f == 1 {
                    game.pos = [0.0, 0.0, -400.0];
                }
                latch.stick = Vec2::ZERO;
                if f == 20 || f == 200 {
                    for mut dm in dummies.iter_mut() {
                        if let Some(st) = dm.foe.as_mut() {
                            st.forced = Some(foes::soldier::DASH);
                        }
                    }
                }
                latch.rc |= game.rc.offer.is_some_and(|o| !o.running && !o.greyed) && f % 20 == 0;
                if auto.trace && f % 5 == 0 {
                    eprintln!("f{f} offer {:?} running {:?} motion {:?} pos {:?}", game.rc.offer, game.rc.running, game.last.as_ref().map(|o| o.motion), game.pos);
                }
            }
            // Low HP: a Drive Form is offered as a reaction command.
            "rcdrive" => {
                if f == 1 {
                    game.sora.hp = game.sora.max_hp / 5;
                }
                latch.rc |= f == 20;
                if auto.trace && f % 10 == 0 {
                    eprintln!("f{f} offer {:?} form {} hp {}", game.rc.offer.map(|o| o.cmd), game.drive.form(), game.sora.hp);
                }
            }
            "roll" => {
                latch.stick = Vec2::X;
                press(2, f % 60 == 5, &mut latch);
            }
            "glide" => {
                // Jump, Aerial Dodge, then hold the action button (Square).
                latch.stick = Vec2::X;
                press(0, f == 5 || f == 30, &mut latch);
                press(2, f > 50, &mut latch);
            }
            "air" => {
                press(0, f == 5, &mut latch);
                press(1, f > 30 && f % 8 < 4, &mut latch);
            }
            _ => {
                latch.stick = if f < 30 { Vec2::Y } else { Vec2::ZERO };
                press(1, f >= 30 && f % 8 < 4, &mut latch);
            }
        }
    }

    // Camera-relative stick.
    let d = Vec2::new(rig.yaw.sin(), rig.yaw.cos());
    let right = Vec2::new(-d.y, d.x);
    // With the camera behind looking along d, screen-right is `right`.
    let s = d * latch.stick.y + right * latch.stick.x;
    // Command menu: up/down move the cursor; attack confirms the selected
    // command (Attack just attacks), jump backs out of a submenu (KH2).
    let mut cast: Option<usize> = latch.shortcut.take();
    menu.shortcuts_open = latch.short_held;
    if let Some(slot) = latch.short_slot.take() {
        let name = hud::SLOTS[slot].0;
        if menu.focus == hud::Focus::Magic {
            // In the Magic list: put the highlighted spell on that button.
            menu.shortcuts.0[slot] = Some(menu.magic);
            let spell = game.kit.as_ref().and_then(|k| hud::spell_list(k, game.magic_level).get(menu.magic).and_then(|&i| k.spells.get(i)).map(|s| s.name.clone()));
            dbg.log.push(format!("shortcut {name}: {}", spell.unwrap_or_default()));
        } else {
            cast = menu.shortcuts.0[slot];
        }
    }
    // Testing aid: fill the Drive gauge (or the form gauge in a form).
    if std::mem::take(&mut latch.refill) {
        game.drive.drive.refill();
        dbg.log.push("Drive gauge refilled".into());
    }
    // A Drive command (form id; 0 = Revert).
    let mut drive_cmd: Option<u8> = latch.drive.take();
    if let Some(kit) = &game.kit {
        let spells = hud::spell_list(kit, game.magic_level);
        let n_root = kit.root.len().max(1);
        let (up, down) = (std::mem::take(&mut latch.menu_up), std::mem::take(&mut latch.menu_down));
        match menu.focus {
            hud::Focus::Root => {
                if down {
                    menu.root = (menu.root + 1) % n_root;
                }
                if up {
                    menu.root = (menu.root + n_root - 1) % n_root;
                }
                if latch.pressed[1] && menu.root != hud::ATTACK {
                    latch.pressed[1] = false;
                    let form = game.drive.form();
                    if menu.root == hud::MAGIC && !spells.is_empty() && drive::magic_allowed(form) {
                        menu.focus = hud::Focus::Magic;
                    }
                    if menu.root == hud::DRIVE {
                        if form != 0 {
                            drive_cmd = Some(0);
                        } else if !game.drive.busy() {
                            menu.focus = hud::Focus::Drive;
                        }
                    }
                }
            }
            hud::Focus::Drive => {
                let n = kit.drives.len().max(1);
                if down {
                    menu.drive = (menu.drive + 1) % n;
                }
                if up {
                    menu.drive = (menu.drive + n - 1) % n;
                }
                if latch.pressed[1] {
                    latch.pressed[1] = false;
                    if let Some(d) = kit.drives.get(menu.drive) {
                        if drive::available(game, d.form) {
                            drive_cmd = Some(d.form);
                            menu.focus = hud::Focus::Root;
                        }
                    }
                }
                if latch.pressed[0] {
                    latch.pressed[0] = false;
                    menu.focus = hud::Focus::Root;
                }
            }
            hud::Focus::Magic => {
                let n = spells.len().max(1);
                if down {
                    menu.magic = (menu.magic + 1) % n;
                }
                if up {
                    menu.magic = (menu.magic + n - 1) % n;
                }
                if latch.pressed[1] {
                    latch.pressed[1] = false;
                    cast = Some(menu.magic);
                }
                if latch.pressed[0] {
                    latch.pressed[0] = false;
                    menu.focus = hud::Focus::Root;
                }
            }
            hud::Focus::Abilities => {}
        }
        if !drive::magic_allowed(game.drive.form()) || game.drive.busy() {
            cast = None;
        }
        // Cast: MP check, then the controller picks the motion.
        if let Some(i) = cast.and_then(|c| spells.get(c).copied()) {
            let sp = kit.spells[i].clone();
            if game.can_afford(sp.mp_cost) {
                let m = |v: i16| (v >= 0).then_some(v as u16);
                if let (Some(ground), Some(air)) = (m(sp.ground_motion), m(sp.air_motion)) {
                    let motions = character_core::CastMotions { ground, air, finisher: m(sp.finish_motion) };
                    if let Some(used) = game.character.cast(motions) {
                        game.cast_now = Some((i, Some(used) == motions.finisher && motions.finisher != Some(motions.ground)));
                        game.mp = if sp.mp_cost == 255 { 0.0 } else { (game.mp - sp.mp_cost as f32).max(0.0) };
                        if game.mp <= 0.0 {
                            game.mp_charge = Some(0.0);
                        }
                        game.casting = Some(i);
                        dbg.log.push(format!("cast {} ({} MP)", sp.name, if sp.mp_cost == 255 { "all".into() } else { sp.mp_cost.to_string() }));
                    }
                }
            }
        }
    } else {
        latch.menu_up = false;
        latch.menu_down = false;
    }
    // The reaction command.
    if std::mem::take(&mut latch.rc) {
        let battle = dummies.iter().any(|d| d.respawn == 0);
        rc::press(game, battle, &mut dbg.log);
    }
    // Drive / Revert. In battle while any enemy is up (Anti Form needs it).
    if let Some(f) = drive_cmd {
        let battle = dummies.iter().any(|d| d.respawn == 0);
        if f == 0 {
            if game.drive.form() != 0 && !game.drive.busy() {
                drive::revert(game, &mut dbg.log);
            }
        } else {
            drive::request(game, f, battle, &mut dbg.log);
        }
    }
    // MP Charge: refills over a fixed time, then MP is full again
    // (approximation: 10 seconds; the game's rate isn't read yet).
    if let Some(c) = game.mp_charge {
        let c = c + 1.0 / 600.0;
        if c >= 1.0 {
            game.mp_charge = None;
            game.mp = game.max_mp;
        } else {
            game.mp_charge = Some(c);
        }
    }

    let btn = |i: usize, l: &Latch| Btn { pressed: l.pressed[i], held: l.held[i] };
    let input = CInput { stick: [s.x, s.y], jump: btn(0, &latch), attack: btn(1, &latch), action: btn(2, &latch), lock: btn(3, &latch) };
    latch.pressed = [false; 4];

    let targets: Vec<TargetInfo> = dummies.iter().filter(|d| d.respawn == 0).map(|d| TargetInfo { id: d.id, pos: d.pos, radius: DUMMY_RADIUS, height: DUMMY_HEIGHT }).collect();
    // Lock-on switch: screen-right is toward smaller yaw in this world.
    if latch.switch != 0 {
        let dir = -(latch.switch as f32);
        latch.switch = 0;
        if game.character.switch_lock(dir, game.pos, &targets) {
            dbg.log.push("lock-on switched".into());
        }
    }
    let arena = Arena::get();
    let ledges = arena.ledges();
    let ground = arena.ground_below(game.pos);
    let env = Env {
        pos: game.pos,
        grounded: game.grounded,
        height_above_ground: game.pos[1] - ground,
        targets: &targets,
        ledges,
    };
    let out = game.character.step(&input, &env);
    // The cast motion's magic-cast event releases the spell (trigger 21);
    // the script itself runs below with the other spells.
    let new_cast = game.cast_now.take();
    let mut released = false;
    if let Some(i) = game.casting {
        let fired = out.events.iter().any(|e| matches!(e, character_core::EventKind::MagicCast(m) if *m as u16 == out.motion));
        if fired {
            game.casting = None;
            released = true;
            if let Some(sp) = game.kit.as_ref().map(|k| k.spells[i].clone()) {
                dbg.log.push(format!("{} released", sp.name));
            }
        } else if out.state != StateName::Magic {
            game.casting = None;
        }
    }

    // Move with the map collision (the game's solver): the frame's move,
    // plus a spell's recoil (Blizzard pushes Sora back) and the push out of
    // standing column dummies, all as one move.
    let start = Vec3::from(game.pos);
    let mut d = Vec3::from(out.displacement);
    // A reaction command's scripted movement (the enemy script's move
    // request).
    d += rc::frame(game, &dummies, out.motion);
    if let Some(sp) = game.spells.as_mut() {
        if let Some((dir, v)) = sp.recoil {
            d.x += dir.x * v;
            d.z += dir.z * v;
            let next = v * 0.9;
            sp.recoil = (next >= 1.0).then_some((dir, next));
        }
    }
    for dm in dummies.iter().filter(|d| d.respawn == 0 && d.foe.is_none()) {
        let p = start + d;
        let (dx, dz) = (p.x - dm.pos[0], p.z - dm.pos[2]);
        let dist = (dx * dx + dz * dz).sqrt();
        let min = SORA_RADIUS + DUMMY_RADIUS;
        if dist < min && p.y < dm.pos[1] + DUMMY_HEIGHT {
            let (nx, nz) = if dist > 1e-3 { (dx / dist, dz / dist) } else { (0.0, -1.0) };
            d.x += dm.pos[0] + nx * min - p.x;
            d.z += dm.pos[2] + nz * min - p.z;
        }
    }
    let air = game.character.airborne();
    let mv = if air { d } else { map_collision::along_ground(d, game.ground_normal) };
    let moved = game.body.solve(&arena.map, start, mv, air);
    let p = moved.pos.to_array();
    // A rise that made less than 49% of its height hit a ceiling.
    if map_collision::hit_ceiling(d.y, moved.pos.y - start.y) {
        game.character.hit_ceiling();
        dbg.log.push(format!("ceiling: rose {:.1} of {:.1}", moved.pos.y - start.y, d.y));
    }
    // Landing needs the move not to be going up (exe).
    game.grounded = moved.ground && (!air || mv.y <= 0.0);
    game.ground_normal = if game.grounded { moved.ground_normal(&arena.map) } else { Vec3::Y };
    let ground = arena.ground_below(p);
    game.ground_y = ground;
    game.vel_display = out.displacement;
    game.pos = p;
    // Drive: a change in progress, the form gauge.
    let battle = dummies.iter().any(|d| d.respawn == 0);
    drive::frame(game, battle, &mut dbg.log);
    if let Some(anim) = game.anim.as_mut() {
        // Out of combat while no enemy is up (KH2's field mode; the rule for
        // when the game switches is ours): the field idle/walk/run, and the
        // Keyblade away while Sora just moves.
        anim.field = !battle;
        anim.update(&out, &game.character, p);
    }

    // Hits: each live window's shapes, plus copies filling the gap since
    // last frame, tested against each target's shapes (the game's test).
    let live: std::collections::HashSet<(u64, u8)> = out.hitboxes.iter().chain(&out.guard_boxes).map(|h| (h.swing, h.weapon)).collect();
    game.volumes.retain(|k, _| live.contains(k));
    game.connected.retain(|s| live.iter().any(|k| k.0 == *s));
    game.clinked.retain(|s| live.iter().any(|k| k.0 == *s));
    game.sparks.retain_mut(|s| {
        s.1 -= 1;
        s.1 > 0
    });
    game.live_attacks.clear();
    game.sora_guards.clear();
    game.sora_hits.clear();
    for h in &out.guard_boxes {
        let shapes = attack_shapes(game, out.facing, h.group, h.weapon);
        let swept = game.volumes.entry((h.swing, h.weapon)).or_default().update(&shapes);
        game.live_attacks.extend(swept.iter().copied());
        game.sora_guards.push((*h, swept));
    }
    for h in &out.hitboxes {
        let shapes = attack_shapes(game, out.facing, h.group, h.weapon);
        let prev0 = game.volumes.get(&(h.swing, h.weapon)).and_then(|v| v.previous().first().copied());
        let swept = game.volumes.entry((h.swing, h.weapon)).or_default().update(&shapes);
        game.live_attacks.extend(swept.iter().copied());
        if game.clinked.contains(&h.swing) {
            // Bounced off a wall or another attack: no longer live.
            continue;
        }
        game.sora_hits.push((*h, swept.clone()));
        // Wall strike (exe): attacks flagged to hit the background cast a
        // ray from the first shape's last centre to its new one (from the
        // second frame on), then, if that misses, straight down by the
        // floor-check length. Only while the swing hasn't connected yet.
        let (bg, floor) = game.bg_flags.get(&h.attack).copied().unwrap_or((false, 0));
        if bg && !game.connected.contains(&h.swing) && !game.clinked.contains(&h.swing) {
            if let (Some(p0), Some(s0)) = (prev0, shapes.first()) {
                let (a, b) = (Vec3::from(p0), Vec3::from(s0.center));
                let mut at = arena.raycast(a, b);
                if at.is_none() && floor > 0 {
                    at = arena.raycast(b, b - Vec3::Y * floor as f32);
                }
                if let Some(at) = at {
                    game.clinked.insert(h.swing);
                    if game.character.register_wall_hit() {
                        game.sparks.push((at, 20));
                        dbg.log.push(format!("clink! atkp {} hit the wall", h.attack));
                    }
                    continue;
                }
            }
        }
        let Some(first) = magic::row(&game.battle, h.attack, 0, 0).cloned() else { continue };
        // The window's group: the row's combo group, else its own.
        let group = if first.combo_group != 0 { first.combo_group as i64 } else { h.swing as i64 };
        for mut dm in dummies.iter_mut() {
            if dm.respawn > 0 || dm.record.has_hit(h.swing) {
                continue;
            }
            let target = match &dm.foe {
                Some(st) if st.npc.is_dead() => continue,
                Some(st) => st.hurt_shapes(),
                None => vec![dummy_shape(dm.pos)],
            };
            if first_overlap(&swept, &target).is_none() {
                continue;
            }
            let adm = dm.record.admit(group, false, false);
            let Some(row) = magic::row(&game.battle, h.attack, 0, adm.count).cloned() else { continue };
            dm.record.mark(h.swing, row.interval);
            if !adm.effective {
                continue;
            }
            let attack = kh2_data::attack_def(&row);
            let r = resolve_hit(&game.rules, &attack, &game.sora, &mut dm.combatant);
            if r.blocked.is_none() {
                game.connected.insert(h.swing);
                game.character.register_hit();
                drive::on_hit(game, row.drive_drain, &mut dbg.log);
                let fwd = [out.facing.sin(), out.facing.cos()];
                if dm.foe.is_some() {
                    // The game's generic reaction (flinch / push / launch).
                    foes::react(&mut dm, &row, out.facing, r.killed);
                } else {
                    // Column dummy: ATKP strength spread over 10 frames
                    // (approximation).
                    let k = r.knockback.0.clamp(0.0, 400.0) / 10.0 * 0.3;
                    dm.knock = [fwd[0] * k, fwd[1] * k];
                    dm.knock_frames = 10;
                }
                dm.flash = 6;
                let line = format!(
                    "hit Soldier#{} with atkp {} hit {} ({}%): {} dmg  HP {}/{}{}",
                    dm.id - 99,
                    h.attack,
                    adm.count + 1,
                    attack.power,
                    r.damage,
                    dm.combatant.hp,
                    dm.combatant.max_hp,
                    if r.killed { "  KO" } else { "" }
                );
                dbg.log.push(line);
                if r.killed {
                    dm.respawn = 120;
                    let status = game.foe_kit.as_ref().map_or(0, |k| k.data.status);
                    drive::on_kill(game, status, &mut dbg.log);
                }
            }
        }
    }

    // Spells: a cast that started this frame (trigger 20), the release
    // (trigger 21), then every spell script runs once.
    if game.spells.is_some() {
        spell_frame(game, &out, new_cast, released, arena, &mut dummies, &mut dbg);
    }

    if auto.trace {
        for l in &dbg.log[log_before.min(dbg.log.len())..] {
            eprintln!("log f{} {l}", game.frame);
        }
    }
    let log_mid = dbg.log.len();
    // Soldiers act, then their attacks meet Sora's guard, attacks and body.
    let kit = game.foe_kit.take();
    if let Some(kit) = kit.as_ref() {
        for mut dm in dummies.iter_mut() {
            let dead_hidden = dm.respawn > 0 && dm.foe.as_ref().map_or(true, |s| !s.npc.is_dead());
            if !dead_hidden {
                foes::step(&mut dm, kit, game.pos, arena);
                if dm.foe.as_ref().is_some_and(|s| !s.npc.is_dead()) {
                    let push = foes::separate(&mut dm, kit, game.pos, SORA_RADIUS, game.weight);
                    if push != [0.0; 2] {
                        let mv = Vec3::new(push[0], 0.0, push[1]);
                        let air = !game.grounded;
                        let mv = if air { mv } else { map_collision::along_ground(mv, game.ground_normal) };
                        let m = game.body.solve(&arena.map, Vec3::from(game.pos), mv, air);
                        game.pos = m.pos.to_array();
                    }
                }
            }
        }
        let sora_hurt = game.anim.as_ref().map(|a| a.hurt_shapes()).unwrap_or_default();
        let sora_facing = out.facing;
        let barriers = game.spells.as_ref().map(|sp| magic::barriers(&sp.magic, &game.battle)).unwrap_or_default();
        let mut live_pairs = std::collections::HashSet::new();
        for mut dm in dummies.iter_mut() {
            let foe_pos = dm.pos;
            let Dummy { foe, combatant, id, .. } = &mut *dm;
            let Some(st) = foe.as_mut() else { continue };
            if st.npc.is_dead() {
                continue;
            }
            let foe_facing = st.npc.facing();
            let to_foe = {
                let d = [foe_pos[0] - game.pos[0], foe_pos[2] - game.pos[2]];
                let l = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-3);
                [d[0] / l, d[1] / l]
            };
            for (h, shapes) in st.live.clone() {
                live_pairs.insert((*id, h.swing));
                if st.spent.contains(&h.swing) || game.sora_hit_by.contains(&(*id, h.swing)) {
                    continue;
                }
                let Some(fatk) = game.atkp_raw.get(&h.attack).cloned() else { continue };
                // A Reflect barrier bounces the attack and counts it (the
                // spell's script then fires back).
                if let Some((key, _, b)) = barriers.iter().find(|(_, r, b)| foes::clashes(r, &fatk) && foes::overlap(b, &shapes)) {
                    st.spent.insert(h.swing);
                    game.sora_hit_by.insert((*id, h.swing));
                    foes::foe_clash_reaction(st, &fatk, sora_facing);
                    if let Some(a) = game.spells.as_mut().and_then(|sp| sp.magic.attack_mut(*key)) {
                        a.reflect_count = a.reflect_count.saturating_add(1);
                    }
                    if let Some(c) = b.first() {
                        game.sparks.push((Vec3::from(c.center), 20));
                    }
                    dbg.log.push(format!("Reflect blocked Soldier#{}'s atkp {}", *id - 99, h.attack));
                    continue;
                }
                // Clash with Sora's guard or attack (exe table and angles).
                let mut clashed = None;
                for (sh, sshapes) in game.sora_guards.iter().chain(game.sora_hits.iter()) {
                    let Some(satk) = game.atkp_raw.get(&sh.attack) else { continue };
                    if !foes::clashes(satk, &fatk) {
                        continue;
                    }
                    let ok = character_core::reaction::clash_angle_ok(satk.reflect_range, satk.reflect_angle, sora_facing, [foe_facing.sin(), foe_facing.cos()])
                        && character_core::reaction::clash_angle_ok(fatk.reflect_range, fatk.reflect_angle, foe_facing, [sora_facing.sin(), sora_facing.cos()]);
                    if ok && foes::overlap(sshapes, &shapes) {
                        clashed = Some((*sh, satk.clone(), sshapes.first().map(|s| s.center)));
                        break;
                    }
                }
                if let Some((sh, satk, at)) = clashed {
                    st.spent.insert(h.swing);
                    game.sora_hit_by.insert((*id, h.swing));
                    if satk.refact_self != 2 {
                        game.clinked.insert(sh.swing);
                    }
                    game.character.clash(satk.refact_self, fatk.refact_other, to_foe);
                    foes::foe_clash_reaction(st, &fatk, sora_facing);
                    if let Some(c) = at {
                        game.sparks.push((Vec3::from(c), 20));
                    }
                    dbg.log.push(format!("clang! Sora's atkp {} met Soldier#{}'s atkp {}", sh.attack, *id - 99, h.attack));
                    continue;
                }
                // Sora's body.
                if out.invulnerable || !foes::overlap(&shapes, &sora_hurt) {
                    continue;
                }
                // Sora's hit record: once per attack; the player side also
                // refuses other attack groups for 30 frames after a hit.
                let key = ((*id as u64) << 40) | h.swing;
                if game.sora_record.has_hit(key) {
                    continue;
                }
                st.spent.insert(h.swing);
                game.sora_hit_by.insert((*id, h.swing));
                let group = if fatk.combo_group != 0 { fatk.combo_group as i64 } else { key as i64 };
                let adm = game.sora_record.admit(group, false, true);
                let Some(row) = magic::row(&game.battle, h.attack, 0, adm.count).cloned() else { continue };
                game.sora_record.mark(key, row.interval);
                if !adm.effective {
                    continue;
                }
                let hp_before = game.sora.hp;
                let mut r = resolve_hit(&game.rules, &kh2_data::attack_def(&row), combatant, &mut game.sora);
                if game.drive.form() == character_core::drive::form::ANTI && r.damage > 0 {
                    // Anti Form takes 1.5x damage (exe).
                    let dmg = (r.damage as f32 * game.drive.drive.params.anti_damage).ceil() as u32;
                    game.sora.hp = hp_before.saturating_sub(dmg);
                    r.damage = dmg;
                }
                if r.blocked.is_none() {
                    game.character.take_hit(&character_core::reaction::Hit {
                        reaction: row.enemy_reaction,
                        knockback: (row.knockback_1, row.knockback_2),
                        dir: [foe_facing.sin(), foe_facing.cos()],
                        part: 0,
                    });
                    dbg.log.push(format!("Soldier#{} hit Sora with atkp {}: {} dmg  HP {}/{}", *id - 99, h.attack, r.damage, game.sora.hp, game.sora.max_hp));
                    if game.sora.hp == 0 {
                        // A KO in a form reverts first (exe).
                        drive::revert(game, &mut dbg.log);
                        game.sora.hp = game.sora.max_hp;
                        dbg.log.push("Sora would be KO'd: HP refilled (playground)".into());
                    }
                }
            }
        }
        game.sora_hit_by.retain(|k| live_pairs.contains(k));
    }
    game.foe_kit = kit;
    // The reaction command offered for next frame.
    rc::select(game, &dummies);

    // Hit records: a frame passes; attacks that ended are forgotten.
    let mut swings_now: std::collections::HashSet<u64> = out.hitboxes.iter().chain(&out.guard_boxes).map(|h| h.swing).collect();
    for dm in dummies.iter() {
        if let Some(st) = &dm.foe {
            swings_now.extend(st.live.iter().chain(&st.guards).map(|(h, _)| ((dm.id as u64) << 40) | h.swing));
        }
    }
    let ended: Vec<u64> = game.live_swings.difference(&swings_now).copied().collect();
    for mut dm in dummies.iter_mut() {
        for k in &ended {
            dm.record.forget(*k);
        }
        // The group timer waits while a hit-reaction motion (10-39) plays.
        let frozen = dm.foe.as_ref().and_then(|s| s.out.as_ref()).is_some_and(|o| (10..40).contains(&o.motion));
        dm.record.tick(1.0, frozen);
    }
    for k in &ended {
        game.sora_record.forget(*k);
    }
    game.sora_record.tick(1.0, (10..40).contains(&out.motion));
    game.live_swings = swings_now;

    if auto.trace {
        for l in &dbg.log[log_mid.min(dbg.log.len())..] {
            eprintln!("log f{} {l}", game.frame);
        }
        for d in dummies.iter() {
            if let Some(o) = d.foe.as_ref().and_then(|s| s.out.as_ref()) {
                if game.frame % 10 == 0 {
                    eprintln!("foe f{} #{} {:?} m{} cf{} pos ({:.0}, {:.0}, {:.0}) hp {}", game.frame, d.id - 99, o.state, o.motion, o.clip_frame, d.pos[0], d.pos[1], d.pos[2], d.combatant.hp);
                }
            }
        }
    }

    if dbg.log.len() > 8 {
        let n = dbg.log.len() - 8;
        dbg.log.drain(..n);
    }

    // F6: send the enemies away (held hidden, short of respawning) or
    // bring them back.
    if std::mem::take(&mut latch.away) {
        game.enemies_away = !game.enemies_away;
        dbg.log.push(if game.enemies_away { "enemies sent away (out of combat)".into() } else { "enemies back".into() });
        if !game.enemies_away {
            for mut dm in dummies.iter_mut() {
                dm.respawn = 1;
            }
        }
    }
    if game.enemies_away {
        for mut dm in dummies.iter_mut() {
            dm.respawn = 60;
        }
    }
    // Dummies.
    for mut dm in dummies.iter_mut() {
        if dm.knock_frames > 0 {
            dm.knock_frames -= 1;
            let k = dm.knock;
            dm.pos[0] += k[0];
            dm.pos[2] += k[1];
        }
        dm.flash = dm.flash.saturating_sub(1);
        if dm.respawn > 0 {
            dm.respawn -= 1;
            if dm.respawn == 0 {
                dm.combatant.hp = dm.combatant.max_hp;
                dm.pos = dm.home;
                if let (Some(kit), true) = (&game.foe_kit, dm.foe.is_some()) {
                    let seed = dm.id.wrapping_mul(31).wrapping_add(game.frame as u32);
                    dm.foe = Some(foes::FoeState::new(kit, seed, PI));
                }
            }
        }
    }

    if auto.trace {
        eprintln!("trace f{} {:?} m{} cf{} pos ({:.1}, {:.1}, {:.1})", game.frame, out.state, out.motion, out.clip_frame, game.pos[0], game.pos[1], game.pos[2]);
    }
    dbg.last = Some(format!("{:?}", out.state));
    game.last = Some(out);
}

/// One frame of spells: snapshot the world for the scripts, start a new
/// cast, release, run every script, then apply what their attacks did.
fn spell_frame(game: &mut Game, out: &Output, new_cast: Option<(usize, bool)>, released: bool, arena: &Arena, dummies: &mut Query<&mut Dummy>, dbg: &mut Debug) {
    let Some(mut sp) = game.spells.take() else { return };
    let mut objs = vec![magic::Snap::from_anim(magic::SORA, game.pos, out.facing, out.motion, true, !game.grounded, !game.grounded, game.anim.as_ref(), 90.0)];
    for dm in dummies.iter() {
        let alive = dm.respawn == 0 && dm.foe.as_ref().map_or(true, |s| !s.npc.is_dead());
        let (facing, motion, anim, air) = match &dm.foe {
            Some(st) => (st.npc.facing(), st.out.as_ref().map_or(0, |o| o.motion), Some(&st.anim), !st.grounded),
            None => (0.0, 0, None, false),
        };
        objs.push(magic::Snap::from_anim(dm.id, dm.pos, facing, motion, alive, air, air, anim, 60.0));
    }
    // Sora's target: the lock-on, else the nearest Soldier in front within
    // 1500, else the nearest (approximation of the game's search).
    let fwd = Vec2::new(out.facing.sin(), out.facing.cos());
    let here = Vec2::new(game.pos[0], game.pos[2]);
    let mut cands: Vec<(f32, bool, u32)> = objs
        .iter()
        .filter(|o| o.id != magic::SORA && o.alive)
        .map(|o| {
            let d = Vec2::new(o.pos.x, o.pos.z) - here;
            (d.length(), d.dot(fwd) >= 0.0 && d.length() < 1500.0, o.id)
        })
        .collect();
    cands.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.total_cmp(&b.0)));
    let target = out.locked.filter(|id| objs.iter().any(|o| o.id == *id && o.alive)).or(cands.first().map(|c| c.2));
    let abilities = game.character.abilities.equipped.clone();
    let centers: HashMap<u32, Vec3> = objs.iter().map(|o| (o.id, o.center)).collect();
    let magic::Spells { magic: m, rng, magi, file_of, object_scripts, .. } = &mut sp;
    let mut world = magic::PlayWorld { objs, arena, abilities: &abilities, target, rng, hitbacks: Vec::new() };

    if let (Some((i, finisher)), Some(kit)) = (new_cast, game.kit.as_ref()) {
        if let (Some(Some(file)), Some(info)) = (file_of.get(i), kit.spells.get(i)) {
            let sub = info.level as i32 * 2 - 2 + finisher as i32 + drive::magic_sub(game.drive.form());
            let row = magi.get(sub).map(|r| r.to_vec()).unwrap_or_else(|| vec![0; kh2_formats::pref::MagiTable::RECORD]);
            if let Err(e) = m.cast(*file, magic::SORA, info.element, sub, &row, &mut world) {
                dbg.log.push(format!("{}: {e}", info.name));
            }
        }
    }
    if released {
        m.release(magic::SORA, &mut world);
    }
    // Frame trigger 1 on a form with a script (Wisdom's shots).
    if let Some(&file) = object_scripts.get(&game.drive.form()) {
        for e in &out.events {
            if let character_core::EventKind::Effect(n) = e {
                if let Err(err) = m.object_effect(file, magic::SORA, *n, &mut world) {
                    dbg.log.push(format!("Wisdom script: {err}"));
                }
            }
        }
    }
    let events = m.update(&mut world);
    let hitbacks = std::mem::take(&mut world.hitbacks);
    drop(world);
    for (o, d, dist) in hitbacks {
        if o == magic::SORA {
            sp.recoil = Some((d, dist * 0.1));
        }
    }
    let strikes = magic::handle_events(events, &mut dbg.log);
    let m = &mut sp.magic;

    // Direct applications (Cure): heal Sora.
    for (key, target) in strikes {
        let Some(a) = m.attack_mut(key) else { continue };
        let (atk, sub) = (a.atk, a.sub);
        if target != magic::SORA {
            continue;
        }
        let Some(row) = magic::row(&game.battle, atk, sub, 0).cloned() else { continue };
        let caster = game.sora.clone();
        let r = resolve_hit(&game.rules, &kh2_data::attack_def(&row), &caster, &mut game.sora);
        dbg.log.push(format!("Cure: +{} HP (atkp {}, {}%)  HP {}/{}", r.healed, atk, row.power, game.sora.hp, game.sora.max_hp));
    }

    // Spell attacks against the Soldiers.
    magic::bg_hits(m, &game.battle, arena);
    for mut dm in dummies.iter_mut() {
        if dm.respawn > 0 || dm.foe.as_ref().is_some_and(|s| s.npc.is_dead()) {
            continue;
        }
        let hurt = match &dm.foe {
            Some(st) => st.hurt_shapes(),
            None => vec![dummy_shape(dm.pos)],
        };
        let foe_pos = Vec3::from(dm.pos);
        let hits = {
            let Dummy { record, id, .. } = &mut *dm;
            magic::hits_on(m, &game.battle, *id, foe_pos, &hurt, record)
        };
        for h in hits {
            let attack = kh2_data::attack_def(&h.row);
            let r = resolve_hit(&game.rules, &attack, &game.sora, &mut dm.combatant);
            if r.blocked.is_some() {
                continue;
            }
            drive::on_hit(game, h.row.drive_drain, &mut dbg.log);
            let name = game.kit.as_ref().and_then(|k| k.spells.iter().find(|s| s.file == m.spells[h.spell].name)).map_or(if sp.element_of.get(h.spell) == Some(&255) { "Wisdom shot".to_string() } else { "Spell".to_string() }, |s| s.name.clone());
            if h.row.enemy_reaction == 5 && dm.foe.is_some() {
                // Magnet: caught and pulled (exe reaction 5).
                let (k1, k2) = (h.row.knockback_1 as f32, h.row.knockback_2 as f32);
                let radius = k1 + magic::rand(&mut sp.rng) * (k2 - k1);
                let swirl = h.row.unknown_16 as f32;
                let st = dm.foe.as_mut().unwrap();
                let fwd = st.npc.forward();
                let front = fwd[0] * h.dir[0] + fwd[1] * h.dir[1] < 0.0;
                st.npc.suction(front, radius, swirl);
                st.caught_by = Some(h.key);
                if r.killed {
                    st.npc.die(foes::soldier::DEATH);
                }
            } else if dm.foe.is_some() {
                foes::react(&mut dm, &h.row, h.dir[0].atan2(h.dir[1]), r.killed);
            }
            dm.flash = 6;
            dbg.log.push(format!(
                "{name} hit Soldier#{} with atkp {} hit {} ({}%): {} dmg  HP {}/{}{}",
                h.foe - 99,
                h.atk,
                h.count + 1,
                h.row.power,
                r.damage,
                dm.combatant.hp,
                dm.combatant.max_hp,
                if r.killed { "  KO" } else { "" }
            ));
            if r.killed {
                dm.respawn = 120;
                let status = game.foe_kit.as_ref().map_or(0, |k| k.data.status);
                drive::on_kill(game, status, &mut dbg.log);
            }
        }
    }
    // Magnet: pull toward the attack's centre while it lives.
    for mut dm in dummies.iter_mut() {
        let id = dm.id;
        let Some(st) = dm.foe.as_mut() else { continue };
        let Some(key) = st.caught_by else { continue };
        let at = m.attacks.iter().find(|a| a.key == key && !a.freed).and_then(|a| a.pos);
        match (at, centers.get(&id)) {
            (Some(c), Some(body)) => st.npc.suction_to = Some((c - *body).into()),
            _ => {
                st.npc.suction_to = None;
                st.caught_by = None;
            }
        }
    }
    // Attacks the scripts freed are forgotten by everyone they hit.
    let freed: Vec<u64> = m.freed().filter(|a| a.owns_id).map(|a| a.id | magic::SPELL_KEY).collect();
    for mut dm in dummies.iter_mut() {
        for k in &freed {
            dm.record.forget(*k);
        }
    }
    game.spells = Some(sp);
}

impl Game {
}

fn sync_segments(
    game: Res<Game>,
    mut pieces: Query<(&Piece, &mut Transform, &MeshMaterial3d<StandardMaterial>, &mut Visibility)>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut glow: Local<Option<[f32; 3]>>,
) {
    let (Some(anim), Some(out)) = (&game.anim, &game.last) else { return };
    // State glow: white while invulnerable, cyan while guarding.
    let want = if out.invulnerable {
        [0.9, 0.9, 0.9]
    } else if out.guarding {
        [0.0, 0.6, 0.8]
    } else {
        [0.0; 3]
    };
    let relight = *glow != Some(want);
    *glow = Some(want);
    // The Keyblade is put away out of combat while Sora just moves.
    let weapons = if anim.weapons_out() { Visibility::Inherited } else { Visibility::Hidden };
    for (piece, mut t, mat, mut vis) in &mut pieces {
        if matches!(piece.part, Part::Weapon(_)) && *vis != weapons {
            *vis = weapons;
        }
        if relight && piece.part == Part::Body {
            if let Some(m) = mats.get_mut(&mat.0) {
                m.emissive = LinearRgba::rgb(want[0], want[1], want[2]);
            }
        }
        match piece.shape {
            Shape::Capsule(a, b, r) => {
                let (Some(a), Some(b)) = (anim.anchor(piece.part, a), anim.anchor(piece.part, b)) else { continue };
                let (a, b) = (a * SCALE, b * SCALE);
                let _ = r;
                *t = Transform {
                    translation: (a + b) * 0.5,
                    rotation: Quat::from_rotation_arc(Vec3::Y, (b - a).normalize_or(Vec3::Y)),
                    scale: Vec3::ONE,
                };
            }
            Shape::Sphere(c, r) => {
                let Some(c) = anim.anchor(piece.part, c) else { continue };
                *t = Transform::from_translation(c * SCALE).with_scale(Vec3::splat(r * 2.0 * SCALE));
            }
        }
    }
}

fn sync_visuals(
    game: Res<Game>,
    mut player: Query<&mut Transform, (With<Player>, Without<PlayerBody>, Without<PlayerBlade>, Without<Dummy>)>,
    mut body: Query<(&mut Transform, &MeshMaterial3d<StandardMaterial>), (With<PlayerBody>, Without<Player>, Without<PlayerBlade>)>,
    mut blade: Query<&mut Transform, (With<PlayerBlade>, Without<Player>, Without<PlayerBody>, Without<Dummy>)>,
    mut dummies: Query<(&Dummy, &mut Transform, &mut Visibility), (Without<Player>, Without<PlayerBody>, Without<PlayerBlade>)>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let Some(out) = &game.last else { return };
    if let Ok(mut t) = player.get_single_mut() {
        t.translation = Vec3::from(game.pos) * SCALE;
        t.rotation = Quat::from_rotation_y(out.facing);
    }
    if let Ok((mut t, mat)) = body.get_single_mut() {
        let base = Vec3::new(0.0, SORA_HEIGHT * 0.5 * SCALE, 0.0);
        let (rot, off) = match out.state {
            StateName::Glide => (Quat::from_rotation_x(1.2), Vec3::new(0.0, 0.2, 0.0)),
            StateName::DodgeRoll => (Quat::from_rotation_x(out.clip_frame as f32 * 0.35), Vec3::new(0.0, -0.3, 0.0)),
            StateName::QuickRun => (Quat::from_rotation_x(0.5), Vec3::ZERO),
            StateName::Guard => (Quat::from_rotation_x(-0.15), Vec3::ZERO),
            StateName::Land => (Quat::IDENTITY, Vec3::new(0.0, -0.15, 0.0)),
            _ => (Quat::IDENTITY, Vec3::ZERO),
        };
        t.translation = base + off;
        t.rotation = rot;
        if let Some(m) = mats.get_mut(&mat.0) {
            m.base_color = if out.invulnerable {
                Color::srgb(1.0, 1.0, 1.0)
            } else if out.guarding {
                Color::srgb(0.3, 0.9, 1.0)
            } else if out.state == StateName::Attack {
                Color::srgb(1.0, 0.45, 0.2)
            } else {
                Color::srgb(0.85, 0.2, 0.25)
            };
        }
    }
    if let Ok(mut t) = blade.get_single_mut() {
        if out.hitboxes.is_empty() {
            *t = Transform::from_xyz(0.35, 0.9, 0.1).with_rotation(Quat::from_rotation_x(-0.6));
        } else {
            let sweep = (out.clip_frame % 12) as f32 / 12.0;
            let (_, dir) = blade_pose([0.0; 3], 0.0, Some(sweep));
            let local = Quat::from_rotation_arc(Vec3::Z, dir);
            *t = Transform::from_xyz(0.0, 0.95, 0.25).with_rotation(local) * Transform::from_xyz(0.0, 0.0, 0.5);
        }
    }
    for (d, mut t, mut vis) in &mut dummies {
        t.translation = Vec3::new(d.pos[0], d.pos[1] + DUMMY_HEIGHT * 0.5, d.pos[2]) * SCALE;
        t.scale = if d.flash > 0 { Vec3::splat(1.12) } else { Vec3::ONE };
        *vis = if d.respawn > 0 { Visibility::Hidden } else { Visibility::Inherited };
    }
}

fn draw_debug(game: Res<Game>, dbg: Res<Debug>, dummies: Query<&Dummy>, mut gizmos: Gizmos) {
    let Some(out) = &game.last else { return };
    for d in &dummies {
        if d.respawn > 0 {
            continue;
        }
        let top = Vec3::new(d.pos[0], d.pos[1] + DUMMY_HEIGHT + 25.0, d.pos[2]) * SCALE;
        let frac = d.combatant.hp as f32 / d.combatant.max_hp.max(1) as f32;
        let w = 0.8;
        gizmos.line(top - Vec3::X * w / 2.0, top + Vec3::X * w / 2.0, Color::srgb(0.3, 0.0, 0.0));
        gizmos.line(top - Vec3::X * w / 2.0, top + Vec3::X * (w * frac - w / 2.0), Color::srgb(0.2, 1.0, 0.3));
        if out.locked == Some(d.id) {
            let iso = Isometry3d::new(Vec3::new(d.pos[0], d.pos[1] + 2.0, d.pos[2]) * SCALE, Quat::from_rotation_x(FRAC_PI_2));
            gizmos.circle(iso, (DUMMY_RADIUS + 25.0) * SCALE, Color::srgb(1.0, 0.9, 0.2));
        }
    }
    for (p, left) in &game.sparks {
        let c = *p * SCALE;
        let r = 0.15 + 0.015 * (20 - *left) as f32;
        let col = Color::srgb(1.0, 0.95, 0.5);
        for d in [Vec3::X, Vec3::Y, Vec3::Z, Vec3::new(1.0, 1.0, 0.0).normalize(), Vec3::new(0.0, 1.0, 1.0).normalize(), Vec3::new(1.0, 0.0, 1.0).normalize()] {
            gizmos.line(c - d * r, c + d * r, col);
        }
    }
    let guard = Color::srgb(0.2, 0.9, 1.0);
    let attack = Color::srgb(1.0, 0.2, 0.2);
    if dbg.show_boxes {
        for s in &game.live_attacks {
            draw_shape(&mut gizmos, s, if out.guarding { guard } else { attack });
        }
        for d in &dummies {
            if let Some(st) = &d.foe {
                for (_, shapes) in &st.live {
                    for s in shapes {
                        draw_shape(&mut gizmos, s, Color::srgb(1.0, 0.5, 0.1));
                    }
                }
                for (_, shapes) in &st.guards {
                    for s in shapes {
                        draw_shape(&mut gizmos, s, guard);
                    }
                }
            }
        }
    }
    if dbg.show_hurt {
        if let Some(anim) = &game.anim {
            for s in anim.hurt_shapes() {
                draw_shape(&mut gizmos, &s, Color::srgba(0.3, 0.7, 1.0, 0.6));
            }
        }
        for d in dummies.iter().filter(|d| d.respawn == 0) {
            let shapes = match &d.foe {
                Some(st) => st.hurt_shapes(),
                None => vec![dummy_shape(d.pos)],
            };
            for s in shapes {
                draw_shape(&mut gizmos, &s, Color::srgba(0.3, 0.7, 1.0, 0.6));
            }
        }
    }
    if let (true, Some(anim)) = (dbg.show_other, &game.anim) {
        for k in [rig::kind::BG, rig::kind::OBJ, rig::kind::TARGET, rig::kind::BG_PLAYER, 5, 7, 8, 9, 10, 11, 12, 14, 15, 16, 17, 18, 19, 20] {
            let hue = (k as f32 * 47.0) % 360.0;
            for (s, _) in anim.shapes_of_kind(k) {
                draw_shape(&mut gizmos, &s, Color::hsla(hue, 0.7, 0.6, 0.7));
            }
        }
    }
    let p = Vec3::from(game.pos) * SCALE;
    let iso = Isometry3d::new(p + Vec3::Y * 0.02, Quat::from_rotation_x(FRAC_PI_2));
    gizmos.circle(iso, SORA_RADIUS * SCALE, if out.invulnerable { Color::WHITE } else { Color::srgb(0.2, 0.6, 1.0) });
}

/// Draws a collision shape as wire lines (KH2 units in, scene units out).
fn draw_shape(gizmos: &mut Gizmos, s: &CShape, color: Color) {
    let c = Vec3::from(s.center) * SCALE;
    let (r, h) = (s.radius * SCALE, s.height * SCALE);
    let flat = |y: f32| Isometry3d::new(c + Vec3::Y * y, Quat::from_rotation_x(FRAC_PI_2));
    match s.kind {
        ShapeKind::Sphere => {
            gizmos.sphere(Isometry3d::from_translation(c), r, color);
        }
        ShapeKind::Ellipsoid => {
            gizmos.ellipse(flat(0.0), Vec2::new(r, r), color);
            gizmos.ellipse(Isometry3d::from_translation(c), Vec2::new(r, h), color);
            gizmos.ellipse(Isometry3d::new(c, Quat::from_rotation_y(FRAC_PI_2)), Vec2::new(r, h), color);
        }
        ShapeKind::Column => {
            gizmos.circle(flat(h), r, color);
            gizmos.circle(flat(-h), r, color);
            for k in 0..4 {
                let a = k as f32 * FRAC_PI_2;
                let o = Vec3::new(a.cos() * r, 0.0, a.sin() * r);
                gizmos.line(c + o - Vec3::Y * h, c + o + Vec3::Y * h, color);
            }
        }
        ShapeKind::Cube => {
            gizmos.cuboid(Transform::from_translation(c).with_scale(Vec3::new(2.0 * r, 2.0 * h, 2.0 * r)), color);
        }
    }
}

fn draw_skeleton(game: Res<Game>, dbg: Res<Debug>, dummies: Query<&Dummy>, mut gizmos: Gizmos) {
    if !dbg.show_skeleton {
        return;
    }
    if let Some(anim) = &game.anim {
        for (a, b, part) in anim.skeleton_lines() {
            let color = if matches!(part, Part::Weapon(_)) { Color::srgb(1.0, 0.85, 0.2) } else { Color::srgb(0.9, 1.0, 0.9) };
            gizmos.line(a * SCALE, b * SCALE, color);
        }
    }
    for d in &dummies {
        if let Some(st) = &d.foe {
            for (a, b, _) in st.anim.skeleton_lines() {
                gizmos.line(a * SCALE, b * SCALE, Color::srgb(0.8, 0.6, 1.0));
            }
        }
    }
}

fn hud(
    mut commands: Commands,
    game: Res<Game>,
    dbg: Res<Debug>,
    mut q: Query<&mut Text, With<Hud>>,
    mut spawned: Local<bool>,
) {
    if !*spawned {
        commands.spawn((
            Hud,
            Text::new(""),
            TextFont { font_size: 14.0, ..default() },
            TextColor(Color::srgb(0.92, 0.92, 0.95)),
            Node { position_type: PositionType::Absolute, top: Val::Px(10.0), left: Val::Px(12.0), ..default() },
        ));
        *spawned = true;
        return;
    }
    let Some(out) = &game.last else { return };
    let a = &game.character.abilities;
    let lv = |v: u8| if v == 0 { "-".to_string() } else if v == 4 { "MAX".to_string() } else { format!("LV{v}") };
    let mut s = format!(
        "state {:?}  motion {}  frame {}  combo {}  {}\n\
         pos ({:.0}, {:.0}, {:.0})  move/frame ({:.1}, {:.1}, {:.1})  {}\n\
         Sora LV{} HP {}/{} (Str {} Mag {} Def {})  {}: Soldier LV{}\n\
         High Jump {}  Quick Run {}  Aerial Dodge {}  Glide {}  Dodge Roll {}  Combo Plus x{}  Air Combo Plus x{}  Finishing Plus x{}\n\
         [Esc] abilities  [Up/Down] menu (J ok, Space back)  [Ctrl] shortcuts  [1-6] cast  [F1-F4] debug  [F5] refill Drive  [F6] no enemies  [R] reset\n\
         WASD move (Alt walk)  Space jump (in air: aerial dodge)  J/LMB attack  K/Shift guard / roll / quick run / glide (air)  Tab lock-on  Q/E/RMB camera\n",
        out.state,
        out.motion,
        out.clip_frame,
        out.combo_count,
        if out.invulnerable { "INVULNERABLE" } else if out.guarding { "GUARD" } else { "" },
        game.pos[0],
        game.pos[1],
        game.pos[2],
        game.vel_display[0],
        game.vel_display[1],
        game.vel_display[2],
        if game.grounded { "grounded" } else { "airborne" },
        game.level,
        game.sora.hp,
        game.sora.max_hp,
        game.sora.stats.strength,
        game.sora.stats.magic,
        game.sora.stats.defense,
        if game.foe_kit.is_some() { "enemies" } else { "dummies" },
        game.dummy_level,
        lv(a.high_jump),
        lv(a.quick_run),
        lv(a.aerial_dodge),
        lv(a.glide),
        lv(a.dodge_roll),
        a.combo_plus,
        a.air_combo_plus,
        a.finishing_plus,
    );
    for l in &dbg.log {
        s.push_str(l);
        s.push('\n');
    }
    if let Ok(mut t) = q.get_single_mut() {
        t.0 = s;
    }
}

/// `--screenshot N`: save `local/screenshot.png` after N frames, then exit.
fn screenshot(mut commands: Commands, mut shot: ResMut<Shot>, mut exit: EventWriter<AppExit>) {
    let Some(after) = shot.after_frames else { return };
    shot.frames += 1;
    if shot.frames == after {
        use bevy::render::view::screenshot::{save_to_disk, Screenshot};
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk("local/screenshot.png"));
    }
    if shot.frames == after + 30 {
        exit.send(AppExit::Success);
    }
}
