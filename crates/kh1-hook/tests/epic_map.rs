//! (tm41) The Epic address table (`src/win/epic_map.in`) covers every `St(0x...)` in the hook's
//! source, sorted by the Steam RVA.

static EPIC: &[(u32, u32)] = &include!("../src/win/epic_map.in");

#[test]
fn sorted() {
    assert!(EPIC.windows(2).all(|w| w[0].0 < w[1].0));
}

#[test]
fn every_st_mapped() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/win");
    let mut missing = Vec::new();
    let mut seen = 0;
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "rs") {
            let text = std::fs::read_to_string(&p).unwrap();
            for (i, _) in text.match_indices("St(0x") {
                let hex: String = text[i + 5..].chars().take_while(|c| c.is_ascii_hexdigit() || *c == '_').filter(|c| *c != '_').collect();
                let v = u32::from_str_radix(&hex, 16).unwrap();
                seen += 1;
                if EPIC.binary_search_by_key(&v, |p| p.0).is_err() {
                    missing.push(format!("{}: {v:#x}", p.display()));
                }
            }
        }
    }
    assert!(seen > 400, "only {seen} addresses found");
    assert!(missing.is_empty(), "no Epic address for {missing:?}");
}
