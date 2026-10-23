//! `kh2-combat` debug CLI. Prints tables from the user's own install to the
//! terminal; it never writes game data to disk.

use std::process::ExitCode;

use combat_core::{resolve_hit, Combatant, DamageRules, Difficulty, Stats, Team};
use kh2_data::{CombatData, Install, Source};

const USAGE: &str = "\
kh2-combat <command> --game <KH2 exe or install dir> [options]

Commands:
  info                      archive paths, key-table check, table sizes
  atkp  [--id N]            attack params (all, or one id)
  enmp  [--id N]            enemy params
  lvpm                      enemy level scaling
  plrp                      player starting stats
  lvup  [--char N]          player level-up table (0 = Sora)
  fmlv                      drive form levels
  ptya  [--set N]           combo tree rows for a moveset id (1 = Sora)
  power-stats               histogram of ATKP power values
  pref                      movement parameters (03system.bin pref: prty, fmab, plyr, sstm)
  mset --file obj/X.mset [--slot N] [--all] [--root]
                            moveset slots with timing and combat triggers
  sora [--data-dir DIR]     load Sora's controller and run scripted checks (jump, roll, combos, glide)
  collision --file obj/X.mdlx
                            model collision shapes (hurtboxes / attack boxes)
  extract --file obj/X.mset --out local/X.mset
                            copy one file out of your archives (for local research; never commit it)
  sound --id N --out x.wav  one KH2 sound effect (bank*1000+index) decoded to a WAV (local research)
  hit --attack ID[:SUB] --enemy ID [--level L] [--str S] [--mag M] [--difficulty beginner|standard|proud|critical]
                            simulate one hit and print every step

Options:
  --backup                  read Image/<lang>/backup (unmodded originals) instead of the live files
";

struct Args(Vec<String>);

impl Args {
    fn flag(&self, name: &str) -> bool {
        self.0.iter().any(|a| a == name)
    }
    fn value(&self, name: &str) -> Option<&str> {
        self.0.iter().position(|a| a == name).and_then(|i| self.0.get(i + 1)).map(|s| s.as_str())
    }
    fn num<T: std::str::FromStr>(&self, name: &str) -> Result<Option<T>, String> {
        match self.value(name) {
            None => Ok(None),
            Some(v) => v.parse().map(Some).map_err(|_| format!("bad value for {name}: {v}")),
        }
    }
}

