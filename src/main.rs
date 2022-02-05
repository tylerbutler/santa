#![allow(unused)]

// extern crate clap_verbosity_flag;
use std::fmt;

use anyhow::bail;
use clap::{AppSettings, Parser, Subcommand};
use console::style;
use std::path::PathBuf;

/// Manage default sets of packages for a variety of package managers.
#[derive(Parser)]
#[clap(author, version = clap::crate_version!(), max_term_width = 100, about)]
#[clap(global_setting(AppSettings::PropagateVersion))]
#[clap(global_setting(AppSettings::UseLongFormatForHelpSubcommand))]
struct Cli {
    /// The pattern to look for
    // #[clap(long)]
    // pattern: String,

    // /// The path to the file to read
    // #[clap(short, long, parse(from_os_str))]
    // path: Option<PathBuf>,

    // #[clap(flatten)]
    // verbose: clap_verbosity_flag::Verbosity,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Prints the status of santa packages
    Status,
    /// Installs packages
    Install {
      elf: Option<String>
    },
    /// Asks an elf to add a package to its tracking list
    Add {
      package: Option<String>,
      elf: Option<String>
    },
}

pub fn execute() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    match &cli.command {
      Commands::Status => {
        println!("santa status");
      }
      Commands::Install {elf} => {
        println!("NYI: santa install {:?}", elf);
      }
      Commands::Add {elf, package} => {
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
    match execute() {
        Ok(()) => {}
        Err(err) => {
            eprintln!("error: {}", err);
            std::process::exit(1);
        }
    }
}
