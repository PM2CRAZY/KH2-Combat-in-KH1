//! KH2's Drive orbs added to KH1's prize file, against the player's own files extracted into a
//! local folder (never committed): `KH2_LOCAL_DATA` (holding `kh1/prize.dpx` and
//! `kh2/obj/PRIZE.mdlx`, as OpenKH extracts them), else `local/re` at the repository root.
//! Skips when the files are absent.

use kh2_formats::kh1dpx;

fn file(name: &str) -> Option<Vec<u8>> {
    let dir = std::env::var_os("KH2_LOCAL_DATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../local/re"));
    std::fs::read(dir.join(name)).ok()
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

#[test]
fn drive_orbs_join_kh1s_prizes() {
    let (Some(kh1), Some(mdlx)) = (file("kh1/prize.dpx"), file("kh2/obj/PRIZE.mdlx")) else { return };
    // (The BAR's one entry, `priz`: its offset at +0x18.)
    let pax_at = u32_at(&mdlx, 0x18) as usize;
    let pax = &mdlx[pax_at..];
    let r = kh1dpx::prize_with_kh2(&kh1, pax, &[7, 8]).unwrap();
    println!("{}", r.report);
    // KH1's header kept; its DPD where it was.
    assert_eq!(r.bytes[..0xA0], kh1[..0xA0]);
    // KH1's 3 HD keys move; its first stays its TBP's first.
    assert_eq!(r.kh1_keys.len(), 4);
    assert_eq!(r.kh1_keys[0].0, 0x320);
    // One KH2 texture (TBP 0x3000 / CBP 0x3E00), keyed by its own pixels or its TBP's first.
    assert_eq!(r.kh2_keys.len(), 1);
    assert_eq!(r.kinds.iter().map(|k| (k.0, k.1)).collect::<Vec<_>>(), vec![(7, 4), (8, 4)]);
    assert_eq!(r.kinds.iter().map(|k| k.3).collect::<Vec<_>>(), vec![0.9, 1.7]);
    // Building it again on the result is refused (KH1 already draws them).
    assert!(kh1dpx::prize_with_kh2(&r.bytes, pax, &[7]).is_err());
}
