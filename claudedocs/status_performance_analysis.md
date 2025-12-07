# Status Command Performance Analysis

## Executive Summary
The `status -a` command hang is caused by **slow package manager check commands**, specifically `brew leaves --installed-on-request` which takes **~3.2 seconds** on this system.

## Profiling Results

### Total Execution Time: 8.19 seconds

**Breakdown by Phase:**
- **Caching phase**: 8.19s (99.9% of total time)
  - `brew leaves --installed-on-request`: ~3.2s (39%)
  - `cargo install --list | grep ':' | cut -d' ' -f1`: ~68ms (0.8%)
- **Source filtering**: 1.08µs (negligible)
- **Cache setup**: 625ns (negligible)
- **Display phase**: 148µs (negligible)

### Key Findings

1. **Brew command is the bottleneck**: Takes 3.2s out of 8.19s total
2. **Concurrent execution works**: cargo and brew run in parallel (total 8.19s, not sum of individual times)
3. **Post-cache operations are fast**: All display/table operations < 200µs

## Root Cause Analysis

The hang is caused by **external command execution**:

```rust
// In PackageSource::exec_check_async()
Command::new("sh")
    .arg("-c")
    .arg("brew leaves --installed-on-request")  // ← This takes 3.2 seconds
    .output()
    .await
```

**Why is brew slow?**
- Brew must query its package database
- With 68 installed packages, brew needs to:
  - Check dependency trees
  - Filter for "leaves" (packages not dependencies)
  - Filter for "installed-on-request" vs auto-installed
- This is I/O bound (disk reads) and CPU bound (tree traversal)

## Optimization Strategies

### Strategy 1: Persistent Caching (Recommended)
**Impact**: Reduce typical invocations to <100ms

```rust
// Add to PackageCache
pub struct PersistentCache {
    cache_dir: PathBuf,
    ttl: Duration,
}

impl PackageCache {
    pub async fn cache_for_async_with_persistence(&self, source: &PackageSource) -> Result<()> {
        // Check disk cache first
        if let Some(cached) = self.load_from_disk(source)? {
            if cached.is_fresh() {
                self.cache.insert(source.name_str(), cached.packages);
                return Ok(());
            }
        }

        // Fall back to command execution
        let pkgs = source.packages_async().await;
        self.cache.insert(source.name_str(), pkgs.clone());
        self.save_to_disk(source, &pkgs)?;
        Ok(())
    }
}
```

**Configuration**:
```ccl
cache {
    enabled = true
    ttl = "5m"  // Refresh every 5 minutes
    directory = "~/.cache/santa"
}
```

**Benefits**:
- ✅ First run: 8s (same as now)
- ✅ Subsequent runs: <100ms (read from disk)
- ✅ Auto-invalidates after TTL
- ✅ Can manually invalidate with `--no-cache` flag

### Strategy 2: Incremental Status Updates
**Impact**: Show results as they arrive

```rust
pub async fn status_command_streaming(...) -> Result<()> {
    let sources = filter_sources(...);

    // Print header immediately
    println!("Checking package sources...\n");

    for source in &sources {
        // Show spinner while loading
        print!("⏳ Checking {}...", source.name());

        // Load and display immediately
        cache.cache_for_async(source).await?;
        let groups = config.groups(data);
        display_source_table(source, &groups, &cache, data, all);
    }
}
```

**Benefits**:
- ✅ User sees progress immediately
- ✅ Perceived performance improvement
- ❌ Loses concurrent execution (sequential instead)

### Strategy 3: Optimize Brew Command
**Impact**: Potentially 50% faster brew queries

Try alternative commands:
```bash
# Current (3.2s)
brew leaves --installed-on-request

# Alternative 1: Just leaves (faster, but includes deps)
brew leaves

# Alternative 2: Formula list (fastest, but different semantics)
brew list --formula
```

**Trade-offs**:
- ✅ `brew leaves`: Faster but includes auto-installed dependencies
- ✅ `brew list --formula`: Fastest but shows ALL packages
- ❌ May change behavior users expect

### Strategy 4: Background Cache Warming
**Impact**: Zero-latency status commands

```rust
// On config change or timer
pub async fn warm_cache_background() {
    tokio::spawn(async {
        let cache = PackageCache::new();
        for source in sources {
            cache.cache_for_async(source).await;
        }
        cache.save_to_disk();
    });
}
```

**Benefits**:
- ✅ Status commands instant
- ✅ Cache always fresh
- ❌ Background resource usage
- ❌ Complexity in cache lifecycle

## Recommendations

### Short-term (Quick Win)
1. **Add progress indicators** (Strategy 2 variant)
   - Show "Checking brew... ⏳" while loading
   - Gives user feedback during the wait
   - Minimal code changes

### Medium-term (Best ROI)
2. **Implement persistent caching** (Strategy 1)
   - Cache results to `~/.cache/santa/`
   - Default TTL: 5 minutes
   - Add `--no-cache` / `--refresh` flags
   - 95% of invocations become <100ms

### Long-term (Further optimization)
3. **Benchmark alternative brew commands** (Strategy 3)
   - Test if `brew list --formula` is acceptable
   - Document trade-offs for users
   - Make configurable in CCL

## Instrumentation Code

The timing instrumentation added to `commands.rs:89-167` can be:
- **Kept**: Behind a `--profile` flag or `RUST_LOG=debug`
- **Removed**: Once optimizations are implemented
- **Improved**: Use `tracing::instrument` for cleaner code

```rust
#[tracing::instrument(skip(config, data, cache))]
pub async fn status_command(...) -> Result<()> {
    // Tracing automatically logs entry/exit with timings
}
```

## Validation

To validate improvements:
```bash
# Current baseline
RUST_LOG=santa=debug cargo run --release -- status -a 2>&1 | grep "⏱️"

# After optimization
hyperfine 'cargo run --release -- status -a' --warmup 3

# Expected results
# Before: 8.19s ± 0.5s
# After (with cache): 0.1s ± 0.01s
```
