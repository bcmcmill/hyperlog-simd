use std::hash::{Hash, Hasher};

use packed_simd::{f64x8, u8x16};
use seahash::SeaHasher;

#[cfg(feature = "serde_support")]
use serde::{de::Deserializer, Deserialize, Serialize, Serializer};

#[cfg(feature = "serde_support")]
use crate::serde::{serialize_registers, CompressedRegistersVisitor};

use crate::{M, P};

/// A HyperLogLog data structure for approximating the cardinality (number of unique elements)
/// of a dataset.
#[derive(Debug, Clone)]
pub struct HyperLogLog {
    /// An array of registers. The number of registers is specified by the constant `M`
    /// and determines the precision and memory usage of the HLL.
    pub registers: Box<[u8; M]>,
}

impl HyperLogLog {
    /// Computes the alpha constant for bias correction based on the size of the register list.
    ///
    /// # Returns
    /// A `f64` alpha constant value for the given `M`.
    #[inline(always)]
    fn get_alpha() -> f64 {
        match M {
            16 => 0.673,
            32 => 0.697,
            64 => 0.709,
            _ => 0.7213 / (1.0 + 1.079 / M as f64),
        }
    }

    /// Creates a new HyperLogLog instance with all registers initialized to zero.
    ///
    /// # Returns
    /// A new `HyperLogLog` instance.
    pub fn new() -> Self {
        Self {
            registers: Box::new([0; M]),
        }
    }

    /// Adds an item to the HyperLogLog. This does not increase the memory footprint
    /// of the HLL as it only updates the registers based on the hash of the item.
    ///
    /// # Parameters
    /// * `item`: An item that implements the `Hash` trait to be added to the HLL.
    #[inline(always)]
    pub fn add<T: Hash>(&mut self, item: T) {
        let mut hasher = SeaHasher::new();
        item.hash(&mut hasher);
        let hashed_value = hasher.finish() as usize;
        let j = hashed_value & (M - 1);
        let w = hashed_value >> P;
        let rho = w.leading_zeros() as u8 + 1;
        self.registers[j] = std::cmp::max(self.registers[j], rho);
    }

    /// Provides an estimate of the number of unique items added to the HLL.
    ///
    /// # Returns
    /// A `f64` approximate count of unique items added to the HLL.
    #[inline(always)]
    pub fn estimate(&self) -> f64 {
        let len = self.registers.len();
        let simd_iteration_count = len / 8;
        let mut z = f64x8::splat(0.0);

        for i in 0..simd_iteration_count {
            z += f64x8::new(
                2f64.powi(-i32::from(self.registers[i * 8])),
                2f64.powi(-i32::from(self.registers[i * 8 + 1])),
                2f64.powi(-i32::from(self.registers[i * 8 + 2])),
                2f64.powi(-i32::from(self.registers[i * 8 + 3])),
                2f64.powi(-i32::from(self.registers[i * 8 + 4])),
                2f64.powi(-i32::from(self.registers[i * 8 + 5])),
                2f64.powi(-i32::from(self.registers[i * 8 + 6])),
                2f64.powi(-i32::from(self.registers[i * 8 + 7])),
            );
        }

        // Processing the remainder
        let rem = len % 8;
        if rem != 0 {
            let mut remainder = f64x8::splat(0.0);
            for i in 0..rem {
                remainder += f64x8::splat(
                    2f64.powi(-i32::from(self.registers[simd_iteration_count * 8 + i])),
                );
            }
            z += remainder;
        }

        let raw_estimate = Self::get_alpha() * (M * M) as f64 / z.sum();
        let num_zeros = self.registers.iter().filter(|&&val| val == 0).count();

        if num_zeros > 0 {
            return M as f64 * (M as f64 / num_zeros as f64).ln();
        }

        raw_estimate
    }

    /// Merges another HyperLogLog into the current HLL. This is useful when you want
    /// to combine the unique counts of two datasets.
    ///
    /// # Parameters
    /// * `other`: A reference to another `HyperLogLog` instance to be merged.
    #[inline(always)]
    pub fn merge(&mut self, other: &HyperLogLog) {
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

impl Default for HyperLogLog {
    /// Creates a default instance of `HyperLogLog`.
    ///
    /// This is equivalent to calling `HyperLogLog::new()`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let hll = HyperLogLog::default();
    /// // ... use the `hll` instance ...
    /// ```
    fn default() -> Self {
        Self::new()
    }
}

impl From<[u8; M]> for HyperLogLog {
    /// Creates a `HyperLogLogPlusPlus` instance from a given array of registers.
    ///
    /// # Arguments
    ///
    /// * `registers`: An array of `u8` representing the internal state
    ///   of the HyperLogLogPlusPlus.
    fn from(registers: [u8; M]) -> Self {
        let r = Box::new(registers);
        HyperLogLog { registers: r }
    }
}

#[cfg(feature = "serde_support")]
impl Serialize for HyperLogLog {
    /// Serializes the `HyperLogLog` instance.
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
impl<'de> Deserialize<'de> for HyperLogLog {
    /// Deserializes data to construct a `HyperLogLog` instance.
    ///
    /// The data is expected to contain a `registers` field in a specific
    /// serialized format. The `CompressedRegistersVisitor` is used to assist
    /// in this deserialization process.
    fn deserialize<D>(deserializer: D) -> Result<HyperLogLog, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(CompressedRegistersVisitor::new())
    }
}

#[cfg(test)]
mod tests {
    use crate::HyperLogLog;
    use std::collections::HashSet;

    #[test]
    fn add_and_estimate_unique_elements() {
        let mut hll = HyperLogLog::new();
        for i in 0..10_000 {
            hll.add(&i);
        }

        let count = dbg!(hll.estimate());
        assert!((count - 10_000 as f64).abs() < 10_000 as f64 * 0.05); // error within 5%
    }

    #[test]
    fn estimate_with_duplicates() {
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
