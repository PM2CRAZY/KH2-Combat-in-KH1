//! KH2's attack power inside KH1's own damage function (testing branch; the thread "KH2 attack
//! data and damage on KH1's swings", `docs/kh1-kh2-attack-data.md` §5 step 2). The tester picked
//! between KH1's curve × KH2's power (a) and KH2's own formula (b): (a), so KH1's enemies' HP and
//! Sora's growth stay balanced as KH1 tuned them, while each KH2 move keeps KH2's power against the
//! others, rounded up per hit as KH2 does (`0x1403C12F0`, Steam KH2, Checked (decomp)).
//!
//! KH1's damage `0x1402BFA80 (attack, target, record kind word)` (Steam; its only caller is the
//! record builder `0x1402BF630`, Checked (decomp)) looks the damage up in a curve: the table at
//! the pointer `0x142DE5DC0` (a u16 count, then u16 values), at (data power + the attack's stat
//! term − the target's Defense); then the data's min / max, the target's floor / cap, its element
//! resistance, Aero, mode 2 and Second Chance. The function loads that pointer once (one
//! `mov rcx, [rip -> 0x142DE5DC0]` at `0x1402BFBB4`, Checked (exe)).
//!
//! Ours, for an attack of Sora's whose KH2 ATKP we know (a borrowed or form move being landed,
//! `combat::landing`; one of our rows, `kh1rows::kh2_atkp`; a native swing on KH1's own rows,
//! `kh1rows::native_atkp`): for the one call, that pointer points at a copy of KH1's curve with
//! each value × KH2's power / 100 (× `damage_scale`), rounded up (a Magic-based ATKP's stat is
//! KH1's own magic one, max MP, from its data: `kh1rows::kh1_magic`, t917); KH1's function then
//! runs whole (its critical, tally, clamps, resistance, Aero, Second Chance, at least 1). Nothing is rewritten after the fact (`combat::scale_record`'s
//! damage part and its carried fractions are off while this is in).
//!
//! (t918) For the same call, a Strength-based KH2 hit on a KH1 finisher row's data has a plain
//! swing's power in place of the finisher's 4 or 6 points, so KH2's own finisher power % is the
//! only finisher boost (`row_power` in `damage`).
//!
//! (t920) A KH2 spell hit on KH1's own spell data (Fire, Blizzard, Thunder at Sora's KH1 level):
//! KH2's power counts against KH2's normal cast at that level (250 / 300 / 375), so a normal cast
//! deals KH1's own spell hit and KH2 sets only the proportions (`kh1_spell_level`, `normal_cast`).
//!
//! The Criticals row (the tester: "make it a config selection (criticals on or off for all moves
//! that call it)", then 2 (b): every attack, enemies included): Off, KH1's critical roll has no
//! chance for any attack (the attack's `+0x94` read as 0 for the call; KH2 has no critical hits).
//! The Party Criticals row (the tester: "in the config menu make a separate option for party
//! member criticals") decides instead for attacks whose owner is on Sora's side and isn't Sora
//! (Donald, Goofy, the guests; `party_member`).
//!
//! Cost: one hook, called once per hit record; a table of at most 256 u16 built per KH2 hit.

use super::addr::Build;
use super::log;
use super::sys::*;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, AtomicUsize, Ordering};

