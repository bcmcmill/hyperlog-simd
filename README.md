# `hyperlog-simd`

A Rust implementation of HyperLogLog and HyperLogLogPlusPlus streaming distinct count algorithms with SIMD (Single Instruction, Multiple Data) support on both ARM and x86_64 platforms. Also features serde compatibility for easy serialization and deserialization.

![Rust](https://img.shields.io/badge/Rust-latest-orange)
![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)

## Features

- 🚀 Implementations of both HyperLogLog (HLL) and HyperLogLog++ (HLL++) algorithms.
- 🏎️ SIMD support for ARM and x86_64 platforms for fast calculations.
- 📦 Serde compatible for seamless serialization/deserialization.
- 📚 Comprehensive documentation and examples.

## Table of Contents

- [Installation](#installation)
- [Usage](#usage)
- [Benchmark](#benchmark)
- [Contribution](#contribution)
- [License](#license)

## Installation

Add `hyperlog-simd` to your `Cargo.toml` dependencies:

```toml
[dependencies]
hyperlog-simd = "0.1.0"
```

## Usage

Here's a simple example to get started:

```rust
use hyperlog_simd::{HyperLogLog, HyperLogLogPlusPlus};

let mut hll = HyperLogLog::new();
hll.add("hello");
hll.add("world");

let count = hll.count();
println!("Estimated distinct count: {}", count);

let mut hllpp = HyperLogLogPlusPlus::new();
hllpp.add("hello");
hllpp.add("world");

let count_pp = hllpp.count();
println!("Estimated distinct count (HLL++): {}", count_pp);
```

For detailed examples and documentation, please refer to the [documentation](https://link-to-docs).

## Benchmark

This library provides impressive performance gains on platforms that support SIMD. Benchmarks will be updated periodically, and you can also run them yourself using:

```bash
cargo bench
```

## Contribution

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines and details.

## License

`hyperlog-simd` is licensed under the MIT License. See [LICENSE](LICENSE) for details.

---

Happy coding! We hope `hyperlog-simd` helps in your streaming distinct count needs with maximum efficiency! 🚀🦀
