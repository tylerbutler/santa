use anyhow::{bail, Context};
use clap::{ArgAction, Command, CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Shell};
use santa::completions::EnhancedCompletions;
use santa::configuration::{SantaConfig, SantaConfigExt};
use santa::data::SantaData;
use tracing::{debug, info, trace, Level};
use tracing_subscriber::{EnvFilter, FmtSubscriber};
extern crate directories;
use directories::BaseDirs;

use std::path::Path;

use santa::commands;
use santa::data_layers::DataLayerManager;
use santa::script_generator::{ExecutionMode, ScriptFormat};
use santa::sources::PackageCache;

#[cfg(test)]
mod tests;

static DEFAULT_CONFIG_FILE_PATH: &str = ".config/santa/config.ccl";

/// Manage default sets of packages for a variety of package managers.
#[derive(Parser)]
#[clap(author, version = clap::crate_version!(), max_term_width = 100, about)]
// #[clap(global_setting(AppSettings::PropagateVersion))]
struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,

    /// Load ONLY the default config
    #[clap(short, long, global = true)]
    builtin_only: bool,

    /// Increase logging level
    #[clap(short, long, global = true, action = ArgAction::Count)]
    verbose: u8,

    /// Enable dangerous direct execution mode (default: safe script generation)
    #[clap(short = 'x', long, global = true)]
    execute: bool,

    /// Script format for safe mode (auto-detects based on platform)
    #[clap(long, global = true, value_enum, default_value = "auto")]
    format: ScriptFormatOption,

    /// Output directory for generated scripts
    #[clap(long, global = true)]
    output_dir: Option<std::path::PathBuf>,

    /// Print help in markdown format (for documentation generation)
    #[clap(long, hide = true)]
    markdown_help: bool,

    /// Path to custom config file
    #[clap(short, long, global = true)]
    config: Option<std::path::PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Prints the status of santa packages
    Status {
        /// List all packages, not just missing ones
        #[clap(short, long)]
        all: bool,

        /// Show only installed packages
        #[clap(short, long, conflicts_with_all = &["missing", "all"])]
        installed: bool,

        /// Show only missing packages
        #[clap(short, long, conflicts_with_all = &["installed", "all"])]
        missing: bool,

        /// Filter by specific source
        #[clap(short, long)]
        source: Option<String>,
    },
    /// Installs packages
    Install { source: Option<String> },
    /// Adds packages to the configuration
    Add {
        /// Package names to add
        packages: Vec<String>,
    },
    /// Removes packages from the configuration
    Remove {
        /// Package names to remove
        packages: Vec<String>,

        /// Uninstall packages before removing from config
        #[clap(short, long)]
        uninstall: bool,
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
    /// Manage package sources
    #[clap(subcommand)]
    Sources(SourcesCommands),
}

/// Subcommands for managing package sources
#[derive(Subcommand)]
enum SourcesCommands {
    /// Download the latest source definitions from GitHub
    Update,
    /// List all available sources (from all layers)
    List {
        /// Show only sources from a specific origin (bundled, downloaded, custom)
        #[clap(long)]
        origin: Option<String>,
    },
    /// Show details about a specific source
    Show {
        /// Name of the source to show
        name: String,
    },
    /// Remove downloaded sources (revert to bundled only)
    Clear,
}

/// Script format options for CLI
#[derive(Clone, ValueEnum, Debug)]
enum ScriptFormatOption {
    /// Auto-detect based on platform (PowerShell on Windows, Shell elsewhere)
    Auto,
    /// Force shell script (.sh) - Unix/Linux/macOS
    Shell,
    /// Force PowerShell script (.ps1) - Windows
    PowerShell,
}

impl ScriptFormatOption {
    /// Convert to ScriptFormat, resolving Auto based on platform
    fn to_script_format(&self) -> ScriptFormat {
        match self {
            ScriptFormatOption::Auto => ScriptFormat::auto_detect(),
            ScriptFormatOption::Shell => ScriptFormat::Shell,
            ScriptFormatOption::PowerShell => ScriptFormat::PowerShell,
        }
    }
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
        .arg(
            clap::Arg::new("execute")
                .short('x')
                .long("execute")
                .help("Enable dangerous direct execution mode (default: safe script generation)")
                .action(ArgAction::SetTrue)
                .global(true),
        )
        .arg(
            clap::Arg::new("format")
                .long("format")
                .help("Script format for safe mode (auto, shell, powershell)")
                .value_parser(clap::value_parser!(ScriptFormatOption))
                .default_value("auto")
                .global(true),
        )
        .arg(
            clap::Arg::new("output-dir")
                .long("output-dir")
                .help("Output directory for generated scripts")
                .value_parser(clap::value_parser!(std::path::PathBuf))
                .global(true),
        )
}

