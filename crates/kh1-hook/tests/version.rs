//! The version the mod shows (`VERSION` in `lib.rs`) is the latest stage in `AGENTS.md`, so a
//! build handed out says which one it is. The testing branch writes it `t<stage>`.

#[test]
fn version_is_the_latest_stage() {
    let lib = include_str!("../src/lib.rs");
    let at = lib.find("pub const VERSION: &str = \"").expect("VERSION in lib.rs") + 27;
    let version = &lib[at..at + lib[at..].find('"').unwrap()];
    let agents = include_str!("../../../AGENTS.md");
    // (The testing-merge branch writes its versions `tm<n>` (`docs/testing-merge.md`): the latest of those.)
    let merge = agents
        .lines()
        .filter_map(|l| l.strip_prefix("### ").and_then(|l| l.split("Stage tm").nth(1)))
        .filter_map(|l| l.split(|c: char| !c.is_ascii_digit()).next()?.parse::<u32>().ok())
        .max();
    if let Some(m) = merge {
        assert_eq!(version, format!("tm{m}"), "bump VERSION in lib.rs with each AGENTS.md stage (`tm` for the testing-merge branch)");
        return;
    }
    let latest = agents
        .lines()
        .filter_map(|l| l.strip_prefix("### ").and_then(|l| l.split("Stage ").nth(1)))
        .filter_map(|l| l.split(|c: char| !c.is_ascii_digit()).next()?.parse::<u32>().ok())
        .max()
        .expect("a stage in AGENTS.md");
    assert_eq!(version, format!("t{latest}"), "bump VERSION in lib.rs with each AGENTS.md stage (`t` for the testing branch)");
}
