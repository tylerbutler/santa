use anyhow::Result;
use clap::{Parser, Subcommand};

mod bridge;
mod commands;
mod input;

/// A developer tool for working with CCL files
#[derive(Parser)]
#[clap(version, about, max_term_width = 100)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert between CCL, JSON, and TOML formats
    Convert(commands::convert::ConvertArgs),
    /// Validate a CCL file
    Validate(commands::validate::ValidateArgs),
    /// Format a CCL file to canonical form
    Fmt(commands::fmt::FmtArgs),
    /// Pretty-print a CCL file with syntax highlighting
    View(commands::view::ViewArgs),
    /// Show flat parsed entries (debug view)
    Parse(commands::parse::ParseArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Convert(args) => commands::convert::run(args),
        Commands::Validate(args) => commands::validate::run(args),
        Commands::Fmt(args) => commands::fmt::run(args),
        Commands::View(args) => commands::view::run(args),
        Commands::Parse(args) => commands::parse::run(args),
    }
}