fn main() -> ExitCode {
    let args = Args(std::env::args().skip(1).collect());
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}\n\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &Args) -> Result<(), String> {
    let cmd = args.0.first().ok_or("missing command")?.clone();
    if let (true, Some(dir)) = (cmd == "sora", args.value("--data-dir")) {
        // Research: Sora's files already extracted into a local folder.
        let data = kh2_data::sora::load_sora_dir(std::path::Path::new(dir)).map_err(|e| e.to_string())?;
        return sora_checks(data);
    }
    if cmd == "kh1-extract" {
        // KH1 (the port in progress): `kh1-extract --kh1 <install> --file <name> --out <path>
        // [--live]`; the unmodded package (BackupImage) is read unless --live.
        let dir = args.value("--kh1").ok_or("missing --kh1")?;
        let install = kh1_data::Kh1Install::locate(dir).map_err(|e| e.to_string())?;
        let source = if args.flag("--live") { kh1_data::Source::Live } else { kh1_data::Source::Original };
        for file in args.value("--file").ok_or("missing --file")?.split(',') {
            let bytes = install.read(source, file).map_err(|e| e.to_string())?;
            let out = args.value("--out").ok_or("missing --out")?;
            let path = if std::path::Path::new(out).is_dir() { format!("{out}/{file}") } else { out.to_string() };
            std::fs::write(&path, &bytes).map_err(|e| e.to_string())?;
            println!("{file}: {} bytes -> {path}", bytes.len());
        }
        return Ok(());
    }
    let game = args.value("--game").ok_or("missing --game")?;
    let install = Install::locate(game).map_err(|e| e.to_string())?;
    let source = if args.flag("--backup") { Source::Backup } else { Source::Live };

    match cmd.as_str() {
        "info" => return info(&install, source),
        "extract" => {
            let file = args.value("--file").ok_or("missing --file")?;
            let out = args.value("--out").ok_or("missing --out")?;
            let bytes = install.read_any(source, file).map_err(|e| e.to_string())?;
            std::fs::write(out, &bytes).map_err(|e| e.to_string())?;
            println!("{} bytes -> {out}", bytes.len());
            return Ok(());
        }
        "sound" => {
            let id: u32 = args.num("--id")?.ok_or("missing --id")?;
            let out = args.value("--out").ok_or("missing --out")?;
            let (bank, index) = kh2_formats::scd::split_id(id);
            let bytes = install.read_any(source, &kh2_formats::scd::bank_file(bank)).map_err(|e| e.to_string())?;
            let scd = kh2_formats::scd::Scd::parse(&bytes).map_err(|e| e.to_string())?;
            let pcm = scd.sound_id(index).map_err(|e| e.to_string())?;
            let data: Vec<u8> = pcm.samples.iter().flat_map(|s| s.to_le_bytes()).collect();
            let mut w = Vec::new();
            let block = pcm.channels as u32 * 2;
            w.extend_from_slice(b"RIFF");
            w.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
            w.extend_from_slice(b"WAVEfmt ");
            for v in [16u32] {
                w.extend_from_slice(&v.to_le_bytes());
            }
            w.extend_from_slice(&1u16.to_le_bytes());
            w.extend_from_slice(&pcm.channels.to_le_bytes());
            w.extend_from_slice(&pcm.rate.to_le_bytes());
            w.extend_from_slice(&(pcm.rate * block).to_le_bytes());
            w.extend_from_slice(&(block as u16).to_le_bytes());
            w.extend_from_slice(&16u16.to_le_bytes());
            w.extend_from_slice(b"data");
            w.extend_from_slice(&(data.len() as u32).to_le_bytes());
            w.extend_from_slice(&data);
            std::fs::write(out, &w).map_err(|e| e.to_string())?;
            println!("sound {id}: bank {bank} index {index}, {} Hz, {} ch, {:.2} s -> {out}", pcm.rate, pcm.channels, pcm.samples.len() as f32 / pcm.channels as f32 / pcm.rate as f32);
            return Ok(());
        }
        "pref" => return pref(&install, source),
        "mset" => return mset(args, &install, source),
        "collision" => return collision(args, &install, source),
        "sora" => return sora(&install, source),
        _ => {}
    }

    let data = install.load_from(source).map_err(|e| e.to_string())?;
    let b = &data.battle;
    match cmd.as_str() {
        "atkp" => {
            let id = args.num::<u16>("--id")?;
            println!("lv:sub  kind stat power team elem react fx  kb1  kb2  unk16 flags rs ro ivl drive rev tr cgrp kind hpd");
            for a in b.atkp.iter().filter(|a| id.map_or(true, |i| a.id == i)) {
                println!(
                    "{:>4} {:<5} {:>3} {:>4} {:>5} {:>4} {:>4} {:>5} {:>3} {:>4} {:>4} {:>6} {:>5} {:>2} {:>2} {:>3} {:>5} {:>3} {:>2} {:>4} {:>4} {:>3}",
                    a.id, format!("{}:{}", a.min_level(), a.sub()), a.kind, a.stat_source, a.power, a.team, a.element, a.enemy_reaction,
                    a.effect_on_hit, a.knockback_1, a.knockback_2, a.unknown_16, a.flags, a.refact_self,
                    a.refact_other, a.interval, a.drive_drain, a.revenge_damage, a.tr_reaction, a.combo_group,
                    a.attack_kind_flags, a.hp_drain
                );
            }
        }
        "enmp" => {
            let id = args.num::<u16>("--id")?;
            println!("  id lvl    hp0 maxdmg mindmg  phys fire bliz thun dark ligh gen   exp prize");
            for e in b.enmp.iter().filter(|e| id.map_or(true, |i| e.id == i)) {
                let w = e.weakness;
                println!(
                    "{:>4} {:>3} {:>6} {:>6} {:>6} {:>5} {:>4} {:>4} {:>4} {:>4} {:>4} {:>3} {:>5} {:>5}",
                    e.id, e.level, e.health[0], e.max_damage, e.min_damage, w[0], w[1], w[2], w[3], w[4], w[5], w[6], e.exp, e.prize
                );
            }
        }
        "lvpm" => {
            println!("lvl   hp%  str  def maxstr minstr  exp");
            for (i, l) in b.lvpm.iter().enumerate() {
                println!("{:>3} {:>5} {:>4} {:>4} {:>6} {:>6} {:>4}", i + 1, l.hp, l.strength, l.defense, l.max_strength, l.min_strength, l.exp);
            }
        }
        "plrp" => {
            println!("  id chr  hp  mp  ap str mag def");
            for p in &b.plrp {
                println!("{:>4} {:>3} {:>3} {:>3} {:>3} {:>3} {:>3} {:>3}", p.id, p.character, p.hp, p.mp, p.ap, p.strength, p.magic, p.defense);
            }
        }
        "lvup" => {
            let c = args.num::<usize>("--char")?.unwrap_or(0);
            let levels = b.lvup.get(c).ok_or("no such character")?;
            println!("lvl      exp str mag def  ap");
            for (i, l) in levels.iter().enumerate() {
                println!("{:>3} {:>8} {:>3} {:>3} {:>3} {:>3}", i + 1, l.exp, l.strength, l.magic, l.defense, l.ap);
            }
        }
        "fmlv" => {
            println!("form lvl anti ablv ability      exp");
            for f in &b.fmlv {
                println!("{:>4} {:>3} {:>4} {:>4} {:>7} {:>8}", f.form, f.level, f.anti_rate, f.ability_level, f.ability, f.exp);
            }
        }
        "power-stats" => power_stats(&data),
        "ptya" => {
            let n = args.num::<usize>("--set")?.unwrap_or(1);
            let set = b.ptya.set(n).ok_or("no such ptya set")?;
            println!("pointers {}", b.ptya.pointers.len());
            println!(" id kind sub cofs     flag  mot next   jump  jmax  jmin  spmin spmax   near    far    low   high  inmin inmax blend dadj  abil score");
            for e in set {
                println!(
                    "{:>3} {:>4} {:>3} {:>4} {:>8x} {:>4} {:>4} {:>6.1} {:>5.1} {:>5.1} {:>6.1} {:>5.1} {:>6.0} {:>6.0} {:>6.0} {:>6.0} {:>8.5} {:>8.5} {:>5.1} {:>4.1} {:>5} {:>5}",
                    e.id, e.kind, e.sub, e.combo_offset, e.flag, e.motion, e.next_motion, e.jump, e.jump_max, e.jump_min,
                    e.speed_min, e.speed_max, e.near, e.far, e.low, e.high, e.inner_min, e.inner_max, e.blend_time,
                    e.distance_adjust, e.ability, e.score
                );
            }
        }
        "hit" => hit(args, &data)?,
        other => return Err(format!("unknown command {other}")),
    }
    Ok(())
}

