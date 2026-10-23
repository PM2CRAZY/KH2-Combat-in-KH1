/// Damage element. The order matches the weakness columns used by KH2's
/// enemy parameter table, so a `[u16; 7]` resistance array can be indexed
/// directly with [`Element::index`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Element {
    #[default]
    Physical,
    Fire,
    Blizzard,
    Thunder,
    Dark,
    Light,
    /// "General" in KH2's tables: an element-less damage source.
    Neutral,
}

impl Element {
    pub const ALL: [Element; 7] = [
        Element::Physical,
        Element::Fire,
        Element::Blizzard,
        Element::Thunder,
        Element::Dark,
        Element::Light,
        Element::Neutral,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    /// Maps a raw element id. Unknown ids fall back to `Neutral`.
    pub fn from_id(id: u8) -> Element {
        Element::ALL.get(id as usize).copied().unwrap_or(Element::Neutral)
    }

    /// True for elements whose damage scales from the attacker's Magic stat
    /// rather than Strength (hypothesis: matches KH2 where spells use Magic).
    pub fn is_magical(self) -> bool {
        matches!(self, Element::Fire | Element::Blizzard | Element::Thunder)
    }
}

/// Percent damage taken per element: 100 = normal, 200 = double, 0 = immune.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resistances(pub [u16; 7]);

impl Default for Resistances {
    fn default() -> Self {
        Resistances([100; 7])
    }
}

impl Resistances {
    pub fn percent(&self, e: Element) -> u16 {
        self.0[e.index()]
    }
}
