//! The BDX virtual machine.
//!
//! Written from plain-language notes on the game's interpreter (see
//! `docs/magic-model.md`). Instruction names follow OpenKH's Bdxio tool.
//!
//! Memory is one flat byte arena addressed by `u32`. The program, every
//! context's stack, the shared work memory and anything the host allocates
//! (vectors it returns, object records, `trap_memory_alloc` blocks) live in
//! it, so script pointer arithmetic works the way it does in the game.
//! Address 0 is null.

use crate::{traps, BdxError, Program};

/// What a stack cell holds. The game keeps a tag beside every value; it
/// matters for address arithmetic there, while here addresses are plain
/// offsets, so the tag is informational.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tag {
    Int,
    Float,
    Addr,
    /// Read from memory (the game marks these "@???").
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cell {
    pub v: u32,
    pub tag: Tag,
}

impl Cell {
    pub fn int(v: i32) -> Cell {
        Cell { v: v as u32, tag: Tag::Int }
    }
    pub fn float(f: f32) -> Cell {
        Cell { v: f.to_bits(), tag: Tag::Float }
    }
    pub fn addr(a: u32) -> Cell {
        Cell { v: a, tag: Tag::Addr }
    }
    pub fn raw(v: u32) -> Cell {
        Cell { v, tag: Tag::Unknown }
    }
    pub fn i(self) -> i32 {
        self.v as i32
    }
    pub fn f(self) -> f32 {
        f32::from_bits(self.v)
    }
}

/// The byte arena.
#[derive(Clone, Debug)]
pub struct Memory {
    bytes: Vec<u8>,
}

impl Default for Memory {
    fn default() -> Self {
        // Keep the first 16 bytes unused so 0 is never a valid block.
        Memory { bytes: vec![0; 16] }
    }
}

impl Memory {
    /// A zeroed block, 16-byte aligned.
    pub fn alloc(&mut self, size: u32) -> u32 {
        let at = (self.bytes.len() + 15) & !15;
        self.bytes.resize(at + size.max(4) as usize, 0);
        at as u32
    }

    pub fn len(&self) -> u32 {
        self.bytes.len() as u32
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.len() <= 16
    }

    fn range(&self, addr: u32, n: u32) -> Result<std::ops::Range<usize>, BdxError> {
        let (a, n) = (addr as usize, n as usize);
        if addr == 0 || a + n > self.bytes.len() {
            return Err(BdxError::BadAddress(addr));
        }
        Ok(a..a + n)
    }

    pub fn bytes(&self, addr: u32, n: u32) -> Result<&[u8], BdxError> {
        let r = self.range(addr, n)?;
        Ok(&self.bytes[r])
    }

    pub fn bytes_mut(&mut self, addr: u32, n: u32) -> Result<&mut [u8], BdxError> {
        let r = self.range(addr, n)?;
        Ok(&mut self.bytes[r])
    }

