//! KH2 effects for KH1's own effect system (the TESTBRANCH effects port, project notes
//! `kh2-effects-in-kh1.md`).
//!
//! KH1 plays effects from DPX files with the same layout and version (`0x82`) as the DPX inside
//! a KH2 PAX, and its DPDs (`0x96`) and particle program machine ("ppp", programs 1..235) are the
//! same as KH2's. What differs is where textures live in the emulated GS memory:
//! - KH1: its resident common textures at TBP `0x3600..0x37FF` / CBP `0x3D00..0x3D3F`; an
//!   effect's own textures from TBP `0x3800` / CBP `0x3D40` (moved into a free slot when drawn).
//! - KH2: common (`effect/texcommon.dpd`) below TBP `0x3000` / CBP `0x3E00`, an effect's own
//!   from there up.
//!
//! [`build_groups`] makes a KH1 DPX out of any KH2 effects of a PAX, one DPD per group (a group's
//! effects all from one DPD of the PAX; KH1 gives each DPD its own run of texture slots when it
//! draws). [`build`] / [`build_with`] are the one-group case. For each DPD: its own texture addresses moved
//! to KH1's own window, the KH2 common textures it draws with copied in as its own (after its
//! own, each at a fresh address, and set in the use-texture mask of every particle data that
//! draws with them), and program types KH1 doesn't have (236..242) set to 1 (`pppDummyFunc`)
//! until they are plugged in. Everything else is copied as it is. KH1 relocates and textures the
//! result with its own code. This module only moves bytes; it reproduces no game code.

use crate::{bad, Result};

const DPD_MAGIC: u32 = 0x96;
const DPX_VERSION: i32 = 0x82;
/// KH1's own-texture windows (`0x14031EC10` / `0x1401EBC60` / `0x1401EC770`, Steam).
const KH1_TBP: i32 = 0x3800;
const KH1_CBP: i32 = 0x3D40;
/// Slots KH1 has for an effect's own textures: 10 of 0x80 TBP blocks, 6 of 0x20 CBP blocks.
const KH1_TBP_SLOTS: i32 = 10;
const KH1_CBP_SLOTS: i32 = 6;
/// The highest program type KH1's table has (`0x14054FF00`, Steam).
pub const KH1_LAST_PROGRAM: u32 = 235;
/// `pppDummyFunc`.
const DUMMY_PROGRAM: u32 = 1;
/// Program types whose KH2 version is KH2's own (not KH1's): 43 `pppColor` (KH2 writes the colour
/// bytes every call and clamps them), 49 `pppVtMime` (KH2's calc keeps the rate on a marker
/// key), 201 `pppRyjMegaBirth` (KH2 rewrote it) and 241 `pppEiDrawShipoly` (KH2 only). [`Kh1Dpx::programs`] lists where they are, so the host can point
/// them at its own entries once KH1 has relocated the DPX (KH1's own DPXs keep KH1's).
pub const KH2_OWN_PROGRAMS: [u32; 4] = [43, 49, 201, 241];

fn u16_at(b: &[u8], at: usize) -> Result<u16> {
    b.get(at..at + 2).map(|s| u16::from_le_bytes(s.try_into().unwrap())).ok_or_else(|| crate::Error::Format(format!("kh1dpx: read past the end at {at:#x}")))
}
fn i16_at(b: &[u8], at: usize) -> Result<i16> {
    Ok(u16_at(b, at)? as i16)
}
fn u32_at(b: &[u8], at: usize) -> Result<u32> {
    b.get(at..at + 4).map(|s| u32::from_le_bytes(s.try_into().unwrap())).ok_or_else(|| crate::Error::Format(format!("kh1dpx: read past the end at {at:#x}")))
}
fn i32_at(b: &[u8], at: usize) -> Result<i32> {
    Ok(u32_at(b, at)? as i32)
}
fn u64_at(b: &[u8], at: usize) -> Result<u64> {
    b.get(at..at + 8).map(|s| u64::from_le_bytes(s.try_into().unwrap())).ok_or_else(|| crate::Error::Format(format!("kh1dpx: read past the end at {at:#x}")))
}
fn put_u16(b: &mut [u8], at: usize, v: u16) {
    b[at..at + 2].copy_from_slice(&v.to_le_bytes());
}
fn put_u32(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}
fn put_u64(b: &mut [u8], at: usize, v: u64) {
    b[at..at + 8].copy_from_slice(&v.to_le_bytes());
}

/// A DPD split into its five item lists (each item's bytes, up to the next item) and its VRAM
/// ranges.
struct Split {
    lists: [Vec<Vec<u8>>; 5],
    /// Each item's offset in the DPD.
    offs: [Vec<usize>; 5],
    vram: [i32; 4],
}

fn split(dpd: &[u8]) -> Result<Split> {
    if u32_at(dpd, 0)? != DPD_MAGIC {
        return bad("kh1dpx: not a DPD");
    }
    let mut at = 4;
    let mut offs: [Vec<usize>; 5] = Default::default();
    for list in offs.iter_mut() {
        let n = i32_at(dpd, at)?;
        if !(0..=4096).contains(&n) {
            return bad("kh1dpx: implausible DPD list");
        }
        at += 4;
        for _ in 0..n {
            list.push(u32_at(dpd, at)? as usize);
            at += 4;
        }
    }
    let vram = [i32_at(dpd, at)?, i32_at(dpd, at + 4)?, i32_at(dpd, at + 8)?, i32_at(dpd, at + 12)?];
    let mut all: Vec<usize> = offs.iter().flatten().copied().collect();
    all.push(dpd.len());
    all.sort_unstable();
    all.dedup();
    let end = |o: usize| all.iter().copied().find(|&x| x > o).unwrap_or(dpd.len());
    let mut lists: [Vec<Vec<u8>>; 5] = Default::default();
    for (k, list) in offs.iter().enumerate() {
        for &o in list {
            let e = end(o);
            if o >= e || e > dpd.len() {
                return bad("kh1dpx: DPD item out of range");
            }
            lists[k].push(dpd[o..e].to_vec());
        }
    }
    Ok(Split { lists, offs, vram })
}

