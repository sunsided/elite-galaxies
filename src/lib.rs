//! A faithful reconstruction of the procedural universe of *Elite* (1984,
//! David Braben & Ian Bell): eight galaxies of 256 star systems each, all
//! unfolded from a single 48-bit seed.
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
pub fn galaxy_from_seed(mut seed: Seed) -> Vec<StarSystem> {
    (0..SYSTEMS_PER_GALAXY)
        .map(|i| StarSystem::generate(i as u8, &mut seed))
        .collect()
}

/// All 256 systems of galaxy `n` (0-based; wraps modulo 8 like the game).
pub fn galaxy(n: usize) -> Vec<StarSystem> {
    galaxy_from_seed(Seed::galaxy(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find<'a>(g: &'a [StarSystem], name: &str) -> &'a StarSystem {
        g.iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("{name} not found"))
    }

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

    #[test]
    fn lave_neighbours_exist() {
        let g = galaxy(0);
        for name in [
            "Leesti", "Diso", "Riedquat", "Zaonce", "Reorte", "Orerve", "Uszaa",
        ] {
            find(&g, name);
        }
    }

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

    #[test]
    fn species_follow_the_6502_not_txtelite() {
        // txtelite.c masks the size adjective with AND 3 and adds
        // "Fierce"/"Small" to these; the 6502's AND 7 prints none.
        let g = galaxy(0);
        assert_eq!(find(&g, "Tiraor").species, "Blue Fat Insects");
        assert_eq!(find(&g, "Oresri").species, "Rodents");
        assert_eq!(find(&g, "Eredve").species, "Insects");
    }

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

    #[test]
    fn galaxy_nine_is_galaxy_one() {
        assert_eq!(galaxy(8), galaxy(0));
    }
}