/// KH1's damage function (Steam).
const DAMAGE_STEAM: usize = 0x2BFA80;
/// Its first three instructions (`mov [rsp+8], rbx; mov [rsp+0x10], rbp; mov [rsp+0x18], rsi`).
const DAMAGE_PROLOGUE: [u8; 15] = [0x48, 0x89, 0x5C, 0x24, 0x08, 0x48, 0x89, 0x6C, 0x24, 0x10, 0x48, 0x89, 0x74, 0x24, 0x18];
/// The pointer to KH1's damage curve (Steam).
const CURVE_PTR_STEAM: usize = 0x2DE5DC0;
/// The attack object: owner handle, data handle, critical chance (f32).
const ATTACK_OWNER: usize = 0x34;
const ATTACK_DATA: usize = 0x5C;
const ATTACK_CRIT_CHANCE: usize = 0x94;
/// The attack data's bone (0x30: the weapon, Sora's Keyblade swings).
const DATA_BONE: usize = 0x10;
/// The attack data's stat source (`+0`, 0 Strength), element (`+4`, low 4 bits) and power against
/// enemies (`+0x48`), `docs/kh1-attack-elements.md`.
const DATA_STAT: usize = 0;
const DATA_ELEMENT: usize = 4;
const DATA_POWER: usize = 0x48;
/// KH1's plain ground swing row (Sora's row 13): a swing's own power, no finisher points (every
/// plain swing row of Sora's has power 0 vs enemies, Checked (files)).
const PLAIN_SWING_ROW: usize = 13;
/// KH1's battle data block (Steam `0x142D22D40`, `btltbl.bin`) and its spells' attack data: Fire,
/// Blizzard, Thunder at levels 1..3 from `+0x5F28`, 0x70 apart (`combat::kh1_spell_data`).
const BLOCK_STEAM: usize = 0x2D22D40;
const KH1_SPELLS: usize = 0x5F28;
const DATA_SIZE: usize = 0x70;
/// KH2 Blizzard's ATKP, whose subs 0, 2, 4 are a normal cast's whole power at levels 1..3 (one
/// shard), and those powers (250, 300, 375; every KH2 spell's normal cast totals the same per
/// target: Fire's ticks and last hit, Checked (files) `00battle.bin`).
const KH2_BLIZZARD_ATKP: u16 = 248;
const KH2_NORMAL_CAST: [u32; 3] = [250, 300, 375];
const WEAPON_BONE: u32 = 0x30;
/// Curve entries we copy at most (KH1's has 42).
const CURVE_MAX: usize = 256;

static BASE: AtomicUsize = AtomicUsize::new(0);
static ORIG: AtomicUsize = AtomicUsize::new(0);
static ON: AtomicBool = AtomicBool::new(false);
/// Criticals: 0 not read yet, 1 off, 2 on.
static CRITS: AtomicU8 = AtomicU8::new(0);
/// KH2 Damage (the Config row; ini `kh2_damage`): the same.
static KH2: AtomicU8 = AtomicU8::new(0);
/// Party Criticals: the same.
static PARTY_CRITS: AtomicU8 = AtomicU8::new(0);
/// An actor's side (`+0x374` bits 5..6; 2 the enemies', Checked (decomp): `0x1402BFA80`).
const ACTOR_FLAGS: usize = 0x374;
static LOGGED: AtomicU32 = AtomicU32::new(0);
/// The scaled curve (count then values); only the game's thread runs the damage function.
static mut CURVE: [u16; CURVE_MAX + 1] = [0; CURVE_MAX + 1];

unsafe fn rd<T: Copy>(p: usize) -> T {
    std::ptr::read_unaligned(p as *const T)
}
unsafe fn wr<T: Copy>(p: usize, v: T) {
    std::ptr::write_unaligned(p as *mut T, v)
}

/// KH2's power is put in by KH1's damage function (the hook is in and KH2 Damage is on):
/// `combat::scale_record` leaves the damage alone.
pub fn active() -> bool {
    ON.load(Ordering::Relaxed) && kh2_damage()
}

/// The Config menu's KH2 Damage row (the tester, 3 (a)): On, KH2's power on Sora's KH2 hits; Off,
/// KH1's damage exactly (no power put in, nothing rewritten after: `combat::scale_record`).
pub fn kh2_damage() -> bool {
    match KH2.load(Ordering::Relaxed) {
        0 => {
            let c = super::settings::get().kh2_damage;
            KH2.store(1 + c as u8, Ordering::Relaxed);
            c
        }
        v => v == 2,
    }
}

