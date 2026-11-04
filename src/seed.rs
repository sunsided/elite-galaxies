//! The 48-bit seed that drives everything in Elite's universe.
//!
//! The 6502 kept three 16-bit words, `s0`, `s1` and `s2` (six bytes,
//! `QQ15` in the BBC Micro source). A galaxy is fully described by the
//! seed of its first system; every following system is reached by
//! "twisting" the seed four times, and the next galaxy is reached by
//! rotating every byte of the galaxy seed one bit to the left.

/// The three-word seed (`s0`, `s1`, `s2`), each word little-endian on the 6502.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Seed {
    /// First word; its high byte is the Y coordinate, bit 6 of its low byte the long-name flag.
    pub s0: u16,
    /// Second word; its high byte is the X coordinate, bits 3-5 of its low byte the government.
    pub s1: u16,
    /// Third word; bits 0-4 of its high byte pick the next name syllable.
    pub s2: u16,
}

impl Seed {
    /// Galaxy 1's seed, as hard-coded in the original game.
    pub const GALAXY_ONE: Seed = Seed::new(0x5A4A, 0x0248, 0xB753);

    /// A seed from its three words.
    pub const fn new(s0: u16, s1: u16, s2: u16) -> Self {
        Self { s0, s1, s2 }
    }

    /// One step of the Tribonacci-like generator (`TT54` in the 6502 source):
    ///
    /// ```text
    /// tmp = s0 + s1 + s2   (mod 2^16)
    /// s0  = s1
    /// s1  = s2
    /// s2  = tmp
    /// ```
    ///
    /// ```
    /// use elite::Seed;
    ///
    /// let mut s = Seed::GALAXY_ONE;
    /// s.twist();
    /// assert_eq!(s, Seed::new(0x0248, 0xB753, 0x13E5));
    /// ```
    pub fn twist(&mut self) {
        let tmp = self.s0.wrapping_add(self.s1).wrapping_add(self.s2);
        self.s0 = self.s1;
        self.s1 = self.s2;
        self.s2 = tmp;
    }

    /// Advances to the seed of the next galaxy (`Ghy` in the 6502 source):
    /// every one of the six bytes is rotated left by one bit, independently.
    ///
    /// Because an 8-bit rotation has period 8, applying this eight times
    /// returns the original seed — which is why there are exactly eight
    /// galaxies and galaxy 9 would be galaxy 1 again.
    ///
    /// ```
    /// use elite::Seed;
    ///
    /// let g2 = Seed::GALAXY_ONE.next_galaxy();
    /// assert_eq!(g2, Seed::new(0xB494, 0x0490, 0x6FA6));
    /// ```
    #[must_use]
    pub fn next_galaxy(self) -> Self {
        Self {
            s0: rotate_bytes_left(self.s0),
            s1: rotate_bytes_left(self.s1),
            s2: rotate_bytes_left(self.s2),
        }
    }

    /// The seed for galaxy `n` (0-based), reached by `n` galactic hyperspace jumps.
    ///
    /// Any `n` is accepted; values `>= 8` wrap around, just like in the game.
    ///
    /// ```
    /// use elite::Seed;
    ///
    /// assert_eq!(Seed::galaxy(0), Seed::GALAXY_ONE);
    /// assert_eq!(Seed::galaxy(9), Seed::galaxy(1));
    /// ```
    pub fn galaxy(n: usize) -> Self {
        (0..n % 8).fold(Self::GALAXY_ONE, |s, _| s.next_galaxy())
    }

    /// Low byte of `s0` (`QQ15` in the 6502 source).
    pub fn s0_lo(&self) -> u8 {
        self.s0 as u8
    }
    /// High byte of `s0` (`QQ15+1`).
    pub fn s0_hi(&self) -> u8 {
        (self.s0 >> 8) as u8
    }
    /// Low byte of `s1` (`QQ15+2`).
    pub fn s1_lo(&self) -> u8 {
        self.s1 as u8
    }
    /// High byte of `s1` (`QQ15+3`).
    pub fn s1_hi(&self) -> u8 {
        (self.s1 >> 8) as u8
    }
    /// Low byte of `s2` (`QQ15+4`).
    pub fn s2_lo(&self) -> u8 {
        self.s2 as u8
    }
    /// High byte of `s2` (`QQ15+5`).
    pub fn s2_hi(&self) -> u8 {
        (self.s2 >> 8) as u8
    }
}

/// Rotates the high and low byte of `w` left by one bit, each on its own.
fn rotate_bytes_left(w: u16) -> u16 {
    let hi = ((w >> 8) as u8).rotate_left(1);
    let lo = (w as u8).rotate_left(1);
    u16::from_be_bytes([hi, lo])
}

/// Tests for the seed twist, galaxy rotation and byte accessors.
#[cfg(test)]
mod tests {
    use super::*;

    /// Eight galaxy jumps return to the starting seed.
    #[test]
    fn galaxies_cycle_after_eight_jumps() {
        let mut s = Seed::GALAXY_ONE;
        for _ in 0..8 {
            s = s.next_galaxy();
        }
        assert_eq!(s, Seed::GALAXY_ONE);
        assert_eq!(Seed::galaxy(8), Seed::galaxy(0));
    }

    /// Galaxy 2's seed is galaxy 1's with every byte rotated once.
    #[test]
    fn galaxy_two_seed() {
        // 0x5A4A -> 0xB494, 0x0248 -> 0x0490, 0xB753 -> 0x6FA6
        assert_eq!(Seed::galaxy(1), Seed::new(0xB494, 0x0490, 0x6FA6));
    }

    /// One twist shifts the words down and appends their 16-bit sum.
    #[test]
    fn twist_shifts_words_and_appends_wrapping_sum() {
        let mut s = Seed::GALAXY_ONE;
        s.twist();
        // 0x5A4A + 0x0248 + 0xB753 = 0x113E5, truncated to 16 bits.
        assert_eq!(s, Seed::new(0x0248, 0xB753, 0x13E5));
    }

    /// Byte rotation never carries a bit into the neighbouring byte.
    #[test]
    fn rotation_keeps_bytes_independent() {
        // The top bit of each byte wraps into that byte's bit 0, never into
        // the neighbouring byte.
        assert_eq!(rotate_bytes_left(0x8001), 0x0102);
        assert_eq!(rotate_bytes_left(0x0080), 0x0001);
        assert_eq!(rotate_bytes_left(0xFFFF), 0xFFFF);
    }

    /// Galaxy numbers 8 and 16 above any galaxy name the same seed.
    #[test]
    fn galaxy_number_wraps_modulo_eight() {
        for n in 0..8 {
            assert_eq!(Seed::galaxy(n + 8), Seed::galaxy(n));
            assert_eq!(Seed::galaxy(n + 16), Seed::galaxy(n));
        }
    }

    /// No two of the eight galaxies share a seed.
    #[test]
    fn all_eight_galaxy_seeds_are_distinct() {
        let seeds: std::collections::HashSet<_> = (0..8).map(Seed::galaxy).collect();
        assert_eq!(seeds.len(), 8);
    }

    /// The byte accessors return low and high bytes of each word.
    #[test]
    fn byte_accessors_split_words_little_endian() {
        let s = Seed::new(0x5A4A, 0x0248, 0xB753);
        assert_eq!((s.s0_lo(), s.s0_hi()), (0x4A, 0x5A));
        assert_eq!((s.s1_lo(), s.s1_hi()), (0x48, 0x02));
        assert_eq!((s.s2_lo(), s.s2_hi()), (0x53, 0xB7));
    }
}
