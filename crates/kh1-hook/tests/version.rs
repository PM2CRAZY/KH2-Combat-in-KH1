//! The version the mod shows (`VERSION` in `lib.rs`) is the latest stage in `AGENTS.md`, so a
//! build handed out says which one it is.

#[test]
fn version_is_the_latest_stage() {
    let lib = include_str!("../src/lib.rs");
    let at = lib.find("pub const VERSION: &str = \"").expect("VERSION in lib.rs") + 27;
    let version = &lib[at..at + lib[at..].find('"').unwrap()];
    let agents = include_str!("../../../AGENTS.md");
    let latest = agents
        .lines()
        .filter_map(|l| l.strip_prefix("### ").and_then(|l| l.split("Stage ").nth(1)))
        .filter_map(|l| l.split(|c: char| !c.is_ascii_digit()).next()?.parse::<u32>().ok())
        .max()
        .expect("a stage in AGENTS.md");
    assert_eq!(version, format!("s{latest}"), "bump VERSION in lib.rs with each AGENTS.md stage");
}