/// The Config menu's row: live, kept in the ini.
pub fn set_kh2_damage(on: bool) {
    KH2.store(1 + on as u8, Ordering::Relaxed);
    super::settings::save("kh2_damage", if on { "on" } else { "off" });
    log!(Combat, "Config menu: KH2 Damage {}", if on { "On" } else { "Off" });
}

/// Read again from the settings on the next call (`experimental` switched in the ini, t922).
pub fn forget_kh2_damage() {
    KH2.store(0, Ordering::Relaxed);
}

/// The Config menu's Criticals row (every attack KH1's damage function sees).
pub fn criticals() -> bool {
    match CRITS.load(Ordering::Relaxed) {
        0 => {
            let c = super::settings::get().criticals;
            CRITS.store(1 + c as u8, Ordering::Relaxed);
            c
        }
        v => v == 2,
    }
}

/// The Config menu's row: live, kept in the ini.
pub fn set_criticals(on: bool) {
    CRITS.store(1 + on as u8, Ordering::Relaxed);
    super::settings::save("criticals", if on { "on" } else { "off" });
    log!(Combat, "Config menu: Criticals {}", if on { "On" } else { "Off" });
}

/// The Config menu's Party Criticals row (attacks of Sora's party members, `party_member`).
pub fn party_criticals() -> bool {
    match PARTY_CRITS.load(Ordering::Relaxed) {
        0 => {
            let c = super::settings::get().party_criticals;
            PARTY_CRITS.store(1 + c as u8, Ordering::Relaxed);
            c
        }
        v => v == 2,
    }
}

/// The Config menu's row: live, kept in the ini.
pub fn set_party_criticals(on: bool) {
    PARTY_CRITS.store(1 + on as u8, Ordering::Relaxed);
    super::settings::save("party_criticals", if on { "on" } else { "off" });
    log!(Combat, "Config menu: Party Criticals {}", if on { "On" } else { "Off" });
}

/// An attack's owner is one of Sora's party (Donald, Goofy, a guest): an actor on Sora's side
/// (`+0x374` bits 5..6 equal to his) that isn't Sora. Assumption: every actor on that side but
/// Sora is a party member (summons' own attacks would count too, if their owner is on it).
unsafe fn party_member(owner: usize, sora: usize) -> bool {
    let side = |a: usize| (rd::<u32>(a + ACTOR_FLAGS) >> 5) & 3;
    owner > 0x10000 && sora > 0x10000 && owner != sora && side(owner) == side(sora)
}

/// Whether KH1's critical roll runs for an attack of `owner`'s.
unsafe fn crits_for(owner: usize, sora: usize) -> bool {
    if party_member(owner, sora) {
        party_criticals()
    } else {
        criticals()
    }
}

/// Hooks KH1's damage function (Steam only; the hook checks its bytes first).
pub unsafe fn install() {
    if Build::current() != Build::Steam {
        return;
    }
    let base = GetModuleHandleA(std::ptr::null()) as usize;
    BASE.store(base, Ordering::Relaxed);
    if super::hook::install(base + DAMAGE_STEAM, &DAMAGE_PROLOGUE, damage as *const () as usize, &ORIG, "KH1's damage (KH2's power, criticals)").is_some() {
        ON.store(true, Ordering::Relaxed);
    }
}

/// KH2's power (percent) and Magic source for an attack of Sora's, if it is a KH2 hit.
unsafe fn kh2_power(attack: usize, sora: usize) -> Option<(u32, bool, &'static str)> {
    if let Some((p, m)) = super::combat::landing() {
        return Some((p, m, "landed"));
    }
    if let Some(a) = super::kh1rows::kh2_atkp(attack) {
        return Some((a.power.max(1) as u32, a.stat_source == 1, "row of ours"));
    }
    let data = super::pose::ptr(rd::<u32>(attack + ATTACK_DATA));
    if data < 0x10000 || rd::<u32>(data + DATA_BONE) != WEAPON_BONE {
        return None;
    }
    let a = super::kh1rows::native_atkp(sora)?;
    Some((a.power.max(1) as u32, a.stat_source == 1, "native swing"))
}