/// The DPD's bytes, and each item's offset in them.
fn join(s: &Split) -> (Vec<u8>, Vec<Vec<u32>>) {
    let header = 4 + s.lists.iter().map(|l| 4 + 4 * l.len()).sum::<usize>() + 16;
    let mut at = (header + 15) & !15;
    let mut offs: Vec<Vec<u32>> = Vec::new();
    for list in &s.lists {
        let mut o = Vec::new();
        for item in list {
            o.push(at as u32);
            at = (at + item.len() + 15) & !15;
        }
        offs.push(o);
    }
    let mut out = vec![0u8; at];
    put_u32(&mut out, 0, DPD_MAGIC);
    let mut h = 4;
    for o in &offs {
        put_u32(&mut out, h, o.len() as u32);
        h += 4;
        for &x in o {
            put_u32(&mut out, h, x);
            h += 4;
        }
    }
    for v in s.vram {
        put_u32(&mut out, h, v as u32);
        h += 4;
    }
    for (list, o) in s.lists.iter().zip(&offs) {
        for (item, &x) in list.iter().zip(o) {
            out[x as usize..x as usize + item.len()].copy_from_slice(item);
        }
    }
    (out, offs)
}

fn tbp(tex0: u64) -> i32 {
    (tex0 & 0x3fff) as i32
}
fn cbp(tex0: u64) -> i32 {
    (tex0 >> 37 & 0x3fff) as i32
}
fn with(tex0: u64, t: i32, c: i32) -> u64 {
    (tex0 & !0x3fff & !(0x3fffu64 << 37)) | (t as u64 & 0x3fff) | ((c as u64 & 0x3fff) << 37)
}

/// The TEX0 words of a shape (`+16` its animation: step count `+6`, steps from `+16`, 8 bytes
/// each, the frame's offset first; the frame's TEX0 at `+16`), as offsets in the shape's bytes.
fn shape_tex0(shape: &[u8]) -> Result<Vec<usize>> {
    let anm = 16;
    let steps = i16_at(shape, anm + 6)?;
    if !(0..=1024).contains(&steps) {
        return bad("kh1dpx: implausible shape");
    }
    let mut v = Vec::new();
    for i in 0..steps as usize {
        let at = i16_at(shape, anm + 16 + i * 8)? as usize;
        let t = anm + at + 16;
        u64_at(shape, t)?;
        if !v.contains(&t) {
            v.push(t);
        }
    }
    Ok(v)
}

/// The TEX0 words of a model's textured primitive groups (`+16` the OMD: primitives at `+4`;
/// each group: kind `+1` (0xFF ends), count `+2`, TEX0 `+8`, then count primitives).
fn model_tex0(model: &[u8]) -> Result<Vec<usize>> {
    const SIZE: [usize; 8] = [0x14, 0x1c, 0x20, 0x28, 0x18, 0x20, 0x28, 0x30];
    let omd = 16;
    let mut at = u32_at(model, omd + 4)? as usize;
    let mut v = Vec::new();
    for _ in 0..512 {
        let kind = *model.get(omd + at + 1).ok_or_else(|| crate::Error::Format("kh1dpx: model out of range".into()))?;
        if kind == 0xff {
            return Ok(v);
        }
        let size = *SIZE.get(kind as usize).ok_or_else(|| crate::Error::Format(format!("kh1dpx: model primitive kind {kind}")))?;
        let count = i16_at(model, omd + at + 2)?;
        if !(0..=8192).contains(&count) {
            return bad("kh1dpx: implausible model");
        }
        if matches!(kind, 2 | 3 | 6 | 7) {
            u64_at(model, omd + at + 8)?;
            v.push(omd + at + 8);
        }
        at += 16 + count as usize * size;
    }
    bad("kh1dpx: model without an end")
}

/// Blocks (256 bytes) a texture takes from its TBP (its upload rectangle `x, y, w, h` in a buffer
/// `bw * 64` pixels wide; 8 or 4 bits a pixel).
/// The texture records of a DPD: each one's offset in it and its TBP.
pub fn texture_records(dpd: &[u8]) -> Result<Vec<(usize, i32)>> {
    let s = split(dpd)?;
    Ok(s.lists[1].iter().zip(&s.offs[1]).map(|(t, &o)| (o, i16_at(t, 0).unwrap_or(-1) as i32)).collect())
}

fn texture_blocks(tex: &[u8]) -> Result<i32> {
    let bw = i16_at(tex, 4)? as i32;
    let psm = i16_at(tex, 6)?;
    let (x, y) = (i16_at(tex, 8)? as i32, i16_at(tex, 10)? as i32);
    let (w, h) = (u16_at(tex, 12)? as i32, u16_at(tex, 14)? as i32);
    let width = (bw * 64).max(x + w);
    let bytes = (y + h) * width * if psm == 0x14 { 1 } else { 2 } / 2;
    Ok((bytes + 255) / 256)
}

/// Where a texture of a [`Kh1Dpx`] came from.
#[derive(Clone, Copy, Debug)]
pub struct TexOrigin {
    /// From the effect's own package (else from the common textures).
    pub own: bool,
    /// Where the DPD it came from starts: in the PAX (own) or in the common file.
    pub src_dpd_at: usize,
    /// Its record's offset in the DPD it came from, and its TBP there.
    pub src_offset: usize,
    pub src_tbp: i32,
    /// Its record's offset in the DPX.
    pub dpx_offset: usize,
}

