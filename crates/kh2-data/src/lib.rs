//! # kh2-data
//!
//! The bridge between a Kingdom Hearts II install the user owns and the
//! engine-agnostic `combat-core` crate. Nothing here is stored in the repo:
//! every table is read from the install at runtime.
//!
//! ```no_run
//! let install = kh2_data::Install::locate(r"D:\epic\KH_1.5_2.5\KINGDOM HEARTS II FINAL MIX.exe").unwrap();
//! let data = install.load().unwrap();
//! let shadow = data.enemy(1, None).unwrap();
//! ```

pub mod enemy;
pub mod kit;
pub mod sora;

use std::path::{Path, PathBuf};

use combat_core::{
    AttackDef, AttackKind, Combatant, Deflect, Element, Resistances, StatBounds, StatSource, Stats, TargetFilter,
    Team,
};
use kh2_formats::battle::{Atkp, Battle, Enmp};
use kh2_formats::egs::{Archive, KeyTable};
pub use kh2_formats::{Error, Result};

const ARCHIVES: [&str; 6] = ["kh2_first", "kh2_second", "kh2_third", "kh2_fourth", "kh2_fifth", "kh2_sixth"];

/// Executable names, in the order tried, that carry the PKG key table.
const EXECUTABLES: &[&str] = &["KINGDOM HEARTS II FINAL MIX.exe", "KINGDOM HEARTS FINAL MIX.exe"];

/// Cleans up a user-supplied path (see [`Install::locate`]).
pub fn repair_path(p: &Path) -> PathBuf {
    let raw = p.to_string_lossy();
    let mut s = raw.trim().trim_matches(|c| c == '"' || c == '\'').trim().to_string();
    if !Path::new(&s).exists() {
        if let Some(i) = s.to_ascii_lowercase().find(".exe") {
            s.truncate(i + 4);
        }
    }
    PathBuf::from(s)
}

/// Where the game files are on disk.
#[derive(Clone, Debug)]
pub struct Install {
    pub root: PathBuf,
    pub executable: PathBuf,
    /// `Image/<lang>` directory holding `kh2_*.hed/.pkg`.
    pub image_dir: PathBuf,
}

/// Which copy of the archives to read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Source {
    /// The live files the game loads (includes any installed mods).
    #[default]
    Live,
    /// `Image/<lang>/backup/`, the untouched originals the OpenKH Mods
    /// Manager keeps when it patches the game.
    Backup,
}

impl Install {
    /// Accepts the KH2 executable path or the install directory. Tolerates
    /// common copy-paste damage: surrounding quotes or spaces, and text glued
    /// on after `.exe` (e.g. `...FINAL MIX.execargo` when two pasted lines
    /// run together).
    pub fn locate(path: impl AsRef<Path>) -> Result<Install> {
        let repaired = repair_path(path.as_ref());
        let p = repaired.as_path();
        if !p.exists() {
            return Err(Error::Format(format!("path does not exist: {}", p.display())));
        }
        let (root, executable) = if p.is_file() {
            (p.parent().unwrap_or(Path::new(".")).to_path_buf(), p.to_path_buf())
        } else {
            let exe = EXECUTABLES
                .iter()
                .map(|n| p.join(n))
                .find(|e| e.is_file())
                .ok_or_else(|| Error::Format(format!("no KH executable found in {}", p.display())))?;
            (p.to_path_buf(), exe)
        };
        let image = root.join("Image");
        let image_dir = ["en", "dt", "jp"]
            .iter()
            .map(|l| image.join(l))
            .find(|d| d.join("kh2_first.hed").is_file())
            .ok_or_else(|| Error::Format(format!("kh2_first.hed not found under {}", image.display())))?;
        Ok(Install { root, executable, image_dir })
    }

    pub fn archive_path(&self, source: Source, name: &str) -> PathBuf {
        match source {
            Source::Live => self.image_dir.join(name),
            Source::Backup => self.image_dir.join("backup").join(name),
        }
    }

    pub fn has_backup(&self) -> bool {
        self.archive_path(Source::Backup, "kh2_first.hed").is_file()
    }

    /// The backup folder only holds the archives a mod manager touched; for
    /// the others the live file is the original. Falls back accordingly.
    fn resolve(&self, source: Source, file: &str) -> PathBuf {
        let p = self.archive_path(source, file);
        if source == Source::Backup && !p.is_file() {
            return self.archive_path(Source::Live, file);
        }
        p
    }

    pub fn key_table(&self) -> Result<KeyTable> {
        KeyTable::from_executable(&self.executable)
    }

    /// Reads one original-format file (e.g. `00battle.bin`) from `kh2_first`.
    pub fn read_file(&self, source: Source, name: &str) -> Result<Vec<u8>> {
        let keys = self.key_table()?;
        let archive = Archive::open(self.archive_path(source, "kh2_first.hed"))?;
        archive.read(&keys, name)
    }

    /// Reads a file from whichever `kh2_*` archive holds it (first match in
    /// the order first..sixth).
    pub fn read_any(&self, source: Source, name: &str) -> Result<Vec<u8>> {
        let keys = self.key_table()?;
        for part in ARCHIVES {
            let hed = self.resolve(source, &format!("{part}.hed"));
            if !hed.is_file() {
                continue;
            }
            let archive = Archive::open(&hed)?;
            if archive.find(name).is_some() {
                return archive.read(&keys, name);
            }
        }
        Err(Error::NotFound(name.to_string()))
    }