/// The level (1..3) of the KH1 spell whose attack data `data` is (KH1's own entry, or a copy of
/// ours with its stat source, element and power: `bodyhits`' spell objects), if it is one.
unsafe fn kh1_spell_level(base: usize, data: usize) -> Option<u8> {
    let first = base + BLOCK_STEAM + KH1_SPELLS;
    (0..9).find_map(|i| {
        let k = first + i * DATA_SIZE;
        let same = data == k
            || (rd::<u32>(data + DATA_STAT) == rd::<u32>(k + DATA_STAT)
                && rd::<u32>(data + DATA_ELEMENT) & 0xF == rd::<u32>(k + DATA_ELEMENT) & 0xF
                && rd::<i32>(data + DATA_POWER) == rd::<i32>(k + DATA_POWER));
        same.then_some((i % 3) as u8 + 1)
    })
}

/// KH2's normal cast at a spell level (KH2 Blizzard's sub 0 / 2 / 4 from the user's KH2 data;
/// `KH2_NORMAL_CAST` without it).
fn normal_cast(level: u8) -> u32 {
    let i = (level.clamp(1, 3) - 1) as usize;
    super::pose::kh2()
        .and_then(|k| k.battle.attack_for_level(KH2_BLIZZARD_ATKP, i as u8 * 2, 0).map(|a| a.power as u32))
        .filter(|&p| p > 0)
        .unwrap_or(KH2_NORMAL_CAST[i])
}

type DamageF = unsafe extern "system" fn(usize, usize, usize) -> u32;

