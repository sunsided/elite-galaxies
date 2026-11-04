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
    fn galaxy_nine_is_galaxy_one() {
        assert_eq!(galaxy(8), galaxy(0));
    }
}
