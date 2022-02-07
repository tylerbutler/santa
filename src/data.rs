use std::{
  fs,
  path::{Path, PathBuf}, collections::{HashSet, HashMap},
};

extern crate yaml_rust;
use log::{debug, error};
use serde::{Serialize, Deserialize};
use yaml_rust::{Yaml, YamlEmitter, YamlLoader};

use crate::elves::Elf;

#[derive(Serialize, Deserialize, Debug)]
pub enum OS {
  Macos,
  Linux,
  Windows,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Arch {
  X64,
  Aarch64,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Distro {
  None,
  ArchLinux,
  Ubuntu,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Platform {
  os: OS,
  arch: Arch,
  distro: Distro,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Package {
  pub name: String,
  pub data: Option<PackageData>,
  pub elves: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PackageData {
  pub name: String,
  pub pre: Option<String>,
  pub post: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SantaData {
  pub packages: HashMap<String, PackageData>,
}

impl SantaData {
  pub fn load_from(file: &str) -> Self {
      let yaml_str = fs::read_to_string(file).unwrap();
      let docs = YamlLoader::load_from_str(yaml_str.as_str()).unwrap();

      // Select the first document
      let doc = &docs[0]; //.as_hash().expect("Failed to parse YAML as HashMap.");

      if let Yaml::Hash(ref h) = *doc {
          for (k, v) in h {
              println!("{:?}", k);
          }
      } else {
          error!("Failed to parse YAML.");
      }

      eprintln!("{:?}", doc);

      let pkgs = &doc["packages"];
      let mut list: HashMap<String, PackageData> = HashMap::new();
      // for pkg in pkgs {
      //     debug!("Packages for loop");
      //     let name = pkg.as_str().unwrap();
      //     list.push(name.to_string());
      // }

      SantaData { packages: list }

      // println!("{:?}", doc);
  }
}
