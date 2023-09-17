use std::hash::{Hash, Hasher};

use packed_simd::f64x2;
use seahash::SeaHasher;

use crate::{M, P};

#[derive(Debug, Clone)]
pub struct HyperLogLog {
    pub registers: [u8; M],
}

impl HyperLogLog {
    #[inline]
    fn get_alpha() -> f64 {
        match M {
            16 => 0.673,
            32 => 0.697,
            64 => 0.709,
            _ => 0.7213 / (1.0 + 1.079 / M as f64),
        }
    }

    pub fn new() -> Self {
        Self { registers: [0; M] }
    }

    #[inline]
    pub fn add<T: Hash>(&mut self, item: T) {
        let mut hasher = SeaHasher::new();
        item.hash(&mut hasher);
        let hashed_value = hasher.finish() as usize;
        let j = hashed_value & (M - 1);
        let w = hashed_value >> P;
        let rho = w.leading_zeros() as u8 + 1;
        self.registers[j] = std::cmp::max(self.registers[j], rho);
    }

    #[inline]
    pub fn estimate(&self) -> f64 {
        let mut z = 0.0;
        for i in (0..M).step_by(2) {
            let val1 = f64x2::new(
                2f64.powi(-i32::from(self.registers[i])),
                2f64.powi(-i32::from(self.registers[i + 1])),
            );
            z += val1.sum();
        }
        let raw_estimate = Self::get_alpha() * (M * M) as f64 / z;
        if let Some(num_zeros) = self
            .registers
            .iter()
            .filter(|&&val| val == 0)
            .count()
            .checked_next_power_of_two()
        {
            M as f64 * (M as f64 / num_zeros as f64).ln()
        } else {
            raw_estimate
        }
    }

    #[inline]
    pub fn merge(&mut self, other: &Self) {
        for i in 0..M {
            self.registers[i] = self.registers[i].max(other.registers[i]);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::HyperLogLog;
    use std::collections::HashSet;

    #[test]
    fn add_and_count_unique_elements() {
        let mut hll = HyperLogLog::new();
        for i in 0..10_000 {
            hll.add(&i);
        }

        let count = dbg!(hll.estimate());
        assert!((count - 10_000 as f64).abs() < 10_000 as f64 * 0.05); // error within 5%
    }

    #[test]
    fn count_with_duplicates() {
        let mut hll = HyperLogLog::new();
        for _ in 0..100 {
            for i in 0..10_000 {
                hll.add(&i);
            }
        }

        let count = dbg!(hll.estimate());
        assert!((count - 10_000 as f64).abs() < 10_000 as f64 * 0.05); // error within 5%
    }

    #[test]
    fn compare_with_hashset() {
        let mut hll = HyperLogLog::new();
        let mut set = HashSet::new();

        for i in 0..10_000 {
            let item = format!("item_{}", i);
            hll.add(&item);
            set.insert(item);
        }

        let hll_count = dbg!(hll.estimate());
        let set_count = dbg!(set.len() as f64);
        assert!((hll_count - set_count).abs() < set_count * 0.05); // error within 5%
    }

    #[test]
    fn test_merge() {
        let mut hll1 = HyperLogLog::new();
        hll1.add(1);
        hll1.add(2);

        let mut hll2 = HyperLogLog::new();
        hll2.add(3);
        hll2.add(4);

        hll1.merge(&hll2);

        assert_eq!(hll1.estimate().round() as u32, 4);
    }
}