fn info(install: &Install, source: Source) -> Result<(), String> {
    println!("install:    {}", install.root.display());
    println!("executable: {}", install.executable.display());
    println!("archives:   {}", install.archive_path(source, "kh2_first.pkg").display());
    println!("backup present: {}", install.has_backup());
    let keys = install.key_table();
    println!("key table found in executable: {}", keys.is_ok());
    let data = install.load_from(source).map_err(|e| e.to_string())?;
    let b = &data.battle;
    println!(
        "00battle.bin: atkp={} enmp={} lvpm={} plrp={} lvup={} chars fmlv={} magc={} btlv={}",
        b.atkp.len(), b.enmp.len(), b.lvpm.len(), b.plrp.len(), b.lvup.len(), b.fmlv.len(), b.magc.len(), b.btlv.len()
    );
    println!("not parsed yet: {}", b.unparsed.join(", "));
    Ok(())
}

fn power_stats(data: &CombatData) {
    let mut counts = std::collections::BTreeMap::new();
    for a in &data.battle.atkp {
        *counts.entry(a.power).or_insert(0u32) += 1;
    }
    println!("power  count");
    for (p, c) in counts {
        println!("{p:>5} {c:>6}");
    }
}

fn hit(args: &Args, data: &CombatData) -> Result<(), String> {
    let spec = args.value("--attack").ok_or("missing --attack")?;
    let (id, sub) = match spec.split_once(':') {
        Some((a, b)) => (a.parse().map_err(|_| "bad attack id")?, b.parse().map_err(|_| "bad sub id")?),
        None => (spec.parse().map_err(|_| "bad attack id")?, 0u16),
    };
    let attack = data.attack(id, sub).ok_or(format!("attack {id}:{sub} not in ATKP"))?;
    let enemy_id = args.num::<u16>("--enemy")?.ok_or("missing --enemy")?;
    let level = args.num::<u16>("--level")?;
    let mut enemy = data.enemy(enemy_id, level).ok_or(format!("enemy {enemy_id} not in ENMP"))?;
    let strength = args.num::<u16>("--str")?.unwrap_or(10);
    let magic = args.num::<u16>("--mag")?.unwrap_or(10);
    let sora = Combatant::new(1, Team::Player, Stats { hp: 20, strength, magic, defense: 2 });
    let difficulty = match args.value("--difficulty").unwrap_or("standard") {
        "beginner" => Difficulty::Beginner,
        "standard" => Difficulty::Standard,
        "proud" => Difficulty::Proud,
        "critical" => Difficulty::Critical,
        d => return Err(format!("unknown difficulty {d}")),
    };
    let rules = DamageRules { difficulty, ..DamageRules::default() };
    println!("attack:  {attack:?}");
    println!("enemy:   hp {} str {} def {} resist {:?} strength bounds {:?} damage bounds {:?}",
        enemy.hp, enemy.stats.strength, enemy.stats.defense, enemy.resist.0, enemy.attack_bounds, enemy.damage_bounds
    );
    let out = resolve_hit(&rules, &attack, &sora, &mut enemy);
    println!("result:  {out:#?}");
    Ok(())
}

