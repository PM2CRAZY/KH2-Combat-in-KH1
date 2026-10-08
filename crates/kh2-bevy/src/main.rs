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
use combat_core::{AttackDef, Combatant, DamageRules, HitTracker, Stats, Team};
use kh2_data::sora::BladeSphere;
use kh2_data::{Install, Source};

use rig::{Part, RigAnim, Segment};
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
    let gravity: Option<f32> = arg("--gravity").and_then(|v| v.parse().ok());
    let screenshot_after: Option<u32> = arg("--screenshot").and_then(|v| v.parse().ok());
    let autoplay = args.iter().any(|a| a == "--autoplay");
    // --scene idle|run|combo|jump|roll|glide: scripted input for screenshots.
    let scene = arg("--scene");
    let cam_distance: f32 = arg("--cam-distance").and_then(|v| v.parse().ok()).unwrap_or(650.0);

    let synthetic = args.iter().any(|a| a == "--synthetic");
    let mut tried = Vec::new();
    let loaded = if synthetic {
        Ok(synthetic::data())
    } else if let Some(dir) = arg("--data-dir") {
        // Research: files already extracted into a local folder.
        kh2_data::sora::load_sora_dir(std::path::Path::new(&dir))
    } else {
        let mut result = Err(kh2_data::Error::Format("no path given".into()));
        for c in &candidates {
            match Install::locate(c).and_then(|i| i.load_sora(source)) {
                Ok(d) => {
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
    .insert_resource(Debug { show_boxes: true, show_hurt: false, log: Vec::new(), last: None })
    .insert_resource(Shot { after_frames: screenshot_after, frames: 0 })
    .insert_resource(Autoplay { on: autoplay || scene.is_some(), frame: 0, scene: scene.unwrap_or_default() })
    .add_systems(Startup, (world::spawn_arena, spawn_camera_and_light));

    match loaded {
        Ok(data) => {
            let lvup = data.combat.battle.lvup.first().and_then(|c| c.get(level - 1)).copied();
            let stats = match lvup {
                Some(l) => Stats { hp: 100, strength: l.strength as u16, magic: l.magic as u16, defense: l.defense as u16 },
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
            let anim = data.rig.clone().filter(|_| !args.iter().any(|a| a == "--capsule")).map(RigAnim::new);
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
                dummy_level,
                combo_plus: 0,
            })
            .add_systems(Startup, (spawn_player, spawn_dummies))
            .add_systems(Update, (latch_input, camera_control, sync_visuals, sync_segments, draw_debug, hud, abilities_keys, screenshot))
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
    dummy_level: u16,
    combo_plus: u8,
}

#[derive(Resource)]
struct Debug {
    show_boxes: bool,
    show_hurt: bool,
    log: Vec<String>,
    last: Option<String>,
}

#[derive(Resource)]
struct Shot {
    after_frames: Option<u32>,
    frames: u32,
}

/// Button presses seen since the last fixed step, so short taps are never lost.
#[derive(Resource, Default)]
struct Latch {
    stick: Vec2,
    walk: bool,
    held: [bool; 4],
    pressed: [bool; 4],
    camera: Vec2,
}

/// `--autoplay`: scripted input for smoke tests and screenshots
/// (walk toward the dummies, then mash attack).
#[derive(Resource)]
struct Autoplay {
    on: bool,
    frame: u32,
    scene: String,
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
        // Mannequin: one unit capsule per bone segment, placed every frame.
        let capsule = meshes.add(Capsule3d::new(0.5, 1.0));
        let body = mats.add(StandardMaterial { base_color: Color::srgb(0.85, 0.2, 0.25), perceptual_roughness: 0.6, ..default() });
        let blade = mats.add(StandardMaterial { base_color: Color::srgb(0.8, 0.8, 0.9), metallic: 0.6, ..default() });
        commands.spawn((Player, Transform::default(), Visibility::default()));
        for (seg, mat) in rig::segments(&anim.rig.skeleton, Part::Body)
            .into_iter()
            .map(|s| (s, body.clone()))
            .chain(rig::segments(&anim.rig.weapon_skeleton, Part::Weapon).into_iter().map(|s| (s, blade.clone())))
        {
            commands.spawn((seg, Mesh3d(capsule.clone()), MeshMaterial3d(mat), Transform::default()));
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
    let spots = [[0.0, 0.0, 0.0], [200.0, 0.0, 150.0], [-250.0, 0.0, 120.0], [600.0, 0.0, 600.0], [-700.0, 0.0, 500.0]];
    for (i, p) in spots.iter().enumerate() {
        let mut c = template.clone();
        c.id = 100 + i as u32;
        commands.spawn((
            Dummy { id: c.id, home: *p, pos: *p, combatant: c, knock: [0.0; 2], knock_frames: 0, respawn: 0, flash: 0 },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(mat.clone()),
            Transform::from_xyz(p[0] * SCALE, DUMMY_HEIGHT * 0.5 * SCALE, p[2] * SCALE),
        ));
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
    if keys.pressed(KeyCode::ArrowUp) {
        cam.y += 1.0;
    }
    if keys.pressed(KeyCode::ArrowDown) {
        cam.y -= 1.0;
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
    let a = &mut game.character.abilities;
    let cycle = |v: &mut u8| *v = (*v + 1) % 5;
    if keys.just_pressed(KeyCode::Digit1) {
        cycle(&mut a.high_jump);
    }
    if keys.just_pressed(KeyCode::Digit2) {
        cycle(&mut a.quick_run);
    }
    if keys.just_pressed(KeyCode::Digit3) {
        cycle(&mut a.aerial_dodge);
    }
    if keys.just_pressed(KeyCode::Digit4) {
        cycle(&mut a.glide);
    }
    if keys.just_pressed(KeyCode::Digit5) {
        cycle(&mut a.dodge_roll);
    }
    if keys.just_pressed(KeyCode::Digit6) {
        a.combo_plus = (a.combo_plus + 1) % 4;
    }
    if keys.just_pressed(KeyCode::Digit7) {
        a.air_combo_plus = (a.air_combo_plus + 1) % 3;
    }
    if keys.just_pressed(KeyCode::Digit8) {
        a.finishing_plus = (a.finishing_plus + 1) % 4;
    }
    if keys.just_pressed(KeyCode::F1) {
        dbg.show_boxes = !dbg.show_boxes;
    }
    if keys.just_pressed(KeyCode::F2) {
        dbg.show_hurt = !dbg.show_hurt;
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

/// Attack spheres for a live hitbox: on the real bones when the skeleton is
/// loaded, otherwise the swept approximation.
fn attack_spheres(game: &Game, facing: f32, group: u8, weapon: bool) -> Vec<(Vec3, f32)> {
    if let Some(anim) = &game.anim {
        let s = anim.attack_spheres(group, weapon);
        if !s.is_empty() {
            return s;
        }
    }
    let (origin, dir) = blade_pose(game.pos, facing, Some(0.5));
    hit_spheres(game, group, origin, dir)
}

fn hit_spheres(game: &Game, group: u8, origin: Vec3, dir: Vec3) -> Vec<(Vec3, f32)> {
    match game.blade.get(&group) {
        Some(s) if !s.is_empty() => s.iter().map(|b| (origin + dir * (40.0 + b.along), b.radius)).collect(),
        // Body/other attack groups (approximation): one sphere in front.
        _ => vec![(origin + dir * 60.0, 80.0)],
    }
}

fn simulate(
    mut game: ResMut<Game>,
    mut latch: ResMut<Latch>,
    mut auto: ResMut<Autoplay>,
    rig: Res<CameraRig>,
    mut dummies: Query<&mut Dummy>,
    mut dbg: ResMut<Debug>,
) {
    let game = &mut *game;
    game.frame += 1;
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
    let s = d * latch.stick.y + right * -latch.stick.x;
    let btn = |i: usize, l: &Latch| Btn { pressed: l.pressed[i], held: l.held[i] };
    let input = CInput { stick: [s.x, s.y], jump: btn(0, &latch), attack: btn(1, &latch), action: btn(2, &latch), lock: btn(3, &latch) };
    latch.pressed = [false; 4];

    let targets: Vec<TargetInfo> = dummies.iter().filter(|d| d.respawn == 0).map(|d| TargetInfo { id: d.id, pos: d.pos, radius: DUMMY_RADIUS, height: DUMMY_HEIGHT }).collect();
    let arena = Arena::new(ARENA_BOXES);
    let ground = arena.ground_below(game.pos, SORA_RADIUS);
    let env = Env {
        pos: game.pos,
        grounded: game.grounded,
        height_above_ground: game.pos[1] - ground,
        targets: &targets,
    };
    let out = game.character.step(&input, &env);

    // Move with collision.
    let mut p = game.pos;
    p[0] += out.displacement[0];
    p[2] += out.displacement[2];
    p = arena.push_out(p, SORA_RADIUS, 30.0);
    // Body collision with standing dummies.
    for d in dummies.iter().filter(|d| d.respawn == 0) {
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
    let ground = arena.ground_below(p, SORA_RADIUS);
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

    // Hits.
    for h in &out.hitboxes {
        let Some(attack) = game.attacks.get(&h.attack).cloned() else { continue };
        let spheres = attack_spheres(game, out.facing, h.group, h.weapon);
        for mut dm in dummies.iter_mut() {
            if dm.respawn > 0 {
                continue;
            }
            let dp = Vec3::from(dm.pos);
            let touching = spheres.iter().any(|(c, r)| {
                let horiz = Vec2::new(c.x - dp.x, c.z - dp.z).length();
                horiz <= r + DUMMY_RADIUS && c.y + r >= dp.y && c.y - r <= dp.y + DUMMY_HEIGHT
            });
            if !touching {
                continue;
            }
            let sora = game.sora.clone();
            let r = game.tracker.try_hit(game.frame, h.swing, &game.rules, &attack, &sora, &mut dm.combatant);
            if r.blocked.is_none() {
                game.character.register_hit();
                let fwd = [out.facing.sin(), out.facing.cos()];
                // Knockback: ATKP strength spread over 10 frames
                // (approximation; the real push depends on enemy weight).
                let k = r.knockback.0.clamp(0.0, 400.0) / 10.0 * 0.3;
                dm.knock = [fwd[0] * k, fwd[1] * k];
                dm.knock_frames = 10;
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
            }
        }
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
    mut segs: Query<(&Segment, &mut Transform, &MeshMaterial3d<StandardMaterial>)>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut tinted: Local<Option<AssetId<StandardMaterial>>>,
) {
    let (Some(anim), Some(out)) = (&game.anim, &game.last) else { return };
    for (seg, mut t, mat) in &mut segs {
        let (a, b) = match seg.part {
            Part::Body => (anim.bone_world(seg.parent), anim.bone_world(seg.child)),
            Part::Weapon => {
                if anim.weapon.is_empty() {
                    continue;
                }
                (anim.weapon_world(seg.parent), anim.weapon_world(seg.child))
            }
        };
        let (a, b) = (a * SCALE, b * SCALE);
        let d = b - a;
        let len = d.length();
        let r = seg.radius * 2.0 * SCALE;
        *t = Transform {
            translation: (a + b) * 0.5,
            rotation: Quat::from_rotation_arc(Vec3::Y, d.normalize_or(Vec3::Y)),
            // The unit capsule is 2 units tall (1 + two 0.5 caps).
            scale: Vec3::new(r, (len / 2.0).max(r * 0.5), r),
        };
        if seg.part == Part::Body && tinted.map_or(true, |id| id == mat.0.id()) {
            *tinted = Some(mat.0.id());
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
    if !dbg.show_boxes {
        return;
    }
    for h in &out.hitboxes {
        for (c, r) in attack_spheres(&game, out.facing, h.group, h.weapon) {
            gizmos.sphere(Isometry3d::from_translation(c * SCALE), r * SCALE, Color::srgb(1.0, 0.2, 0.2));
        }
    }
    if let (true, Some(anim)) = (dbg.show_hurt, &game.anim) {
        for (c, r) in anim.hurt_spheres() {
            gizmos.sphere(Isometry3d::from_translation(c * SCALE), r * SCALE, Color::srgba(0.3, 0.7, 1.0, 0.35));
        }
    }
    let p = Vec3::from(game.pos) * SCALE;
    let iso = Isometry3d::new(p + Vec3::Y * 0.02, Quat::from_rotation_x(FRAC_PI_2));
    gizmos.circle(iso, SORA_RADIUS * SCALE, if out.invulnerable { Color::WHITE } else { Color::srgb(0.2, 0.6, 1.0) });
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
         Sora LV{} (Str {} Mag {} Def {})  dummies: Shadow LV{}\n\
         [1] High Jump {}  [2] Quick Run {}  [3] Aerial Dodge {}  [4] Glide {}  [5] Dodge Roll {}\n\
         [6] Combo Plus x{}  [7] Air Combo Plus x{}  [8] Finishing Plus x{}  [F1] hitboxes  [F2] hurtboxes  [R] reset\n\
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
        game.sora.stats.strength,
        game.sora.stats.magic,
        game.sora.stats.defense,
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
