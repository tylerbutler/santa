use anyhow::{bail, Context};
use clap::{ArgAction, Command, Parser, Subcommand};
use clap_complete::{generate, Shell};
use configuration::SantaConfig;
use tracing::{debug, info, trace, Level};
use tracing_subscriber::{EnvFilter, FmtSubscriber};
extern crate directories;
use directories::BaseDirs;

use std::path::Path;

use crate::data::SantaData;
use crate::sources::PackageCache;
use crate::traits::Exportable;

mod commands;
mod configuration;
mod data;
mod sources;
mod traits;

static DEFAULT_CONFIG_FILE_PATH: &str = ".config/santa/config.yaml";

/// Manage default sets of packages for a variety of package managers.
#[derive(Parser)]
#[clap(author, version = clap::crate_version!(), max_term_width = 100, about)]
// #[clap(global_setting(AppSettings::PropagateVersion))]
struct Cli {
    #[clap(subcommand)]
    command: Commands,

    /// Load ONLY the default config
    #[clap(short, long, global = true)]
    builtin_only: bool,

    /// Increase logging level
    #[clap(short, long, global = true, action = ArgAction::Count)]
    verbose: u8,
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
    Install { source: Option<String> },
    /// Adds a package to the tracking list for a package source
    Add {
        package: Option<String>,
        source: Option<String>,
    },
    Config {
        /// Show full config
        #[clap(short, long)]
        packages: bool,

        #[clap(long)]
        pipe: bool,
    },
    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        shell: Shell,
    },
}

fn load_config(path: &Path) -> Result<SantaConfig, anyhow::Error> {
    let dir = BaseDirs::new().context("Failed to get base directories")?;
    let home_dir = dir.home_dir();
    let config_file = home_dir.join(path);
    let config = SantaConfig::load_from(&config_file)?;
    trace!("{:?}", config);
    Ok(config)
}

/// Build the modern CLI with enhanced features
fn build_cli() -> Command {
    Command::new("santa")
        .about("Manage default sets of packages for a variety of package managers")
        .version(clap::crate_version!())
        .author("Tyler Butler <tyler@tylerbutler.com>")
        .max_term_width(100)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .after_help("Use 'santa <command> --help' for more information about a specific command.")
        .arg(
            clap::Arg::new("builtin-only")
                .short('b')
                .long("builtin-only")
                .help("Load ONLY the default config")
                .action(ArgAction::SetTrue)
                .global(true),
        )
        .arg(
            clap::Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Increase logging level (-v: info, -vv: debug, -vvv: trace)")
                .action(ArgAction::Count)
                .global(true),
        )
}

pub fn run() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    // Handle shell completions
    if let Commands::Completions { shell } = &cli.command {
        let mut cmd = build_cli();
        generate(*shell, &mut cmd, "santa", &mut std::io::stdout());
        return Ok(());
    }

    // Initialize tracing subscriber with structured logging
    let log_level = match cli.verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        3 => Level::TRACE,
        _ => Level::TRACE,
    };

    let env_filter = EnvFilter::builder()
        .with_default_directive(log_level.into())
        .from_env_lossy();

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(env_filter)
        .with_target(false)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .context("Failed to set tracing subscriber")?;

    debug!("Argument parsing complete.");
    let data = SantaData::default();
    // let d = data.export();
    // trace!("data: {}", d);

    let mut config = if cli.builtin_only {
        info!("loading built-in config because of CLI flag.");
        SantaConfig::default()
    } else {
        load_config(Path::new(DEFAULT_CONFIG_FILE_PATH))?
    };
    config.log_level = cli.verbose;

    // let mut data = data; // re-declare variable to make it mutable
    // data.update_from_config(&config);

    let cache: PackageCache = PackageCache::new();

    match &cli.command {
        Commands::Status { all } => {
            debug!("santa status");
            crate::commands::status_command(&mut config, &data, cache, all)?;
        }
        Commands::Install { source: _ } => {
            crate::commands::install_command(&mut config, &data, cache)?;
        }
        Commands::Add { source, package } => {
            bail!(
                "Add command not yet implemented for source: {:?}, package: {:?}",
                source,
                package
            );
        }
        Commands::Config { packages, pipe: _ } => {
            crate::commands::config_command(&config, &data, *packages, cli.builtin_only)?;
        }
        Commands::Completions { shell: _ } => {
            // This is handled earlier in the function
            unreachable!("Completions should be handled before this point");
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