unsafe extern "system" fn damage(attack: usize, target: usize, kind: usize) -> u32 {
    let f: DamageF = std::mem::transmute(ORIG.load(Ordering::Acquire));
    let sora = super::takeover::sora_actor();
    let owner = if attack > 0x10000 { super::pose::ptr(rd::<u32>(attack + ATTACK_OWNER)) } else { 0 };
    let mine = sora > 0x10000 && owner == sora;
    // Criticals off: no chance for the call, whoever owns the attack (the tester, 2 (b): Sora
    // and enemies alike; his party by its own row; KH1 rolls for every attack here, Checked
    // (decomp)).
    let crit = (attack > 0x10000 && !crits_for(owner, sora)).then(|| {
        let c = rd::<f32>(attack + ATTACK_CRIT_CHANCE);
        wr(attack + ATTACK_CRIT_CHANCE, 0.0f32);
        c
    });
    if !mine {
        let d = f(attack, target, kind);
        if let Some(c) = crit {
            wr(attack + ATTACK_CRIT_CHANCE, c);
        }
        return d;
    }
    let kh2 = if active() { kh2_power(attack, sora) } else { None };
    let base = BASE.load(Ordering::Relaxed);
    let at = base + CURVE_PTR_STEAM;
    let kh1_curve = rd::<usize>(at);
    let mut swapped = false;
    // A KH2 spell hit on KH1's own spell data (t920): its KH1 level and KH2's normal cast there.
    let mut spell = None;
    if let Some((power, magic, from)) = kh2 {
        if kh1_curve > 0x10000 {
            if magic && from == "native swing" {
                // (A swing on KH1's own row: its data is KH1's own, shared with vanilla swings, so
                // its stat stays the row's Strength.)
                static NATIVE_MAGIC: AtomicBool = AtomicBool::new(false);
                if !NATIVE_MAGIC.swap(true, Ordering::Relaxed) {
                    log!(Combat, "KH2 damage: a Magic-based KH2 window on KH1's own swing row: Strength (KH1's row data), not KH1's magic stat");
                }
            }
            let set = super::settings::get();
            // (A Magic-based hit's stat is KH1's own magic one, its data's max MP source, t917:
            // `kh1rows::kh1_magic`; nothing on top here.)
            // A KH2 spell on KH1's own spell data (t920, the owner: KH1's own magic formula, KH2
            // only the proportions): KH1's spell hit (its power points + max MP) is KH2's normal
            // cast at that level, so KH2's power counts against that cast (Blizzaga 375 / 375 = 1,
            // its finisher 550 / 375, Firaga's 50% ticks 50 / 375), not against 100.
            let data = super::pose::ptr(rd::<u32>(attack + ATTACK_DATA));
            spell = if magic && data > 0x10000 { kh1_spell_level(base, data).map(|l| (l, normal_cast(l))) } else { None };
            let per = spell.map_or(100.0, |(_, cast)| cast as f64);
            let factor = set.damage_scale as f64 * power as f64 / per;
            let n = (rd::<u16>(kh1_curve) as usize).min(CURVE_MAX);
            let curve = std::ptr::addr_of_mut!(CURVE) as *mut u16;
            *curve = n as u16;
            for i in 0..n {
                let v = rd::<u16>(kh1_curve + 2 + i * 2) as f64;
                // (KH2 rounds up; a tiny margin so 100% gives KH1's value exactly.)
                *curve.add(1 + i) = (v * factor - 1e-6).ceil().clamp(0.0, u16::MAX as f64) as u16;
            }
            wr(at, curve as usize);
            swapped = true;
        }
    }
    // KH1's finisher points off a KH2 hit (t918, the owner: "the bonus on top should not be in the
    // game but work through KH1's systems"): KH1's finisher rows add 4 or 6 power points to the
    // curve's index (data `+0x48`) and KH2's finisher power % then applied on top. For the call,
    // a Strength-based KH2 hit's data has a plain swing's power (`PLAIN_SWING_ROW`'s), so KH2's
    // own power % is the finisher's whole strength on KH1's curve; put back after. Not for KH1's
    // magic stat (Ripple Drive's on Magic-based hits, `kh1rows::kh1_magic`; KH1 spells' data) nor
    // the elements whose power is the effect itself (8 gravity %, 9 stop, 11 cure, 12 aero).
    let mut row_power = None;
    if swapped {
        let data = super::pose::ptr(rd::<u32>(attack + ATTACK_DATA));
        if data > 0x10000 && rd::<u32>(data + DATA_STAT) == 0 && !matches!(rd::<u32>(data + DATA_ELEMENT) & 0xF, 8 | 9 | 11 | 12) {
            if let Some((plain, _)) = super::kh1rows::kh1_row(PLAIN_SWING_ROW) {
                let (was, swing) = (rd::<i32>(data + DATA_POWER), rd::<i32>(plain + DATA_POWER));
                if was != swing {
                    wr(data + DATA_POWER, swing);
                    row_power = Some((data, was, swing));
                }
            }
        }
    }
    let d = f(attack, target, kind);
    if let Some((data, was, _)) = row_power {
        wr(data + DATA_POWER, was);
    }
    if swapped {
        wr(at, kh1_curve);
    }
    if let Some(c) = crit {
        wr(attack + ATTACK_CRIT_CHANCE, c);
    }
    if let Some((power, magic, from)) = kh2 {
        if LOGGED.fetch_add(1, Ordering::Relaxed) < 40 {
            let crit_hit = kind > 0x10000 && rd::<u32>(kind) & 0x20 != 0;
            log!(Combat,
                "KH2 damage: {from} power {power}%{}{} on actor {target:#x}: {d}{}{}{}",
                if magic { " (Magic)" } else { "" },
                spell.map_or(String::new(), |(l, cast)| format!(" (KH1 spell level {l}: of {cast}%)")),
                row_power.map_or(String::new(), |(_, was, swing)| format!(" (KH1 row power {was} -> {swing})")),
                if crit_hit { " (critical)" } else { "" },
                if swapped { "" } else { " (KH1's curve not found: KH1's damage)" }
            );
        }
    }
    d
}
