use crate::data::SantaData;
use crate::data::SourceList;
use crate::errors::{Result, SantaError};
use crate::traits::Exportable;
use crate::{configuration::SantaConfig, sources::PackageCache};
use futures::future::try_join_all;
use std::sync::Arc;
use tokio::sync::RwLock;

use tracing::debug;

#[cfg(test)]
mod tests;

pub async fn status_command(
    config: &mut SantaConfig,
    data: &SantaData,
    cache: PackageCache,
    all: &bool,
) -> Result<()> {
    // filter sources to those enabled in the config
    let sources: SourceList = data
        .sources
        .clone()
        .into_iter()
        .filter(|source| config.source_is_enabled(source))
        .collect();

    // Use structured concurrency to cache data for all sources concurrently
    let cache = Arc::new(RwLock::new(cache));
    let cache_tasks: Vec<_> = sources
        .iter()
        .map(|source| {
            let cache_clone: Arc<RwLock<PackageCache>> = Arc::clone(&cache);
            let source = source.clone();
            async move {
                let cache = cache_clone.write().await;
                cache.cache_for_async(&source).await
            }
        })
        .collect();

    // All tasks are structured under this scope - they'll be awaited together
    match try_join_all(cache_tasks).await {
        Ok(_) => debug!("Successfully cached data for all sources"),
        Err(e) => tracing::error!("Some cache operations failed: {}", e),
    }

    // Extract cache from Arc<Mutex<>> for further use
    let cache = Arc::try_unwrap(cache)
        .map_err(|_| SantaError::Concurrency("Failed to unwrap cache - still in use".to_string()))?
        .into_inner();
    for source in &sources {
        let groups = config.groups(data);
        for (key, pkgs) in groups {
            if source.name() == &key {
                let pkg_count = pkgs.len();
                let table = format!("{}", source.table(&pkgs, &cache, *all));
                println!("{source} ({pkg_count} packages total)");
                println!("{table}");
                break;
            }
        }
    }
    Ok(())
}

pub fn config_command(
    config: &SantaConfig,
    data: &SantaData,
    packages: bool,
    builtin: bool,
) -> Result<()> {
    if !builtin {
        println!("{}", config.export());
    } else if packages {
        println!("{}", data.export());
    } else {
        println!("{}", data.sources.export())
    }
    Ok(())
}

pub async fn install_command(
    config: &mut SantaConfig,
    data: &SantaData,
    cache: PackageCache,
) -> Result<()> {
    // let config = config.clone();
    // filter sources to those enabled in the config
    let sources: SourceList = data
        .sources
        .clone()
        .into_iter()
        .filter(|source| config.source_is_enabled(source))
        .collect();

    // for (k, v) in config.groups(&data) {
    //     error!("{} {:?}", k, v);
    // }

    // Use structured concurrency to cache data for all sources concurrently
    let cache = Arc::new(RwLock::new(cache));
    let cache_tasks: Vec<_> = sources
        .iter()
        .map(|source| {
            let cache_clone: Arc<RwLock<PackageCache>> = Arc::clone(&cache);
            let source = source.clone();
            async move {
                debug!("Async stats for {}", source.name());
                let cache = cache_clone.write().await;
                cache.cache_for_async(&source).await
            }
        })
        .collect();

    // All caching tasks are structured under this scope
    match try_join_all(cache_tasks).await {
        Ok(_) => debug!("Successfully cached data for install operation"),
        Err(e) => tracing::error!("Some install cache operations failed: {}", e),
    }

    // Extract cache from Arc<Mutex<>> for further use
    let cache = Arc::try_unwrap(cache)
        .map_err(|_| SantaError::Concurrency("Failed to unwrap install cache - still in use".to_string()))?
        .into_inner();

    // let config = config.clone();
    for source in &sources {
        let groups = config.groups(data);
        for (key, pkgs) in groups {
            if source.name() == &key {
                let pkgs: Vec<String> = pkgs
                    .iter()
                    .filter(|p| !cache.check(source, p))
                    .map(|p| p.to_string())
                    .collect();
                source.exec_install(config, data, pkgs);
            }
        }
    }
    Ok(())
}
