use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

extern crate yaml_rust;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use yaml_rust::{Yaml, YamlEmitter, YamlLoader};

use crate::elves::Elf;

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(untagged)]
pub enum ElfOrBool {
  KnownElves,
  Boolean(bool),
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum KnownElves {
    Apt,
    Aur,
    Brew,
    Cargo,
    Pacman,
    Scoop,
}

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PackageData {
    pub name: Option<String>,
    pub pre: Option<String>,
    pub post: Option<String>,
    pub elves: Option<Vec<String>>,
}

impl PackageData {
    pub fn new(name: &str) -> Self {
        PackageData {
            name: Some(name.to_string()),
            pre: None,
            post: None,
            elves: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Package {
    pub name: String,
    // pub data: Option<PackageData>,
    pub elves: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SantaData {
    pub packages: HashMap<String, HashMap<KnownElves, Option<PackageData>>>,
    // pub elf_settings: HashMap<KnownElves, PackageData>,
}

impl SantaData {
    pub fn default() -> Self {
        let brew_scoop_pacman: HashMap<KnownElves, Option<PackageData>> = vec![
            (KnownElves::Brew, None),
            (KnownElves::Scoop, None),
            (KnownElves::Pacman, None),
        ]
        .into_iter()
        .collect();
        let apt_brew_scoop_pacman: HashMap<KnownElves, Option<PackageData>> = vec![
            (KnownElves::Apt, None),
            (KnownElves::Scoop, None),
            (KnownElves::Pacman, None),
            (KnownElves::Brew, None),
        ]
        .into_iter()
        .collect();
        let mut pkgs: HashMap<String, HashMap<KnownElves, Option<PackageData>>> = HashMap::new();
        let mut settings: HashMap<KnownElves, PackageData> = HashMap::new();
        pkgs.insert("bat".to_string(), brew_scoop_pacman.clone());
        pkgs.insert("bottom".to_string(), brew_scoop_pacman.clone());
        pkgs.insert("chezmoi".to_string(), brew_scoop_pacman.clone());
        pkgs.insert("direnv".to_string(), brew_scoop_pacman.clone());
        pkgs.insert("dust".to_string(), brew_scoop_pacman.clone());
        pkgs.insert("exa".to_string(), brew_scoop_pacman.clone());
        pkgs.insert("fd".to_string(), brew_scoop_pacman.clone());
        pkgs.insert(
            "fnm".to_string(),
            vec![(KnownElves::Brew, None), (KnownElves::Scoop, None)].into_iter().collect(),
        );
        pkgs.insert("fzf".to_string(), brew_scoop_pacman.clone());
        pkgs.insert(
            "github-cli".to_string(),
            vec![
                (KnownElves::Apt, None),
                (KnownElves::Scoop, None),
                (KnownElves::Pacman, None),
                (KnownElves::Brew, Some(PackageData::new("gh"))),
            ].into_iter().collect(),
        );
        pkgs.insert(
            "ghq".to_string(),
            vec![(KnownElves::Scoop, None), (KnownElves::Brew, None)].into_iter().collect(),
        );
        // pkgs.insert(
        //     "git-delta".to_string(),
        //     vec![
        //         (KnownElves::Scoop, Some(PackageData::new("delta"))),
        //         (KnownElves::Pacman, None),
        //         (KnownElves::Brew, None),
        //     ],
        // );
        // pkgs.insert(
        //     "gitui".to_string(),
        //     vec![
        //         (KnownElves::Brew, None),
        //         (KnownElves::Scoop, None),
        //         (KnownElves::Pacman, None),
        //     ],
        // );
        // pkgs.insert(
        //     "go".to_string(),
        //     vec![
        //         (KnownElves::Apt, None),
        //         (KnownElves::Scoop, None),
        //         (KnownElves::Pacman, None),
        //         (KnownElves::Brew, Some(PackageData::new("golang"))),
        //     ],
        // );
        // pkgs.insert("grex".to_string(), brew_scoop_pacman.to_vec());
        // pkgs.insert("hub".to_string(), brew_scoop_pacman.to_vec());
        // pkgs.insert("jq".to_string(), brew_scoop_pacman.to_vec());
        // pkgs.insert("lsd".to_string(), brew_scoop_pacman.to_vec());
        // pkgs.insert("mc".to_string(), apt_brew_scoop_pacman.to_vec());
        // pkgs.insert("micro".to_string(), brew_scoop_pacman.to_vec());
        // pkgs.insert("nnn".to_string(), brew_scoop_pacman.to_vec());
        // pkgs.insert("oh-my-posh".to_string(), brew_scoop_pacman.to_vec());
        // pkgs.insert("pueue".to_string(), brew_scoop_pacman.to_vec());
        // pkgs.insert("procs".to_string(), brew_scoop_pacman.to_vec());
        // pkgs.insert("pyenv".to_string(), brew_scoop_pacman.to_vec());
        // pkgs.insert("python".to_string(), apt_brew_scoop_pacman.to_vec());
        // pkgs.insert(
        //     "ripgrep".to_string(),
        //     vec![
        //         (KnownElves::Apt, None),
        //         (KnownElves::Scoop, None),
        //         (KnownElves::Pacman, None),
        //         (KnownElves::Brew, Some(PackageData::new("rg"))),
        //     ],
        // );
        // pkgs.insert("rustup".to_string(), brew_scoop_pacman.to_vec());
        // pkgs.insert("sd".to_string(), brew_scoop_pacman.to_vec());
        // pkgs.insert("tealdeer".to_string(), brew_scoop_pacman.to_vec());
        // pkgs.insert("thefuck".to_string(), apt_brew_scoop_pacman.to_vec());
        // pkgs.insert("tmux".to_string(), apt_brew_scoop_pacman.to_vec());
        // pkgs.insert("viu".to_string(), brew_scoop_pacman.to_vec());
        // pkgs.insert("when-cli".to_string(), vec![(KnownElves::Cargo, None)]);
        // pkgs.insert("zellij".to_string(), brew_scoop_pacman.to_vec());
        // pkgs.insert("zoxide".to_string(), brew_scoop_pacman.to_vec());
        // pkgs.insert("act".to_string(), vec![(KnownElves::Scoop, None)]);
        // pkgs.insert("aria2".to_string(), vec![(KnownElves::Scoop, None)]);
        // pkgs.insert("emplace".to_string(), vec![(KnownElves::Scoop, None)]);
        // pkgs.insert("findutils".to_string(), vec![(KnownElves::Scoop, None)]);
        // pkgs.insert("gsudo".to_string(), vec![(KnownElves::Scoop, None)]);
        // pkgs.insert("ln".to_string(), vec![(KnownElves::Scoop, None)]);
        // pkgs.insert("nircmd".to_string(), vec![(KnownElves::Scoop, None)]);
        // pkgs.insert("nvs".to_string(), vec![(KnownElves::Scoop, None)]);
        // pkgs.insert(
        //     "tre-command".to_string(),
        //     vec![(KnownElves::Brew, None), (KnownElves::Scoop, None)],
        // );
        // pkgs.insert("wget".to_string(), apt_brew_scoop_pacman.to_vec());
        // pkgs.insert("which".to_string(), vec![(KnownElves::Scoop, None)]);

        SantaData {
            packages: pkgs,
            // elf_settings: HashMap::new(),
        }
    }

    pub fn load_from(file: &str) -> Self {
        debug!("Loading");
        let yaml_str = fs::read_to_string(file).unwrap();
        let data: SantaData = serde_yaml::from_str(&yaml_str).unwrap();
        data
        // let docs = YamlLoader::load_from_str(yaml_str.as_str()).unwrap();

        // // Select the first document
        // let doc = &docs[0]; //.as_hash().expect("Failed to parse YAML as HashMap.");

        // if let Yaml::Hash(ref h) = *doc {
        //     for (k, v) in h {
        //         println!("{:?}", k);
        //     }
        // } else {
        //     error!("Failed to parse YAML.");
        // }

        // eprintln!("{:?}", doc);

        // let pkgs = &doc["packages"];
        // let mut list: HashMap<String, Vec<KnownElves>> = HashMap::new();
        // for pkg in pkgs {
        //     debug!("Packages for loop");
        //     let name = pkg.as_str().unwrap();
        //     list.push(name.to_string());
        // }

        // SantaData::default()

        // println!("{:?}", doc);
    }

    pub fn export(&self) {
        let serialized = serde_yaml::to_string(&self).unwrap();
        println!("serialized = {}", serialized);
    }
}
