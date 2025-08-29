//! Performance benchmarks comparing sync vs async subprocess operations
//!
//! This benchmark suite measures:
//! - Sequential vs concurrent package manager queries
//! - Sync vs async subprocess execution performance  
//! - Timeout handling overhead
//! - Memory usage during concurrent operations

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use santa::{
    data::KnownSources,
    sources::{PackageCache, PackageSource},
};
use std::time::Duration;
use tokio::runtime::Runtime;

/// Create a mock package source for benchmarking
fn create_mock_source(name: &str, command: &str) -> PackageSource {
    PackageSource::new_for_test(
        KnownSources::Unknown(name.to_string()),
        "📦",
        "echo", // Use echo for fast, predictable commands
        command,
        command,
        None,
        None,
    )
}

/// Create multiple mock sources for concurrent testing
fn create_mock_sources(count: usize) -> Vec<PackageSource> {
    (0..count)
        .map(|i| {
            create_mock_source(
                &format!("mock{}", i),
                &format!("package{}\npackage{}_2\npackage{}_3", i, i, i),
            )
        })
        .collect()
}

/// Benchmark sync package cache operations (sequential)
fn bench_sync_cache_operations(c: &mut Criterion) {
    let sources = create_mock_sources(5);

    c.bench_function("sync_cache_sequential", |b| {
        b.iter(|| {
            let mut cache = PackageCache::new();
            for source in &sources {
                cache.cache_for(black_box(source));
            }
            cache
        })
    });
}

/// Benchmark async package cache operations (concurrent)
fn bench_async_cache_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let sources = create_mock_sources(5);

    c.bench_function("async_cache_concurrent", |b| {
        b.to_async(&rt).iter(|| async {
            let mut cache = PackageCache::new();
            // Simulate concurrent caching (though limited by &mut self)
            for source in &sources {
                cache.cache_for_async(black_box(source)).await.unwrap();
            }
            cache
        })
    });
}

/// Benchmark sync vs async with varying number of package managers
fn bench_scalability(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    for size in [1, 3, 5, 10].iter() {
        let sources = create_mock_sources(*size);

        // Sync benchmark
        c.bench_with_input(
            BenchmarkId::new("sync_packages", size),
            size,
            |b, &_size| {
                b.iter(|| {
                    let mut cache = PackageCache::new();
                    for source in &sources {
                        let _packages = source.packages();
                        // Simulate processing time
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    cache
                })
            },
        );

        // Async benchmark
        c.bench_with_input(
            BenchmarkId::new("async_packages", size),
            size,
            |b, &_size| {
                b.to_async(&rt).iter(|| async {
                    let tasks: Vec<_> = sources
                        .iter()
                        .map(|source| async move {
                            let _packages = source.packages_async().await;
                            // Simulate processing time
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        })
                        .collect();

                    futures::future::join_all(tasks).await;
                })
            },
        );
    }
}

/// Benchmark timeout behavior
fn bench_timeout_overhead(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Fast command (no timeout needed)
    let fast_source = create_mock_source("fast", "package1\npackage2");

    // Slow command (will timeout) - simulate with sleep
    let slow_source = PackageSource::new_for_test(
        KnownSources::Unknown("slow".to_string()),
        "🐌",
        "sleep",
        "sleep 2 && echo package1", // 2 second delay
        "sleep 2 && echo package1",
        None,
        None,
    );

    c.bench_function("async_with_timeout_fast", |b| {
        b.to_async(&rt)
            .iter(|| async { black_box(fast_source.packages_async().await) })
    });

    c.bench_function("async_with_timeout_slow", |b| {
        b.to_async(&rt).iter(|| async {
            // This will timeout after 30 seconds, but we'll measure the overhead
            black_box(slow_source.packages_async().await)
        })
    });

    // Compare with sync version (no timeout protection)
    c.bench_function("sync_no_timeout_fast", |b| {
        b.iter(|| black_box(fast_source.packages()))
    });
}

/// Memory usage benchmark for concurrent operations
fn bench_memory_usage(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let sources = create_mock_sources(20); // More sources to test memory

    c.bench_function("memory_sync_sequential", |b| {
        b.iter(|| {
            let mut results = Vec::new();
            for source in &sources {
                let packages = source.packages();
                results.push(packages);
            }
            black_box(results)
        })
    });

    c.bench_function("memory_async_concurrent", |b| {
        b.to_async(&rt).iter(|| async {
            let tasks: Vec<_> = sources
                .iter()
                .map(|source| async move { source.packages_async().await })
                .collect();

            let results = futures::future::join_all(tasks).await;
            black_box(results)
        })
    });
}

/// Real-world simulation benchmark
fn bench_realistic_scenario(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    // Simulate real package managers with realistic output
    let realistic_sources = vec![
        PackageSource::new_for_test(
            KnownSources::Brew,
            "🍺",
            "echo",
            "git\ncurl\nvim\nnode\nnpm\nrust\ncargo\nwget\ntree\njq", // 10 packages
            "echo installed",
            None,
            None,
        ),
        PackageSource::new_for_test(
            KnownSources::Apt,
            "📦",
            "echo",
            "git\ncurl\nvim-common\nnodejs\nwget\ntree\njq\nbuild-essential", // 8 packages
            "echo installed",
            None,
            None,
        ),
        PackageSource::new_for_test(
            KnownSources::Cargo,
            "📦",
            "echo",
            "serde\ntokio\nclap\nanyhow\ntracing", // 5 packages
            "echo installed",
            None,
            None,
        ),
    ];

    c.bench_function("realistic_sync_sequential", |b| {
        b.iter(|| {
            let mut cache = PackageCache::new();
            for source in &realistic_sources {
                cache.cache_for(black_box(source));
                // Simulate network/disk latency
                std::thread::sleep(Duration::from_millis(50));
            }
            cache
        })
    });

    c.bench_function("realistic_async_concurrent", |b| {
        b.to_async(&rt).iter(|| async {
            let mut cache = PackageCache::new();
            // Simulate concurrent operations with realistic delays
            let tasks: Vec<_> = realistic_sources
                .iter()
                .map(|source| async move {
                    let result = source.packages_async().await;
                    // Simulate network/disk latency
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    result
                })
                .collect();

            let _results = futures::future::join_all(tasks).await;
            cache
        })
    });
}

criterion_group!(
    benches,
    bench_sync_cache_operations,
    bench_async_cache_operations,
    bench_scalability,
    bench_timeout_overhead,
    bench_memory_usage,
    bench_realistic_scenario
);
criterion_main!(benches);