/// A KH1 DPX made from KH2 effects of one PAX: one DPD per [`build_groups`] group.
pub struct Kh1Dpx {
    /// The DPX: header, an entry per effect, the DPD list, then the DPDs.
    pub bytes: Vec<u8>,
    /// MD5 of `bytes` (KH1 keys a loaded file's HD textures by its file's MD5).
    pub md5: [u8; 16],
    /// Each texture's origin, every DPD's in turn.
    pub textures: Vec<TexOrigin>,
    /// Where the first group's source DPD starts in the PAX, and the common DPD in the common
    /// file.
    pub own_dpd_at: usize,
    pub common_dpd_at: usize,
    /// The effect numbers it holds, in entry order.
    pub effects: Vec<u32>,
    /// Each entry's program types (its particle data's type list and every emitter's records,
    /// as KH2 numbers them, each once), in entry order.
    pub types: Vec<Vec<u32>>,
    /// Each program-type word (an emitter's program record, or a particle data's type list) whose
    /// KH2 type is one of [`KH2_OWN_PROGRAMS`]: its offset in `bytes` and the KH2 type. In `bytes`
    /// 43, 49 and 201 stay as they are and 241 is `pppDummyFunc`; KH1's relocation turns each word into a handle of
    /// its program table entry.
    pub programs: Vec<(usize, u32)>,
    /// The groups left out (their index and why: e.g. more texture slots than KH1 has).
    pub skipped: Vec<(usize, String)>,
    /// What was moved and copied, for the log.
    pub report: String,
}

/// The program types of a particle data (its program file at `+0x110`: the type list at `+8`,
/// the emitters at `+0xC`, each with its program count at `+0x26` and 16-byte programs from
/// `+0x28`, the type first): (offset, type) of each.
fn program_types(pd: &[u8]) -> Result<Vec<(usize, u32)>> {
    let pf = 0x110;
    let mut v = Vec::new();
    let types_at = pf + u32_at(pd, pf + 8)? as usize;
    let nt = i32_at(pd, types_at)?;
    for k in 0..nt.clamp(0, 256) as usize {
        let at = types_at + 4 + k * 4;
        v.push((at, u32_at(pd, at)?));
    }
    let em_at = pf + u32_at(pd, pf + 0xC)? as usize;
    let ne = i32_at(pd, em_at)?;
    for k in 0..ne.clamp(0, 256) as usize {
        let e = pf + u32_at(pd, em_at + 4 + k * 4)? as usize;
        let np = i16_at(pd, e + 0x26)?;
        for j in 0..np.clamp(0, 256) as usize {
            let at = e + 0x28 + j * 16;
            v.push((at, u32_at(pd, at)?));
        }
    }
    Ok(v)
}

fn bit(mask_at: usize, pd: &[u8], i: usize) -> bool {
    pd.get(mask_at + i / 8).is_some_and(|b| b >> (i % 8) & 1 != 0)
}

/// A copy of one effect's particle data under a new effect number, with its header's local
/// transform under the effect root (`+0x80` offset, `+0x90` angles; KH1 builds the root as the
/// instance's position, angles and scale times this, `0x14031CEA0`, Steam) replaced. A KH2
/// element's own offset and rotation are folded into it, so KH1's root (its bone binding) carries
/// them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Variant {
    /// The new effect number (below `0x800`: KH1's element start keeps 11 bits).
    pub number: u32,
    /// The effect it copies.
    pub effect: u32,
    pub offset: [f32; 3],
    pub angle: [i32; 3],
}

/// A particle data header's local transform (`+0x80` offset, `+0x90` angles 65536 = 360°,
/// `+0xA0` scale) for effect `effect` of `pax`.
pub fn header_local(pax: &[u8], effect: u32) -> Result<([f32; 3], [i32; 3], [f32; 3])> {
    let dpx_at = u32_at(pax, 12)? as usize;
    let dpx = pax.get(dpx_at..).ok_or_else(|| crate::Error::Format("kh1dpx: DPX out of range".into()))?;
    let n = i32_at(dpx, 12)?.clamp(0, 4096) as usize;
    let e = (0..n).map(|i| 16 + i * 32).find(|&o| u32_at(dpx, o + 8).ok() == Some(effect)).ok_or_else(|| crate::Error::Format(format!("kh1dpx: no effect {effect}")))?;
    let dpd = dpx.get(u32_at(dpx, e)? as usize..).ok_or_else(|| crate::Error::Format("kh1dpx: DPD out of range".into()))?;
    let pd = u32_at(dpd, 8 + u32_at(dpx, e + 4)? as usize * 4)? as usize;
    let f = |at: usize| u32_at(dpd, pd + at).map(f32::from_bits);
    Ok(([f(0x80)?, f(0x84)?, f(0x88)?], [i32_at(dpd, pd + 0x90)?, i32_at(dpd, pd + 0x94)?, i32_at(dpd, pd + 0x98)?], [f(0xA0)?, f(0xA4)?, f(0xA8)?]))
}

/// KH1's DPX for KH2 effects `effects` of `pax` (a PAX, `PAX_` first; all in one DPD), with the
/// KH2 common textures they use taken from `common` (KH2's `effect/texcommon.dpd`).
///
/// Only the particle datas of those effects are kept (renumbered in the DPX entries), and only
/// the textures they draw with: the DPD's own and the common ones, packed from KH1's own window
/// (TBP `0x3800`, CBP `0x3D40`) so the effect takes as few of KH1's texture slots as it can.
/// Shapes, models and vertex sets are kept whole (they take no texture memory).
pub fn build(pax: &[u8], common: &[u8], effects: &[u32]) -> Result<Kh1Dpx> {
    build_with(pax, common, effects, &[])
}

/// [`build`], plus `variants` (each a copy of one of `effects`' particle datas under its own
/// number, its header's local transform replaced) after them.
pub fn build_with(pax: &[u8], common: &[u8], effects: &[u32], variants: &[Variant]) -> Result<Kh1Dpx> {
    let p = Pax::read(pax)?;
    let mut items = Vec::new();
    for &e in effects {
        if p.entry(e).is_some() {
            items.push(Item { number: e, effect: e, local: None });
        }
    }
    if items.is_empty() {
        return bad(format!("kh1dpx: none of effects {effects:?}"));
    }
    for v in variants {
        if !effects.contains(&v.effect) {
            return bad(format!("kh1dpx: variant {} of effect {}, which isn't built", v.number, v.effect));
        }
        items.push(Item { number: v.number, effect: v.effect, local: Some((v.offset, v.angle)) });
    }
    let r = assemble(&p, pax, common, &[items])?;
    match r.skipped.first() {
        Some((_, why)) => Err(crate::Error::Format(why.clone())),
        None => Ok(r),
    }
}

