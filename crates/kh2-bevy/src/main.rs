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

mod foes;
mod hud;
mod magic;
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
use combat_core::{first_overlap, AttackDef, AttackVolume, Combatant, DamageRules, HitTracker, Shape as CShape, ShapeKind, Stats, Team};
use kh2_data::sora::BladeSphere;
use kh2_data::{Install, Source};

use rig::{Paint, Part, Piece, RigAnim, Shape};
use world::{Arena, ARENA_BOXES};

/// KH2 units (about 1 cm) to Bevy meters.
const SCALE: f32 = 0.01;
const SORA_RADIUS: f32 = 35.0;
const SORA_HEIGHT: f32 = 150.0;
const DUMMY_RADIUS: f32 = 50.0;
const DUMMY_HEIGHT: f32 = 110.0;
/// Shadow (ENMP id 34).
const DUMMY_ENEMY: u16 = 34;

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
    let cam_distance: f32 = arg("--cam-distance").and_then(|v| v.parse().ok()).unwrap_or(650.0);

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
    // The Shadows: their own files from the same place.
    let enemy = match (&loaded, synthetic) {
        (Ok(d), false) => {
            let r = match (&used, arg("--data-dir")) {
                (Some(i), _) => i.load_enemy(source, kh2_data::enemy::SHADOW, &d.combat),
                (None, Some(dir)) => kh2_data::enemy::load_enemy_dir(std::path::Path::new(&dir), kh2_data::enemy::SHADOW, &d.combat),
                _ => Err(kh2_data::Error::Format("no install".into())),
            };
            match r {
                Ok(e) => Some(e),
                Err(e) => {
                    eprintln!("note: the Shadow could not be loaded ({e}); using column dummies");
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
    .insert_resource(CameraRig { yaw: 0.0, pitch: 0.35, distance: cam_distance, manual_timer: 0.0 })
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
            let mut character = data.character.clone();
            character.abilities = Abilities::max_growth();
            // Sub 0 of every attack id, at Sora's level (the game picks
            // level variants this way).
            let ids: std::collections::BTreeSet<u16> = data.combat.battle.atkp.iter().filter(|a| a.sub() == 0).map(|a| a.id).collect();
            let attacks: HashMap<u16, AttackDef> = ids
                .into_iter()
                .filter_map(|id| data.combat.attack_at_level(id, 0, level as u8).map(|a| (id, a)))
                .collect();
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
            app.insert_resource(Game {
                anim,
                character,
                pos: [0.0, 0.0, -400.0],
                vel_display: [0.0; 3],
                last: None,
                sora: Combatant::new(1, Team::Player, stats),
                level,
                attacks,
                blade: data.blade_groups.clone(),
                tracker: HitTracker::new(),
                rules: DamageRules::default(),
                frame: 0,
                grounded: true,
                ground_y: 0.0,
                dummy_template,
                weight: data.weight,
                enemy_count: arg("--enemies").and_then(|v| v.parse().ok()).unwrap_or(1),
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
                foe_kit: enemy.map(|e| foes::FoeKit::new(e, &data.combat, dummy_level)),
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
                casting: None,
                kit,
            })
            .insert_resource(hud::MenuState::default())
            .add_systems(Startup, (spawn_player, spawn_dummies, hud::spawn))
            .add_systems(Update, (latch_input, camera_control, sync_visuals, sync_segments, foes::sync_pieces, draw_debug, draw_skeleton, hud, abilities_keys, screenshot))
            .add_systems(Update, (hud::ability_menu, hud::draw))
            .add_systems(FixedUpdate, simulate);
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
    attacks: HashMap<u16, AttackDef>,
    blade: HashMap<u8, Vec<BladeSphere>>,
    tracker: HitTracker,
    rules: DamageRules,
    frame: u64,
    grounded: bool,
    ground_y: f32,
    dummy_template: Option<Combatant>,
    enemy_count: usize,
    /// Sora's objentry weight.
    weight: f32,
    dummy_level: u16,
    combo_plus: u8,
    /// Live attack windows' shapes from the previous frame (for the sweep),
    /// by window id.
    volumes: HashMap<(u64, bool), AttackVolume>,
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
    /// Shadows' data (absent without an install or with --synthetic).
    foe_kit: Option<foes::FoeKit>,
    /// Sora's live attack / guard windows and their shapes this frame.
    sora_hits: Vec<(character_core::ActiveHitbox, Vec<CShape>)>,
    sora_guards: Vec<(character_core::ActiveHitbox, Vec<CShape>)>,
    /// (Shadow id, swing) pairs that already hit or clashed with Sora.
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
    /// Command-menu cursor presses and magic shortcuts since the last step.
    menu_up: bool,
    menu_down: bool,
    shortcut: Option<usize>,
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
    yaw: f32,
    pitch: f32,
    distance: f32,
    manual_timer: f32,
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
    /// A Shadow on its own skeleton (absent: a plain column dummy).
    foe: Option<foes::FoeState>,
}

fn spawn_camera_and_light(mut commands: Commands) {
    commands.spawn((Camera3d::default(), Transform::from_xyz(0.0, 3.0, -8.0).looking_at(Vec3::ZERO, Vec3::Y)));
    commands.spawn((
        DirectionalLight { illuminance: 9000.0, shadows_enabled: true, ..default() },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 0.6, 0.0)),
    ));
}

fn spawn_player(mut commands: Commands, game: Res<Game>, mut meshes: ResMut<Assets<Mesh>>, mut mats: ResMut<Assets<StandardMaterial>>) {
    if let Some(anim) = &game.anim {
        // Mannequin: rounded pieces on the bones, placed every frame.
        let sphere = meshes.add(Sphere::new(0.5).mesh().uv(24, 16));
        let paints: HashMap<Paint, Handle<StandardMaterial>> = Paint::ALL.iter().map(|p| (*p, mats.add(p.material()))).collect();
        commands.spawn((Player, Transform::default(), Visibility::default()));
        for piece in rig::pieces(&anim.rig) {
            // Bones are rigid, so each capsule is sized once from the bind pose.
            let mesh = match piece.shape {
                Shape::Capsule(a, b, r) => {
                    let len = match (anim.anchor(piece.part, a), anim.anchor(piece.part, b)) {
                        (Some(a), Some(b)) => a.distance(b),
                        _ => 0.0,
                    };
                    meshes.add(Capsule3d::new(r * SCALE, len * SCALE).mesh().rings(4).latitudes(12).longitudes(16))
                }
                Shape::Sphere(..) => sphere.clone(),
            };
            commands.spawn((piece, Mesh3d(mesh), MeshMaterial3d(paints[&piece.paint].clone()), Transform::default()));
        }
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
    // One Shadow (`--enemies N` for more).
    let all = [[0.0, 0.0, 0.0], [200.0, 0.0, 150.0], [-250.0, 0.0, 120.0], [600.0, 0.0, 600.0], [-700.0, 0.0, 500.0]];
    let spots = &all[..game.enemy_count.clamp(1, all.len())];
    let sphere = meshes.add(Sphere::new(0.5).mesh().uv(16, 10));
    let paints: HashMap<Paint, Handle<StandardMaterial>> = Paint::ALL.iter().map(|p| (*p, mats.add(p.material()))).collect();
    for (i, p) in spots.iter().enumerate() {
        let mut c = template.clone();
        c.id = 100 + i as u32;
        let mut dm = Dummy { id: c.id, home: *p, pos: *p, combatant: c, knock: [0.0; 2], knock_frames: 0, respawn: 0, flash: 0, foe: None };
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
    pads: Query<&Gamepad>,
    mut latch: ResMut<Latch>,
) {
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
        if r.length() > 0.2 {
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
    for i in 0..4 {
        if held[i] && !latch.held[i] {
            latch.pressed[i] = true;
        }
        latch.held[i] = held[i];
    }
    latch.camera = cam;
}

fn camera_control(
    time: Res<Time>,
    latch: Res<Latch>,
    game: Res<Game>,
    dummies: Query<&Dummy>,
    mut rig: ResMut<CameraRig>,
    mut cam: Query<&mut Transform, With<Camera3d>>,
) {
    let dt = time.delta_secs();
    if latch.camera.length() > 0.0 {
        rig.yaw -= latch.camera.x * 2.5 * dt;
        rig.pitch = (rig.pitch - latch.camera.y * 1.5 * dt).clamp(-0.2, 1.2);
        rig.manual_timer = 1.5;
    } else {
        rig.manual_timer -= dt;
    }
    let p = Vec3::from(game.pos) * SCALE;
    let focus = p + Vec3::Y * 1.1;
    let locked = game.last.as_ref().and_then(|o| o.locked).and_then(|id| dummies.iter().find(|d| d.id == id));
    if let Some(d) = locked {
        // Lock-on: keep both in view.
        let t = Vec3::from(d.pos) * SCALE;
        let dir = (t - p).with_y(0.0).normalize_or_zero();
        if dir != Vec3::ZERO {
            let want = dir.x.atan2(dir.z);
            rig.yaw = lerp_angle(rig.yaw, want, (4.0 * dt).min(1.0));
        }
    } else if rig.manual_timer <= 0.0 && latch.stick.length() > 0.1 {
        // KH2-style lazy follow: drift behind the character while moving.
        let want = game.character.facing();
        rig.yaw = lerp_angle(rig.yaw, want, (0.8 * dt).min(1.0));
    }
    let d = Vec3::new(rig.yaw.sin(), 0.0, rig.yaw.cos());
    let back = -d * rig.pitch.cos() + Vec3::Y * rig.pitch.sin();
    if let Ok(mut t) = cam.get_single_mut() {
        let eye = focus + back * rig.distance * SCALE;
        *t = Transform::from_translation(eye).looking_at(focus, Vec3::Y);
    }
}

fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let mut d = (b - a) % (2.0 * PI);
    if d > PI {
        d -= 2.0 * PI;
    } else if d < -PI {
        d += 2.0 * PI;
    }
    a + d * t
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
fn attack_shapes(game: &Game, facing: f32, group: u8, weapon: bool) -> Vec<CShape> {
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
            // Stand still and guard every 40 frames: Shadows' swipes bounce off.
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
            "roll" => {
                latch.stick = Vec2::X;
                press(2, f % 60 == 5, &mut latch);
            }
            "glide" => {
                latch.stick = Vec2::X;
                press(0, f == 5 || f == 30 || f > 50, &mut latch);
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
                    if menu.root == hud::MAGIC && !spells.is_empty() {
                        menu.focus = hud::Focus::Magic;
                    }
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
        // Cast: MP check, then the controller picks the motion.
        if let Some(i) = cast.and_then(|c| spells.get(c).copied()) {
            let sp = kit.spells[i].clone();
            if game.can_afford(sp.mp_cost) {
                let m = |v: i16| (v >= 0).then_some(v as u16);
                if let (Some(ground), Some(air)) = (m(sp.ground_motion), m(sp.air_motion)) {
                    let motions = character_core::CastMotions { ground, air, finisher: m(sp.finish_motion) };
                    if game.character.cast(motions).is_some() {
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
    let arena = Arena::new(ARENA_BOXES);
    let ledges = arena.ledges();
    let ground = arena.ground_below(game.pos, SORA_RADIUS);
    let env = Env {
        pos: game.pos,
        grounded: game.grounded,
        height_above_ground: game.pos[1] - ground,
        targets: &targets,
        ledges: &ledges,
    };
    let out = game.character.step(&input, &env);
    // The cast motion's magic-cast event releases the spell.
    if let Some(i) = game.casting {
        let released = out.events.iter().any(|e| matches!(e, character_core::EventKind::MagicCast(m) if *m as u16 == out.motion));
        if released {
            game.casting = None;
            if let Some(sp) = game.kit.as_ref().map(|k| k.spells[i].clone()) {
                dbg.log.push(format!("{} released", sp.name));
                magic::release(game, &sp);
            }
        } else if out.state != StateName::Magic {
            game.casting = None;
        }
    }

    // Move with collision.
    let mut p = game.pos;
    p[0] += out.displacement[0];
    p[2] += out.displacement[2];
    p = arena.push_out(p, SORA_RADIUS, 30.0);
    // Body collision with standing column dummies (Shadows are separated
    // by weight after they move).
    for d in dummies.iter().filter(|d| d.respawn == 0 && d.foe.is_none()) {
        let (dx, dz) = (p[0] - d.pos[0], p[2] - d.pos[2]);
        let dist = (dx * dx + dz * dz).sqrt();
        let min = SORA_RADIUS + DUMMY_RADIUS;
        if dist < min && p[1] < d.pos[1] + DUMMY_HEIGHT {
            let (nx, nz) = if dist > 1e-3 { (dx / dist, dz / dist) } else { (0.0, -1.0) };
            p[0] = d.pos[0] + nx * min;
            p[2] = d.pos[2] + nz * min;
        }
    }
    p[1] += out.displacement[1];
    let ground = arena.ground_crossed(game.pos[1], p, SORA_RADIUS);
    game.grounded = false;
    if p[1] <= ground + 0.5 && out.displacement[1] <= 0.5 {
        // Step down small ledges while grounded, otherwise land.
        p[1] = ground;
        game.grounded = true;
    } else if game.grounded_was_close(p[1], ground) && out.displacement[1] <= 0.0 {
        p[1] = ground;
        game.grounded = true;
    }
    game.ground_y = ground;
    game.vel_display = out.displacement;
    game.pos = p;
    if let Some(anim) = game.anim.as_mut() {
        anim.update(&out, &game.character, p);
    }

    // Hits: each live window's shapes, plus copies filling the gap since
    // last frame, tested against each target's shapes (the game's test).
    let live: std::collections::HashSet<(u64, bool)> = out.hitboxes.iter().chain(&out.guard_boxes).map(|h| (h.swing, h.weapon)).collect();
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
        let Some(attack) = game.attacks.get(&h.attack).cloned() else { continue };
        for mut dm in dummies.iter_mut() {
            if dm.respawn > 0 {
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
            let sora = game.sora.clone();
            let r = game.tracker.try_hit(game.frame, h.swing, &game.rules, &attack, &sora, &mut dm.combatant);
            if r.blocked.is_none() {
                game.connected.insert(h.swing);
                game.character.register_hit();
                let fwd = [out.facing.sin(), out.facing.cos()];
                if dm.foe.is_some() {
                    // The game's generic reaction (flinch / push / launch).
                    if let Some(atk) = game.atkp_raw.get(&h.attack).cloned() {
                        foes::react(&mut dm, &atk, out.facing, r.killed);
                    }
                } else {
                    // Column dummy: ATKP strength spread over 10 frames
                    // (approximation).
                    let k = r.knockback.0.clamp(0.0, 400.0) / 10.0 * 0.3;
                    dm.knock = [fwd[0] * k, fwd[1] * k];
                    dm.knock_frames = 10;
                }
                dm.flash = 6;
                let line = format!(
                    "hit Shadow#{} with atkp {} ({}%): {} dmg  HP {}/{}{}",
                    dm.id - 99,
                    h.attack,
                    attack.power,
                    r.damage,
                    dm.combatant.hp,
                    dm.combatant.max_hp,
                    if r.killed { "  KO" } else { "" }
                );
                dbg.log.push(line);
                if r.killed {
                    dm.respawn = 120;
                }
            }
        }
    }
    if auto.trace {
        for l in &dbg.log[log_before.min(dbg.log.len())..] {
            eprintln!("log f{} {l}", game.frame);
        }
    }
    let log_mid = dbg.log.len();
    // Shadows act, then their attacks meet Sora's guard, attacks and body.
    let kit = game.foe_kit.take();
    if let Some(kit) = kit.as_ref() {
        for mut dm in dummies.iter_mut() {
            let dead_hidden = dm.respawn > 0 && dm.foe.as_ref().map_or(true, |s| !s.npc.is_dead());
            if !dead_hidden {
                foes::step(&mut dm, kit, game.pos, &arena);
                if dm.foe.as_ref().is_some_and(|s| !s.npc.is_dead()) {
                    let push = foes::separate(&mut dm, kit, game.pos, SORA_RADIUS, game.weight);
                    let q = arena.push_out([game.pos[0] + push[0], game.pos[1], game.pos[2] + push[1]], SORA_RADIUS, 30.0);
                    game.pos = q;
                }
            }
        }
        let sora_hurt = game.anim.as_ref().map(|a| a.hurt_shapes()).unwrap_or_default();
        let sora_facing = out.facing;
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
                    dbg.log.push(format!("clang! Sora's atkp {} met Shadow#{}'s atkp {}", sh.attack, *id - 99, h.attack));
                    continue;
                }
                // Sora's body.
                if out.invulnerable || !foes::overlap(&shapes, &sora_hurt) {
                    continue;
                }
                let Some(adef) = kit.attacks.get(&h.attack) else { continue };
                st.spent.insert(h.swing);
                game.sora_hit_by.insert((*id, h.swing));
                let attacker = combatant.clone();
                let r = game.tracker.try_hit(game.frame, ((*id as u64) << 40) | h.swing, &game.rules, adef, &attacker, &mut game.sora);
                if r.blocked.is_none() {
                    game.character.take_hit(&character_core::reaction::Hit {
                        reaction: fatk.enemy_reaction,
                        knockback: (fatk.knockback_1, fatk.knockback_2),
                        dir: [foe_facing.sin(), foe_facing.cos()],
                        part: 0,
                    });
                    dbg.log.push(format!("Shadow#{} hit Sora with atkp {}: {} dmg  HP {}/{}", *id - 99, h.attack, r.damage, game.sora.hp, game.sora.max_hp));
                    if game.sora.hp == 0 {
                        game.sora.hp = game.sora.max_hp;
                        dbg.log.push("Sora would be KO'd: HP refilled (playground)".into());
                    }
                }
            }
        }
        game.sora_hit_by.retain(|k| live_pairs.contains(k));
    }
    game.foe_kit = kit;
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

impl Game {
    /// While grounded, snap down onto ground up to 20 units below (stairs).
    fn grounded_was_close(&self, y: f32, ground: f32) -> bool {
        self.grounded && y - ground < 20.0
    }
}

fn sync_segments(
    game: Res<Game>,
    mut pieces: Query<(&Piece, &mut Transform, &MeshMaterial3d<StandardMaterial>)>,
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
    for (piece, mut t, mat) in &mut pieces {
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
            let color = if part == Part::Weapon { Color::srgb(1.0, 0.85, 0.2) } else { Color::srgb(0.9, 1.0, 0.9) };
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
         Sora LV{} HP {}/{} (Str {} Mag {} Def {})  {}: Shadow LV{}\n\
         High Jump {}  Quick Run {}  Aerial Dodge {}  Glide {}  Dodge Roll {}  Combo Plus x{}  Air Combo Plus x{}  Finishing Plus x{}\n\
         [Esc] abilities  [Up/Down] command menu (J confirm, Space back)  [1-6] magic shortcuts  [F1-F4] debug shapes  [R] reset\n\
         WASD move (Alt walk)  Space jump (in air: aerial dodge, then hold: glide)  J/LMB attack  K/Shift guard / roll / quick run  Tab lock-on  Q/E/RMB camera\n",
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
