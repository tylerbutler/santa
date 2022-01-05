extern crate clap_verbosity_flag;
use structopt::StructOpt;
use std::path::PathBuf;
use clap::{Parser, Args, Subcommand, ArgEnum};

/// Manage default sets of packages for a variety of package managers.
#[derive(Debug, Parser)]
#[clap(author, version, about)]
struct Cli {
    /// The pattern to look for
    #[clap(long)]
    pattern: String,

    /// The path to the file to read
    #[clap(short, long, parse(from_os_str))]
    path: Option<PathBuf>,

    #[clap(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
}

fn main() {
    let args = Cli::parse();
}
