use anyhow::{bail, Result};
use std::path::PathBuf;

use crate::bridge;
use crate::input::{self, Format, InputSource};

#[derive(clap::Args)]
pub struct ConvertArgs {
    /// Input file (reads from stdin if omitted or -)
    pub file: Option<PathBuf>,

    /// Input format (auto-detected from file extension)
    #[clap(long)]
    pub from: Option<Format>,

    /// Output format (required)
    #[clap(long)]
    pub to: Format,

    /// Compact JSON output (default: pretty-printed)
    #[clap(long)]
    pub compact: bool,

    /// Skip interactive prompts (e.g., comment loss warning)
    #[clap(short, long)]
    pub yes: bool,
}

pub fn run(args: ConvertArgs) -> Result<()> {
    let from = input::detect_format(args.file.as_deref(), args.from)?;
    let source = InputSource::from_arg(args.file.as_deref());
    let input = source.read()?;

    // Check for comment loss when converting FROM CCL
    if matches!(from, Format::Ccl)
        && !matches!(args.to, Format::Ccl)
        && bridge::has_comments(&input.content)
        && !args.yes
    {
        warn_comment_loss()?;
    }

    let output = convert(&input.content, from, args.to, !args.compact)?;
    print!("{}", output);
    Ok(())
}

fn warn_comment_loss() -> Result<()> {
    use dialoguer::Confirm;
    let proceed = Confirm::new()
        .with_prompt("Warning: CCL comments will be lost in the conversion. Continue?")
        .default(false)
        .interact()?;
    if !proceed {
        bail!("Conversion cancelled.");
    }
    Ok(())
}

fn convert(content: &str, from: Format, to: Format, pretty: bool) -> Result<String> {
    let value: serde_json::Value = match from {
        Format::Ccl => {
            let obj = sickle::load(content).map_err(|e| anyhow::anyhow!("{}", e))?;
            bridge::ccl_to_value(&obj)
        }
        Format::Json => {
            serde_json::from_str(content).map_err(|e| anyhow::anyhow!("Invalid JSON: {}", e))?
        }
        Format::Toml => {
            let toml_val: toml::Value =
                toml::from_str(content).map_err(|e| anyhow::anyhow!("Invalid TOML: {}", e))?;
            serde_json::to_value(toml_val)
                .map_err(|e| anyhow::anyhow!("TOML to JSON conversion error: {}", e))?
        }
    };

    match to {
        Format::Ccl => Ok(bridge::value_to_ccl_string(&value)),
        Format::Json => {
            if pretty {
                serde_json::to_string_pretty(&value)
                    .map_err(|e| anyhow::anyhow!("JSON serialization error: {}", e))
            } else {
                serde_json::to_string(&value)
                    .map_err(|e| anyhow::anyhow!("JSON serialization error: {}", e))
            }
        }
        Format::Toml => {
            let toml_val: toml::Value = serde_json::from_value(value)
                .map_err(|e| anyhow::anyhow!("Cannot represent as TOML: {}", e))?;
            toml::to_string_pretty(&toml_val)
                .map_err(|e| anyhow::anyhow!("TOML serialization error: {}", e))
        }
    }
}