    pub fn u32(&self, addr: u32) -> Result<u32, BdxError> {
        let b = self.bytes(addr, 4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn i32(&self, addr: u32) -> Result<i32, BdxError> {
        self.u32(addr).map(|v| v as i32)
    }

    pub fn f32(&self, addr: u32) -> Result<f32, BdxError> {
        self.u32(addr).map(f32::from_bits)
    }

    pub fn set_u32(&mut self, addr: u32, v: u32) -> Result<(), BdxError> {
        self.bytes_mut(addr, 4)?.copy_from_slice(&v.to_le_bytes());
        Ok(())
    }

    pub fn set_f32(&mut self, addr: u32, v: f32) -> Result<(), BdxError> {
        self.set_u32(addr, v.to_bits())
    }

    /// A 16-byte vector (x, y, z, w).
    pub fn vec4(&self, addr: u32) -> Result<[f32; 4], BdxError> {
        Ok([self.f32(addr)?, self.f32(addr + 4)?, self.f32(addr + 8)?, self.f32(addr + 12)?])
    }

    pub fn set_vec4(&mut self, addr: u32, v: [f32; 4]) -> Result<(), BdxError> {
        for (i, x) in v.iter().enumerate() {
            self.set_f32(addr + 4 * i as u32, *x)?;
        }
        Ok(())
    }

    pub fn copy(&mut self, dst: u32, src: u32, n: u32) -> Result<(), BdxError> {
        if n == 0 {
            return Ok(());
        }
        let s = self.range(src, n)?;
        let d = self.range(dst, n)?;
        self.bytes.copy_within(s, d.start);
        Ok(())
    }

    pub fn write(&mut self, addr: u32, data: &[u8]) -> Result<(), BdxError> {
        self.bytes_mut(addr, data.len() as u32)?.copy_from_slice(data);
        Ok(())
    }
}

pub type CtxId = usize;

/// One thread of execution: its own stack and evaluation stack; work memory
/// may be shared with the context it was started from.
#[derive(Clone, Debug)]
pub struct Context {
    pub pc: u32,
    pub sp: u32,
    pub stack_base: u32,
    pub stack_end: u32,
    pub wp: u32,
    pub tp: Vec<Cell>,
    pub done: bool,
    /// Child threads started with `trap_thread_start`, run after this one.
    pub children: Vec<CtxId>,
    /// Free for the host (the game stores a key here).
    pub key: i32,
}

/// Why `run` returned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// `halt`: carry on next frame.
    Yield,
    /// `exit`.
    Exit,
    /// Returned from the trigger's frame.
    Done,
}

/// The traps. `args[0]` is the first argument; a trap that returns a value
/// writes it into `args[0]`.
pub trait Host {
    fn syscall(&mut self, vm: &mut Vm, ctx: CtxId, table: u8, index: u16, args: &mut [Cell]) -> Result<(), BdxError>;
}

/// The game's frame-rate adjustments, applied while `frametime < 2`: some
/// float constants pushed at particular code positions (byte offset from
/// the start of the code) are replaced, so timings written for 30 fps
/// behave at 60 fps. Values read from the executable.
fn pc_patch(name: &str, byte_pos: u32, v: f32) -> Option<f32> {
    // The game's easing curve for two of them.
    let ease = |x: f32| ((x * 8.0 + 1.0).sqrt() - 1.0) * 0.5 * 0.35 + (2.0 * x / (x + 1.0)) * 0.65;
    if v == 0.83 && matches!(byte_pos, 0x2e8 | 0x3aa4 | 0x734 | 0x378) {
        return Some(ease(v));
    }
    if v == 0.8 && byte_pos == 0x40f2 {
        return Some(ease(v));
    }
    if (v == 2.4 && byte_pos == 0x262a) || (v == 0.08 && byte_pos == 0x8410) || (v == 0.2 && byte_pos == 0x6804) || (v == 0.005 && byte_pos == 0x6380) || (v == 0.05 && byte_pos == 0x109e) {
        return Some(v * 0.5);
    }
    if v == 20.0 && byte_pos == 0x180c {
        return Some(18.0);
    }
    if v == 30.0 && byte_pos == 0x1a8c && name == "b_ex240" {
        return Some(60.0);
    }
    None
}

pub struct Vm {
    pub prog: Program,
    pub mem: Memory,
    pub ctxs: Vec<Context>,
    prog_base: u32,
    /// The game's frame length in 60 Hz frames (1.0 on PC).
    pub frametime: f32,
    /// The context whose tree is running (the game's "current owner" that
    /// thread-starting traps copy from).
    pub owner: Option<CtxId>,
    /// Instructions one `run` may execute before it is treated as a hang.
    pub budget: u32,
    trap_info: std::collections::HashMap<(u8, u16), u32>,
}

impl Vm {
    pub fn new(prog: Program) -> Vm {
        let mut mem = Memory::default();
        let prog_base = mem.alloc(prog.bytes().len() as u32);
        mem.write(prog_base, prog.bytes()).expect("program fits");
        let trap_info = traps::TRAPS.iter().map(|(t, i, _, f)| ((*t, *i), *f)).collect();
        Vm { prog, mem, ctxs: Vec::new(), prog_base, frametime: 1.0, owner: None, budget: 200_000, trap_info }
    }

    /// Address of code word `pc` (the `bd` space).
    pub fn code_addr(&self, pc: u32) -> u32 {
        self.prog_base + 16 + pc * 2
    }

    /// A new context with its own stack. `work` shares an existing work
    /// block (threads do); `None` allocates a fresh one.
    pub fn spawn(&mut self, work: Option<u32>) -> CtxId {
        // The game forces 0x400 bytes for a few scripts (Blizzard among
        // them); never give less.
        let stack_size = self.prog.stack_size.max(0x400) + 0x100;
        let stack_base = self.mem.alloc(stack_size);
        let wp = work.unwrap_or_else(|| {
            let n = self.prog.work_size.max(4);
            self.mem.alloc(n)
        });
        self.ctxs.push(Context { pc: 0, sp: stack_base, stack_base, stack_end: stack_base + stack_size, wp, tp: Vec::new(), done: true, children: Vec::new(), key: 0 });
        self.ctxs.len() - 1
    }