fn pref(install: &Install, source: Source) -> Result<(), String> {
    use kh2_formats::pref::*;
    let p = install.load_pref(source).map_err(|e| e.to_string())?;
    let show = |title: &str, t: &PrefTable, names: &[&str]| {
        println!("== {title} (pointers: {})", t.pointers.len());
        print!("   ptr ->");
        for (i, _) in t.pointers.iter().enumerate() {
            if let Some(u) = t.unique().position(|r| Some(r) == t.get(i)) {
                print!(" {i}:{u}");
            }
        }
        println!();
        for (u, rec) in t.unique().enumerate() {
            println!("  record {u}");
            for (i, n) in names.iter().enumerate() {
                if let Some(v) = rec.get(i) {
                    println!("    {n:<28} {v:>10.3}");
                }
            }
        }
    };
    show("prty", &p.prty, PRTY_FIELDS);
    show("fmab", &p.fmab, FMAB_FIELDS);
    show("plyr", &p.plyr, PLYR_FIELDS);
    show("sstm", &p.sstm, SSTM_FIELDS);
    Ok(())
}

fn mset(args: &Args, install: &Install, source: Source) -> Result<(), String> {
    let file = args.value("--file").unwrap_or("obj/P_EX100.mset");
    let m = install.load_mset(source, file).map_err(|e| e.to_string())?;
    let only = args.num::<usize>("--slot")?;
    let all = args.flag("--all");
    println!("{file}: type {} slots {}", m.kind, m.slots.len());
    for s in &m.slots {
        if only.map_or(false, |o| o != s.index) {
            continue;
        }
        let Some(a) = &s.anb else { continue };
        let interesting = a.triggers.ranges.iter().any(|r| matches!(r.id, 10 | 11 | 27 | 33 | 35 | 50));
        if !all && only.is_none() && !interesting {
            continue;
        }
        let (fps, fs, fe, fr) = a.motion.as_ref().map_or((0.0, 0.0, 0.0, 0.0), |m| (m.fps, m.frame_start, m.frame_end, m.frame_return));
        println!(
            "slot {:>4} (motion {:>3}.{}) {:<5} fps {fps:>4.0} frames {fs:.0}..{fe:.0} return {fr:.0}",
            s.index, s.index / 4, s.index % 4, s.name
        );
        if args.flag("--root") {
            if let Some(m) = &a.motion {
                let mut f = m.frame_start;
                while f <= m.frame_end + 0.01 {
                    let p = m.root_at(f);
                    println!("    root lfr {f:>5.1}  x {:>8.2} y {:>8.2} z {:>8.2}", p[0], p[1], p[2]);
                    f += 2.0;
                }
            }
        }
        for r in &a.triggers.ranges {
            println!("    range {:>4}..{:<4} id {:>2} {:?}", r.start, r.end, r.id, r.params);
        }
        for f in &a.triggers.frames {
            println!("    frame {:>4}      id {:>2} {:?}", f.frame, f.id, f.params);
        }
    }
    Ok(())
}

