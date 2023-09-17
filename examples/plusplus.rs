#![feature(slice_pattern)]

use hyperlog_simd::plusplus::HyperLogLogPlusPlus;
use nanorand::Rng;

use std::{
    fs::File,
    io::{Read, Write},
};

fn main() {
    // Initialise Random number generator
    let mut rng = nanorand::tls_rng();

    // Initialise two HyperLogLog objects, hll1 and hll2
    let mut hll1 = HyperLogLogPlusPlus::new();
    let mut hll2 = HyperLogLogPlusPlus::new();

    // Generate random number of visits(1 to 99) for first 50000 users and add these visits to hll1
    let visits = rng.generate_range(1..100);
    for user_id in 1..50_000 {
        for _ in 0..visits {
            hll1.add(&format!("user-{}", user_id));
        }
    }

    // Generate random number of visits(1 to 99) for next 50000 users and add these visits to hll2
    let visits = rng.generate_range(1..100);
    for user_id in 50_000..100_000 {
        for _ in 0..visits {
            hll2.add(&format!("user-{}", user_id));
        }
    }

    // Merge hll1 and hll2
    hll1.merge(&hll2);

    // Log the total visits and estimated unique visits after merging
    println!("Total visits recorded: {}", 100_000 * 50);
    println!("Estimated unique users after merging: {}", hll1.estimate());

    // Serialize the merged HyperLogLog state to JSON and write to the file "hyperloglog.json"
    let encoded: String = serde_json::to_string(&hll1).unwrap();
    let mut file = File::create("hyperloglog.json").unwrap();
    file.write_all(encoded.as_bytes()).unwrap();
    file.flush().unwrap();

    // Deserialize contents of the "hyperloglog.json" file back into a HyperLogLog object
    let mut file = File::open("hyperloglog.json").unwrap();
    let mut encoded = String::new();
    file.read_to_string(&mut encoded).unwrap();
    let hll3: HyperLogLogPlusPlus = serde_json::from_slice(encoded.as_bytes()).unwrap();

    // Print the estimated count of unique users after deserialization
    println!(
        "Estimated unique users after deserializing: {}",
        hll3.estimate()
    );
}