/// KH1's DPX for KH2 effects of `pax` (a PAX, `PAX_` first), in groups: each group's variants
/// (each a copy of a KH2 effect's particle data under its own number, its header's local
/// transform replaced) become one DPD of the DPX, holding only those particle datas and the
/// textures they draw with (the source DPD's own and the KH2 common ones from `common`, KH2's
/// `effect/texcommon.dpd`), packed from KH1's own window (TBP `0x3800`, CBP `0x3D40`). A
/// group's effects must all be in one DPD of the PAX; different groups may come from different
/// DPDs. KH1 gives each DPD its own run of texture slots when drawn (`0x1401EDB00`, keyed by the
/// DPD), so a group is what may draw at once with one run. A group that can't be built (more
/// texture slots than KH1 has, a texture missing) is left out and listed in
/// [`Kh1Dpx::skipped`]; the effect numbers must be unique and below `0x800`.
pub fn build_groups(pax: &[u8], common: &[u8], groups: &[Vec<Variant>]) -> Result<Kh1Dpx> {
    let p = Pax::read(pax)?;
    let groups: Vec<Vec<Item>> = groups.iter().map(|g| g.iter().map(|v| Item { number: v.number, effect: v.effect, local: Some((v.offset, v.angle)) }).collect()).collect();
    assemble(&p, pax, common, &groups)
}

/// One entry of a built DPD: its number, the KH2 effect it copies, and its new local transform
/// (none: the effect's own).
#[derive(Clone, Copy)]
struct Item {
    number: u32,
    effect: u32,
    local: Option<([f32; 3], [i32; 3])>,
}

/// The PAX's DPX: its entries and the DPDs' places.
struct Pax<'a> {
    dpx_at: usize,
    dpx: &'a [u8],
    entries: Vec<&'a [u8]>,
    dpds: Vec<usize>,
}

impl<'a> Pax<'a> {
    fn read(pax: &'a [u8]) -> Result<Pax<'a>> {
        let dpx_at = u32_at(pax, 12)? as usize;
        let dpx = pax.get(dpx_at..).ok_or_else(|| crate::Error::Format("kh1dpx: DPX out of range".into()))?;
        if i32_at(dpx, 0)? != DPX_VERSION {
            return bad("kh1dpx: unexpected DPX version");
        }
        let n = i32_at(dpx, 12)?;
        if !(0..=4096).contains(&n) {
            return bad("kh1dpx: implausible DPX");
        }
        let entries: Vec<&[u8]> = (0..n as usize).map(|i| dpx.get(16 + i * 32..16 + i * 32 + 32)).collect::<Option<_>>().ok_or_else(|| crate::Error::Format("kh1dpx: DPX entries out of range".into()))?;
        let list_at = 16 + n as usize * 32;
        let nd = i32_at(dpx, list_at)?;
        let dpds = (0..nd.clamp(0, 64) as usize).filter_map(|i| u32_at(dpx, list_at + 4 + i * 4).ok().map(|x| x as usize)).collect();
        Ok(Pax { dpx_at, dpx, entries, dpds })
    }

    /// The entry of effect number `e` (the first).
    fn entry(&self, e: u32) -> Option<&'a [u8]> {
        self.entries.iter().copied().find(|x| u32_at(x, 8).ok() == Some(e))
    }

    /// The DPD at `off` in the DPX: it runs to the next DPD of the list (or the DPX's end).
    fn dpd(&self, off: usize) -> Result<&'a [u8]> {
        let end = self.dpds.iter().copied().filter(|&x| x > off).chain(std::iter::once(self.dpx.len())).min().unwrap();
        self.dpx.get(off..end).ok_or_else(|| crate::Error::Format("kh1dpx: DPD out of range".into()))
    }
}

/// One built DPD.
struct Built {
    dpd: Vec<u8>,
    /// Each item's particle data index in it.
    pds: Vec<usize>,
    /// Each texture's origin (`dpx_offset` relative to the DPD until placed).
    origins: Vec<TexOrigin>,
    /// KH2-own program words: (particle data, offset in it, type).
    own: Vec<(usize, usize, u32)>,
    types: Vec<Vec<u32>>,
    report: String,
}

