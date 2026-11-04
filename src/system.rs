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

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a seed from its six bytes, named as in the 6502 source.
    fn seed(s0_hi: u8, s0_lo: u8, s1_hi: u8, s1_lo: u8, s2_hi: u8, s2_lo: u8) -> Seed {
        Seed::new(
            u16::from_be_bytes([s0_hi, s0_lo]),
            u16::from_be_bytes([s1_hi, s1_lo]),
            u16::from_be_bytes([s2_hi, s2_lo]),
        )
    }

    /// Government sits in bits 3-5 of `s1_lo`.
    fn gov(g: u8) -> u8 {
        g << 3
    }

    #[test]
    fn position_comes_from_high_bytes() {
        let s = StarSystem::from_seed(0, seed(173, 0, 20, 0, 0, 0));
        assert_eq!((s.x, s.y), (20, 173));
    }

    #[test]
    fn anarchy_and_feudal_worlds_are_never_rich() {
        for g in 0..=1 {
            for e in 0..8 {
                let s = StarSystem::from_seed(0, seed(e, 0, 0, gov(g), 0, 0));
                assert_eq!(s.economy, e | 2, "government {g}, economy bits {e}");
            }
        }
        // Other governments keep the raw economy bits.
        let s = StarSystem::from_seed(0, seed(0, 0, 0, gov(2), 0, 0));
        assert_eq!(s.economy, 0);
    }

    #[test]
    fn economy_ignores_bits_above_two() {
        let s = StarSystem::from_seed(0, seed(0b1111_1101, 0, 0, gov(7), 0, 0));
        assert_eq!(s.economy, 5);
    }

    #[test]
    fn tech_level_includes_carry_from_government_shift() {
        // Dictatorship (3): LSR gives 1 with carry set, so ADC adds 2 in total.
        let s = StarSystem::from_seed(0, seed(0, 0, 0, gov(3), 0, 0));
        // (s1_hi & 3) = 0, economy 0 ^ 7 = 7, 3 >> 1 = 1, carry 1.
        assert_eq!(s.tech_level, 7 + 1 + 1);

        // Communist (4): LSR gives 2, no carry.
        let s = StarSystem::from_seed(0, seed(0, 0, 0, gov(4), 0, 0));
        assert_eq!(s.tech_level, 7 + 2);
    }

    #[test]
    fn tech_level_adds_low_bits_of_s1_hi() {
        let s = StarSystem::from_seed(0, seed(0, 0, 0b1111_1111, gov(7), 0, 0));
        // 3 + (0 ^ 7) + (7 >> 1) + 1
        assert_eq!(s.tech_level, 14);
        assert_eq!(s.displayed_tech_level(), 15);
    }

    #[test]
    fn population_productivity_and_radius_formulas() {
        let s = StarSystem::from_seed(0, seed(0, 0, 0, gov(3), 0, 0));
        // 4 * tech 9 + economy 0 + government 3 + 1
        assert_eq!(s.population, 4 * 9 + 3 + 1);
        assert_eq!(s.productivity, (7 + 3) * (3 + 4) * 40 * 8);
        assert_eq!(s.radius, 256 * 11);

        // Radius uses only the low nibble of s2_hi, plus x.
        let s = StarSystem::from_seed(0, seed(0, 0, 200, 0, 0xFF, 0));
        assert_eq!(s.radius, 256 * (15 + 11) + 200);
    }

    #[test]
    fn humans_when_bit_seven_of_s2_lo_is_clear() {
        let s = StarSystem::from_seed(0, seed(0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F));
        assert_eq!(s.species, "Human Colonials");
    }

    #[test]
    fn alien_with_all_three_adjectives() {
        // s2_hi = 001 001 10: size 1 (Fierce), colour 1 (Red), low bits 2.
        // look = (s0_hi ^ s1_hi) & 7 = 3 (Bony); kind = (2 + 3) & 7 = 5.
        let s = StarSystem::from_seed(0, seed(0, 0, 3, 0, 0b0010_0110, 0x80));
        assert_eq!(s.species, "Fierce Red Bony Humanoids");
    }

    #[test]
    fn alien_adjectives_are_skipped_when_out_of_range() {
        // size 4, colour 6 and look 6 have no words; kind = (0 + 6) & 7 = 6.
        // txtelite.c masks the size with AND 3 and would print "Large" here;
        // the 6502 uses AND 7 and prints nothing.
        let s = StarSystem::from_seed(0, seed(6, 0, 0, 0, 0b1101_0000, 0x80));
        assert_eq!(s.species, "Felines");
    }

    #[test]
    fn species_kind_wraps_within_three_bits() {
        // look 7 (no word) + low bits 3 = 10 -> 2 (Lizards).
        let s = StarSystem::from_seed(0, seed(7, 0, 0, 0, 0b1111_1111, 0x80));
        assert_eq!(s.species, "Lizards");
    }

    #[test]
    fn generate_advances_seed_by_exactly_four_twists() {
        for start in [Seed::GALAXY_ONE, Seed::galaxy(3), Seed::new(1, 2, 3)] {
            let mut advanced = start;
            StarSystem::generate(0, &mut advanced);

            let mut expected = start;
            for _ in 0..4 {
                expected.twist();
            }
            assert_eq!(advanced, expected);
        }
    }

    #[test]
    fn short_and_long_names() {
        // Bit 6 of s0_lo selects a four-pair name. Tibedied's seed has it set
        // (0x4A); clearing it drops the fourth pair "ED".
        let s = Seed::GALAXY_ONE;
        assert_eq!(s.s0_lo() & 0x40, 0x40);
        assert_eq!(name_from_seed(s), "Tibedied");
        assert_eq!(
            name_from_seed(Seed::new(s.s0 & !0x40, s.s1, s.s2)),
            "Tibedi"
        );
    }

    #[test]
    fn dots_in_pairs_print_nothing() {
        // s2_hi & 31 == 0 selects the ".." pair for every syllable.
        let s = Seed::new(0, 0, 0);
        assert_eq!(name_from_seed(s), "");
    }

    #[test]
    fn capitalize_lowercases_the_tail() {
        assert_eq!(capitalize("LAVE"), "Lave");
        assert_eq!(capitalize("X"), "X");
        assert_eq!(capitalize(""), "");
    }

    #[test]
    fn distance_halves_y_and_uses_integer_sqrt() {
        let a = StarSystem::from_seed(0, seed(0, 0, 0, 0, 0, 0));
        let b = StarSystem::from_seed(1, seed(8, 0, 3, 0, 0, 0));
        // dx = 3, dy = 8 / 2 = 4 -> 4 * isqrt(25) / 10 = 2.0 ly
        assert_eq!(a.distance_to(&b), 2.0);
        assert_eq!(b.distance_to(&a), 2.0);
        assert_eq!(a.distance_to(&a), 0.0);

        // isqrt truncates: dx = 1, dy = 1 -> 4 * isqrt(2) = 4 -> 0.4 ly
        let c = StarSystem::from_seed(2, seed(2, 0, 1, 0, 0, 0));
        assert_eq!(a.distance_to(&c), 0.4);
    }

    #[test]
    fn names_tables_match_their_indices() {
        assert_eq!(ECONOMIES[5], "Rich Agricultural");
        assert_eq!(GOVERNMENTS[3], "Dictatorship");
        assert_eq!(&PAIRS[24..26], b"..");
        assert_eq!(&NAME_PAIRS[..4], b"..LE");
    }

    #[test]
    fn display_shows_the_game_data_sheet() {
        let lave = crate::galaxy(0).swap_remove(7);
        let text = lave.to_string();
        for line in [
            "System:       Lave (#7)",
            "Position:     (20, 173)",
            "Economy:      Rich Agricultural",
            "Government:   Dictatorship",
            "Tech Level:   5",
            "Population:   2.5 Billion (Human Colonials)",
            "Productivity: 7000 M CR",
            "Radius:       4116 km",
        ] {
            assert!(text.contains(line), "missing {line:?} in\n{text}");
        }
        assert!(text.ends_with("the Lavian tree grub."));
    }
}
