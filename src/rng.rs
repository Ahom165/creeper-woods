/// Générateur pseudo-aléatoire xorshift64* déterministe — sert au niveau (même carte
/// à chaque partie) et aux textures procédurales.
pub struct Rng {
    s: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { s: seed | 1 }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.s;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.s = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Nombre dans [0, 1).
    pub fn f32(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn range_f32(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f32()
    }

    pub fn range_usize(&mut self, a: usize, b: usize) -> usize {
        a + (self.next_u64() % ((b - a + 1) as u64)) as usize
    }

    pub fn chance(&mut self, p: f32) -> bool {
        self.f32() < p
    }
}