/// One DPD of `items` (all of one source DPD).
fn build_dpd(p: &Pax, com: &Split, common_dpd_at: usize, items: &[Item]) -> Result<Built> {
    let entries: Vec<&[u8]> = items.iter().map(|i| p.entry(i.effect).ok_or_else(|| crate::Error::Format(format!("kh1dpx: no effect {}", i.effect)))).collect::<Result<_>>()?;
    let dpd_off = u32_at(entries[0], 0)? as usize;
    if entries.iter().any(|e| u32_at(e, 0).ok() != Some(dpd_off as u32)) {
        return bad("kh1dpx: the effects are in more than one DPD");
    }
    let src = split(p.dpd(dpd_off)?)?;
    let src_dpd_at = p.dpx_at + dpd_off;
    let mut report = Vec::new();
    // Each item's source particle data.
    let mut srcs: Vec<usize> = Vec::new();
    for e in &entries {
        let i = u32_at(e, 4)? as usize;
        if i >= src.lists[0].len() {
            return bad("kh1dpx: particle data index out of range");
        }
        srcs.push(i);
    }
    // The TEX0 words of every shape and model, and which (TBP, CBP) each item's particle data draws with.
    let tex0: Vec<Vec<usize>> = src.lists[2].iter().map(|x| shape_tex0(x)).chain(src.lists[3].iter().map(|x| model_tex0(x))).collect::<Result<_>>()?;
    let item = |k: usize| if k < src.lists[2].len() { &src.lists[2][k] } else { &src.lists[3][k - src.lists[2].len()] };
    let ns = src.lists[2].len();
    let mut drawn: Vec<Vec<(i32, i32)>> = Vec::new();
    for &s in &srcs {
        let pd = &src.lists[0][s];
        let mut v = Vec::new();
        for (k, words) in tex0.iter().enumerate() {
            let used = if k < ns { bit(0xC0, pd, k) } else { bit(0xD0, pd, k - ns) };
            if used {
                for &w in words {
                    let x = u64_at(item(k), w)?;
                    if !v.contains(&(tbp(x), cbp(x))) {
                        v.push((tbp(x), cbp(x)));
                    }
                }
            }
        }
        drawn.push(v);
    }
    // The textures they need (by pixels or by palette): the DPD's own first, then the common ones.
    let all: Vec<(i32, i32)> = drawn.iter().flatten().copied().collect();
    let key = |t: &[u8]| (i16_at(t, 0).unwrap_or(-1) as i32, i16_at(t, 2).unwrap_or(-1) as i32);
    let mut needed: Vec<(Vec<u8>, bool, usize)> = Vec::new();
    for (t, &o) in src.lists[1].iter().zip(&src.offs[1]) {
        let (a, b) = key(t);
        if all.iter().any(|&(x, y)| x == a || y == b) {
            needed.push((t.clone(), true, o));
        }
    }
    for (t, &o) in com.lists[1].iter().zip(&com.offs[1]) {
        let (a, b) = key(t);
        if all.iter().any(|&(x, y)| (x == a || y == b) && !src.lists[1].iter().any(|o| key(o).0 == x || key(o).1 == y)) {
            needed.push((t.clone(), false, o));
        }
    }
    for &(x, y) in &all {
        if !needed.iter().any(|(t, _, _)| key(t).0 == x) || !needed.iter().any(|(t, _, _)| key(t).1 == y) {
            return bad(format!("kh1dpx: no texture for {x:#x}/{y:#x}"));
        }
    }
    // New places, packed.
    let (mut tcur, mut ccur) = (KH1_TBP, KH1_CBP);
    let mut tmap: Vec<(i32, i32)> = Vec::new();
    let mut cmap: Vec<(i32, i32)> = Vec::new();
    let mut textures = Vec::new();
    let mut origins = Vec::new();
    // Textures sharing a TBP (one picture, several palettes or upload rectangles) get the space of
    // the largest.
    let mut span: Vec<(i32, i32)> = Vec::new();
    for (t, _, _) in &needed {
        let (a, blocks) = (key(t).0, texture_blocks(t)?);
        match span.iter_mut().find(|m| m.0 == a) {
            Some(m) => m.1 = m.1.max(blocks),
            None => span.push((a, blocks)),
        }
    }
    for (t, own, src_offset) in &needed {
        let (a, b) = key(t);
        let blocks = span.iter().find(|m| m.0 == a).map_or(0, |m| m.1);
        let mut copy = t.clone();
        let nt = match tmap.iter().find(|m| m.0 == a) {
            Some(m) => m.1,
            None => {
                let nt = tcur;
                tcur += (blocks + 0x1f) & !0x1f;
                tmap.push((a, nt));
                nt
            }
        };
        let nc = match cmap.iter().find(|m| m.0 == b) {
            Some(m) => m.1,
            None => {
                let nc = ccur;
                ccur += 4;
                cmap.push((b, nc));
                nc
            }
        };
        put_u16(&mut copy, 0, nt as u16);
        put_u16(&mut copy, 2, nc as u16);
        // The record's own TEX0 (`+0x10`) names the same places (KH1's records keep them equal).
        let t0 = u64_at(&copy, 0x10)?;
        put_u64(&mut copy, 0x10, with(t0, nt, nc));
        origins.push(TexOrigin { own: *own, src_dpd_at: if *own { src_dpd_at } else { common_dpd_at }, src_offset: *src_offset, src_tbp: a, dpx_offset: 0 });
        report.push(format!("{} {a:#x}/{b:#x} -> {nt:#x}/{nc:#x}", if *own { "own" } else { "common" }));
        textures.push(copy);
    }
    let tslots = (tcur - 0x3781) >> 7;
    let cslots = (ccur - 0x3d21) >> 5;
    if tslots > KH1_TBP_SLOTS || cslots > KH1_CBP_SLOTS {
        return bad(format!("kh1dpx: needs {tslots} TBP / {cslots} CBP slots, KH1 has {KH1_TBP_SLOTS} / {KH1_CBP_SLOTS}"));
    }
    let mut out = Split { lists: Default::default(), offs: Default::default(), vram: [KH1_TBP, tcur, KH1_CBP, ccur] };
    // Shapes' and models' TEX0, through the maps (words whose texture isn't kept stay as they are:
    // no kept particle data draws them).
    for (k, words) in tex0.iter().enumerate() {
        let mut it = item(k).clone();
        for &w in words {
            let x = u64_at(&it, w)?;
            if let (Some(t), Some(c)) = (tmap.iter().find(|m| m.0 == tbp(x)), cmap.iter().find(|m| m.0 == cbp(x))) {
                put_u64(&mut it, w, with(x, t.1, c.1));
            }
        }
        out.lists[if k < ns { 2 } else { 3 }].push(it);
    }
    out.lists[4] = src.lists[4].clone();
    out.lists[1] = textures;
    // The particle datas: their texture mask rebuilt for the new list, programs KH1 lacks set to
    // `pppDummyFunc`, KH2's own programs noted (particle data, offset in it, type). An item with
    // its own local transform gets its own copy; items of the same effect without one share it.
    let mut own: Vec<(usize, usize, u32)> = Vec::new();
    let mut pds: Vec<usize> = Vec::new();
    let mut types: Vec<Vec<u32>> = Vec::new();
    let mut plain: Vec<(usize, usize)> = Vec::new();
    for (ii, it) in items.iter().enumerate() {
        let s = srcs[ii];
        if it.local.is_none() {
            if let Some(&(_, at)) = plain.iter().find(|x| x.0 == s) {
                pds.push(at);
                types.push(types[pds.iter().position(|&x| x == at).unwrap()].clone());
                continue;
            }
        }
        let mut pd = src.lists[0][s].clone();
        for b in &mut pd[0xB0..0xC0] {
            *b = 0;
        }
        for (ti, t) in out.lists[1].iter().enumerate() {
            let (a, b) = key(t);
            let old = (tmap.iter().find(|m| m.1 == a).map(|m| m.0), cmap.iter().find(|m| m.1 == b).map(|m| m.0));
            if drawn[ii].iter().any(|&(x, y)| Some(x) == old.0 || Some(y) == old.1) && ti < 128 {
                pd[0xB0 + ti / 8] |= 1 << (ti % 8);
            }
        }
        if let Some((offset, angle)) = it.local {
            for k in 0..3 {
                put_u32(&mut pd, 0x80 + k * 4, offset[k].to_bits());
                put_u32(&mut pd, 0x90 + k * 4, angle[k] as u32);
            }
            report.push(format!("effect {} as {} (offset {offset:?}, angles {angle:?})", it.effect, it.number));
        }
        let at = out.lists[0].len();
        let mut swapped = Vec::new();
        let mut ty = Vec::new();
        for (o, t) in program_types(&pd)? {
            if !ty.contains(&t) {
                ty.push(t);
            }
            if KH2_OWN_PROGRAMS.contains(&t) {
                own.push((at, o, t));
            }
            if t > KH1_LAST_PROGRAM {
                put_u32(&mut pd, o, DUMMY_PROGRAM);
                swapped.push(t);
            }
        }
        if !swapped.is_empty() && it.local.is_none() {
            report.push(format!("particle data {s}: programs {swapped:?} not in KH1 (dummy until plugged in)"));
        }
        if it.local.is_none() {
            plain.push((s, at));
        }
        pds.push(at);
        types.push(ty);
        out.lists[0].push(pd);
    }
    let kept: Vec<usize> = srcs.clone();
    report.insert(0, format!("particle datas {kept:?}, {} textures, vram {KH1_TBP:#x}..{tcur:#x} / {KH1_CBP:#x}..{ccur:#x} ({tslots} TBP, {cslots} CBP slots)", out.lists[1].len()));
    let (dpd, dpd_offs) = join(&out);
    for (o, &x) in origins.iter_mut().zip(&dpd_offs[1]) {
        o.dpx_offset = x as usize;
    }
    let own = own.into_iter().map(|(pd, off, ty)| (pd, dpd_offs[0][pd] as usize + off, ty)).collect();
    Ok(Built { dpd, pds, origins, own, types, report: report.join("; ") })
}

