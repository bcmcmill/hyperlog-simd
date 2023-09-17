use criterion::{black_box, criterion_group, criterion_main, Criterion};
use hyperloglog::HyperLogLog;
use nanorand::{Rng, WyRand};

fn generate_random_numbers(n: usize) -> Vec<u32> {
    let mut rng = WyRand::new();
    (0..n).map(|_| rng.generate()).collect()
}

fn bench_add(c: &mut Criterion) {
    let mut hll = HyperLogLog::new();
    let mut group = c.benchmark_group("add");
    let items = generate_random_numbers(1_000_000);

    group.bench_function("HyperLogLog", |b| {
        b.iter(|| {
            for item in &items {
                hll.add(item);
            }
        })
    });

    group.finish();
}

fn bench_count(c: &mut Criterion) {
    let mut hll = HyperLogLog::new();
    let mut group = c.benchmark_group("count");
    let items = generate_random_numbers(1_000_000);

    for item in &items {
        hll.add(item);
    }

    group.bench_function("HyperLogLog", |b| b.iter(|| hll.estimate()));

    group.finish();
}

fn process_users(c: &mut Criterion) {
    let mut group = c.benchmark_group("process_users");

    group.bench_function("HyperLogLog", |b| {
        b.iter(|| {
            let mut hll = HyperLogLog::new();
            let mut rng = nanorand::tls_rng();
            let visits = rng.generate_range(1..3);

            for user_id in 1..50_792 {
                for _ in 0..visits {
                    hll.add(black_box(format!("user-{}", user_id)));
                }
            }
        })
    });

    group.finish();
}

criterion_group!(benches, bench_add, bench_count, process_users);
criterion_main!(benches);
