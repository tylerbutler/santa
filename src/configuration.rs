use crate::data::SourceList;
use crate::sources::PackageSource;
use crate::Exportable;
use std::{collections::HashMap, fs, path::Path};
use anyhow::Context;

use tracing::{debug, trace, warn};
// use memoize::memoize;
use serde::{Deserialize, Serialize};

use crate::data::{constants, KnownSources, SantaData};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SantaConfig {
    pub sources: Vec<KnownSources>,
    pub packages: Vec<String>,
    pub custom_sources: Option<SourceList>,

    #[serde(skip)]
    _groups: Option<HashMap<KnownSources, Vec<String>>>,
    #[serde(skip)]
    pub log_level: u8,
}

impl Default for SantaConfig {
    fn default() -> Self {
        SantaConfig::load_from_str(constants::DEFAULT_CONFIG)
            .expect("Failed to load default config - this should never fail")
    }
}

impl Exportable for SantaConfig {}

impl SantaConfig {
    pub fn load_from_str(yaml_str: &str) -> Result<Self, anyhow::Error> {
        let data: SantaConfig = serde_yaml::from_str(yaml_str)
            .with_context(|| format!("Failed to parse config from YAML: {}", yaml_str))?;
        Ok(data)
    }

    pub fn load_from(file: &Path) -> Result<Self, anyhow::Error> {
        debug!("Loading config from: {}", file.display());
        if file.exists() {
            let yaml_str = fs::read_to_string(file)
                .with_context(|| format!("Failed to read config file: {}", file.display()))?;
            SantaConfig::load_from_str(&yaml_str)
        } else {
            warn!("Can't find config file: {}", file.display());
            warn!("Loading default config");
            Ok(SantaConfig::default())
        }
    }

    pub fn source_is_enabled(&self, source: &PackageSource) -> bool {
        trace!("Checking if {} is enabled", source);
        return self.sources.contains(&source.name);
    }

    /// Groups the configured (enabled) packages by source.
    pub fn groups(&mut self, data: &SantaData) -> HashMap<KnownSources, Vec<String>> {
        match &self._groups {
            Some(groups) => groups.clone(),
            None => {
                let configured_sources: Vec<KnownSources> = self.sources.clone();
                // let s2 = self.sources.clone();
                let mut groups: HashMap<KnownSources, Vec<String>> = HashMap::new();
                for source in configured_sources.clone() {
                    groups.insert(source, Vec::new());
                }

                for pkg in &self.packages {
                    for source in configured_sources.clone() {
                        if data.packages.contains_key(pkg) {
                            let available_sources = data.packages.get(pkg).expect("Package should exist in data");
                            trace!("available_sources: {:?}", available_sources);

                            if available_sources.contains_key(&source) {
                                trace!("Adding {} to {} list.", pkg, source);
                                match groups.get_mut(&source) {
                                    Some(v) => {
                                        // trace!("Adding {} to {} list.", pkg, source);
                                        v.push(pkg.to_string());
                                        break;
                                    }
                                    None => {
                                        warn!("Group for source {} not found, creating new group", source);
                                        groups.insert(source, vec![pkg.to_string()]);
                                    }
                                }
                            }
                        }
                    }
                }
                self._groups = Some(groups);
                self._groups.clone().expect("Groups should be populated")
            }
        }
    }
}
