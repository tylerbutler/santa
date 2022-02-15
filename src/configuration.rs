use std::{collections::HashMap, fs, path::Path};

use log::{debug, trace, warn};
use serde::{Deserialize, Serialize};

use crate::data::{KnownElves, SantaData, constants};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SantaConfig {
    pub sources: Vec<KnownElves>,
    pub packages: Vec<String>,

    #[serde(skip)]
    pub log_level: usize,
}

impl SantaConfig {
    pub fn default() -> Self {
      SantaConfig::load_from_str(constants::DEFAULT_CONFIG)
    }

    pub fn load_from_str(yaml_str: &str) -> Self {
        let data: SantaConfig = serde_yaml::from_str(&yaml_str).unwrap();
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

    /// Groups the configured (enabled) packages by elf.
    pub fn groups(self, data: &SantaData) -> HashMap<KnownElves, Vec<String>> {
        let configured_sources: Vec<KnownElves> = self.sources;
        // let s2 = self.sources.clone();
        let mut groups: HashMap<KnownElves, Vec<String>> = HashMap::new();
        for elf in configured_sources.clone() {
            groups.insert(elf, Vec::new());
        }

        for pkg in &self.packages {
            trace!("Grouping {}", pkg);
            for elf in configured_sources.clone() {
                if data.packages.contains_key(pkg) {
                    let available_sources = data.packages.get(pkg).unwrap();
                    trace!("available_sources: {:?}", available_sources);

                    if available_sources.contains_key(&elf) {
                        match groups.get_mut(&elf) {
                            Some(v) => {
                                trace!("Adding {} to {} list.", pkg, elf);
                                v.push(pkg.to_string());
                                break;
                            }
                            None => todo!(),
                        }
                    }
                }
            }
        }
        groups
    }
}
