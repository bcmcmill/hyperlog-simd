/// `hyperlog-simd` - A SIMD accelerated HLL/HLL++ implementation
///
/// # Modules
/// * `hll` - Contains implementations of canonical HyperLogLog
/// * `plusplus` - Contains the improved HyperLogLog++ variant
/// * `serde` - Contains serialization/deserialization utilities for HyperLogLog structures
pub mod hll;
pub mod plusplus;

#[cfg(feature = "serde_support")]
pub mod serde;

/// `hll::HyperLogLog` made available at the top level
pub use hll::HyperLogLog;
/// `plusplus::HyperLogLogPlusPlus` made available at the top level
pub use plusplus::HyperLogLogPlusPlus;

/// Number of distinct register tracks in the HyperLogLog structures,
/// defined as 2^P where P is the number of bits used to select a register
pub const P: usize = 20;
/// Number of registers, it is computed as 2^P
pub const M: usize = 1 << P;
/// Constant used for bias correction in the estimation formula.
/// It is defined as  0.7213 / (1 + 1.079 / M), where M is the number of registers.
pub const ALPHA: f64 = 0.7213 / (1.0 + 1.079 / (M as f64));

pub static mut EMPTY_REGISTERS: [u8; M] = [0; M];
