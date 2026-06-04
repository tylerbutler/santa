use anyhow::Result;
use colored::Colorize;
use std::path::PathBuf;

use crate::input::InputSource;

#[derive(clap::Args)]
pub(crate) struct ViewArgs {
    /// Input file (reads from stdin if omitted or -)
    pub file: Option<PathBuf>,
}

pub(crate) fn run(args: ViewArgs) -> Result<()> {
    let source = InputSource::from_arg(args.file.as_deref());
    let input = source.read()?;

    let entries = sickle::parse(&input.content)
        .map_err(|e| anyhow::anyhow!("{}: {}", input.source_name, e))?;

    for entry in &entries {
        if entry.key == "/" {
            // Comment (CCL parses `/= text` as key="/", value="text")
            println!("{}", format!("/= {}", entry.value).dimmed());
        } else if entry.key.is_empty() {
            // Bare list item
            println!("{} {}", "=".dimmed(), entry.value.cyan());
        } else if entry.value.is_empty() {
            // Key with empty value (section header or empty)
            println!("{} {}", entry.key.yellow(), "=".dimmed());
        } else {
            // Normal key = value
            println!(
                "{} {} {}",
                entry.key.yellow(),
                "=".dimmed(),
                entry.value.cyan()
            );
        }
    }

    Ok(())
}
