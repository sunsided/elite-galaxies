//! Dumps every system of all eight galaxies as TSV (for diffing against txtelite).

fn main() {
    for g in 0..elite::GALAXIES {
        for s in elite::galaxy(g) {
            println!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                g + 1,
                s.index,
                s.name,
                s.x,
                s.y,
                s.economy,
                s.government,
                s.displayed_tech_level(),
                s.population,
                s.productivity,
                s.radius,
                s.species,
                s.description()
            );
        }
    }
}
