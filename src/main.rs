#![allow(unused)]
#[macro_use]
// extern crate clap_verbosity_flag;
use anyhow::bail;
use clap::{AppSettings, Parser, Subcommand};
use config::{Config, File, FileSourceFile, Value};
use configuration::SantaConfig;
use log::{debug, info, trace, warn, LevelFilter};
use simplelog::{TermLogger, TerminalMode};
use std::collections::HashSet;
use std::sync::RwLock;
use std::{env, fmt};
extern crate directories;
use console::style;
use directories::BaseDirs;
extern crate lazy_static;
use lazy_static::lazy_static;

use std::path::{Path, PathBuf};

use crate::commands::*;
use crate::data::SantaData;
use crate::elves::PackageCache;
use crate::traits::Exportable;

mod commands;
mod configuration;
mod data;
mod elves;
mod traits;

// static CONFIG: Config = ;

// lazy_static! {
//   // let Some(CONFIG_PATH) = BaseDirs::new();
//   static ref SETTINGS: RwLock<Config> = RwLock::new(Config::new());
// }

static DEFAULT_CONFIG_FILE_PATH: &str = ".config/santa/config.yaml";

/// Manage default sets of packages for a variety of package managers.
#[derive(Parser)]
#[clap(author, version = clap::crate_version!(), max_term_width = 100, about)]
// #[clap(global_setting(AppSettings::PropagateVersion))]
struct Cli {
    #[clap(subcommand)]
    command: Commands,

    /// Load ONLY the default config
    #[clap(long, global = true)]
    builtin_config: bool,

    /// Increase logging level
    #[clap(short, long, global = true, parse(from_occurrences))]
    verbose: usize,
}

#[derive(Subcommand)]
enum Commands {
    /// Prints the status of santa packages
    Status {
        /// List all packages, not just missing ones
        #[clap(short, long)]
        all: bool,
    },
    /// Installs packages
    Install { elf: Option<String> },
    /// Asks an elf to add a package to its tracking list
    Add {
        package: Option<String>,
        elf: Option<String>,
    },
    Config {
        /// Show full config
        #[clap(short, long)]
        full: bool,

        // #[clap(short, long)]
        // packages: bool,

        #[clap(short, long)]
        local: bool,

        #[clap(short, long)]
        pipe: bool,
    },
}

fn load_config(path: &Path) -> SantaConfig {
    let dir = BaseDirs::new().unwrap();
    let home_dir = dir.home_dir();
    let config_file = home_dir.join(path);
    let config = SantaConfig::load_from(&config_file);
    trace!("{:?}", config);
    config
}

pub fn run() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    let mut log_level = LevelFilter::Info;

    match &cli.verbose {
        1 => log_level = LevelFilter::Info,
        2 => log_level = LevelFilter::Debug,
        3 => log_level = LevelFilter::Trace,
        _ => log_level = LevelFilter::Off,
    }

    TermLogger::init(
        log_level,
        simplelog::Config::default(),
        TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    );

    debug!("Argument parsing complete.");
    let data = SantaData::default();
    let d = data.export();
    // trace!("data: {}", d);

    let mut config = if cli.builtin_config {
        info!("loading built-in config because of CLI flag.");
        SantaConfig::default()
    } else {
        load_config(Path::new(DEFAULT_CONFIG_FILE_PATH))
    };
    config.log_level = cli.verbose;

    let mut data = data; // re-declare variable to make it mutable
    // data.update_from_config(&config);

    let mut cache: PackageCache = PackageCache::new();

    match &cli.command {
        Commands::Status { all } => {
            debug!("santa status");
            commands::status_command(&config, &data, cache, all);
        }
        Commands::Install { elf } => {
            println!("NYI: santa install {:?}", elf);
            todo!();
        }
        Commands::Add { elf, package } => {
            println!("NYI: santa add {:?} {:?}", elf, package);
            todo!();
        }
        Commands::Config { full, local, pipe } => {
            commands::config_command(&config, &data, &full, &local, &pipe);
        }
    }

    Ok(())
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(err) => {
            eprintln!("error: {}", err);
            std::process::exit(1);
        }
    }
}
