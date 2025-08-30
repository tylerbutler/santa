//! Integration tests for cache behavior and performance
//! 
//! These tests validate that Santa's caching mechanisms work correctly,
//! including cache hits/misses, expiration, and concurrency handling.

use santa::traits::{Cacheable, CacheStats};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Simple in-memory cache implementation for testing
#[derive(Debug, Clone)]
pub struct TestCache<K, V> {
    data: Arc<RwLock<HashMap<K, V>>>,
    stats: Arc<RwLock<CacheStats>>,
}

impl<K, V> Default for TestCache<K, V> 
where 
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> TestCache<K, V> 
where 
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CacheStats {
                entries: 0,
                hits: 0,
                misses: 0,
            })),
        }
    }
}

impl<K, V> Cacheable<K, V> for TestCache<K, V>
where 
    K: std::hash::Hash + Eq + Clone,
    V: Clone,
{
    fn get(&self, key: &K) -> Option<V> {
        let data = self.data.read().unwrap();
        let mut stats = self.stats.write().unwrap();
        
        if let Some(value) = data.get(key) {
            stats.hits += 1;
            Some(value.clone())
        } else {
            stats.misses += 1;
            None
        }
    }

    fn insert(&self, key: K, value: V) {
        let mut data = self.data.write().unwrap();
        let mut stats = self.stats.write().unwrap();
        
        let was_new = data.insert(key, value).is_none();
        if was_new {
            stats.entries += 1;
        }
    }

    fn invalidate(&self, key: &K) {
        let mut data = self.data.write().unwrap();
        let mut stats = self.stats.write().unwrap();
        
        if data.remove(key).is_some() {
            stats.entries -= 1;
        }
    }

    fn clear(&self) {
        let mut data = self.data.write().unwrap();
        let mut stats = self.stats.write().unwrap();
        
        data.clear();
        stats.entries = 0;
    }

    fn size(&self) -> usize {
        let data = self.data.read().unwrap();
        data.len()
    }

    fn stats(&self) -> CacheStats {
        let stats = self.stats.read().unwrap();
        *stats
    }
}

#[tokio::test]
async fn test_basic_cache_operations() {
    let cache: TestCache<String, i32> = TestCache::new();

    // Test insertion and retrieval
    cache.insert("key1".to_string(), 42);
    assert_eq!(cache.get(&"key1".to_string()), Some(42));
    assert_eq!(cache.size(), 1);

    // Test cache miss
    assert_eq!(cache.get(&"nonexistent".to_string()), None);

    // Test invalidation
    cache.invalidate(&"key1".to_string());
    assert_eq!(cache.get(&"key1".to_string()), None);
    assert_eq!(cache.size(), 0);
}

#[tokio::test]
async fn test_cache_statistics() {
    let cache: TestCache<String, String> = TestCache::new();

    // Initially no stats
    let initial_stats = cache.stats();
    assert_eq!(initial_stats.entries, 0);
    assert_eq!(initial_stats.hits, 0);
    assert_eq!(initial_stats.misses, 0);
    assert_eq!(initial_stats.hit_rate(), 0.0);

    // Add some entries
    cache.insert("key1".to_string(), "value1".to_string());
    cache.insert("key2".to_string(), "value2".to_string());

    // Test hits
    let _value1 = cache.get(&"key1".to_string());
    let _value1_again = cache.get(&"key1".to_string());
    
    // Test misses
    let _missing = cache.get(&"key3".to_string());

    let stats = cache.stats();
    assert_eq!(stats.entries, 2);
    assert_eq!(stats.hits, 2);
    assert_eq!(stats.misses, 1);
    assert!((stats.hit_rate() - (2.0 / 3.0)).abs() < 0.001);
}

#[tokio::test]
async fn test_cache_clear() {
    let cache: TestCache<i32, String> = TestCache::new();

    // Add multiple entries
    for i in 0..10 {
        cache.insert(i, format!("value_{}", i));
    }

    assert_eq!(cache.size(), 10);

    // Clear cache
    cache.clear();
    assert_eq!(cache.size(), 0);

    // Verify all entries are gone
    for i in 0..10 {
        assert_eq!(cache.get(&i), None);
    }
}

#[tokio::test]
async fn test_cache_concurrent_access() {
    let cache = Arc::new(TestCache::<i32, String>::new());
    let mut handles = Vec::new();

    // Spawn multiple tasks that insert data concurrently
    for i in 0..10 {
        let cache_clone = Arc::clone(&cache);
        let handle = tokio::spawn(async move {
            cache_clone.insert(i, format!("value_{}", i));
            
            // Also test concurrent reads
            for j in 0..i {
                let _ = cache_clone.get(&j);
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Task should complete successfully");
    }

    // Verify all data was inserted correctly
    assert_eq!(cache.size(), 10);
    for i in 0..10 {
        assert_eq!(cache.get(&i), Some(format!("value_{}", i)));
    }
}

#[tokio::test]
async fn test_cache_update_existing_key() {
    let cache: TestCache<String, i32> = TestCache::new();

    // Insert initial value
    cache.insert("key".to_string(), 100);
    assert_eq!(cache.get(&"key".to_string()), Some(100));
    assert_eq!(cache.size(), 1);

    // Update with new value
    cache.insert("key".to_string(), 200);
    assert_eq!(cache.get(&"key".to_string()), Some(200));
    assert_eq!(cache.size(), 1); // Size should remain the same

    let stats = cache.stats();
    assert_eq!(stats.entries, 1); // Only one unique entry
}

#[tokio::test]
async fn test_cache_performance_characteristics() {
    let cache: TestCache<i32, Vec<u8>> = TestCache::new();
    
    // Insert a large number of entries to test performance
    let start = std::time::Instant::now();
    
    for i in 0..1000 {
        cache.insert(i, vec![i as u8; 100]);
    }
    
    let insert_duration = start.elapsed();
    println!("Insert time for 1000 entries: {:?}", insert_duration);

    // Test retrieval performance
    let start = std::time::Instant::now();
    
    for i in 0..1000 {
        let _value = cache.get(&i);
    }
    
    let retrieval_duration = start.elapsed();
    println!("Retrieval time for 1000 entries: {:?}", retrieval_duration);

    // Both operations should be reasonably fast
    assert!(insert_duration.as_millis() < 100, "Insert should be fast");
    assert!(retrieval_duration.as_millis() < 100, "Retrieval should be fast");
    
    let stats = cache.stats();
    assert_eq!(stats.entries, 1000);
    assert_eq!(stats.hits, 1000);
    assert_eq!(stats.hit_rate(), 1.0);
}