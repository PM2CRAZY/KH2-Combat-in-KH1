//! # combat-core
//!
//! An engine-agnostic combat rules crate modelled on Kingdom Hearts II.
//!
//! It holds **no game data** and depends on **no engine**. You describe
//! attacks ([`AttackDef`]) and fighters ([`Combatant`]) with plain numbers,
//! and the crate decides what a hit does: damage, healing, reaction,
//! knockback, drive/HP drain, revenge build-up and multi-hit gating.
//!
//! Where those numbers come from is up to you:
//! - your own game's design data, or
//! - the `kh2-data` crate, which builds them from a KH2 install the player owns.
//!
//! The formula and its sources are documented in `docs/combat-model.md`.
//! The formula's order and rounding were checked against KH2 Final Mix's
//! executable; the difficulty table is exposed through [`DamageRules`].
//!
//! ## Minimal example
//!
//! ```
//! use combat_core::*;
//!
//! let slash = AttackDef { power: 100, ..AttackDef::default() };
//! let sora = Combatant::new(1, Team::Player, Stats { hp: 20, strength: 30, magic: 10, defense: 5 });
//! let mut shadow = Combatant::new(2, Team::Enemy, Stats { hp: 50, strength: 8, magic: 8, defense: 4 });
//!
//! let rules = DamageRules::default();
//! let out = resolve_hit(&rules, &slash, &sora, &mut shadow);
//! assert_eq!(out.damage, 26); // (30 - 4) x 100%
//! assert_eq!(shadow.hp, 24);
//! ```

mod attack;
mod combatant;
mod damage;
mod element;
mod hit;
mod record;
mod shape;

pub use attack::*;
pub use combatant::*;
pub use damage::*;
pub use element::*;
pub use hit::*;
pub use record::*;
pub use shape::*;
