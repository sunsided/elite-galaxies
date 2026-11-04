//! A faithful reconstruction of the procedural universe of *Elite* (1984,
//! David Braben & Ian Bell): eight galaxies of 256 star systems each, all
//! unfolded from a single 48-bit seed.
//!
//! A [`Seed`] holds three 16-bit words. Each [`StarSystem`] is read from the
//! bits of the current seed, which is then twisted four times to reach the
//! next system; rotating every seed byte left by one bit reaches the next
//! galaxy, and eight rotations lead back to the first.
//!
//! ```
//! let galaxy = elite::galaxy(0);
//! let lave = &galaxy[7];
//! assert_eq!(lave.name, "Lave");
//! assert_eq!(lave.economy_name(), "Rich Agricultural");
//! assert_eq!(
//!     lave.description(),
//!     "Lave is most famous for its vast rain forests and the Lavian tree grub."
//! );
//! ```
//!
//! Where Ian Bell's `txtelite.c` and the BBC Micro's 6502 code disagree
//! (the size adjective of alien species, the capital letter of some made-up
//! words), this crate follows the 6502.
//!
//! References: the BBC Micro source as annotated by Mark Moxon
//! (<https://elite.bbcelite.com>) and Ian Bell's `txtelite.c`.

mod goat_soup;
pub mod seed;
pub mod system;

pub use seed::Seed;
pub use system::StarSystem;

/// Systems per galaxy.
pub const SYSTEMS_PER_GALAXY: usize = 256;

/// The galaxies reachable with the galactic hyperdrive.
pub const GALAXIES: usize = 8;

/// All 256 systems of the galaxy whose first system has seed `seed`.
///
/// Any 48-bit seed produces a valid galaxy — the game simply only ever
/// visits the eight seeds reachable from [`Seed::GALAXY_ONE`].
///
/// ```
/// use elite::{Seed, galaxy_from_seed};
///
/// let systems = galaxy_from_seed(Seed::new(0x1234, 0xABCD, 0x9F01));
/// assert_eq!(systems.len(), elite::SYSTEMS_PER_GALAXY);
/// assert_eq!(galaxy_from_seed(Seed::GALAXY_ONE), elite::galaxy(0));
/// ```
pub fn galaxy_from_seed(mut seed: Seed) -> Vec<StarSystem> {
    (0..SYSTEMS_PER_GALAXY)
        .map(|i| StarSystem::generate(i as u8, &mut seed))
        .collect()
}

/// All 256 systems of galaxy `n` (0-based; wraps modulo 8 like the game).
///
/// ```
/// assert_eq!(elite::galaxy(0)[0].name, "Tibedied");
/// assert_eq!(elite::galaxy(8), elite::galaxy(0));
/// ```
pub fn galaxy(n: usize) -> Vec<StarSystem> {
    galaxy_from_seed(Seed::galaxy(n))
}

/// Tests against known game data across whole galaxies.
#[cfg(test)]
mod tests {
    use super::*;

    /// Finds a system by exact name, panicking with the name if it is missing.
    fn find<'a>(g: &'a [StarSystem], name: &str) -> &'a StarSystem {
        g.iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("{name} not found"))
    }

    /// Galaxy 1 opens with Tibedied, Qube, Leleer, …, Lave, as in the game.
    #[test]
    fn first_systems_of_galaxy_one() {
        let g = galaxy(0);
        let names: Vec<_> = g.iter().take(8).map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Tibedied", "Qube", "Leleer", "Biarge", "Xequerin", "Tiraor", "Rabedira", "Lave"
            ]
        );
    }

    /// Lave's whole data sheet and description match the game.
    #[test]
    fn lave_matches_the_game() {
        let g = galaxy(0);
        let lave = find(&g, "Lave");
        assert_eq!(lave.index, 7);
        assert_eq!((lave.x, lave.y), (20, 173));
        assert_eq!(lave.economy_name(), "Rich Agricultural");
        assert_eq!(lave.government_name(), "Dictatorship");
        assert_eq!(lave.displayed_tech_level(), 5);
        assert_eq!(lave.population, 25);
        assert_eq!(lave.productivity, 7000);
        assert_eq!(lave.radius, 4116);
        assert_eq!(lave.species, "Human Colonials");
        assert_eq!(
            lave.description(),
            "Lave is most famous for its vast rain forests and the Lavian tree grub."
        );
    }

    /// The famous systems around Lave are all in galaxy 1.
    #[test]
    fn lave_neighbours_exist() {
        let g = galaxy(0);
        for name in [
            "Leesti", "Diso", "Riedquat", "Zaonce", "Reorte", "Orerve", "Uszaa",
        ] {
            find(&g, name);
        }
    }

    /// Descriptions of Zaonce, Leesti and Diso match the game, and Diso's species.
    #[test]
    fn known_descriptions() {
        let g = galaxy(0);
        for (name, text) in [
            ("Zaonce", "This planet is a tedious place."),
            (
                "Leesti",
                "The planet Leesti is reasonably fabled for Zero-G cricket and Leestian evil juice.",
            ),
            (
                "Diso",
                "This planet is mildly noted for its ancient Ma corn plantations but beset by frequent solar activity.",
            ),
        ] {
            assert_eq!(find(&g, name).description(), text);
        }
        assert_eq!(find(&g, "Diso").species, "Black Furry Felines");
    }

    /// Species use the 6502's AND 7 size mask where txtelite.c uses AND 3.
    #[test]
    fn species_follow_the_6502_not_txtelite() {
        // txtelite.c masks the size adjective with AND 3 and adds
        // "Fierce"/"Small" to these; the 6502's AND 7 prints none.
        let g = galaxy(0);
        assert_eq!(find(&g, "Tiraor").species, "Blue Fat Insects");
        assert_eq!(find(&g, "Oresri").species, "Rodents");
        assert_eq!(find(&g, "Eredve").species, "Insects");
    }

    /// A made-up word starting with the blank syllable still gets its capital letter.
    #[test]
    fn random_word_capitalised_after_blank_syllable() {
        // MT18 sets Sentence Case, so the first *printed* letter is upper
        // case even when the word's first syllable is the blank "..".
        let g = galaxy(5);
        assert_eq!(
            find(&g, "Latibe").description(),
            "This planet is mildly noted for its ancient Latibian Atnu banana plantations but beset by deadly earthquakes."
        );
    }

    /// Every system in every galaxy has a well-formed name and in-range stats.
    #[test]
    fn every_galaxy_has_256_well_formed_systems() {
        for n in 0..GALAXIES {
            let g = galaxy(n);
            assert_eq!(g.len(), SYSTEMS_PER_GALAXY);
            for (i, s) in g.iter().enumerate() {
                assert_eq!(s.index as usize, i);
                assert!((2..=8).contains(&s.name.len()), "{:?}", s.name);
                let mut chars = s.name.chars();
                assert!(chars.next().unwrap().is_ascii_uppercase(), "{:?}", s.name);
                assert!(chars.all(|c| c.is_ascii_lowercase()), "{:?}", s.name);
                assert!(s.tech_level <= 14);
                assert!(s.economy < 8 && s.government < 8);
                assert!((2816..=6911).contains(&s.radius));
            }
        }
    }

    /// Jumping eight times returns to galaxy 1.
    #[test]
    fn galaxy_nine_is_galaxy_one() {
        assert_eq!(galaxy(8), galaxy(0));
    }
}
