//! # character-core
//!
//! An engine-agnostic, 60 Hz action-character controller modelled on
//! Kingdom Hearts II Final Mix: walk/run with capped turning, jump with
//! takeoff frames, High Jump, Glide, Dodge Roll, Quick Run, Aerial Dodge,
//! Guard, landing recovery, lock-on, and ground/air combos chosen from a
//! combo table with input buffering and finisher chains.
//!
//! It holds no game data. Moves are described by [`Clip`]s (length, root
//! motion, windows such as "hitbox live" or "combo input accepted") and
//! [`ComboRow`]s. `kh2-data` builds both from a KH2 install the player
//! owns; another game can author its own.
//!
//! The host owns physics: call [`Character::step`] at 60 Hz, move the body
//! by `displacement` with your collision, report `grounded` next frame,
//! and resolve `hitboxes` with your hit detection (e.g. `combat-core`).

mod clip;
mod combo;
mod controller;
pub mod drive;
pub mod npc;
mod params;
pub mod reaction;

pub use clip::*;
pub use combo::*;
pub use controller::*;
pub use params::*;
