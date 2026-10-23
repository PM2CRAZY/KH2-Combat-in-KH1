//! Runs KH2's BDX scripts: the bytecode that drives spells, enemies and
//! many objects. The VM is engine-agnostic; whoever embeds it implements
//! the traps (syscalls) through [`Host`].
//!
//! See `docs/magic-model.md` for the instruction semantics and what is
//! known about each trap.

pub mod disasm;
pub mod program;
pub mod traps;
pub mod vm;

pub use program::Program;
pub use vm::{Cell, Context, CtxId, Host, Memory, Status, Tag, Vm};

#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum BdxError {
    #[error("bad BDX file: {0}")]
    Format(String),
    #[error("pc {0} is outside the code")]
    BadPc(u32),
    #[error("bad instruction {1:#06x} at pc {0}")]
    BadOpcode(u32, u16),
    #[error("address {0:#x} is outside script memory")]
    BadAddress(u32),
    #[error("unknown trap {0}:{1}")]
    UnknownTrap(u8, u16),
    #[error("stack overflow")]
    StackOverflow,
    #[error("stack underflow")]
    StackUnderflow,
    #[error("script ran too long without yielding (pc {0})")]
    Hang(u32),
    #[error("trap failed: {0}")]
    Trap(String),
}

/// The trap's name, if known.
pub fn trap_name(table: u8, index: u16) -> Option<&'static str> {
    traps::TRAPS.iter().find(|(t, i, _, _)| *t == table && *i == index).map(|r| r.2)
}
