//! KH2's enemy HP gauge drawn by KH1's own enemy HP bar draw (tm45; the owner, chat 2026-11-11: "Can
//! you add an option in the config to have the choice between KH1 and KH2's enemy health bars?
//! (Scan)"; notes `docs/kh1-kh2-enemy-hp.md`). Misc > Enemy HP: KH1 | KH2 (ini `enemy_hp`).
//!
//! **KH1's bar** (Steam, Checked (decomp)): KH1's target cursor draw `0x140274B20` asks
//! `0x1402A4D40` each frame: with Scan equipped (`0x140284BE0 (10)`), a target of kind 3 (an enemy,
//! `+6`) not flagged `(+0x374 & 0x60) == 0x20`, it gives the target's HP and max HP (its stats `+0x3C`
//! / `+0x40`) to `0x140274E70`, which keeps them (`0x14284FE6C` / `0x14284FE70`), signals the frame
//! (`0x142850900`) and starts the bar (`0x14284FE60` 0 -> 1). KH1's task `0x1402758B0` runs the
//! bar's states (1 -> 2 fading in 0.2 a frame, 3 shown, 4 once a frame passes without the signal,
//! 5 fading out, 0) and calls the draw **`0x140275000 (hp, max, fade)`** while not 0.
//!
//! **Here**: that draw is wrapped. KH1 mode (or KH2's art not ready): KH1's own draw, untouched. KH2
//! mode: KH1's draw's own set-up (`0x1402775D0` / `0x14027D190` on `0x142851A00`, its packet pointer
//! `0x142851910` at `0x142EE8F60`), then KH2's gauge (`crate::enemygauge`'s rules, KH2's `gaug`
//! sequence groups 38..46 played by `crate::seqplay`) through KH1's sprite-to-quad and submit as the
//! Drive gauge draws (`gaugehud::Kh1`), and nothing of KH1's bar. So KH1 still decides when a bar
//! shows (Scan, the target, events, the HUD): the owner's answer (b), "B". KH2 has no fade: shown
//! while KH1's bar is in states 2 / 3, gone at once in 4 / 5.
//!
//! Per frame: one load and a call while KH1's bar task runs; the gauge's steps and quads only while
//! a bar is up in KH2 mode.

use super::addr::St;
use super::log;
use crate::enemygauge::{Gauge, Pip, PIPS};
use crate::seqplay::{Player, Quad, Seq};
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::sync::Mutex;

/// KH1's enemy HP bar draw (Steam `0x140275000`), first bytes `mov rax, rsp; mov [rax+8], rbx`
/// (Checked (exe), both builds' bytes the same, no RIP-relative operand).
const DRAW_RVA: St = St(0x275000);
const DRAW_BYTES: [u8; 7] = [0x48, 0x8B, 0xC4, 0x48, 0x89, 0x58, 0x08];
/// KH1's draw's set-up (Checked (decomp + exe), `0x140275000`'s first statements).
const SETUP_RVA: St = St(0x2775D0);
const SETUP_BYTES: [u8; 6] = [0x33, 0xD2, 0xC7, 0x44, 0x24, 0x08];
const REGS_RVA: St = St(0x27D190);
const REGS_BYTES: [u8; 8] = [0x48, 0x89, 0x5C, 0x24, 0x10, 0x55, 0x48, 0x8B];
const BLOCK_RVA: St = St(0x2851A00);
const PACKET_RVA: St = St(0x2851910);
const PACKET_START_RVA: St = St(0x2EE8F60);
/// The bar's state (`0x14284FE60`) and KH1's target (`0x142DB3518`, the cursor's).
const STATE_RVA: St = St(0x284FE60);
const TARGET_RVA: St = St(0x2DB3518);
/// KH1's HUD's side margin (`0x14232DCCC`, made with the screen's shape by `0x1401703C0`; the HUD's
/// left edge `0x14232DCC8` is its minus).
const MARGIN_RVA: St = St(0x232DCCC);
/// The texture descriptor KH1's bar's quads are built with (`0x1427F1430`, its TEX0 at `+0x18`).
const KH1_DESC_RVA: St = St(0x27F1430);
const FRAME_SCALE_RVA: St = St(0x233FBDC);

