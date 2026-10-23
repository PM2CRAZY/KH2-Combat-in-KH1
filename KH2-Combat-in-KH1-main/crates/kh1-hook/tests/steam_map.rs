//! The Steam address table (`src/win/steam_map.in`) covers every `Rva(0x...)` in the hook's
//! source, in order.

static STEAM: &[(u32, u32)] = &include!("../src/win/steam_map.in");

#[test]
fn sorted() {
    assert!(STEAM.windows(2).all(|w| w[0].0 < w[1].0));
}

#[test]
fn every_rva_mapped() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/win");
    let mut missing = Vec::new();
    let mut seen = 0;
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "rs") {
            let text = std::fs::read_to_string(&p).unwrap();
            for (i, _) in text.match_indices("Rva(0x") {
                let hex: String = text[i + 6..].chars().take_while(|c| c.is_ascii_hexdigit() || *c == '_').filter(|c| *c != '_').collect();
                let v = u32::from_str_radix(&hex, 16).unwrap();
                seen += 1;
                if STEAM.binary_search_by_key(&v, |p| p.0).is_err() {
                    missing.push(format!("{}: {v:#x}", p.display()));
                }
            }
        }
    }
    assert!(seen > 150, "only {seen} addresses found");
    assert!(missing.is_empty(), "no Steam address for {missing:?}");
}
