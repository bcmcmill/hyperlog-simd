use std::hash::{Hash, Hasher};

use packed_simd::{f64x4, u32x2};
use seahash::SeaHasher;

use crate::{ALPHA, M, P};

#[derive(Debug, Clone)]
// HyperLogLogPlusPlus struct containing an array of registers
pub struct HyperLogLogPlusPlus {
    pub registers: [u8; M],
}

impl HyperLogLogPlusPlus {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            // Initialize all registers with zero
            registers: [0; M],
        }
    }

    #[inline(always)]
    pub fn add<T: Hash>(&mut self, item: T) {
        // Get a hash for the input item
        let mut h = SeaHasher::default();
        item.hash(&mut h);
        let mut hash = h.finish();

        for _ in 0..2 {
            // Perform bitwise operations on hash to get vec_w and vec_rank
            let vec_hash = u32x2::new(
                (hash & (M as u64 - 1)) as u32,
                ((hash >> 32) & (M as u64 - 1)) as u32,
            );
            let vec_w = u32x2::new((hash >> P) as u32, (hash >> (32 + P)) as u32);
            let vec_rank = vec_w.min_element().leading_zeros() as u8 + 1;
            let max_index = vec_hash.extract(0) as usize;

            // Update the register values with calculated vec_rank if vec_rank is greater
            if self.registers[max_index] < vec_rank {
                self.registers[max_index] = vec_rank;
            }

            hash = hash.wrapping_shr(64);
        }
    }

    #[inline(always)]
    pub fn estimate(&self) -> f64 {
        let simd_chunk_parts = unsafe {
            std::slice::from_raw_parts(
                self.registers.as_ptr() as *const u64,
                self.registers.len() / 4,
            )
        };
        let simd_vecs: Vec<f64x4> = simd_chunk_parts
            .iter()
            .map(|&chunk| {
                f64x4::new(
                    chunk as f64,
                    (chunk.wrapping_shr(32)) as f64,
                    (chunk.wrapping_shr(64)) as f64,
                    (chunk.wrapping_shr(96)) as f64,
                )
            })
            .collect();

        let acc_sum = simd_vecs
            .iter()
            .fold(f64x4::splat(0.0), |acc, &element_rank| {
                acc + f64x4::splat(2.0).powf(-element_rank)
            });

        let harmonic_mean: f64 = 1.0 / acc_sum.sum();
        let approx_cardinality: f64 = ALPHA * (M * M) as f64 * harmonic_mean;

        let zero_reg_count: f64 = self.registers.iter().filter(|&rank| *rank == 0).count() as f64;

        if approx_cardinality <= 2.5 * M as f64 && zero_reg_count > 0.0 {
            return M as f64 * (M as f64 / zero_reg_count).ln();
        }

        approx_cardinality
    }

    #[inline(always)]
    pub fn merge(&mut self, other: &HyperLogLogPlusPlus) {
        for i in 0..M {
            self.registers[i] = std::cmp::max(self.registers[i], other.registers[i]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use nanorand::Rng;

    #[test]
    fn test_add_and_estimate() {
        let mut hllpp = HyperLogLogPlusPlus::new();
        for i in 0..1000 {
            hllpp.add(i);
        }

        let estimate = hllpp.estimate();
        assert!(
            (950..1050).contains(&(estimate as usize)),
            "Estimate out of expected range"
        );
    }

    #[test]
    fn test_add_same_value_multiple_times() {
        let mut hllpp = HyperLogLogPlusPlus::new();
        for _ in 0..1000 {
            hllpp.add(500);
        }

        let estimate = hllpp.estimate();
        assert_eq!(
            estimate as usize, 1,
            "Estimate should be 1 for identical elements"
        );
    }

    #[test]
    fn test_empty_estimate() {
        let hllpp = HyperLogLogPlusPlus::new();
        let estimate = hllpp.estimate();
        assert_eq!(estimate, 0.0, "Empty HLL++ should estimate to 0");
    }

    #[test]
    fn test_random_values() {
        let mut hllpp = HyperLogLogPlusPlus::new();
        let mut rng = nanorand::tls_rng();
        let mut unique_values = std::collections::HashSet::new();

        for _ in 0..100_000 {
            let val = rng.generate_range(0..50_000);
            unique_values.insert(val);
            hllpp.add(val);
        }

        let estimate = hllpp.estimate();
        assert!(
            unique_values.len() as f64 * 0.9 <= estimate
                && estimate <= unique_values.len() as f64 * 1.1,
            "Estimate out of expected range"
        );
    }

    #[test]
    fn test_large_number_of_values() {
        let mut hllpp = HyperLogLogPlusPlus::new();

        for i in 0..1_000_000 {
            hllpp.add(i);
        }

        let estimate = hllpp.estimate();
        assert!(
            (990_000..1_010_000).contains(&(dbg!(estimate) as usize)),
            "Estimate out of expected range"
        );
    }

    #[test]
    fn test_merge() {
        let mut hll1 = HyperLogLogPlusPlus::new();
        hll1.add(1);
        hll1.add(2);

        let mut hll2 = HyperLogLogPlusPlus::new();
        hll2.add(3);
        hll2.add(4);

        hll1.merge(&hll2);

        assert_eq!(hll1.estimate().round() as u32, 4);
    }
}