/// KH2's pieces (`0x140182DD0`): groups and the order KH2 makes them in (its draw order,
/// Assumption: KH2's sprites of one priority draw in the order they were made).
const ROOT: usize = 38;
const TOP: usize = 42;
const FRAME: usize = 39;
const END: usize = 40;
const DAMAGE: usize = 41;
const FILL: usize = 43;
const PIP_FULL: usize = 44;
const PIP_BACK: usize = 45;
const PIP_LOST: usize = 46;
/// Each piece's x offset (`+0x1DC`, `0x14016A900`'s last argument for every piece of the gauge: 85,
/// Checked (decomp); 85 is (512 × 4/3 − 512) / 2, KH2's shift to a 16:9 screen's right edge:
/// Assumption).
const SHIFT: i32 = 0x55;
/// KH2's screen: 512 wide for 4:3, its 16:9 right edge 85.33 further (Assumption, as `SHIFT`).
const KH2_RIGHT: f32 = 512.0 + 512.0 / 6.0;

/// Misc > Enemy HP: 0 not read, 1 KH1, 2 KH2.
static MODE: AtomicU8 = AtomicU8::new(0);
static ORIGINAL: AtomicUsize = AtomicUsize::new(0);
static BASE: AtomicUsize = AtomicUsize::new(0);

// (b), the owner (chat 2026-11-11, "B"): KH2's gauge only where KH1's Scan shows a bar (KH1's own
// test in `0x1402A4D40`, left as it is). KH2 has the same kind of rule: its gauge needs an ability
// bit (`+0x1D0` bit 19, `0x1403DD440`; Assumption that it is KH2's Scan). The other choice, a gauge
// for every target without Scan, would be a wrapper on `0x1402A4D40` asking it without the Scan
// test; not built.

/// KH2's gauge in KH1's bar's place (the default, as the other KH1 | KH2 rows).
pub fn kh2() -> bool {
    match MODE.load(Ordering::Relaxed) {
        0 => {
            let v = super::settings::get().enemy_hp_kh2;
            MODE.store(1 + v as u8, Ordering::Relaxed);
            v
        }
        v => v == 2,
    }
}

/// The Config row: live, kept in the ini; KH2's art loaded if it wasn't (Drive off).
pub fn set_kh2(v: bool) {
    MODE.store(1 + v as u8, Ordering::Relaxed);
    super::settings::save("enemy_hp", if v { "kh2" } else { "kh1" });
    log!(Menus, "Config menu: Enemy HP {}", if v { "KH2" } else { "KH1" });
    if v {
        super::kh2hud::start();
    }
}

pub unsafe fn install(base: usize) {
    if super::addr::Build::current() == super::addr::Build::Unknown {
        return;
    }
    BASE.store(base, Ordering::Relaxed);
    for (rva, bytes) in [(SETUP_RVA, &SETUP_BYTES[..]), (REGS_RVA, &REGS_BYTES[..])] {
        if std::slice::from_raw_parts((base + rva) as *const u8, bytes.len()) != bytes {
            log!(General, "Enemy HP: unexpected code at 0x{:X}; KH1's bar only", 0x1_4000_0000usize + rva);
            return;
        }
    }
    if !super::gaugehud::calls_ok(base) {
        log!(General, "Enemy HP: KH1's sprite calls aren't as expected; KH1's bar only");
        return;
    }
    super::hook::install(base + DRAW_RVA, &DRAW_BYTES, draw as *const () as usize, &ORIGINAL, "KH1's enemy HP bar draw (Enemy HP)");
}

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

unsafe extern "system" fn draw(hp: i32, max: i32, fade: f32) {
    let orig: unsafe extern "system" fn(i32, i32, f32) = std::mem::transmute(ORIGINAL.load(Ordering::Relaxed));
    let base = BASE.load(Ordering::Relaxed);
    if base == 0 || !kh2() || !ready() {
        if let Ok(mut g) = PIECES.try_lock() {
            if let Some(p) = g.as_mut() {
                p.shown = false;
            }
        }
        return orig(hp, max, fade);
    }
    if std::panic::catch_unwind(|| ours(base, hp, max)).is_err() {
        orig(hp, max, fade);
    }
}

