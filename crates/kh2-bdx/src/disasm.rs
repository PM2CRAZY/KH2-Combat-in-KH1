//! A small disassembler for reading scripts while researching (mnemonics
//! follow OpenKH's Bdxio tool).

use crate::{trap_name, Program};

const MODES: [&str; 4] = ["sp", "wp", "sp.d", "bd"];

/// One line per instruction: `pc  mnemonic operands`.
pub fn disassemble(p: &Program) -> String {
    let mut out = String::new();
    let first = (12 + 8 * (p.triggers.len() as u32 + 1)) / 2;
    let mut pc = first;
    let end = p.words();
    while pc < end {
        let (text, size) = decode(p, pc);
        out.push_str(&format!("{pc:5}  {text}\n"));
        pc += size;
    }
    out
}

/// Decodes one instruction; returns its text and size in words.
pub fn decode(p: &Program, pc: u32) -> (String, u32) {
    let w = p.word(pc).unwrap_or(0);
    let (op, sub, ssub) = (w & 0xf, (w >> 4) & 3, (w >> 6) as u32);
    let i16_at = |k: u32| p.word(pc + k).unwrap_or(0) as i16;
    let u16_at = |k: u32| p.word(pc + k).unwrap_or(0);
    let i32_at = |k: u32| (u16_at(k) as u32 | (u16_at(k + 1) as u32) << 16) as i32;
    let mode = |m: u32| MODES.get(m as usize).copied().unwrap_or("?");
    match op {
        0 => match sub {
            0 => (format!("push       {}", i32_at(1)), 3),
            1 => (format!("push.s     {}", f32::from_bits(i32_at(1) as u32)), 3),
            2 => (format!("push.{:<6} {}", mode(ssub), u16_at(1)), 2),
            _ => (format!("push.d.{:<4} {}", mode(ssub), u16_at(1)), 2),
        },
        1 => (format!("pop.{:<7} {}", mode(ssub), u16_at(1)), 2),
        2 => (format!("memcpy.{:<4} {}, {}", mode(ssub), u16_at(1), u16_at(2)), 3),
        3 => (format!("push.d.pop {}", u16_at(1)), 2),
        4 => (if w < 0x40 { "store".to_string() } else { format!("memcpy     {}", w >> 6) }, 1),
        5 => {
            let names_i = ["cvt.w.s", "?", "neg", "not", "seqz", "abs", "sltz", "slez", "seqz", "snez", "sgez", "sgtz"];
            let names_f = ["?", "cvt.s.w", "neg.s", "?", "?", "abs.s", "sltz.s", "slez.s", "seqz.s", "snez.s", "sgez.s", "sgtz.s"];
            let t = if sub == 0 { names_i.get(ssub as usize) } else { names_f.get(ssub as usize) };
            (t.copied().unwrap_or("?").to_string(), 1)
        }
        6 => {
            let names_i = ["add", "sub", "mul", "div", "mod", "and", "or", "xor", "sll", "sra", "land", "lor"];
            let names_f = ["add.s", "sub.s", "mul.s", "div.s", "mod.s"];
            let t = if sub == 0 { names_i.get(ssub as usize) } else { names_f.get(ssub as usize) };
            (t.copied().unwrap_or("?").to_string(), 1)
        }
        7 => {
            let t = ["b", "beqz", "bnez"].get(ssub as usize).copied().unwrap_or("b?");
            (format!("{t:<10} -> {}", pc as i32 + 2 + i16_at(1) as i32), 2)
        }
        8 => (format!("jal        frame={ssub} -> {}", pc as i32 + 2 + i16_at(1) as i32), 2),
        11 => (format!("jal32      frame={ssub} -> {}", pc as i32 + 3 + i32_at(1)), 3),
        9 => {
            let t = ["halt", "exit", "ret", "drop", "?", "dup", "sin", "cos", "degr", "radd"].get(ssub as usize).copied().unwrap_or("?");
            (t.to_string(), 1)
        }
        10 => {
            let i = u16_at(1);
            (format!("syscall    {ssub}:{i}  ; {}", trap_name(ssub as u8, i).unwrap_or("?")), 2)
        }
        _ => (format!("?? {w:#06x}"), 1),
    }
}
