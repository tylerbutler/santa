use crate::data::SourceList;
use crate::sources::PackageSource;
use crate::Exportable;
use std::{collections::HashMap, fs, path::Path};

use log::{debug, trace, warn};
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
    }
}

impl Exportable for SantaConfig {}

impl SantaConfig {
    pub fn load_from_str(yaml_str: &str) -> Self {
        let data: SantaConfig = serde_yaml::from_str(yaml_str).unwrap();
        data
    }

    pub fn load_from(file: &Path) -> Self {
        debug!("Loading config from: {}", file.display());
        let mut yaml_str: String;
        if file.exists() {
            yaml_str = fs::read_to_string(file).unwrap();
            SantaConfig::load_from_str(&yaml_str)
        } else {
            warn!("Can't find config file: {}", file.display());
            warn!("Loading default config");
            SantaConfig::default()
        }
    }

    pub fn source_is_enabled(self, source: &PackageSource) -> bool {
        trace!("Checking if {} is enabled", source);
        return self.sources.contains(&source.name);
    }

    /// Groups the configured (enabled) packages by source.
    pub fn groups(mut self, data: &SantaData) -> HashMap<KnownSources, Vec<String>> {
        match &self._groups {
            Some(groups) => groups.clone(),
            None => {
                let configured_sources: Vec<KnownSources> = self.sources;
                // let s2 = self.sources.clone();
                let mut groups: HashMap<KnownSources, Vec<String>> = HashMap::new();
                for source in configured_sources.clone() {
                    groups.insert(source, Vec::new());
                }

                for pkg in &self.packages {
                    for source in configured_sources.clone() {
                        if data.packages.contains_key(pkg) {
                            let available_sources = data.packages.get(pkg).unwrap();
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
                                        todo!();
                                    }
                                }
                            }
                        }
                    }
                }
                self._groups = Some(groups);
                self._groups.unwrap()
            }
        }
    }
}
