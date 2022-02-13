#![allow(unused)]
#[macro_use]
// extern crate clap_verbosity_flag;
use anyhow::bail;
use clap::{AppSettings, Parser, Subcommand};
use config::{Config, File, FileSourceFile, Value};
use log::{debug, info, warn, LevelFilter, trace};
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
use crate::data::{SantaData, SantaConfig};
use crate::elves::PackageCache;
use crate::traits::Exportable;

mod commands;
mod data;
mod elves;
mod traits;

// static CONFIG: Config = ;

lazy_static! {
  // let Some(CONFIG_PATH) = BaseDirs::new();
  static ref SETTINGS: RwLock<Config> = RwLock::new(Config::new());
}

/// Manage default sets of packages for a variety of package managers.
#[derive(Parser)]
#[clap(author, version = clap::crate_version!(), max_term_width = 100, about)]
// #[clap(global_setting(AppSettings::PropagateVersion))]
#[clap(global_setting(AppSettings::UseLongFormatForHelpSubcommand))]
struct Cli {
    /// The pattern to look for
    // #[clap(long)]
    // pattern: String,

    // /// The path to the file to read
    // #[clap(short, long, parse(from_os_str))]
    // path: Option<PathBuf>,

    #[clap(subcommand)]
    command: Commands,

    /// Increase logging level
    #[clap(short, long, global = true, parse(from_occurrences))]
    verbose: usize,
}

#[derive(Subcommand)]
enum Commands {
    /// Prints the status of santa packages
    Status,
    /// Installs packages
    Install { elf: Option<String> },
    /// Asks an elf to add a package to its tracking list
    Add {
        package: Option<String>,
        elf: Option<String>,
    },
}

pub fn run() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    let mut log_level = LevelFilter::Info;

    match &cli.verbose {
        1 => log_level = LevelFilter::Info,
        2 => log_level = LevelFilter::Debug,
        _ => log_level = LevelFilter::Off,
    }

    TermLogger::init(
        log_level,
        simplelog::Config::default(),
        TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    );

    debug!("Argument parsing complete.");
    let dir = BaseDirs::new().unwrap();
    let home_dir = dir.home_dir();
    let config_file = home_dir.join(".config/santa/config.yaml");
    let data = SantaData::load_from(Path::new("santa-data.yaml"));
    let d = data.export();
    trace!("{}", d);

    let config = SantaConfig::load_from(&config_file);
    trace!("{:?}", config);

    // for (k, v) in data.packages {
    //   println!("{}: {:?}", k, v);
    // }

    // env_logger::init();

    let mut cache: PackageCache = PackageCache::new();

    match &cli.command {
        Commands::Status => {
            info!("santa status");
            commands::status_command(&config, &data, cache);
        }
        Commands::Install { elf } => {
            println!("NYI: santa install {:?}", elf);
        }
        Commands::Add { elf, package } => {
            println!("NYI: santa add {:?} {:?}", elf, package);
        }
    }

    Ok(())

    // match cli.colors.as_deref() {
    //     None | Some("auto") => {}
    //     Some("always") => console::set_colors_enabled(true),
    //     Some("never") => console::set_colors_enabled(false),
    //     Some(other) => bail!("unknown value for --colors ({})", other),
    // };

    // if cli.list_timezones {
    //     return list_timezones();
    // }

    // let expr = InputExpr::parse(cli.expr.as_deref().unwrap_or("now"))?;
    // let timestamps = expr.process()?;

    // if cli.json {
    //     println!("{}", serde_json::to_string_pretty(&timestamps).unwrap());
    // } else if cli.short {
    //     for t in timestamps.iter() {
    //         println!(
    //             "{} ({})",
    //             t.datetime().format("%Y-%m-%d %H:%M:%S %z"),
    //             t.zone()
    //         );
    //     }
    // } else {
    //     let now = Utc::now();
    //     for (idx, t) in timestamps.iter().enumerate() {
    //         if idx > 0 {
    //             println!();
    //         }
    //         print_date(t, now);
    //     }
    // }
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
