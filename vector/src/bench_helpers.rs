//! Helpers for criterion benches in `vector/benches/`. Compiled only
//! under the `bench-internals` cargo feature so they cost nothing in
//! the regular build.
//!
//! Exposes a minimal task-level API rather than crate-private types,
//! so benches don't have to widen `pub(crate)` visibility on internals
//! like `AllCentroidsCache` or `VectorId`.

use crate::serde::vector_id::VectorId;
use crate::write::indexer::tree::centroids::{
    AllCentroidsCache, AllCentroidsCacheWriter, CentroidCache,
};
use crate::write::indexer::tree::posting_list::{Posting, PostingList};
use std::sync::Arc;

/// Opaque handle around a populated `AllCentroidsCache`. Built by
/// `build_centroid_cache_bench`. The only operation exposed is
/// `run_concurrent_reads`, which spawns N reader threads and counts
/// total `posting()` lookups completed.
pub struct CentroidCacheBenchHandle {
    cache: Arc<AllCentroidsCache>,
    num_centroids: u64,
}

impl CentroidCacheBenchHandle {
    /// Spawn `workers` threads, each issuing `lookups_per_thread`
    /// `posting()` lookups against random centroid ids in the cache.
    /// Returns the total number of completed lookups (should equal
    /// `workers * lookups_per_thread` on success — any short return
    /// indicates a missing centroid, which would be a fixture bug).
    pub fn run_concurrent_reads(&self, workers: usize, lookups_per_thread: u64) -> u64 {
        let mut handles = Vec::with_capacity(workers);
        for w in 0..workers {
            let cache = Arc::clone(&self.cache);
            let num_centroids = self.num_centroids;
            handles.push(std::thread::spawn(move || {
                let mut local = 0u64;
                for q in 0..lookups_per_thread {
                    let idx = (w as u64 * lookups_per_thread + q) % num_centroids;
                    let id = VectorId::centroid_id(2, 100 + idx);
                    if cache.posting(id, u64::MAX).is_some() {
                        local += 1;
                    }
                }
                local
            }));
        }
        let mut total = 0u64;
        for h in handles {
            total += h.join().expect("reader task should not panic");
        }
        total
    }
}

/// Build a `CentroidCacheBenchHandle` populated with `num_centroids`
/// posting lists at level 2. Each posting list contains a single inner
/// posting with a small fixed vector. Suitable for read-throughput
/// benchmarks; the actual vector contents are not exercised.
pub fn build_centroid_cache_bench(num_centroids: u64) -> CentroidCacheBenchHandle {
    let mut postings = Vec::with_capacity(num_centroids as usize);
    for i in 0..num_centroids {
        let centroid_id = VectorId::centroid_id(2, 100 + i);
        let inner_id = VectorId::centroid_id(1, i + 1);
        let plist: PostingList =
            std::iter::once(Posting::from_vec(inner_id, vec![i as f32, 0.0])).collect();
        postings.push((centroid_id, Arc::new(plist)));
    }
    let writer = AllCentroidsCacheWriter::new(Arc::new(PostingList::empty()), postings);
    CentroidCacheBenchHandle {
        cache: Arc::new(writer.cache()),
        num_centroids,
    }
}
