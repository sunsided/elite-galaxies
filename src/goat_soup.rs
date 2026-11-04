//! The "goat soup" planet description generator.
//!
//! A tiny grammar: byte codes 0x81..=0xA4 pick one of five phrases (which may
//! contain further codes), 0xB0 is the planet name, 0xB1 the adjective
//! ("Lavian") and 0xB2 a random made-up word. Tables follow Ian Bell's
//! `txtelite.c`, which mirrors the BBC Micro's extended token table.

use crate::system::{PAIRS, StarSystem};

/// Phrase table for codes 0x81..=0xA4, five options each.
const DESC: [[&str; 5]; 36] = [
    /* 81 */ ["fabled", "notable", "well known", "famous", "noted"],
    /* 82 */ ["very", "mildly", "most", "reasonably", ""],
    /* 83 */ ["ancient", "\u{95}", "great", "vast", "pink"],
    /* 84 */
    [
        "\u{9E} \u{9D} plantations",
        "mountains",
        "\u{9C}",
        "\u{94} forests",
        "oceans",
    ],
    /* 85 */
    [
        "shyness",
        "silliness",
        "mating traditions",
        "loathing of \u{86}",
        "love for \u{86}",
    ],
    /* 86 */ ["food blenders", "tourists", "poetry", "discos", "\u{8E}"],
    /* 87 */ ["talking tree", "crab", "bat", "lobst", "\u{B2}"],
    /* 88 */ ["beset", "plagued", "ravaged", "cursed", "scourged"],
    /* 89 */
    [
        "\u{96} civil war",
        "\u{9B} \u{98} \u{99}s",
        "a \u{9B} disease",
        "\u{96} earthquakes",
        "\u{96} solar activity",
    ],
    /* 8A */
    [
        "its \u{83} \u{84}",
        "the \u{B1} \u{98} \u{99}",
        "its inhabitants' \u{9A} \u{85}",
        "\u{A1}",
        "its \u{8D} \u{8E}",
    ],
    /* 8B */ ["juice", "brandy", "water", "brew", "gargle blasters"],
    /* 8C */
    [
        "\u{B2}",
        "\u{B1} \u{99}",
        "\u{B1} \u{B2}",
        "\u{B1} \u{9B}",
        "\u{9B} \u{B2}",
    ],
    /* 8D */ ["fabulous", "exotic", "hoopy", "unusual", "exciting"],
    /* 8E */ ["cuisine", "night life", "casinos", "sit coms", " \u{A1} "],
    /* 8F */
    [
        "\u{B0}",
        "The planet \u{B0}",
        "The world \u{B0}",
        "This planet",
        "This world",
    ],
    /* 90 */
    [
        "n unremarkable",
        " boring",
        " dull",
        " tedious",
        " revolting",
    ],
    /* 91 */ ["planet", "world", "place", "little planet", "dump"],
    /* 92 */ ["wasp", "moth", "grub", "ant", "\u{B2}"],
    /* 93 */ ["poet", "arts graduate", "yak", "snail", "slug"],
    /* 94 */ ["tropical", "dense", "rain", "impenetrable", "exuberant"],
    /* 95 */ ["funny", "wierd", "unusual", "strange", "peculiar"],
    /* 96 */
    [
        "frequent",
        "occasional",
        "unpredictable",
        "dreadful",
        "deadly",
    ],
    /* 97 */
    [
        "\u{82} \u{81} for \u{8A}",
        "\u{82} \u{81} for \u{8A} and \u{8A}",
        "\u{88} by \u{89}",
        "\u{82} \u{81} for \u{8A} but \u{88} by \u{89}",
        "a\u{90} \u{91}",
    ],
    /* 98 */ ["\u{9B}", "mountain", "edible", "tree", "spotted"],
    /* 99 */ ["\u{9F}", "\u{A0}", "\u{87}oid", "\u{93}", "\u{92}"],
    /* 9A */ ["ancient", "exceptional", "eccentric", "ingrained", "\u{95}"],
    /* 9B */ ["killer", "deadly", "evil", "lethal", "vicious"],
    /* 9C */
    [
        "parking meters",
        "dust clouds",
        "ice bergs",
        "rock formations",
        "volcanoes",
    ],
    /* 9D */ ["plant", "tulip", "banana", "corn", "\u{B2}weed"],
    /* 9E */
    [
        "\u{B2}",
        "\u{B1} \u{B2}",
        "\u{B1} \u{9B}",
        "inhabitant",
        "\u{B1} \u{B2}",
    ],
    /* 9F */ ["shrew", "beast", "bison", "snake", "wolf"],
    /* A0 */ ["leopard", "cat", "monkey", "goat", "fish"],
    /* A1 */
    [
        "\u{8C} \u{8B}",
        "\u{B1} \u{9F} \u{A2}",
        "its \u{8D} \u{A0} \u{A2}",
        "\u{A3} \u{A4}",
        "\u{8C} \u{8B}",
    ],
    /* A2 */ ["meat", "cutlet", "steak", "burgers", "soup"],
    /* A3 */ ["ice", "mud", "Zero-G", "vacuum", "\u{B1} ultra"],
    /* A4 */ ["hockey", "cricket", "karate", "polo", "tennis"],
];

