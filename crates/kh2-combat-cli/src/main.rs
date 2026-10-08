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
  power-stats               histogram of ATKP power values
  hit --attack ID[:SUB] --enemy ID [--level L] [--str S] [--mag M] [--difficulty beginner|standard|proud|critical]
                            simulate one hit and print every step

Options:
  --backup                  read Image/<lang>/backup (unmodded originals) instead of the live files
  --power-scale F           ATKP power -> multiplier factor (default 0.01)
  --clamp hp|base|off       how to read ENMP min/max damage (default hp, see docs)
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
    let game = args.value("--game").ok_or("missing --game")?;
    let install = Install::locate(game).map_err(|e| e.to_string())?;
    let source = if args.flag("--backup") { Source::Backup } else { Source::Live };

    if cmd == "info" {
        return info(&install, source);
    }

    let mut data = install.load_from(source).map_err(|e| e.to_string())?;
    if let Some(s) = args.num::<f32>("--power-scale")? {
        data.power_scale = s;
    }
    data.clamp_reading = match args.value("--clamp").unwrap_or("hp") {
        "hp" => kh2_data::ClampReading::HpFraction,
        "base" => kh2_data::ClampReading::BaseRaw,
        "off" => kh2_data::ClampReading::Off,
        c => return Err(format!("unknown --clamp {c}")),
    };
    let b = &data.battle;
    match cmd.as_str() {
        "atkp" => {
            let id = args.num::<u16>("--id")?;
            println!("id:sub  kind crit power team elem react fx  kb1  kb2  unk16 flags rs ro ivl drive rev tr cgrp kind hpd");
            for a in b.atkp.iter().filter(|a| id.map_or(true, |i| a.id == i)) {
                println!(
                    "{:>4}:{:<3} {:>3} {:>4} {:>5} {:>4} {:>4} {:>5} {:>3} {:>4} {:>4} {:>6} {:>5} {:>2} {:>2} {:>3} {:>5} {:>3} {:>2} {:>4} {:>4} {:>3}",
                    a.id, a.sub_id, a.kind, a.critical_adjust, a.power, a.team, a.element, a.enemy_reaction,
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
    println!("enemy:   hp {} str {} def {} resist {:?} base clamp {:?} hit clamp {:?}",
        enemy.hp, enemy.stats.strength, enemy.stats.defense, enemy.resist.0, enemy.base_clamp,
        enemy.hit_clamp.map(|c| c.bounds(enemy.max_hp))
    );
    let out = resolve_hit(&rules, &attack, &sora, &mut enemy);
    println!("result:  {out:#?}");
    Ok(())
}
