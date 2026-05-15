//! Criterion microbenchmark for `AllCentroidsCache` concurrent read
//! throughput.
//!
//! Measures aggregate `posting()` lookups per second as the number of
//! reader threads scales. Before the `Mutex` -> `RwLock` change in
//! `centroids.rs`, every reader serialized on the same exclusive lock
//! and aggregate throughput plateaued; after the change, reads are
//! concurrent.
//!
//! Run with:
//!   cargo bench -p opendata-vector --features bench-internals \
//!     --bench centroid_cache_concurrent_read

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use vector::bench_helpers::build_centroid_cache_bench;

const NUM_CENTROIDS: u64 = 1024;
const LOOKUPS_PER_THREAD: u64 = 10_000;

fn bench_concurrent_reads(c: &mut Criterion) {
    let handle = build_centroid_cache_bench(NUM_CENTROIDS);

    let mut group = c.benchmark_group("centroid_cache_concurrent_read");
    for workers in &[1usize, 4, 8, 16, 32] {
        group.throughput(Throughput::Elements((*workers as u64) * LOOKUPS_PER_THREAD));
        group.bench_with_input(
            BenchmarkId::from_parameter(workers),
            workers,
            |b, &workers| {
                b.iter(|| {
                    let total = handle.run_concurrent_reads(workers, LOOKUPS_PER_THREAD);
                    black_box(total);
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_concurrent_reads);
criterion_main!(benches);