/// The 4-byte generator used only by the description text.
struct SoupRng([u8; 4]);

impl SoupRng {
    /// 6502-style: two 8-bit adds chained through the carry flag.
    fn next(&mut self) -> u8 {
        let [a, b, c, d] = self.0;
        let x = a.wrapping_mul(2);
        let sum = x as u16 + c as u16 + (a > 127) as u16;
        let new_a = sum as u8;
        let carry = (sum >> 8) as u8;
        let new_b = carry.wrapping_add(b).wrapping_add(d);
        self.0 = [new_a, new_b, x, b];
        new_b
    }
}

pub(crate) fn describe(system: &StarSystem) -> String {
    let s = system.seed;
    let mut rng = SoupRng([s.s1_lo(), s.s1_hi(), s.s2_lo(), s.s2_hi()]);
    let mut out = String::new();
    expand("\u{8F} is \u{97}.", system, &mut rng, &mut out);
    out
}

fn expand(source: &str, sys: &StarSystem, rng: &mut SoupRng, out: &mut String) {
    for ch in source.chars() {
        let code = ch as u32;
        match code {
            0x00..=0x7F => out.push(ch),
            0x81..=0xA4 => {
                let r = rng.next();
                let option = (r >= 0x33) as usize
                    + (r >= 0x66) as usize
                    + (r >= 0x99) as usize
                    + (r >= 0xCC) as usize;
                expand(DESC[(code - 0x81) as usize][option], sys, rng, out);
            }
            0xB0 => out.push_str(&sys.name),
            0xB1 => out.push_str(&adjective(&sys.name)),
            0xB2 => random_word(rng, out),
            _ => unreachable!("bad code {code:#X} in description data"),
        }
    }
}

/// "Lave" -> "Lavian": drop a trailing E or I, then append "ian".
fn adjective(name: &str) -> String {
    let stem = name
        .strip_suffix(['e', 'i'])
        .filter(|s| !s.is_empty())
        .unwrap_or(name);
    format!("{stem}ian")
}