    /// Starts `key`'s handler on `ctx` (see [`Vm::start_at`]). Returns
    /// false if the program has no such trigger (the context then ends).
    pub fn trigger(&mut self, ctx: CtxId, key: i32, args: &[Cell]) -> Result<bool, BdxError> {
        let Some(entry) = self.prog.trigger(key) else {
            self.ctxs[ctx].done = true;
            return Ok(false);
        };
        self.start_at(ctx, entry, args)?;
        Ok(true)
    }

    /// Starts code at `pc` on `ctx`: four zero words open a frame on the
    /// stack (so the code's final `ret` finds a return address of 0) and
    /// `args` go on the evaluation stack. Thread-starting traps use this
    /// with a code address.
    pub fn start_at(&mut self, ctx: CtxId, pc: u32, args: &[Cell]) -> Result<(), BdxError> {
        let c = &mut self.ctxs[ctx];
        c.done = false;
        c.pc = pc;
        let sp = c.sp;
        if sp + 16 > c.stack_end {
            return Err(BdxError::StackOverflow);
        }
        for i in 0..4 {
            self.mem.set_u32(sp + 4 * i, 0)?;
        }
        let c = &mut self.ctxs[ctx];
        c.sp = sp + 16;
        c.tp.extend_from_slice(args);
        Ok(())
    }

    pub fn trap_flags(&self, table: u8, index: u16) -> Option<u32> {
        self.trap_info.get(&(table, index)).copied()
    }

    fn word(&self, pc: u32) -> Result<u16, BdxError> {
        self.prog.word(pc).ok_or(BdxError::BadPc(pc))
    }