/// Handle the sources subcommand
async fn handle_sources_command(
    cmd: &SourcesCommands,
    config: &SantaConfig,
) -> Result<(), anyhow::Error> {
    use colored::Colorize;
    use santa::data_layers::DataOrigin;

    let manager = DataLayerManager::with_default_config_dir()?;

    match cmd {
        SourcesCommands::Update => {
            use std::io::Write;
            println!("Fetching latest data from GitHub...");

            // Update sources
            print!("  Updating sources... ");
            std::io::stdout().flush()?;
            let sources_count = manager.update_sources()?;
            println!("{} ({} sources)", "✓".green(), sources_count);

            // Update packages
            print!("  Updating packages... ");
            std::io::stdout().flush()?;
            let packages_count = manager.update_packages()?;
            println!("{} ({} packages)", "✓".green(), packages_count);

            println!("{}", "\nData updated successfully!".green());

            // Show summary
            let merged_sources = manager.merge_sources(None)?;
            let sources_summary = manager.sources_summary(&merged_sources);
            let merged_packages = manager.merge_packages(None)?;
            let packages_summary = manager.packages_summary(&merged_packages);

            println!(
                "\nSources: {} total ({} bundled, {} downloaded)",
                merged_sources.len(),
                sources_summary.get(&DataOrigin::Bundled).unwrap_or(&0),
                sources_summary.get(&DataOrigin::Downloaded).unwrap_or(&0)
            );
            println!(
                "Packages: {} total ({} bundled, {} downloaded)",
                merged_packages.len(),
                packages_summary.get(&DataOrigin::Bundled).unwrap_or(&0),
                packages_summary.get(&DataOrigin::Downloaded).unwrap_or(&0)
            );
            println!(
                "\nData location: {}",
                manager.config_dir().display().to_string().dimmed()
            );
        }
        SourcesCommands::List { origin } => {
            // Convert user custom sources from config to SourcesDefinition format
            let user_custom = config.custom_sources.as_ref().map(|sources| {
                sources
                    .iter()
                    .map(|s| {
                        (
                            s.name.to_string(),
                            santa::data::schemas::SourceDefinition {
                                emoji: s.emoji.clone(),
                                install: s.install_command.clone(),
                                check: s.check_command.clone(),
                                prefix: s.prepend_to_package_name.clone(),
                                overrides: None, // TODO: convert overrides
                            },
                        )
                    })
                    .collect::<std::collections::HashMap<_, _>>()
            });

            let merged = manager.merge_sources(user_custom.as_ref())?;

            // Filter by origin if specified
            let filtered: Vec<_> = if let Some(origin_filter) = origin {
                let target_origin = match origin_filter.to_lowercase().as_str() {
                    "bundled" => DataOrigin::Bundled,
                    "downloaded" => DataOrigin::Downloaded,
                    "custom" | "user" => DataOrigin::UserCustom,
                    _ => {
                        eprintln!(
                            "Unknown origin '{}'. Valid options: bundled, downloaded, custom",
                            origin_filter
                        );
                        return Ok(());
                    }
                };
                merged
                    .into_iter()
                    .filter(|s| s.origin == target_origin)
                    .collect()
            } else {
                merged
            };

            if filtered.is_empty() {
                println!("No sources found.");
                return Ok(());
            }

            println!(
                "{:<15} {:<8} {:<12} {}",
                "NAME".bold(),
                "EMOJI".bold(),
                "ORIGIN".bold(),
                "INSTALL COMMAND".bold()
            );
            println!("{}", "-".repeat(70));

            for source in &filtered {
                let origin_str = match source.origin {
                    DataOrigin::Bundled => "bundled".dimmed(),
                    DataOrigin::Downloaded => "downloaded".cyan(),
                    DataOrigin::UserCustom => "custom".yellow(),
                };
                // Truncate install command for display
                let install_display = if source.definition.install.len() > 35 {
                    format!("{}...", &source.definition.install[..32])
                } else {
                    source.definition.install.clone()
                };
                println!(
                    "{:<15} {:<8} {:<12} {}",
                    source.name, source.definition.emoji, origin_str, install_display
                );
            }

            println!("\nTotal: {} sources", filtered.len());
            println!(
                "Data location: {}",
                manager.config_dir().display().to_string().dimmed()
            );
        }
        SourcesCommands::Show { name } => {
            // Convert user custom sources from config
            let user_custom = config.custom_sources.as_ref().map(|sources| {
                sources
                    .iter()
                    .map(|s| {
                        (
                            s.name.to_string(),
                            santa::data::schemas::SourceDefinition {
                                emoji: s.emoji.clone(),
                                install: s.install_command.clone(),
                                check: s.check_command.clone(),
                                prefix: s.prepend_to_package_name.clone(),
                                overrides: None,
                            },
                        )
                    })
                    .collect::<std::collections::HashMap<_, _>>()
            });

            let merged = manager.merge_sources(user_custom.as_ref())?;

            if let Some(source) = merged.iter().find(|s| s.name == *name) {
                println!("{}: {}", "Name".bold(), source.name);
                println!("{}: {}", "Emoji".bold(), source.definition.emoji);
                println!("{}: {}", "Origin".bold(), source.origin);
                println!("{}: {}", "Install".bold(), source.definition.install);
                println!("{}: {}", "Check".bold(), source.definition.check);
                if let Some(prefix) = &source.definition.prefix {
                    println!("{}: {}", "Prefix".bold(), prefix);
                }
                if let Some(overrides) = &source.definition.overrides {
                    println!("{}:", "Platform Overrides".bold());
                    for (platform, override_def) in overrides {
                        println!("  {}:", platform);
                        if let Some(install) = &override_def.install {
                            println!("    install: {}", install);
                        }
                        if let Some(check) = &override_def.check {
                            println!("    check: {}", check);
                        }
                    }
                }
            } else {
                eprintln!("Source '{}' not found.", name);
            }
        }
        SourcesCommands::Clear => {
            let had_sources = manager.has_downloaded_sources();
            let had_packages = manager.has_downloaded_packages();

            if had_sources || had_packages {
                manager.clear_all()?;
                if had_sources {
                    println!("{}", "Downloaded sources removed.".green());
                }
                if had_packages {
                    println!("{}", "Downloaded packages removed.".green());
                }
                println!("Santa will now use bundled data only.");
            } else {
                println!("No downloaded data to remove.");
            }
        }
    }

    Ok(())
}