    /// Movement parameters from `03system.bin`.
    pub fn load_pref(&self, source: Source) -> Result<kh2_formats::pref::Pref> {
        kh2_formats::pref::Pref::from_03system(&self.read_any(source, "03system.bin")?)
    }

    /// A moveset, e.g. `obj/P_EX100.mset` (Sora).
    pub fn load_mset(&self, source: Source, name: &str) -> Result<kh2_formats::motion::Mset> {
        kh2_formats::motion::Mset::parse(&self.read_any(source, name)?)
    }

    pub fn load(&self) -> Result<CombatData> {
        self.load_from(Source::Live)
    }

    pub fn load_from(&self, source: Source) -> Result<CombatData> {
        let bytes = self.read_file(source, "00battle.bin")?;
        Ok(CombatData {
            battle: Battle::parse(&bytes)?,
            source,
        })
    }
}

/// KH2's combat tables plus helpers that turn rows into `combat-core` types.
#[derive(Clone, Debug)]
pub struct CombatData {
    pub battle: Battle,
    pub source: Source,
}

impl CombatData {
    pub fn attack(&self, id: u16, sub_id: u16) -> Option<AttackDef> {
        self.battle.attack(id, sub_id).map(attack_def)
    }

    /// The level-appropriate variant, as the game picks it.
    pub fn attack_at_level(&self, id: u16, sub: u8, level: u8) -> Option<AttackDef> {
        self.battle.attack_for_level(id, sub, level).map(attack_def)
    }

    /// Builds an enemy combatant. `level` overrides the table level (needed
    /// when the table says 0 = "use the world's battle level").
    pub fn enemy(&self, id: u16, level: Option<u16>) -> Option<Combatant> {
        let e = self.battle.enemy(id)?;
        let lvl = level.unwrap_or(e.level).max(1);
        let lv = self.battle.lvpm_for(lvl)?;
        Some(enemy_combatant(e, lv))
    }
}

/// Converts one ATKP row.
pub fn attack_def(a: &Atkp) -> AttackDef {
    AttackDef {
        id: ((a.id as u32) << 16) | a.sub_id as u32,
        kind: AttackKind::from_id(a.kind),
        power: a.power,
        element: Element::from_id(a.element),
        // Upstream: 0/1/2 hit enemies, 3/4/5 enemies and allies.
        targets: if a.team >= 3 { TargetFilter::Everyone } else { TargetFilter::Opponents },
        stat: StatSource::from_id(a.stat_source),
        reaction: a.enemy_reaction,
        knockback: (a.knockback_1 as f32, a.knockback_2 as f32),
        hit_interval: a.interval,
        drive_gain: a.drive_drain,
        hp_drain: a.hp_drain,
        revenge: a.revenge_damage,
        on_reflect_self: Deflect::from_id(a.refact_self),
        on_reflect_other: Deflect::from_id(a.refact_other),
        combo_finisher: a.attack_kind_flags & 1 != 0,
        air_combo_finisher: a.attack_kind_flags & 2 != 0,
        reaction_command: a.attack_kind_flags & 4 != 0,
        combo_group: a.combo_group,
    }
}

/// Converts one ENMP row at a level, the way the game's enemy status init
/// does: HP scaled by LVPM (rounded up), Strength/Defense and the strength
/// bounds from LVPM, and ENMP's min/max damage as parts-per-10000 of max HP
/// (rounded up) bounding the base damage term.
pub fn enemy_combatant(e: &Enmp, lv: &kh2_formats::battle::Lvpm) -> Combatant {
    let hp = lv.scale_hp(e.health[0].max(0) as i32).max(1) as u32;
    let mut weakness = [0u16; 7];
    for (w, &v) in weakness.iter_mut().zip(&e.weakness) {
        *w = v.min(255); // the game keeps these as bytes
    }
    Combatant::new(
        0x1000_0000 | e.id as u32,
        Team::Enemy,
        Stats { hp, strength: lv.strength, magic: lv.strength, defense: lv.defense },
    )
    .with_resist(Resistances(weakness))
    .with_attack_bounds(StatBounds { min: lv.min_strength as u32, max: lv.max_strength as u32 })
    .with_damage_bounds(StatBounds::from_permyriad(hp, e.min_damage as u32, e.max_damage as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repairs_pasted_paths() {
        let dir = std::env::temp_dir().join("kh2-combat-locate-test");
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("KINGDOM HEARTS II FINAL MIX.exe");
        std::fs::write(&exe, b"").unwrap();
        let e = exe.to_string_lossy().to_string();
        assert_eq!(repair_path(Path::new(&format!("{e}cargo"))), exe);
        assert_eq!(repair_path(Path::new(&format!("\"{e}\" "))), exe);
        assert_eq!(repair_path(Path::new(&e)), exe);
        assert!(Install::locate(dir.join("nope.exe")).unwrap_err().to_string().contains("does not exist"));
    }
}
