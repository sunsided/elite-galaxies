//! Turning a seed into a star system: name, position, economy, government,
//! tech level, population, productivity, radius and inhabitants.
//!
//! Bit layout follows the BBC Micro version (and Ian Bell's `txtelite.c`).

use std::fmt;

use crate::goat_soup;
use crate::seed::Seed;

/// Two-letter syllables, as in Ian Bell's `txtelite.c`. Names use the 64
/// bytes from offset 24 on (`QQ16` in the 6502 source), indexed by
/// `2 * (s2_hi & 31)`; the description generator's random words use the
/// first 64 bytes. A `.` means "print nothing", which is how names with an
/// odd number of letters arise ("Lave" has pairs "LA", "VE" and "..").
pub(crate) const PAIRS: &[u8; 88] =
    b"ABOUSEITILETSTONLONUTHNO..LEXEGEZACEBISOUSESARMAINDIREA.ERATENBERALAVETIEDORQUANTEISRION";
const NAME_PAIRS: &[u8] = PAIRS.split_at(24).1;

pub const ECONOMIES: [&str; 8] = [
    "Rich Industrial",
    "Average Industrial",
    "Poor Industrial",
    "Mainly Industrial",
    "Mainly Agricultural",
    "Rich Agricultural",
    "Average Agricultural",
    "Poor Agricultural",
];

pub const GOVERNMENTS: [&str; 8] = [
    "Anarchy",
    "Feudal",
    "Multi-Government",
    "Dictatorship",
    "Communist",
    "Confederacy",
    "Democracy",
    "Corporate State",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarSystem {
    /// Index of the system within its galaxy (0..=255).
    pub index: u8,
    pub name: String,
    /// Galactic chart X coordinate (0..=255).
    pub x: u8,
    /// Galactic chart Y coordinate (0..=255; the game draws the chart at half height).
    pub y: u8,
    /// 0..=7, see [`ECONOMIES`].
    pub economy: u8,
    /// 0..=7, see [`GOVERNMENTS`].
    pub government: u8,
    /// Internal tech level 0..=14; the game displays this plus one.
    pub tech_level: u8,
    /// In units of 100 million (the game shows `population / 10` billion).
    pub population: u8,
    /// In millions of credits.
    pub productivity: u16,
    /// Average radius in km.
    pub radius: u16,
    pub species: String,
    /// The four seed bytes used to seed the "goat soup" description generator.
    pub(crate) soup_seed: [u8; 4],
}

impl StarSystem {
    /// Builds the system described by `seed`.
    ///
    /// This only *reads* the seed; use [`StarSystem::generate`] to also
    /// advance the seed to the next system, as the game does.
    pub fn from_seed(index: u8, seed: Seed) -> Self {
        let government = (seed.s1_lo() >> 3) & 7;

        let mut economy = seed.s0_hi() & 7;
        if government <= 1 {
            // Anarchies and feudal worlds are never rich.
            economy |= 2;
        }

        // 6502: (s1_hi AND 3) + (economy EOR 7) + (government LSR 1), with the
        // carry left over from the LSR fed into the ADC.
        let tech_level = (seed.s1_hi() & 3) + (economy ^ 7) + (government >> 1) + (government & 1);

        let population = 4 * tech_level + economy + government + 1;

        let productivity =
            ((economy ^ 7) as u16 + 3) * (government as u16 + 4) * population as u16 * 8;

        let radius = 256 * ((seed.s2_hi() & 15) as u16 + 11) + seed.s1_hi() as u16;

        Self {
            index,
            name: name_from_seed(seed),
            x: seed.s1_hi(),
            y: seed.s0_hi(),
            economy,
            government,
            tech_level,
            population,
            productivity,
            radius,
            species: species(seed),
            soup_seed: [seed.s1_lo(), seed.s1_hi(), seed.s2_lo(), seed.s2_hi()],
        }
    }

    /// Builds the system for the current seed and advances the seed to the next
    /// system (four twists, always — whether or not the name used all four pairs).
    pub fn generate(index: u8, seed: &mut Seed) -> Self {
        let system = Self::from_seed(index, *seed);
        for _ in 0..4 {
            seed.twist();
        }
        system
    }

    pub fn economy_name(&self) -> &'static str {
        ECONOMIES[self.economy as usize]
    }

    pub fn government_name(&self) -> &'static str {
        GOVERNMENTS[self.government as usize]
    }

    /// The tech level as shown in the game (1-based).
    pub fn displayed_tech_level(&self) -> u8 {
        self.tech_level + 1
    }

    /// The planet description, e.g. "Lave is most famous for its vast rain forests
    /// and the Lavian tree grub."
    pub fn description(&self) -> String {
        goat_soup::describe(self)
    }

    /// Light-year distance as the game computes it (integer maths, Y halved).
    pub fn distance_to(&self, other: &StarSystem) -> f64 {
        let dx = self.x.abs_diff(other.x) as u32;
        let dy = (self.y.abs_diff(other.y) / 2) as u32;
        // The game: 4 * isqrt(dx² + dy²), shown with one decimal place.
        (4 * (dx * dx + dy * dy).isqrt()) as f64 / 10.0
    }
}

