use std::process::ExitCode;

use elite::{GALAXIES, Seed, StarSystem, galaxy_from_seed};

const USAGE: &str = "\
Usage:
  elite list [GALAXY]            List all 256 systems of GALAXY (1-8, default 1)
  elite describe [GALAXY]        Print every system's planet description
  elite show NAME [GALAXY]       Show one system's data sheet and description
  elite find NAME                Search all eight galaxies for NAME
  elite seed S0 S1 S2 [NAME]     Use any 48-bit seed (hex words), e.g. 5A4A 0248 B753
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();

    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}\n\n{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[&str]) -> Result<(), String> {
    match args {
        [] | ["list"] => list(&galaxy_from_seed(Seed::galaxy(0))),
        ["list", g] => list(&galaxy_from_seed(Seed::galaxy(parse_galaxy(g)?))),
        ["describe"] => describe(&galaxy_from_seed(Seed::galaxy(0))),
        ["describe", g] => describe(&galaxy_from_seed(Seed::galaxy(parse_galaxy(g)?))),
        ["show", name] => show(&galaxy_from_seed(Seed::galaxy(0)), name),
        ["show", name, g] => show(&galaxy_from_seed(Seed::galaxy(parse_galaxy(g)?)), name),
        ["find", name] => find_everywhere(name),
        ["seed", s0, s1, s2, rest @ ..] => {
            let seed = Seed::new(parse_hex(s0)?, parse_hex(s1)?, parse_hex(s2)?);
            let systems = galaxy_from_seed(seed);
            match rest {
                [] => list(&systems),
                [name] => show(&systems, name),
                _ => Err("too many arguments".into()),
            }
        }
        ["-h" | "--help" | "help"] => {
            print!("{USAGE}");
            Ok(())
        }
        _ => Err("unrecognized arguments".into()),
    }
}

fn parse_galaxy(s: &str) -> Result<usize, String> {
    match s.parse::<usize>() {
        // 1-based like the game; values past 8 wrap, demonstrating the cycle.
        Ok(n) if n >= 1 => Ok(n - 1),
        _ => Err(format!("invalid galaxy number {s:?}")),
    }
}

fn parse_hex(s: &str) -> Result<u16, String> {
    u16::from_str_radix(s.trim_start_matches("0x"), 16)
        .map_err(|e| format!("bad seed word {s:?}: {e}"))
}

fn list(systems: &[StarSystem]) -> Result<(), String> {
    println!(
        "{:>3}  {:<9} {:>3} {:>3}  {:<20} {:<17} {:>2}  {:>4}",
        "#", "Name", "X", "Y", "Economy", "Government", "TL", "Pop"
    );
    for s in systems {
        println!(
            "{:>3}  {:<9} {:>3} {:>3}  {:<20} {:<17} {:>2}  {:>2}.{}",
            s.index,
            s.name,
            s.x,
            s.y,
            s.economy_name(),
            s.government_name(),
            s.displayed_tech_level(),
            s.population / 10,
            s.population % 10,
        );
    }
    Ok(())
}

fn describe(systems: &[StarSystem]) -> Result<(), String> {
    for s in systems {
        println!("{:<9} {}", s.name, s.description());
    }
    Ok(())
}

fn show(systems: &[StarSystem], name: &str) -> Result<(), String> {
    let matches: Vec<_> = systems
        .iter()
        .filter(|s| s.name.eq_ignore_ascii_case(name))
        .collect();
    if matches.is_empty() {
        return Err(format!("no system named {name:?} in this galaxy"));
    }
    for s in matches {
        println!("{s}\n");
    }
    Ok(())
}

fn find_everywhere(name: &str) -> Result<(), String> {
    let mut found = false;
    for g in 0..GALAXIES {
        for s in galaxy_from_seed(Seed::galaxy(g)) {
            if s.name.eq_ignore_ascii_case(name) {
                println!("Galaxy {}:\n{s}\n", g + 1);
                found = true;
            }
        }
    }
    if found {
        Ok(())
    } else {
        Err(format!("{name:?} not found in any galaxy"))
    }
}
