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
    pub s0: u16,
    pub s1: u16,
    pub s2: u16,
}

impl Seed {
    /// Galaxy 1's seed, as hard-coded in the original game.
    pub const GALAXY_ONE: Seed = Seed::new(0x5A4A, 0x0248, 0xB753);

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
    pub fn galaxy(n: usize) -> Self {
        (0..n % 8).fold(Self::GALAXY_ONE, |s, _| s.next_galaxy())
    }

    pub fn s0_lo(&self) -> u8 {
        self.s0 as u8
    }
    pub fn s0_hi(&self) -> u8 {
        (self.s0 >> 8) as u8
    }
    pub fn s1_lo(&self) -> u8 {
        self.s1 as u8
    }
    pub fn s1_hi(&self) -> u8 {
        (self.s1 >> 8) as u8
    }
    pub fn s2_lo(&self) -> u8 {
        self.s2 as u8
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn galaxies_cycle_after_eight_jumps() {
        let mut s = Seed::GALAXY_ONE;
        for _ in 0..8 {
            s = s.next_galaxy();
        }
        assert_eq!(s, Seed::GALAXY_ONE);
        assert_eq!(Seed::galaxy(8), Seed::galaxy(0));
    }

    #[test]
    fn galaxy_two_seed() {
        // 0x5A4A -> 0xB494, 0x0248 -> 0x0490, 0xB753 -> 0x6FA6
        assert_eq!(Seed::galaxy(1), Seed::new(0xB494, 0x0490, 0x6FA6));
    }
}