fn random_word(rng: &mut SoupRng, out: &mut String) {
    let len = rng.next() & 3;
    let mut printed_any = false;
    for _ in 0..=len {
        let x = (rng.next() & 0x3E) as usize;
        for &c in &PAIRS[x..x + 2] {
            if c == b'.' {
                continue;
            }
            // Only the first letter actually printed stays upper case.
            let c = c as char;
            out.push(if printed_any {
                c.to_ascii_lowercase()
            } else {
                c
            });
            printed_any = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seed::Seed;

    /// Literal port of `gen_rnd_number` from txtelite.c, using `int` arithmetic.
    fn reference_rng(s: &mut [i32; 4]) -> u8 {
        let mut x = (s[0] * 2) & 0xFF;
        let mut a = x + s[2];
        if s[0] > 127 {
            a += 1;
        }
        s[0] = a & 0xFF;
        s[2] = x;
        a /= 256;
        x = s[1];
        a = (a + x + s[3]) & 0xFF;
        s[1] = a;
        s[3] = x;
        a as u8
    }

    fn some_system() -> StarSystem {
        StarSystem::from_seed(0, Seed::GALAXY_ONE)
    }

    #[test]
    fn rng_matches_txtelite_reference() {
        let starts = [
            [0u8; 4],
            [0xFF; 4],
            [0x48, 0x02, 0x53, 0xB7],
            [0x80, 0x7F, 0x01, 0xFE],
        ];
        for start in starts {
            let mut ours = SoupRng(start);
            let mut theirs = start.map(i32::from);
            for step in 0..1000 {
                assert_eq!(
                    ours.next(),
                    reference_rng(&mut theirs),
                    "start {start:?}, step {step}"
                );
            }
        }
    }

    #[test]
    fn adjective_drops_trailing_e_or_i() {
        assert_eq!(adjective("Lave"), "Lavian");
        assert_eq!(adjective("Rabedi"), "Rabedian");
        assert_eq!(adjective("Diso"), "Disoian");
        assert_eq!(adjective("Leesti"), "Leestian");
        // A one-letter name keeps its letter, as in txtelite.c.
        assert_eq!(adjective("E"), "Eian");
    }

    #[test]
    fn plain_text_passes_through_without_using_rng() {
        let mut rng = SoupRng([1, 2, 3, 4]);
        let mut out = String::new();
        expand("Hello.", &some_system(), &mut rng, &mut out);
        assert_eq!(out, "Hello.");
        assert_eq!(rng.0, [1, 2, 3, 4]);
    }

    #[test]
    fn name_codes_expand_to_planet_name_and_adjective() {
        let sys = some_system();
        let mut out = String::new();
        expand("\u{B0}/\u{B1}", &sys, &mut SoupRng([0; 4]), &mut out);
        assert_eq!(out, format!("{0}/{0}ian", sys.name));
    }

    #[test]
    fn option_thresholds_split_the_byte_into_fifths() {
        // With a = b = c = 0 the first RNG output is exactly d.
        let cases = [
            (0x00, "fabled"),
            (0x32, "fabled"),
            (0x33, "notable"),
            (0x65, "notable"),
            (0x66, "well known"),
            (0x98, "well known"),
            (0x99, "famous"),
            (0xCB, "famous"),
            (0xCC, "noted"),
            (0xFF, "noted"),
        ];
        for (d, expected) in cases {
            assert_eq!(SoupRng([0, 0, 0, d]).next(), d);
            let mut out = String::new();
            expand(
                "\u{81}",
                &some_system(),
                &mut SoupRng([0, 0, 0, d]),
                &mut out,
            );
            assert_eq!(out, expected, "rng byte {d:#04X}");
        }
    }

    #[test]
    fn random_word_is_at_most_eight_letters_in_sentence_case() {
        for a in 0..=255u8 {
            let mut out = String::new();
            random_word(&mut SoupRng([a, a.wrapping_mul(7), 3, 5]), &mut out);
            assert!(out.len() <= 8, "{out:?}");
            assert!(out.chars().all(|c| c.is_ascii_alphabetic()), "{out:?}");
            let mut chars = out.chars();
            if let Some(first) = chars.next() {
                assert!(first.is_ascii_uppercase(), "{out:?}");
                assert!(chars.all(|c| c.is_ascii_lowercase()), "{out:?}");
            }
        }
    }

    #[test]
    #[should_panic(expected = "bad code")]
    fn unknown_code_panics() {
        expand(
            "\u{A5}",
            &some_system(),
            &mut SoupRng([0; 4]),
            &mut String::new(),
        );
    }

    #[test]
    fn every_description_is_one_finished_sentence() {
        for g in 0..8 {
            for sys in crate::galaxy(g) {
                let d = sys.description();
                assert!(d.ends_with('.'), "{}: {d}", sys.name);
                assert!(
                    d.starts_with(['T', sys.name.chars().next().unwrap()]),
                    "{d}"
                );
                assert!(d.is_ascii(), "{d}");
            }
        }
    }
}
