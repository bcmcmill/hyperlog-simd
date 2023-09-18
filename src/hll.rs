use std::hash::{Hash, Hasher};

use packed_simd::f64x2;
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
    pub registers: [u8; M],
}

impl HyperLogLog {
    /// Computes the alpha constant for bias correction based on the size of the register list.
    ///
    /// # Returns
    /// A `f64` alpha constant value for the given `M`.
    #[inline]
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
        Self { registers: [0; M] }
    }

    /// Adds an item to the HyperLogLog. This does not increase the memory footprint
    /// of the HLL as it only updates the registers based on the hash of the item.
    ///
    /// # Parameters
    /// * `item`: An item that implements the `Hash` trait to be added to the HLL.
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

    /// Provides an estimate of the number of unique items added to the HLL.
    ///
    /// # Returns
    /// A `f64` approximate count of unique items added to the HLL.
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
    #[inline]
    pub fn merge(&mut self, other: &Self) {
        for i in 0..M {
            self.registers[i] = self.registers[i].max(other.registers[i]);
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
        HyperLogLog { registers }
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