/// KH2's art and its KH1 texture are there (else KH1's own bar, logged once).
fn ready() -> bool {
    static SAID: AtomicU8 = AtomicU8::new(0);
    let ok = super::kh2hud::art().is_some() && super::kh2hud::texture().is_some();
    if !ok && SAID.swap(1, Ordering::Relaxed) == 0 {
        log!(Loading, "Enemy HP: KH2's gauge art isn't ready (see the Drive gauge art line); KH1's bar until it is");
    }
    ok
}

// ------------------------------------------------------------------------------- KH2's pieces

#[derive(Default)]
struct Pieces {
    seq: Option<Seq>,
    gauge: Gauge,
    target: u32,
    shown: bool,
    root: Player,
    top: Player,
    frame: Player,
    end: Player,
    damage: Player,
    fill: Player,
    backs: [Player; PIPS],
    fulls: [Player; PIPS],
    /// The pips' step (`0x140192E00`: group 44's record `+0x20`, Assumption that KH2 asks it of the
    /// pair's second piece).
    step: i32,
}

static PIECES: Mutex<Option<Pieces>> = Mutex::new(None);

impl Pieces {
    /// KH2's HUD making (`0x140182DD0`): every piece on its group from its start.
    fn make(&mut self) {
        let Some(seq) = self.seq.as_ref() else { return };
        for (p, g) in [(&mut self.root, ROOT), (&mut self.top, TOP), (&mut self.frame, FRAME), (&mut self.end, END), (&mut self.damage, DAMAGE), (&mut self.fill, FILL)] {
            p.restart(seq, g);
        }
        for i in 0..PIPS {
            self.backs[i].restart(seq, PIP_BACK);
            self.fulls[i].restart(seq, PIP_FULL);
        }
        self.step = seq.groups.get(PIP_FULL).map_or(0, |g| g.extra[2]);
    }

    fn fulls_restart(&mut self, n: usize) {
        let Some(seq) = self.seq.as_ref() else { return };
        for p in self.fulls.iter_mut().take(n) {
            p.restart(seq, PIP_FULL);
        }
    }

    /// KH1's values this frame into KH2's gauge (a new target or max: `show`; less HP: a hit; more:
    /// a heal).
    fn values(&mut self, target: u32, hp: i32, max: i32) {
        if !self.shown || target != self.target || max != self.gauge.max {
            self.shown = true;
            self.target = target;
            let (g, n) = Gauge::show(hp, max);
            self.gauge = g;
            self.fulls_restart(n);
        } else if hp < self.gauge.now {
            let pips = self.gauge.hit(hp);
            let Some(seq) = self.seq.as_ref() else { return };
            self.damage.restart(seq, DAMAGE);
            for (p, what) in self.fulls.iter_mut().zip(pips) {
                match what {
                    Pip::Full => p.restart(seq, PIP_FULL),
                    Pip::Lost => p.restart(seq, PIP_LOST),
                    Pip::Keep => {}
                }
            }
        } else if hp > self.gauge.now {
            let n = self.gauge.heal(hp);
            self.fulls_restart(n);
        }
    }