fn collision(args: &Args, install: &Install, source: Source) -> Result<(), String> {
    let file = args.value("--file").ok_or("missing --file")?;
    let bytes = install.read_any(source, file).map_err(|e| e.to_string())?;
    let c = kh2_formats::motion::parse_model_collision(&bytes).map_err(|e| e.to_string())?;
    println!("grp parts  arg type shape bone      x      y      z     ph radius height");
    for o in c {
        println!(
            "{:>3} {:>5} {:>4} {:>4} {:>5} {:>4} {:>6} {:>6} {:>6} {:>6} {:>6} {:>6}",
            o.group, o.parts, o.argument, o.kind, o.shape, o.bone, o.position[0], o.position[1], o.position[2],
            o.position_height, o.radius, o.height
        );
    }
    Ok(())
}

fn sora(install: &Install, source: Source) -> Result<(), String> {
    sora_checks(install.load_sora(source).map_err(|e| e.to_string())?)
}

fn sora_checks(data: kh2_data::sora::SoraData) -> Result<(), String> {
    use character_core::*;
    match (&data.rig, &data.rig_error) {
        (Some(r), _) => println!(
            "rig: {} bones, {} motions; Keyblade {} bones, {} motions, attached to bone {:?}; collision shapes: {} on Sora, {} on the Keyblade",
            r.skeleton.len(), r.motions.len(), r.weapons.first().map_or(0, |w| w.skeleton.len()), r.weapons.first().map_or(0, |w| w.motions.len()), r.weapons.first().and_then(|w| w.bone), r.body_shapes.len(), r.weapons.first().map_or(0, |w| w.shapes.len())
        ),
        (None, e) => println!("rig: not loaded ({e:?})"),
    }
    let base = data.character.clone();
    println!("params: {:?}", base.params);
    println!("attack: {:?}", base.attack);
    println!("growth levels: {}", base.growth.len());
    println!("clips: {} motions, combo rows: {}", data.motions.len(), base.moveset.combo_rows.len());
    for m in [3u16, 5, 42, 151, 152, 153, 154, 155, 181, 182, 183, 184, 201, 202, 203, 209] {
        if let Some(c) = base.moveset.clips.get(&m) {
            let hits: Vec<String> = c.windows.iter().filter_map(|w| match w.kind {
                WindowKind::Hitbox { attack, group, .. } => Some(format!("atk{attack}/g{group}@{}..{}", w.start, w.end.map_or("end".into(), |e| e.to_string()))),
                WindowKind::Guard { attack, group, weapon } => Some(format!("guard atkp {attack} group {group}{}@{}..{:?}", if weapon > 0 { " (keyblade)" } else { "" }, w.start, w.end)),
                _ => None,
            }).collect();
            let combo = c.first(|k| matches!(k, WindowKind::Combo));
            let fin = c.first(|k| matches!(k, WindowKind::Finisher));
            println!("  motion {m:>3}: {:>3} frames, root {:>6.1}, combo@{combo:?} finisher@{fin:?} {}", c.length, c.root_distance(), hits.join(" "));
        }
    }
    println!("blade groups: {:?}", data.blade_groups.keys().collect::<Vec<_>>());

    #[derive(Clone, Copy, Default)]
    struct Pad { stick: [f32; 2], jump: bool, attack: bool, action: bool }
    struct W { ch: Character, pos: [f32; 3], prev: [bool; 3], targets: Vec<TargetInfo> }
    impl W {
        fn step(&mut self, p: Pad) -> Output {
            let input = Input {
                stick: p.stick,
                jump: Button::from_held(self.prev[0], p.jump),
                attack: Button::from_held(self.prev[1], p.attack),
                action: Button::from_held(self.prev[2], p.action),
                lock: Button::default(),
            };
            self.prev = [p.jump, p.attack, p.action];
            let env = Env { pos: self.pos, grounded: self.pos[1] <= 0.0, height_above_ground: self.pos[1].max(0.0), targets: &self.targets, ledges: &[] };
            let o = self.ch.step(&input, &env);
            for i in 0..3 { self.pos[i] += o.displacement[i]; }
            self.pos[1] = self.pos[1].max(0.0);
            o
        }
    }
    let new = |levels: Abilities| { let mut ch = base.clone(); ch.abilities = levels; W { ch, pos: [0.0; 3], prev: [false; 3], targets: Vec::new() } };

    // Jump
    let mut w = new(Abilities::default());
    let mut apex = (0.0f32, 0);
    let mut takeoff = None;
    for f in 0..200 {
        let o = w.step(Pad { jump: f == 0, ..Default::default() });
        if takeoff.is_none() && w.pos[1] > 0.0 { takeoff = Some(f); }
        if w.pos[1] > apex.0 { apex = (w.pos[1], f); }
        if f > 5 && w.pos[1] == 0.0 && o.state != StateName::Air { println!("jump: takeoff frame {takeoff:?}, apex {:.1} at frame {}, landed frame {f}", apex.0, apex.1); break; }
    }
    // Roll per level
    for lv in 1..=4u8 {
        let mut w = new(Abilities { dodge_roll: lv, ..Default::default() });
        let mut inv = 0; let mut frames = 0;
        for f in 0..120 {
            let o = w.step(Pad { stick: [0.0, 1.0], action: f == 0, ..Default::default() });
            if o.state == StateName::DodgeRoll { frames += 1; if o.invulnerable { inv += 1; } } else if f > 0 { break; }
        }
        println!("dodge roll LV{lv}: {frames} frames, {inv} invulnerable, distance {:.1}", w.pos[2]);
    }
    // Combo, no target, mashing
    // Shadow-sized target (radius/height approximate). Swings "connect"
    // when a hitbox is live and the target is within 200 units.
    let t = |z: f32| TargetInfo { id: 1, pos: [0.0, 0.0, z], radius: 40.0, height: 90.0 };
    for (label, targets) in [("no target", vec![]), ("target 120 ahead", vec![t(120.0)]), ("target 250 ahead", vec![t(250.0)])] {
        let mut w = new(Abilities::default());
        w.targets = targets;
        let mut seq = Vec::new();
        for f in 0..300 {
            let o = w.step(Pad { attack: f % 2 == 0, ..Default::default() });
            if o.state == StateName::Attack && o.clip_frame == 0 { seq.push(format!("f{f}:m{}#{}", o.motion, o.combo_count)); }
            let near = w.targets.iter().any(|t| ((t.pos[0] - w.pos[0]).powi(2) + (t.pos[2] - w.pos[2]).powi(2)).sqrt() < 200.0);
            if near && !o.hitboxes.is_empty() { w.ch.register_hit(); }
            for h in &o.hitboxes { if seq.last().map_or(false, |s| !s.contains('[')) { let l = seq.pop().unwrap(); seq.push(format!("{l}[hit atk{} from f{f}]", h.attack)); } }
        }
        println!("ground combo ({label}): {}", seq.join(" -> "));
    }
    // Air combo
    let mut w = new(Abilities::default());
    let mut seq = Vec::new();
    for f in 0..300 {
        let o = w.step(Pad { jump: f == 0, attack: f > 20 && f % 2 == 0, ..Default::default() });
        if o.state == StateName::Attack && o.clip_frame == 0 { seq.push(format!("f{f}:m{}@y{:.0}", o.motion, w.pos[1])); }
    }
    println!("air combo: {}", seq.join(" -> "));
    // Jump height by how long jump is held (normal jump, then High Jump MAX).
    for (label, abil) in [("jump", Abilities::default()), ("high jump MAX", Abilities { high_jump: 4, ..Default::default() })] {
        let mut row = Vec::new();
        for hold in [1usize, 8, 14, 20, 30, 200] {
            let mut w = new(abil.clone());
            let mut apex = 0.0f32;
            for f in 0..240 {
                w.step(Pad { jump: f < hold, ..Default::default() });
                apex = apex.max(w.pos[1]);
            }
            row.push(format!("hold {hold}: {apex:.0}"));
        }
        println!("{label} apex by hold: {}", row.join(", "));
    }
    // Quick Run distance by hold, and Dodge Roll held into Quick Run.
    for (label, abil) in [("quick run MAX", Abilities { quick_run: 4, ..Default::default() }), ("roll+quick run MAX", Abilities { quick_run: 4, dodge_roll: 4, ..Default::default() })] {
        let mut row = Vec::new();
        for hold in [2usize, 12, 30, 200] {
            let mut w = new(abil.clone());
            let mut states = Vec::new();
            for f in 0..120 {
                let o = w.step(Pad { stick: [0.0, 1.0], action: f < hold, ..Default::default() });
                if states.last() != Some(&o.state) { states.push(o.state); }
            }
            row.push(format!("hold {hold}: {:.0} {:?}", w.pos[2], states));
        }
        println!("{label}: {}", row.join(" | "));
    }
    // Aerial Dodge (all growth MAX), pressed 20 frames after the jump.
    let mut w = new(Abilities::max_growth());
    let mut log = Vec::new();
    for f in 0..90 {
        let o = w.step(Pad { jump: f == 0 || f == 20, stick: [0.0, 1.0], ..Default::default() });
        if (16..60).contains(&f) && f % 2 == 0 { log.push(format!("f{f}:{:?}/m{}/y{:.0}", o.state, o.motion, w.pos[1])); }
    }
    println!("aerial dodge: {}", log.join(" "));
    // Glide per level
    for lv in 1..=4u8 {
        let mut w = new(Abilities { glide: lv, high_jump: 4, ..Default::default() });
        let mut glide_frames = 0; let mut start = None;
        for f in 0..2000 {
            let o = w.step(Pad { jump: f == 0 || f > 30, stick: [0.0, 1.0], ..Default::default() }); // aerial dodge off: jump in air = glide
            if o.state == StateName::Glide { glide_frames += 1; if start.is_none() { start = Some((w.pos[1], w.pos[2])); } }
            if f > 31 && o.state != StateName::Glide && start.is_some() { break; }
        }
        if let Some((y0, z0)) = start { println!("glide LV{lv}: {glide_frames} frames from y {y0:.0}, travelled {:.0}", w.pos[2] - z0); }
    }
    Ok(())
}
