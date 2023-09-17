pub mod classic;
pub mod plusplus;
pub mod serde;

pub use classic::HyperLogLog;
pub use plusplus::HyperLogLogPlusPlus;

// Constants used in estimation formula
pub const P: usize = 18;
pub const M: usize = 1 << P;
pub const ALPHA: f64 = 0.7213 / (1.0 + 1.079 / (M as f64));
