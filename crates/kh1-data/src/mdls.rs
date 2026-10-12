//! KH1 character models (`.mdls`): just the skeleton, which is all the port needs (KH2's
//! motions are retargeted onto it; KH1 draws its own meshes).
//!
//! Layout, worked out from the user's `xa_ex_0010.mdls` and the executable's loader
//! (`0x1401D5AD0`, which relocates a `MOBJ`; `0x1401DCF90` walks the tagged chunks):
//! - the file: `count u32`, then `count` section offsets; section 0 is the model object.
//! - a section is a run of chunks `{tag u32, size u32, data}`, the next at `+size+8`: `MOBJ`
//!   (model), `MENV`, `TEXA`, `BONE`, `MMTN`, `URSL`.
//! - `MOBJ`: offsets from its start at word 2 (mesh-group table, 16-byte entries, count =
//!   word 3 >> 4), 4, 6, 8 (the model header), 10, 12.
//! - The model header (`MOBJ + word 8`): `bone_count u32`, an offset (from the header) to the
//!   bone table, another offset, `mesh_count u32`, then `mesh_count` 16-byte mesh entries.
//! - A bone is 0x30 bytes: `scale[3] f32, index u32, rotation[3] f32 (radians, KH2's Euler
//!   order), pad u32, translation[3] f32, links u32`, where `links` packs the parent (bits
//!   0-9), the next sibling (bits 10-19) and the first child (bits 20-29), 0x3FF meaning none.
//!   This is KH2's MDLX bone (`kh2_formats::anim::Bone`) packed tighter, so the KH2 tools pose
//!   it unchanged.

use kh2_formats::anim::Bone;
use kh2_formats::{Error, Result};

const MOBJ: u32 = u32::from_le_bytes(*b"MOBJ");
const NONE: u32 = 0x3FF;
const BONE_SIZE: usize = 0x30;

fn u32_at(b: &[u8], at: usize) -> Result<u32> {
    b.get(at..at + 4).map(|s| u32::from_le_bytes(s.try_into().unwrap())).ok_or_else(|| Error::Format(format!("mdls: read past the end at {at:#x}")))
}

fn f32_at(b: &[u8], at: usize) -> Result<f32> {
    Ok(f32::from_bits(u32_at(b, at)?))
}

/// The skeleton of a KH1 `.mdls`, as KH2 bones (parents before children, as stored).
pub fn parse_skeleton(mdls: &[u8]) -> Result<Vec<Bone>> {
    let count = u32_at(mdls, 0)? as usize;
    if count == 0 || count > 64 {
        return Err(Error::Format(format!("mdls: {count} sections")));
    }
    let mobj = u32_at(mdls, 4)? as usize;
    if u32_at(mdls, mobj)? != MOBJ {
        return Err(Error::Format("mdls: section 0 is not a MOBJ".into()));
    }
    let header = mobj + u32_at(mdls, mobj + 8 * 4)? as usize;
    let bone_count = u32_at(mdls, header)? as usize;
    let table = header + u32_at(mdls, header + 4)? as usize;
    parse_bones(mdls, table, bone_count)
}

/// `count` bones of the 0x30-byte table at `table` in `bytes` (a whole `.mdls`, or the table
/// itself read from the game's memory with `table` 0).
pub fn parse_bones(mdls: &[u8], table: usize, bone_count: usize) -> Result<Vec<Bone>> {
    if bone_count == 0 || bone_count > 1024 {
        return Err(Error::Format(format!("mdls: {bone_count} bones")));
    }
    let link = |v: u32, shift: u32| -> i16 {
        let x = (v >> shift) & NONE;
        if x == NONE { -1 } else { x as i16 }
    };
    (0..bone_count)
        .map(|i| {
            let at = table + i * BONE_SIZE;
            let f3 = |o: usize| -> Result<[f32; 3]> { Ok([f32_at(mdls, at + o)?, f32_at(mdls, at + o + 4)?, f32_at(mdls, at + o + 8)?]) };
            let index = u32_at(mdls, at + 0x0C)?;
            if index as usize != i {
                return Err(Error::Format(format!("mdls: bone {i} says it is {index}")));
            }
            let links = u32_at(mdls, at + 0x2C)?;
            Ok(Bone {
                index: i as i16,
                sibling: link(links, 10),
                parent: link(links, 0),
                child: link(links, 20),
                flags: (links >> 30) as i32,
                scale: f3(0)?,
                length: 0.0,
                rotation: f3(0x10)?,
                translation: f3(0x20)?,
            })
        })
        .collect()
}
