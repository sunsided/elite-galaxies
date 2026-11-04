//! Command-line front end to the generator: list galaxies, print
//! descriptions, show a system's data sheet, search all eight galaxies, or
//! explore an arbitrary 48-bit seed. Run `elite help` for the commands.

use std::process::ExitCode;

use elite::{GALAXIES, Seed, StarSystem, galaxy_from_seed};

/// Help text shown for `help` and after errors.
const USAGE: &str = "\
Usage:
  elite list [GALAXY]            List all 256 systems of GALAXY (1-8, default 1)
  elite describe [GALAXY]        Print every system's planet description
  elite show NAME [GALAXY]       Show one system's data sheet and description
  elite find NAME                Search all eight galaxies for NAME
  elite seed S0 S1 S2 [NAME]     Use any 48-bit seed (hex words), e.g. 5A4A 0248 B753
";

/// Runs the command given on the command line and maps errors to a non-zero exit code.
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

/// Dispatches the command line; returns the error message to print on failure.
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

/// Parses a 1-based galaxy number into a 0-based one; values above 8 wrap around.
fn parse_galaxy(s: &str) -> Result<usize, String> {
    match s.parse::<usize>() {
        // 1-based like the game; values past 8 wrap, demonstrating the cycle.
        Ok(n) if n >= 1 => Ok(n - 1),
        _ => Err(format!("invalid galaxy number {s:?}")),
    }
}

/// Parses one 16-bit seed word in hex, with or without a `0x` prefix.
fn parse_hex(s: &str) -> Result<u16, String> {
    u16::from_str_radix(s.trim_start_matches("0x"), 16)
        .map_err(|e| format!("bad seed word {s:?}: {e}"))
}

/// Prints all systems as a table, one per line.
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

/// Prints every system's name and description.
fn describe(systems: &[StarSystem]) -> Result<(), String> {
    for s in systems {
        println!("{:<9} {}", s.name, s.description());
    }
    Ok(())
}

/// Prints the data sheet of every system named `name` (case-insensitive).
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

/// Prints every system named `name` across all eight galaxies.
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

/// Tests for argument parsing and command dispatch.
#[cfg(test)]
mod tests {
    use super::*;

    /// Galaxy numbers are 1-based on the command line and wrap past 8.
    #[test]
    fn parse_galaxy_is_one_based() {
        assert_eq!(parse_galaxy("1"), Ok(0));
        assert_eq!(parse_galaxy("8"), Ok(7));
        assert_eq!(parse_galaxy("9"), Ok(8));
        assert!(parse_galaxy("0").is_err());
        assert!(parse_galaxy("x").is_err());
    }

    /// Seed words parse as hex, with or without `0x`.
    #[test]
    fn parse_hex_accepts_optional_prefix() {
        assert_eq!(parse_hex("5A4A"), Ok(0x5A4A));
        assert_eq!(parse_hex("0x0248"), Ok(0x0248));
        assert!(parse_hex("10000").is_err());
        assert!(parse_hex("xyz").is_err());
    }

    /// Unknown commands and unknown systems come back as errors, not panics.
    #[test]
    fn run_reports_errors() {
        assert!(run(&["frobnicate"]).is_err());
        assert!(run(&["show", "Nowhere"]).is_err());
        assert!(run(&["find", "Nowhere"]).is_err());
        assert!(run(&["list", "0"]).is_err());
        assert!(run(&["seed", "1", "2", "3", "a", "b"]).is_err());
    }

    /// Valid commands succeed, including names in any case and arbitrary seeds.
    #[test]
    fn run_accepts_valid_commands() {
        assert!(run(&["show", "lave"]).is_ok());
        assert!(run(&["show", "Riedquat", "1"]).is_ok());
        assert!(run(&["find", "Diso"]).is_ok());
        assert!(run(&["describe", "2"]).is_ok());
        assert!(run(&["seed", "1234", "ABCD", "9F01"]).is_ok());
        assert!(run(&["help"]).is_ok());
    }
}
