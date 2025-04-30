use core::hash::{Hasher, BuildHasher};

pub struct SimpleHasher {
    state: u64,
}

impl Hasher for SimpleHasher {
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.state = self.state.wrapping_mul(131).wrapping_add(*byte as u64);
        }
    }

    fn finish(&self) -> u64 {
        self.state
    }
}

pub struct SimpleBuildHasher {
    seed: u64,
}

impl SimpleBuildHasher {
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }
}

impl BuildHasher for SimpleBuildHasher {
    type Hasher = SimpleHasher;

    fn build_hasher(&self) -> Self::Hasher {
        SimpleHasher { state: self.seed }
    }
}