/// The DPX of `groups` (each one DPD).
fn assemble(p: &Pax, pax: &[u8], common: &[u8], groups: &[Vec<Item>]) -> Result<Kh1Dpx> {
    let _ = pax;
    let common_dpd_at = if u32_at(common, 0).ok() == Some(DPD_MAGIC) { 0 } else { 16 };
    let com = split(common.get(common_dpd_at..).ok_or_else(|| crate::Error::Format("kh1dpx: common DPD out of range".into()))?)?;
    let mut seen: Vec<u32> = Vec::new();
    for it in groups.iter().flatten() {
        if it.number >= 0x800 || seen.contains(&it.number) {
            return bad(format!("kh1dpx: effect number {} taken or too large", it.number));
        }
        seen.push(it.number);
    }
    let mut built: Vec<(usize, Built)> = Vec::new();
    let mut skipped = Vec::new();
    for (gi, g) in groups.iter().enumerate() {
        if g.is_empty() {
            continue;
        }
        match build_dpd(p, &com, common_dpd_at, g) {
            Ok(b) => built.push((gi, b)),
            Err(e) => skipped.push((gi, e.to_string())),
        }
    }
    if built.is_empty() {
        return bad(format!("kh1dpx: nothing built ({})", skipped.iter().map(|s| s.1.clone()).collect::<Vec<_>>().join("; ")));
    }
    let count: usize = built.iter().map(|(gi, _)| groups[*gi].len()).sum();
    let head = 16 + count * 32 + 4 + 4 * built.len();
    let mut at = (head + 15) & !15;
    let mut places = Vec::new();
    for (_, b) in &built {
        places.push(at);
        at = (at + b.dpd.len() + 15) & !15;
    }
    let mut bytes = vec![0u8; at];
    put_u32(&mut bytes, 0, DPX_VERSION as u32);
    put_u32(&mut bytes, 12, count as u32);
    let (mut nums, mut types, mut textures, mut programs, mut report) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut e = 0;
    for ((gi, b), &place) in built.iter().zip(&places) {
        for (k, it) in groups[*gi].iter().enumerate() {
            let o = 16 + e * 32;
            bytes[o..o + 32].copy_from_slice(p.entry(it.effect).unwrap());
            put_u32(&mut bytes, o, place as u32);
            put_u32(&mut bytes, o + 4, b.pds[k] as u32);
            put_u32(&mut bytes, o + 8, it.number);
            nums.push(it.number);
            types.push(b.types[k].clone());
            e += 1;
        }
        bytes[place..place + b.dpd.len()].copy_from_slice(&b.dpd);
        textures.extend(b.origins.iter().map(|o| TexOrigin { dpx_offset: place + o.dpx_offset, ..*o }));
        programs.extend(b.own.iter().map(|&(_, off, ty)| (place + off, ty)));
        report.push(format!("[DPD {}: {}]", places.iter().position(|&x| x == place).unwrap(), b.report));
    }
    let list = 16 + count * 32;
    put_u32(&mut bytes, list, built.len() as u32);
    for (i, &place) in places.iter().enumerate() {
        put_u32(&mut bytes, list + 4 + i * 4, place as u32);
    }
    for (gi, why) in &skipped {
        report.push(format!("[group {gi} left out: {why}]"));
    }
    let own_dpd_at = p.dpx_at + u32_at(p.entry(groups[built[0].0][0].effect).unwrap(), 0)? as usize;
    use md5::{Digest, Md5};
    let md5: [u8; 16] = Md5::digest(&bytes).into();
    Ok(Kh1Dpx { bytes, md5, textures, own_dpd_at, common_dpd_at, effects: nums, types, programs, skipped, report: report.join(" ") })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A PAX with one effect (number 33) of one particle data without programs or textures, and
    /// an empty common DPD.
    fn tiny() -> (Vec<u8>, Vec<u8>) {
        tiny_with(&[])
    }

    /// [`tiny`] with one emitter running `programs` (each also in the type list).
    fn tiny_with(programs: &[u32]) -> (Vec<u8>, Vec<u8>) {
        let dpd_of = |pds: &[Vec<u8>]| {
            let head = 4 + (4 + 4 * pds.len()) + 4 * 4 + 16;
            let mut d = vec![0u8; head];
            put_u32(&mut d, 0, DPD_MAGIC);
            put_u32(&mut d, 4, pds.len() as u32);
            let mut at = head;
            for (i, pd) in pds.iter().enumerate() {
                put_u32(&mut d, 8 + i * 4, at as u32);
                at += pd.len();
            }
            for pd in pds {
                d.extend_from_slice(pd);
            }
            d
        };
        let n = programs.len();
        let mut pd = vec![0u8; 0x110 + 0x18 + 4 * n + 8 + 0x28 + 16 * n];
        for (k, v) in [1.0f32, 2.0, 3.0].iter().enumerate() {
            put_u32(&mut pd, 0x80 + k * 4, v.to_bits());
            put_u32(&mut pd, 0xA0 + k * 4, 1f32.to_bits());
        }
        for (k, v) in [100i32, 200, 300].iter().enumerate() {
            put_u32(&mut pd, 0x90 + k * 4, *v as u32);
        }
        put_u32(&mut pd, 0x110 + 8, 0x10);
        if n == 0 {
            put_u32(&mut pd, 0x110 + 0xC, 0x14);
        } else {
            // Type list at +0x10, emitter list after it, one emitter after that.
            put_u32(&mut pd, 0x110 + 0x10, n as u32);
            let el = 0x14 + 4 * n;
            put_u32(&mut pd, 0x110 + 0xC, el as u32);
            put_u32(&mut pd, 0x110 + el, 1);
            let em = el + 8;
            put_u32(&mut pd, 0x110 + el + 4, em as u32);
            put_u16(&mut pd, 0x110 + em + 0x26, n as u16);
            for (k, &t) in programs.iter().enumerate() {
                put_u32(&mut pd, 0x110 + 0x14 + 4 * k, t);
                put_u32(&mut pd, 0x110 + em + 0x28 + 16 * k, t);
            }
        }
        let dpd = dpd_of(&[pd]);
        let mut dpx = vec![0u8; 16 + 32 + 8];
        put_u32(&mut dpx, 0, DPX_VERSION as u32);
        put_u32(&mut dpx, 12, 1);
        let n = dpx.len() as u32;
        put_u32(&mut dpx, 16, n);
        put_u32(&mut dpx, 16 + 8, 33);
        put_u32(&mut dpx, 48, 1);
        put_u32(&mut dpx, 52, n);
        dpx.extend_from_slice(&dpd);
        let mut pax = b"PAX_".to_vec();
        pax.extend_from_slice(&[0; 8]);
        pax.extend_from_slice(&16u32.to_le_bytes());
        pax.extend_from_slice(&dpx);
        (pax, dpd_of(&[]))
    }

    #[test]
    fn variant_copies_the_particle_data_with_its_local_transform() {
        let (pax, common) = tiny();
        assert_eq!(header_local(&pax, 33).unwrap(), ([1.0, 2.0, 3.0], [100, 200, 300], [1.0; 3]));
        let v = Variant { number: 0x72E, effect: 33, offset: [4.0, 5.0, 6.0], angle: [0, 0, 0x8000] };
        let r = build_with(&pax, &common, &[33], &[v]).unwrap();
        assert_eq!(r.effects, vec![33, 0x72E]);
        let b = &r.bytes;
        assert_eq!(i32_at(b, 12).unwrap(), 2);
        // Both entries in the one DPD; the variant's particle data is the second.
        let dpd_at = u32_at(b, 16).unwrap() as usize;
        assert_eq!(u32_at(b, 48).unwrap() as usize, dpd_at);
        assert_eq!((u32_at(b, 16 + 4).unwrap(), u32_at(b, 48 + 4).unwrap(), u32_at(b, 48 + 8).unwrap()), (0, 1, 0x72E));
        let dpd = &b[dpd_at..];
        assert_eq!(u32_at(dpd, 4).unwrap(), 2);
        let (p0, p1) = (u32_at(dpd, 8).unwrap() as usize, u32_at(dpd, 12).unwrap() as usize);
        assert_eq!(f32::from_bits(u32_at(dpd, p0 + 0x80).unwrap()), 1.0);
        assert_eq!(f32::from_bits(u32_at(dpd, p1 + 0x84).unwrap()), 5.0);
        assert_eq!(i32_at(dpd, p1 + 0x98).unwrap(), 0x8000);
        assert_eq!(dpd[p0 + 0x110..p0 + 0x128], dpd[p1 + 0x110..p1 + 0x128]);
        // A number KH1 can't start, or one taken, is refused.
        assert!(build_with(&pax, &common, &[33], &[Variant { number: 33, ..v }]).is_err());
        assert!(build_with(&pax, &common, &[33], &[Variant { number: 0x800, ..v }]).is_err());
    }
    #[test]
    fn kh2_own_programs_are_listed() {
        let (pax, common) = tiny_with(&[201, 241, 142, 43]);
        let v = Variant { number: 0x72E, effect: 33, offset: [0.0; 3], angle: [0; 3] };
        let r = build_with(&pax, &common, &[33], &[v]).unwrap();
        // The type list and the emitter's records, in the particle data and in its variant's copy.
        assert_eq!(r.programs.len(), 12);
        assert_eq!(r.programs.iter().filter(|p| p.1 == 43).count(), 4);
        for &(at, ty) in &r.programs {
            let word = u32_at(&r.bytes, at).unwrap();
            assert_eq!((ty, word), (ty, if ty == 241 { DUMMY_PROGRAM } else { ty }));
        }
        assert_eq!(r.programs.iter().filter(|p| p.1 == 201).count(), 4);
    }

    /// A particle data without textures running `programs` in one emitter (each also in the type
    /// list), its header offset `x` along x.
    fn pd_with(programs: &[u32], x: f32) -> Vec<u8> {
        let n = programs.len();
        let mut pd = vec![0u8; 0x110 + 0x18 + 4 * n + 8 + 0x28 + 16 * n];
        put_u32(&mut pd, 0x80, x.to_bits());
        put_u32(&mut pd, 0x110 + 8, 0x10);
        put_u32(&mut pd, 0x110 + 0x10, n as u32);
        let el = 0x14 + 4 * n;
        put_u32(&mut pd, 0x110 + 0xC, el as u32);
        put_u32(&mut pd, 0x110 + el, 1);
        let em = el + 8;
        put_u32(&mut pd, 0x110 + el + 4, em as u32);
        put_u16(&mut pd, 0x110 + em + 0x26, n as u16);
        for (k, &t) in programs.iter().enumerate() {
            put_u32(&mut pd, 0x110 + 0x14 + 4 * k, t);
            put_u32(&mut pd, 0x110 + em + 0x28 + 16 * k, t);
        }
        pd
    }

    /// A PAX of two DPDs: effects 10 and 11 (particle datas 0, 1 of the first), 20 (the second's
    /// 0).
    fn two_dpds() -> (Vec<u8>, Vec<u8>) {
        let dpd_of = |pds: &[Vec<u8>]| {
            let head = 4 + (4 + 4 * pds.len()) + 4 * 4 + 16;
            let mut d = vec![0u8; head];
            put_u32(&mut d, 0, DPD_MAGIC);
            put_u32(&mut d, 4, pds.len() as u32);
            let mut at = head;
            for (i, pd) in pds.iter().enumerate() {
                put_u32(&mut d, 8 + i * 4, at as u32);
                at += pd.len();
            }
            for pd in pds {
                d.extend_from_slice(pd);
            }
            d
        };
        let a = dpd_of(&[pd_with(&[201, 30], 1.0), pd_with(&[43], 2.0)]);
        let b = dpd_of(&[pd_with(&[241, 38], 3.0)]);
        let head = 16 + 3 * 32 + 4 + 8;
        let mut dpx = vec![0u8; head];
        put_u32(&mut dpx, 0, DPX_VERSION as u32);
        put_u32(&mut dpx, 12, 3);
        let (oa, ob) = (head as u32, (head + a.len()) as u32);
        for (i, (o, pd, num)) in [(oa, 0u32, 10u32), (oa, 1, 11), (ob, 0, 20)].into_iter().enumerate() {
            put_u32(&mut dpx, 16 + i * 32, o);
            put_u32(&mut dpx, 16 + i * 32 + 4, pd);
            put_u32(&mut dpx, 16 + i * 32 + 8, num);
            put_u32(&mut dpx, 16 + i * 32 + 12, 0xABC0 + i as u32);
        }
        put_u32(&mut dpx, 16 + 3 * 32, 2);
        put_u32(&mut dpx, 16 + 3 * 32 + 4, oa);
        put_u32(&mut dpx, 16 + 3 * 32 + 8, ob);
        dpx.extend_from_slice(&a);
        dpx.extend_from_slice(&b);
        let mut pax = b"PAX_".to_vec();
        pax.extend_from_slice(&[0; 8]);
        pax.extend_from_slice(&16u32.to_le_bytes());
        pax.extend_from_slice(&dpx);
        (pax, dpd_of(&[]))
    }

    #[test]
    fn groups_make_one_dpd_each() {
        let (pax, common) = two_dpds();
        let v = |number, effect| Variant { number, effect, offset: [number as f32, 0.0, 0.0], angle: [0; 3] };
        let r = build_groups(&pax, &common, &[vec![v(0x70A, 10), v(0x70B, 11), v(0x70C, 10)], vec![v(0x714, 20)]]).unwrap();
        assert!(r.skipped.is_empty(), "{:?}", r.skipped);
        assert_eq!(r.effects, vec![0x70A, 0x70B, 0x70C, 0x714]);
        let b = &r.bytes;
        assert_eq!(i32_at(b, 12).unwrap(), 4);
        // Two DPDs listed; the first three entries in the first, each its own particle data.
        let list = 16 + 4 * 32;
        assert_eq!(u32_at(b, list).unwrap(), 2);
        let (d0, d1) = (u32_at(b, list + 4).unwrap(), u32_at(b, list + 8).unwrap());
        for (k, (d, pd)) in [(d0, 0), (d0, 1), (d0, 2), (d1, 0)].into_iter().enumerate() {
            assert_eq!((u32_at(b, 16 + k * 32).unwrap(), u32_at(b, 16 + k * 32 + 4).unwrap()), (d, pd));
            // The source entry's other words are kept.
            assert_eq!(u32_at(b, 16 + k * 32 + 12).unwrap(), 0xABC0 + [0, 1, 0, 2][k]);
        }
        let dpd = &b[d0 as usize..];
        assert_eq!(u32_at(dpd, 4).unwrap(), 3);
        let at = |i: usize| u32_at(dpd, 8 + 4 * i).unwrap() as usize;
        assert_eq!(f32::from_bits(u32_at(dpd, at(2) + 0x80).unwrap()), 0x70C as f32);
        assert_eq!(dpd[at(0) + 0x110..at(0) + 0x130], dpd[at(2) + 0x110..at(2) + 0x130]);
        assert_eq!(r.types, vec![vec![201, 30], vec![43], vec![201, 30], vec![241, 38]]);
        // KH2's own programs in both DPDs (type list and record each), 241 as the dummy.
        assert_eq!(r.programs.iter().filter(|p| p.1 == 201).count(), 4);
        assert_eq!(r.programs.iter().filter(|p| p.1 == 43).count(), 2);
        assert_eq!(r.programs.iter().filter(|p| p.1 == 241).count(), 2);
        for &(at, ty) in &r.programs {
            assert_eq!(u32_at(b, at).unwrap(), if ty == 241 { DUMMY_PROGRAM } else { ty });
            assert_eq!(at >= d1 as usize, ty == 241);
        }
        // A group across two DPDs is left out, the rest still built; numbers must be unique.
        let r = build_groups(&pax, &common, &[vec![v(0x70A, 10), v(0x714, 20)], vec![v(0x70B, 11)]]).unwrap();
        assert_eq!((r.skipped.len(), r.skipped[0].0, r.effects.clone()), (1, 0, vec![0x70B]));
        assert!(build_groups(&pax, &common, &[vec![v(0x70A, 10)], vec![v(0x70A, 20)]]).is_err());
    }
}