    /// Runs one context until it yields or ends.
    pub fn run(&mut self, ctx: CtxId, host: &mut dyn Host) -> Result<Status, BdxError> {
        let mut steps = 0u32;
        let pc_fixes = self.frametime < 2.0;
        loop {
            steps += 1;
            if steps > self.budget {
                return Err(BdxError::Hang(self.ctxs[ctx].pc));
            }
            let at = self.ctxs[ctx].pc;
            let w = self.word(at)?;
            let (op, sub, ssub) = (w & 0xf, (w >> 4) & 3, (w >> 6) as u32);
            let imm16 = |vm: &Vm| vm.word(at + 1);
            match op {
                // push
                0 => match sub {
                    0 | 1 => {
                        let lo = self.word(at + 1)? as u32;
                        let hi = self.word(at + 2)? as u32;
                        let v = lo | hi << 16;
                        let mut cell = if sub == 0 { Cell::int(v as i32) } else { Cell::float(f32::from_bits(v)) };
                        if sub == 1 && pc_fixes {
                            if let Some(p) = pc_patch(&self.prog.name, at * 2, cell.f()) {
                                cell = Cell::float(p);
                            }
                        }
                        self.push(ctx, cell)?;
                        self.ctxs[ctx].pc = at + 3;
                    }
                    2 => {
                        let imm = imm16(self)? as u32;
                        let a = self.mode_addr(ctx, ssub, imm)?;
                        self.push(ctx, Cell::addr(a))?;
                        self.ctxs[ctx].pc = at + 2;
                    }
                    _ => {
                        let imm = imm16(self)? as u32;
                        let cell = if ssub == 3 {
                            // A 32-bit value stored in the code.
                            let lo = self.word(imm)? as u32;
                            let hi = self.word(imm + 1)? as u32;
                            Cell::raw(lo | hi << 16)
                        } else {
                            let a = self.mode_addr(ctx, ssub, imm)?;
                            Cell::raw(self.mem.u32(a)?)
                        };
                        self.push(ctx, cell)?;
                        self.ctxs[ctx].pc = at + 2;
                    }
                },
                // pop to an address
                1 => {
                    let imm = imm16(self)? as u32;
                    let mut cell = self.pop(ctx)?;
                    if pc_fixes && at * 2 == 0x26a && self.prog.name == "fire" && cell.f() == 44.0 {
                        cell = Cell::float(44.0 - 2.0);
                    }
                    // Mode 3 writes to a scratch value in the game (code is
                    // read-only), so it is dropped here.
                    if ssub != 3 {
                        let a = self.mode_addr(ctx, ssub, imm)?;
                        self.mem.set_u32(a, cell.v)?;
                    }
                    self.ctxs[ctx].pc = at + 2;
                }
                // memcpy from a popped address to (mode + imm)
                2 => {
                    let src = self.pop(ctx)?.v;
                    let size = self.word(at + 1)? as u32;
                    let imm = self.word(at + 2)? as u32;
                    let dst = self.mode_addr(ctx, ssub, imm)?;
                    if dst != src {
                        self.mem.copy(dst, src, size)?;
                    }
                    self.ctxs[ctx].pc = at + 3;
                }
                // push *(pop + imm)
                3 => {
                    let a = self.pop(ctx)?.v;
                    let imm = imm16(self)? as u32;
                    let v = self.mem.u32(a.wrapping_add(imm))?;
                    self.push(ctx, Cell::raw(v))?;
                    self.ctxs[ctx].pc = at + 2;
                }
                // store / memcpy through a popped address
                4 => {
                    let value = self.pop(ctx)?;
                    let dst = self.pop(ctx)?.v;
                    if w < 0x40 {
                        self.mem.set_u32(dst, value.v)?;
                    } else {
                        self.mem.copy(dst, value.v, (w >> 6) as u32)?;
                    }
                    self.ctxs[ctx].pc = at + 1;
                }
                5 => {
                    let x = self.pop(ctx)?;
                    let r = unary(sub, ssub, x).ok_or(BdxError::BadOpcode(at, w))?;
                    self.push(ctx, r)?;
                    self.ctxs[ctx].pc = at + 1;
                }
                6 => {
                    let b = self.pop(ctx)?;
                    let a = self.pop(ctx)?;
                    let r = binary(sub, ssub, a, b).ok_or(BdxError::BadOpcode(at, w))?;
                    self.push(ctx, r)?;
                    self.ctxs[ctx].pc = at + 1;
                }
                // branches, relative to the next instruction
                7 => {
                    let rel = imm16(self)? as i16 as i32;
                    let next = at + 2;
                    let target = (next as i32 + rel) as u32;
                    let jump = match ssub {
                        0 => true,
                        1 => self.pop(ctx)?.v == 0,
                        2 => self.pop(ctx)?.v != 0,
                        _ => return Err(BdxError::BadOpcode(at, w)),
                    };
                    self.ctxs[ctx].pc = if jump { target } else { next };
                }
                // jal: `ssub` is the caller's frame size in words, header included
                8 | 11 => {
                    let (rel, next) = if op == 8 {
                        (imm16(self)? as i16 as i32, at + 2)
                    } else {
                        let lo = self.word(at + 1)? as u32;
                        let hi = self.word(at + 2)? as u32;
                        ((lo | hi << 16) as i32, at + 3)
                    };
                    let sp = self.ctxs[ctx].sp + ssub * 4;
                    if sp > self.ctxs[ctx].stack_end || sp < 8 {
                        return Err(BdxError::StackOverflow);
                    }
                    self.mem.set_u32(sp - 4, next)?;
                    self.mem.set_u32(sp - 8, ssub)?;
                    let c = &mut self.ctxs[ctx];
                    c.sp = sp;
                    c.pc = (next as i32 + rel) as u32;
                }
                9 => {
                    self.ctxs[ctx].pc = at + 1;
                    match ssub {
                        0 => return Ok(Status::Yield),
                        1 => return Ok(Status::Exit),
                        2 => {
                            let sp = self.ctxs[ctx].sp;
                            let ret = self.mem.u32(sp - 4)?;
                            let words = self.mem.u32(sp - 8)?;
                            let c = &mut self.ctxs[ctx];
                            c.pc = ret;
                            c.sp = sp.wrapping_sub(words.wrapping_mul(4));
                            if c.sp < c.stack_base {
                                return Err(BdxError::StackUnderflow);
                            }
                            if ret == 0 {
                                return Ok(Status::Done);
                            }
                        }
                        3 => {
                            self.pop(ctx)?;
                        }
                        5 => {
                            let x = *self.ctxs[ctx].tp.last().ok_or(BdxError::StackUnderflow)?;
                            self.push(ctx, x)?;
                        }
                        6..=9 => {
                            let x = self.pop(ctx)?.f();
                            self.push(ctx, Cell::float(trig(ssub, x)))?;
                        }
                        _ => return Err(BdxError::BadOpcode(at, w)),
                    }
                }
                10 => {
                    let index = imm16(self)?;
                    let table = ssub as u8;
                    let flags = self.trap_flags(table, index).ok_or(BdxError::UnknownTrap(table, index))?;
                    let argc = (flags & 0xffff) as usize;
                    let tp = &mut self.ctxs[ctx].tp;
                    if tp.len() < argc {
                        return Err(BdxError::StackUnderflow);
                    }
                    let mut args: Vec<Cell> = tp.split_off(tp.len() - argc);
                    if args.is_empty() {
                        args.push(Cell::int(0));
                    }
                    self.ctxs[ctx].pc = at + 2;
                    host.syscall(self, ctx, table, index, &mut args)?;
                    if flags & 0x4000_0000 != 0 {
                        self.push(ctx, args[0])?;
                    }
                }
                _ => return Err(BdxError::BadOpcode(at, w)),
            }
        }
    }