    /// The frame's quads (KH2 units × 16 on KH2's screen), in KH2's order.
    fn quads(&mut self, dt: f32) -> Vec<Quad> {
        let mut out = Vec::new();
        let Some(seq) = self.seq.as_ref() else { return out };
        let l = self.gauge.layout();
        for p in [&mut self.root, &mut self.top, &mut self.frame, &mut self.end, &mut self.damage, &mut self.fill].into_iter().chain(self.backs.iter_mut()).chain(self.fulls.iter_mut()) {
            p.step(dt);
        }
        // (`0x140183640`: each bar piece its length / 100 wide, its length left of the anchor.)
        self.frame.scale_x = l.frame as f32 / 100.0;
        self.damage.scale_x = l.before as f32 / 100.0;
        self.fill.scale_x = l.fill as f32 / 100.0;
        // (The root at (+0x28, +0x2C) = (0, KH2's top-right stack, 0 with nothing above it:
        // Assumption, KH1 has none of KH2's pieces there); the others from its attach point,
        // `0x14016ABF0`.)
        self.root.draw(seq, [SHIFT, 0], &mut out);
        let at = self.root.attach(seq);
        let child = |off: i32| [SHIFT + off + at[0], at[1]];
        if l.on_top {
            self.top.draw(seq, child(0), &mut out);
        }
        self.frame.draw(seq, child(-l.frame), &mut out);
        self.end.draw(seq, child(-l.frame), &mut out);
        self.damage.draw(seq, child(-l.before), &mut out);
        self.fill.draw(seq, child(-l.fill), &mut out);
        // (Each pip pair `i` at −i × step, `0x140182DD0`: backs first, then the full one, KH2's
        // making order per pair.)
        for i in 0..PIPS {
            let x = -(i as i32) * self.step;
            if i < l.backs {
                self.backs[i].draw(seq, child(x), &mut out);
            }
            if i < l.fulls {
                self.fulls[i].draw(seq, child(x), &mut out);
            }
        }
        out
    }
}

// ------------------------------------------------------------------------------- KH1's draw

/// KH2 mode, the bar up: KH1's draw's set-up, then KH2's gauge through KH1's quads.
unsafe fn ours(base: usize, hp: i32, max: i32) {
    let state = rd::<i32>(base + STATE_RVA);
    let Ok(mut g) = PIECES.try_lock() else { return };
    let p = g.get_or_insert_with(Pieces::default);
    if p.seq.is_none() {
        let Some(art) = super::kh2hud::art() else { return };
        match Seq::parse(&art.seqd) {
            Ok(s) if s.groups.len() > PIP_LOST => {
                p.seq = Some(s);
                p.make();
                log!(Combat, "Enemy HP: KH2's gauge made from the gaug sequence (pip step {})", p.step);
            }
            Ok(s) => {
                log!(Combat, "Enemy HP: KH2's gaug sequence has {} groups, not the gauge's 38..46", s.groups.len());
                return;
            }
            Err(e) => {
                log!(Combat, "Enemy HP: KH2's gaug sequence not read ({e})");
                return;
            }
        }
    }
    // (KH2 has no fade: KH1's fading in and holding are shown, its fading out isn't.)
    if state != 2 && state != 3 {
        p.shown = false;
        return;
    }
    p.values(rd::<u32>(base + TARGET_RVA), hp, max);
    let dt = rd::<f32>(base + FRAME_SCALE_RVA).clamp(0.0, 5.0);
    let quads = p.quads(dt);
    drop(g);
    let Some((tex, tex0)) = super::kh2hud::texture() else { return };
    // KH1's draw's own first steps (`0x140275000`).
    let setup: unsafe extern "system" fn(usize) = std::mem::transmute(base + SETUP_RVA);
    let regs: unsafe extern "system" fn(usize) = std::mem::transmute(base + REGS_RVA);
    let block = base + BLOCK_RVA;
    setup(block);
    wr(block + 0xE0, 5u64);
    wr(block + 0x60, 0x30003u64);
    regs(block);
    wr(base + PACKET_RVA, base + PACKET_START_RVA);
    super::gaugehud::bind_ours(base, tex, tex0);
    // KH2's screen's top-right corner on KH1's HUD's, at the Drive gauge's KH2 unit (Assumption).
    let k = super::gaugehud::KH2_UNIT;
    let right = 512.0 + rd::<i32>(base + MARGIN_RVA).clamp(0, 1024) as f32;
    let mut kh1 = super::gaugehud::Kh1 { base, alpha: 1.0, tex0, blend: 0, ox: right - KH2_RIGHT * k, oy: 0.0, k, packet: PACKET_RVA };
    for q in &quads {
        kh1.kh2(q);
    }
    kh1.blend(0);
    let own = rd::<u64>(base + KH1_DESC_RVA + 0x18);
    if own != 0 {
        super::gaugehud::bind_tex0(base, own);
    }
}
