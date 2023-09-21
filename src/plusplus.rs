use std::hash::{Hash, Hasher};

use packed_simd::{f64x8, u32x2, u8x16};
use seahash::SeaHasher;

#[cfg(feature = "serde_support")]
use crate::serde::{serialize_registers, CompressedRegistersVisitor};
#[cfg(feature = "serde_support")]
use serde::{de::Deserializer, Deserialize, Serialize, Serializer};

use crate::{ALPHA, EMPTY_REGISTERS, M, P};

/// An enhanced HyperLogLog data structure, often termed HyperLogLog++,
/// for estimating the cardinality of a dataset without storing individual elements.
#[derive(Debug, Clone)]
pub struct HyperLogLogPlusPlus {
    /// Registers used for maintaining the cardinality estimate.
    /// The number of registers (`M`) impacts precision and memory usage.
    pub registers: Box<[u8; M]>,
}

impl HyperLogLogPlusPlus {
    /// Constructs a new instance of HyperLogLog++ with all registers initialized to zero.
    ///
    /// # Returns
    /// A new `HyperLogLogPlusPlus` instance.
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            registers: Box::new(unsafe { EMPTY_REGISTERS.clone() }),
        }
    }

    /// Adds an item to the HyperLogLog++. This will update the registers based on
    /// the hash of the item but won't store the item itself.
    ///
    /// # Parameters
    /// * `item`: The item to be added. It should implement the `Hash` trait.
    #[inline(always)]
    pub fn add<T: Hash>(&mut self, item: T) {
        let mut h = SeaHasher::default();

        item.hash(&mut h);

        let mut hash = h.finish();

        for _ in 0..2 {
            let vec_hash = u32x2::new(
                (hash & (M as u64 - 1)) as u32,
                ((hash >> 32) & (M as u64 - 1)) as u32,
            );

            let vec_w = u32x2::new((hash >> P) as u32, (hash >> (32 + P)) as u32);
            let vec_rank = vec_w.min_element().leading_zeros() as u8 + 1;
            let max_index = vec_hash.extract(0) as usize;

            if self.registers[max_index] < vec_rank {
                self.registers[max_index] = vec_rank;
            }

            hash = hash.wrapping_shr(64);
        }
    }

    /// Estimates the cardinality or unique count of the items added to the HyperLogLog++.
    ///
    /// # Returns
    /// An approximate count (as `f64`) of unique items added.
    #[inline(always)]
    pub fn estimate(&self) -> f64 {
        let mut acc_sum = f64x8::splat(0.0);
        let len = self.registers.len();
        let simd_iteration_count = len / 8;

        for i in 0..simd_iteration_count {
            let chunk = self.registers[i * 8..(i + 1) * 8]
                .iter()
                .map(|&x| x as f64)
                .collect::<Vec<f64>>();
            let vector = f64x8::from_slice_unaligned(&chunk);
            acc_sum += f64x8::splat(2.0).powf(-vector);
        }

        let rem = len % 8;

        if rem > 0 {
            let chunk = self.registers[len - rem..]
                .iter()
                .map(|&x| x as f64)
                .collect::<Vec<f64>>();
            let vector = f64x8::from_slice_unaligned(&chunk);
            acc_sum += f64x8::splat(2.0).powf(-vector);
        }

        let harmonic_mean: f64 = 1.0 / acc_sum.sum();
        let approx_cardinality: f64 = ALPHA * (M * M) as f64 * harmonic_mean;
        let zero_reg_count: f64 = self.registers.iter().filter(|&rank| *rank == 0).count() as f64;

        if approx_cardinality <= 2.5 * M as f64 && zero_reg_count > 0.0 {
            M as f64 * (M as f64 / zero_reg_count).ln()
        } else {
            approx_cardinality
        }
    }

    /// Merges the state of another HyperLogLog++ instance into this one.
    /// This is useful for combining the cardinality estimates of two separate datasets.
    ///
    /// # Parameters
    /// * `other`: The other `HyperLogLogPlusPlus` instance whose state is to be merged into this one.
    #[inline(always)]
    pub fn merge(&mut self, other: &HyperLogLogPlusPlus) {
        const CHUNKS: usize = M / 16; // This needs to be a const

        unsafe {
            let self_regs =
                std::slice::from_raw_parts_mut(self.registers.as_mut_ptr() as *mut u8x16, CHUNKS);
            let other_regs =
                std::slice::from_raw_parts(other.registers.as_ptr() as *const u8x16, CHUNKS);

            for i in 0..CHUNKS {
                self_regs[i] = self_regs[i].max(other_regs[i]);
            }
        }

        // If M is not a multiple of 16, process remaining elements
        for i in (CHUNKS * 16)..M {
            self.registers[i] = std::cmp::max(self.registers[i], other.registers[i]);
        }
    }
}

impl Default for HyperLogLogPlusPlus {
    /// Creates a default instance of `HyperLogLogPlusPlus`.
    ///
    /// This is equivalent to calling `HyperLogLogPlusPlus::new()`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let hll = HyperLogLogPlusPlus::default();
    /// // ... use the `hll` instance ...
    /// ```
    fn default() -> Self {
        Self::new()
    }
}

impl From<[u8; M]> for HyperLogLogPlusPlus {
    /// Creates a `HyperLogLogPlusPlus` instance from a given array of registers.
    ///
    /// # Arguments
    ///
    /// * `registers`: An array of `u8` representing the internal state
    ///   of the HyperLogLogPlusPlus.
    fn from(registers: [u8; M]) -> Self {
        HyperLogLogPlusPlus {
            registers: Box::new(registers),
        }
    }
}

#[cfg(feature = "serde_support")]
impl Serialize for HyperLogLogPlusPlus {
    /// Serializes the `HyperLogLogPlusPlus` instance.
    ///
    /// The `registers` field will be serialized in a format suitable
    /// for transmission or storage using the `serialize_registers` function.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serialize_registers(&self.registers, serializer)
    }
}

#[cfg(feature = "serde_support")]
impl<'de> Deserialize<'de> for HyperLogLogPlusPlus {
    /// Deserializes data to construct a `HyperLogLogPlusPlus` instance.
    ///
    /// The data is expected to contain a `registers` field in a specific
    /// serialized format. The `CompressedRegistersVisitor` is used to assist
    /// in this deserialization process.
    fn deserialize<D>(deserializer: D) -> Result<HyperLogLogPlusPlus, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(CompressedRegistersVisitor::new())
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

        let estimate = dbg!(hllpp.estimate());
        assert!(
            unique_values.len() as f64 * 0.9 <= estimate
                && estimate <= unique_values.len() as f64 * 1.1,
            "Estimate out of expected range"
        );
    }

    #[test]
    fn test_large_number_of_values() {
        let mut hllpp = HyperLogLogPlusPlus::new();

        for i in 0..500_000 {
            hllpp.add(i);
        }

        let estimate = hllpp.estimate();
        assert!(
            (490_000..510_000).contains(&(dbg!(estimate) as usize)),
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