    /// Runs a context and then its child threads, like the game's per-frame
    /// update: a context that stops yielding is finished; finished children
    /// are dropped. Returns true when the context and all its children are
    /// finished.
    pub fn run_tree(&mut self, ctx: CtxId, host: &mut dyn Host) -> Result<bool, BdxError> {
        if self.owner.is_none() {
            self.owner = Some(ctx);
        }
        if !self.ctxs[ctx].done {
            let st = self.run(ctx, host)?;
            if st != Status::Yield {
                self.ctxs[ctx].done = true;
            }
        }
        let mut i = 0;
        while i < self.ctxs[ctx].children.len() {
            let child = self.ctxs[ctx].children[i];
            let prev = self.owner.replace(ctx);
            let finished = self.run_tree(child, host)?;
            self.owner = prev;
            if finished {
                self.ctxs[ctx].children.remove(i);
            } else {
                i += 1;
            }
        }
        Ok(self.ctxs[ctx].done && self.ctxs[ctx].children.is_empty())
    }

    fn mode_addr(&self, ctx: CtxId, mode: u32, imm: u32) -> Result<u32, BdxError> {
        let c = &self.ctxs[ctx];
        Ok(match mode {
            0 => c.sp + imm,
            1 => c.wp + imm,
            2 => self.mem.u32(c.sp)?.wrapping_add(imm),
            3 => self.code_addr(imm),
            _ => return Err(BdxError::BadOpcode(c.pc, mode as u16)),
        })
    }

    pub fn push(&mut self, ctx: CtxId, c: Cell) -> Result<(), BdxError> {
        let limit = (self.prog.temp_size.max(64) / 4) as usize;
        let tp = &mut self.ctxs[ctx].tp;
        if tp.len() >= limit {
            return Err(BdxError::StackOverflow);
        }
        tp.push(c);
        Ok(())
    }