/// Reads two letters per twist from bits 0-4 of `s2_hi`, three or four times.
fn name_from_seed(mut seed: Seed) -> String {
    let long_name = seed.s0_lo() & 0x40 != 0;
    let pair_count = if long_name { 4 } else { 3 };

    let mut name = String::with_capacity(8);
    for i in 0..4 {
        let idx = 2 * (seed.s2_hi() & 31) as usize;
        if i < pair_count {
            name.extend(
                NAME_PAIRS[idx..idx + 2]
                    .iter()
                    .filter(|&&c| c != b'.')
                    .map(|&c| c as char),
            );
        }
        seed.twist();
    }

    capitalize(&name)
}

fn capitalize(upper: &str) -> String {
    let mut chars = upper.chars();
    match chars.next() {
        Some(first) => {
            first.to_ascii_uppercase().to_string() + &chars.as_str().to_ascii_lowercase()
        }
        None => String::new(),
    }
}

/// Species as generated by the BBC Micro version (txtelite lacks this).
fn species(seed: Seed) -> String {
    if seed.s2_lo() & 0x80 == 0 {
        return "Human Colonials".to_owned();
    }

    const SIZE: [&str; 3] = ["Large", "Fierce", "Small"];
    const COLOUR: [&str; 6] = ["Green", "Red", "Yellow", "Blue", "Black", "Harmless"];
    const LOOK: [&str; 6] = ["Slimy", "Bug-Eyed", "Horned", "Bony", "Fat", "Furry"];
    const KIND: [&str; 8] = [
        "Rodents",
        "Frogs",
        "Lizards",
        "Lobsters",
        "Birds",
        "Humanoids",
        "Felines",
        "Insects",
    ];

    let mut words: Vec<&str> = Vec::with_capacity(4);

    let a = (seed.s2_hi() >> 2) & 7;
    if let Some(w) = SIZE.get(a as usize) {
        words.push(w);
    }

    let b = seed.s2_hi() >> 5;
    if let Some(w) = COLOUR.get(b as usize) {
        words.push(w);
    }

    let c = (seed.s0_hi() ^ seed.s1_hi()) & 7;
    if let Some(w) = LOOK.get(c as usize) {
        words.push(w);
    }

    let d = ((seed.s2_hi() & 3) + c) & 7;
    words.push(KIND[d as usize]);

    words.join(" ")
}

impl fmt::Display for StarSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "System:       {} (#{})", self.name, self.index)?;
        writeln!(f, "Position:     ({}, {})", self.x, self.y)?;
        writeln!(f, "Economy:      {}", self.economy_name())?;
        writeln!(f, "Government:   {}", self.government_name())?;
        writeln!(f, "Tech Level:   {}", self.displayed_tech_level())?;
        writeln!(
            f,
            "Population:   {}.{} Billion ({})",
            self.population / 10,
            self.population % 10,
            self.species
        )?;
        writeln!(f, "Productivity: {} M CR", self.productivity)?;
        writeln!(f, "Radius:       {} km", self.radius)?;
        write!(f, "\n{}", self.description())
    }
}