pub async fn run() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    // Handle markdown help generation (for documentation)
    if cli.markdown_help {
        clap_markdown::print_help_markdown::<Cli>();
        return Ok(());
    }

    // Show help when no command is provided
    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            Cli::command().print_help()?;
            return Ok(());
        }
    };

    // Handle shell completions with enhanced suggestions
    if let Commands::Completions { shell } = &command {
        let mut cmd = build_cli();

        // Try to load config and data for enhanced completions
        let config_result = if cli.builtin_only {
            Ok(SantaConfig::default_for_platform())
        } else {
            load_config(Path::new(DEFAULT_CONFIG_FILE_PATH))
        };

        match config_result {
            Ok(config) => {
                let data = SantaData::default();
                // Use enhanced completions with config and data
                EnhancedCompletions::generate_enhanced_shell_completions(
                    *shell,
                    &mut cmd,
                    "santa",
                    &mut std::io::stdout(),
                    &config,
                    &data,
                )
                .unwrap_or_else(|_| {
                    // Fall back to standard completions if enhanced ones fail
                    generate(*shell, &mut cmd, "santa", &mut std::io::stdout());
                });
            }
            Err(_) => {
                // Fall back to standard completions if config loading fails
                generate(*shell, &mut cmd, "santa", &mut std::io::stdout());
            }
        }
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
        SantaConfig::default_for_platform()
    } else {
        // Use custom config path if provided, otherwise use default
        let config_path = cli
            .config
            .as_deref()
            .unwrap_or_else(|| Path::new(DEFAULT_CONFIG_FILE_PATH));
        load_config(config_path)?
    };
    config.log_level = cli.verbose;

    // Validate config against available package data (emits warnings for issues)
    config.validate_with_data(&data)?;

    let cache: PackageCache = PackageCache::new();

    // Determine execution mode based on CLI flags
    let execution_mode = if cli.execute {
        ExecutionMode::Execute
    } else {
        ExecutionMode::Safe
    };

    let script_format = cli.format.to_script_format();
    let output_dir = cli
        .output_dir
        .unwrap_or_else(|| std::env::current_dir().expect("Failed to get current directory"));

    match &command {
        Commands::Status {
            all,
            installed,
            missing,
            source,
        } => {
            debug!("santa status");
            crate::commands::status_command(
                &mut config,
                &data,
                cache,
                all,
                installed,
                missing,
                source.as_deref(),
            )
            .await?;
        }
        Commands::Install { source: _ } => {
            crate::commands::install_command(
                &mut config,
                &data,
                cache,
                execution_mode,
                script_format,
                &output_dir,
            )
            .await?;
        }
        Commands::Add { packages } => {
            if packages.is_empty() {
                bail!("No packages specified. Usage: santa add <package1> [package2 ...]");
            }
            let config_path = cli
                .config
                .as_deref()
                .unwrap_or_else(|| Path::new(DEFAULT_CONFIG_FILE_PATH));
            let config_path = BaseDirs::new()
                .context("Failed to get base directories")?
                .home_dir()
                .join(config_path);
            crate::commands::add_command(&config_path, packages.clone(), &data).await?;
        }
        Commands::Remove { packages, uninstall } => {
            if packages.is_empty() {
                bail!("No packages specified. Usage: santa remove <package1> [package2 ...]");
            }
            let config_path = cli
                .config
                .as_deref()
                .unwrap_or_else(|| Path::new(DEFAULT_CONFIG_FILE_PATH));
            let config_path = BaseDirs::new()
                .context("Failed to get base directories")?
                .home_dir()
                .join(config_path);
            crate::commands::remove_command(&config_path, packages.clone(), *uninstall).await?;
        }
        Commands::Config { packages, pipe: _ } => {
            crate::commands::config_command(&config, &data, *packages, cli.builtin_only)?;
        }
        Commands::Completions { shell: _ } => {
            // This is handled earlier in the function
            unreachable!("Completions should be handled before this point");
        }
        Commands::Sources(sources_cmd) => {
            handle_sources_command(sources_cmd, &config).await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    match run().await {
        Ok(()) => {}
        Err(err) => {
            eprintln!("error: {err}");
            std::process::exit(1);
        }
    }
}