    pub fn pop(&mut self, ctx: CtxId) -> Result<Cell, BdxError> {
        self.ctxs[ctx].tp.pop().ok_or(BdxError::StackUnderflow)
    }
}

fn b(v: bool) -> Cell {
    Cell::int(v as i32)
}

fn unary(sub: u16, ssub: u32, x: Cell) -> Option<Cell> {
    let (i, f) = (x.i(), x.f());
    Some(match (sub, ssub) {
        (0, 0) => Cell::float(i as f32),
        (0, 2) => Cell::int(i.wrapping_neg()),
        (0, 3) => Cell::int(!i),
        (0, 4) | (0, 8) => b(i == 0),
        (0, 5) => Cell::int(i.wrapping_abs()),
        (0, 6) => b(i < 0),
        (0, 7) => b(i < 1),
        (0, 9) => b(i != 0),
        (0, 10) => b(i >= 0),
        (0, 11) => b(i > 0),
        (1, 1) => Cell::int(f as i32),
        (1, 2) => Cell::float(-f),
        (1, 5) => Cell::float(f.abs()),
        (1, 6) => b(f < 0.0),
        (1, 7) => b(f <= 0.0),
        (1, 8) => b(f == 0.0),
        (1, 9) => b(f != 0.0),
        (1, 10) => b(f >= 0.0),
        (1, 11) => b(f > 0.0),
        _ => return None,
    })
}

fn binary(sub: u16, ssub: u32, a: Cell, bb: Cell) -> Option<Cell> {
    if sub == 1 {
        let (x, y) = (a.f(), bb.f());
        return Some(Cell::float(match ssub {
            0 => x + y,
            1 => x - y,
            2 => x * y,
            // Division by zero leaves the left operand.
            3 => {
                if y != 0.0 {
                    x / y
                } else {
                    x
                }
            }
            4 => x % y,
            _ => return None,
        }));
    }
    if sub != 0 {
        return None;
    }
    let (x, y) = (a.i(), bb.i());
    Some(match ssub {
        0 => {
            let tag = if a.tag == Tag::Addr || bb.tag == Tag::Addr { Tag::Addr } else { Tag::Int };
            Cell { v: x.wrapping_add(y) as u32, tag }
        }
        1 => Cell::int(x.wrapping_sub(y)),
        2 => Cell::int(x.wrapping_mul(y)),
        3 => {
            if y != 0 {
                Cell::int(x.wrapping_div(y))
            } else {
                a
            }
        }
        4 => {
            if y != 0 {
                Cell::int(x.wrapping_rem(y))
            } else {
                a
            }
        }
        5 => Cell::int(x & y),
        6 => Cell::int(x | y),
        7 => Cell::int(x ^ y),
        8 => Cell::int(x.wrapping_shl(y as u32 & 31)),
        9 => Cell::int(x.wrapping_shr(y as u32 & 31)),
        10 => b(x != 0 && y != 0),
        11 => b(x != 0 || y != 0),
        _ => return None,
    })
}

/// sin, cos (wrapped into [-pi, pi) first, as the game does), degrees to
/// radians (wrapped) and radians to degrees.
fn trig(ssub: u32, x: f32) -> f32 {
    use std::f32::consts::PI;
    let tau = 2.0 * PI;
    match ssub {
        6 => (((x + PI) % tau) - PI).sin(),
        7 => {
            let w = (x + PI) % tau;
            // The game returns exactly 0 at -pi/2.
            if ((PI / 2.0) % tau) - PI == w - PI {
                0.0
            } else {
                (w - PI).cos()
            }
        }
        8 => ((x * PI) / 180.0 + PI) % tau - PI,
        _ => x * 180.0 / PI,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Assembles words into a program with one trigger (key 1) at the first
    /// code word.
    fn program(code: &[u16]) -> Program {
        let mut b = vec![0u8; 0x10];
        b[..4].copy_from_slice(b"test");
        b.extend_from_slice(&16i32.to_le_bytes()); // work
        b.extend_from_slice(&256i32.to_le_bytes()); // stack
        b.extend_from_slice(&256i32.to_le_bytes()); // temp
        // header so far is 0x1c bytes; trigger table: (1, entry), (0, 0)
        let entry = (0x1c + 16 - 16) / 2; // code starts right after the table
        b.extend_from_slice(&1i32.to_le_bytes());
        b.extend_from_slice(&(entry as i32).to_le_bytes());
        b.extend_from_slice(&0i32.to_le_bytes());
        b.extend_from_slice(&0i32.to_le_bytes());
        for w in code {
            b.extend_from_slice(&w.to_le_bytes());
        }
        Program::parse(&b).unwrap()
    }

    fn push(v: i32) -> [u16; 3] {
        [0x0000, v as u16, (v >> 16) as u16]
    }
    fn pushf(v: f32) -> [u16; 3] {
        let b = v.to_bits();
        [0x0010, b as u16, (b >> 16) as u16]
    }

    struct Rec(Vec<(u8, u16, Vec<Cell>)>);
    impl Host for Rec {
        fn syscall(&mut self, _: &mut Vm, _: CtxId, t: u8, i: u16, args: &mut [Cell]) -> Result<(), BdxError> {
            self.0.push((t, i, args.to_vec()));
            if (t, i) == (0, 3) {
                args[0] = Cell::float(1.0);
            }
            Ok(())
        }
    }

    const RET: u16 = 0x0009 | 2 << 6;
    const HALT: u16 = 0x0009;

    fn run(code: &[u16]) -> (Vm, Rec, Status) {
        let mut vm = Vm::new(program(code));
        let c = vm.spawn(None);
        vm.trigger(c, 1, &[]).unwrap();
        let mut h = Rec(Vec::new());
        let st = vm.run(c, &mut h).unwrap();
        (vm, h, st)
    }

    #[test]
    fn arithmetic_and_puti() {
        // puti(7 * 6 - 2, 0)
        let mut code = Vec::new();
        code.extend(push(7));
        code.extend(push(6));
        code.push(0x0006 | 2 << 6); // mul
        code.extend(push(2));
        code.push(0x0006 | 1 << 6); // sub
        code.extend(push(0));
        code.extend([0x000a, 0]); // syscall 0,0 trap_puti (2 args)
        code.push(RET);
        let (_, h, st) = run(&code);
        assert_eq!(st, Status::Done);
        assert_eq!(h.0, vec![(0, 0, vec![Cell::int(40), Cell::int(0)])]);
    }

    #[test]
    fn floats_branches_and_locals() {
        // sp[0] = 3.0; loop: sp[0] -= 1.0; halt; while sp[0] > 0
        let mut code = Vec::new();
        code.extend(pushf(3.0));
        code.extend([0x0001, 0]); // pop.sp 0
        let top = code.len();
        code.extend([0x0030, 0]); // push.d.sp 0
        code.extend(pushf(1.0));
        code.push(0x0016 | 1 << 6); // sub.s
        code.extend([0x0001, 0]); // pop.sp 0
        code.push(HALT);
        code.extend([0x0030, 0]); // push.d.sp 0
        code.push(0x0015 | 11 << 6); // sgtz.s
        // bnez back to top: rel = top - (here + 2)
        let here = code.len();
        code.extend([0x0007 | 2 << 6, (top as i32 - (here as i32 + 2)) as i16 as u16]);
        code.push(RET);
        let mut vm = Vm::new(program(&code));
        let c = vm.spawn(None);
        vm.trigger(c, 1, &[]).unwrap();
        let mut h = Rec(Vec::new());
        let mut yields = 0;
        loop {
            match vm.run(c, &mut h).unwrap() {
                Status::Yield => yields += 1,
                Status::Done => break,
                Status::Exit => panic!(),
            }
        }
        assert_eq!(yields, 3);
        let sp = vm.ctxs[c].sp;
        assert_eq!(vm.mem.f32(sp).unwrap(), 0.0);
    }

    #[test]
    fn calls_and_returns() {
        // main: push 5; jal f (frame 4 words); puti(result, 0); ret
        // f: pop.sp 0; push.d.sp 0; push.d.sp 0; add; ret
        let mut code = Vec::new();
        code.extend(push(5));
        let jal_at = code.len();
        code.extend([0x0008 | 4 << 6, 0]); // patched below
        code.extend(push(0));
        code.extend([0x000a, 0]);
        code.push(RET);
        let f = code.len();
        code[jal_at + 1] = (f as i32 - (jal_at as i32 + 2)) as u16;
        code.extend([0x0001, 0]);
        code.extend([0x0030, 0]);
        code.extend([0x0030, 0]);
        code.push(0x0006);
        code.push(RET);
        let (_, h, st) = run(&code);
        assert_eq!(st, Status::Done);
        assert_eq!(h.0[0].2[0], Cell::int(10));
    }

    #[test]
    fn work_memory_and_trap_returns() {
        // wp[4] = frametime(); push.wp 0; push.d.pop 4 -> puti(that, 0)
        let mut code = Vec::new();
        code.extend([0x000a, 3]); // frametime
        code.extend([0x0001 | 1 << 6, 4]); // pop.wp 4
        code.extend([0x0020 | 1 << 6, 0]); // push.wp 0
        code.extend([0x0003, 4]); // push.d.pop 4
        code.push(0x0005 | 1 << 4 | 1 << 6); // cvt.s.w
        code.extend(push(0));
        code.extend([0x000a, 0]);
        code.push(RET);
        let (_, h, _) = run(&code);
        assert_eq!(h.0[1].2[0], Cell::int(1));
    }

    #[test]
    fn trig_wraps_like_the_game() {
        assert!((trig(6, std::f32::consts::FRAC_PI_2) - 1.0).abs() < 1e-6);
        assert_eq!(trig(7, -std::f32::consts::FRAC_PI_2), 0.0);
        assert!((trig(8, 180.0) + std::f32::consts::PI).abs() < 1e-5);
        assert!((trig(9, std::f32::consts::PI) - 180.0).abs() < 1e-4);
    }
}
