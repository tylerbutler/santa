use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;
use std::process;

use crate::input::InputSource;

#[derive(clap::Args)]
pub struct ValidateArgs {
    /// Input file (reads from stdin if omitted or -)
    pub file: Option<PathBuf>,

    /// Suppress output on success (exit code only)
    #[clap(short, long)]
    pub quiet: bool,
}

pub fn run(args: ValidateArgs) -> Result<()> {
    let source = InputSource::from_arg(args.file.as_deref());
    let input = source.read()?;

    match sickle::load(&input.content) {
        Ok(_) => {
            if !args.quiet {
                eprintln!("{} {}", input.source_name, "OK".green());
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("{}: {}", input.source_name, format!("{}", e).red());
            process::exit(1);
        }
    }
}
