use anyhow::Result;
use std::path::PathBuf;

use crate::input::InputSource;

/// Show the flat `key = value` pairs the CCL lexer produces, before any
/// hierarchy is built. This is a debugging view of Sickle's first pass.
#[derive(clap::Args)]
pub(crate) struct ParseArgs {
    /// Input file (reads from stdin if omitted or -)
    pub file: Option<PathBuf>,

    /// Output entries as JSON array
    #[clap(long)]
    pub json: bool,
}

pub(crate) fn run(args: ParseArgs) -> Result<()> {
    let source = InputSource::from_arg(args.file.as_deref());
    let input = source.read()?;

    let entries = sickle::unstable::flat_entries(&input.content, &sickle::Options::new())
        .map_err(|e| anyhow::anyhow!("{}: {}", input.source_name, e))?;

    if args.json {
        let rows: Vec<serde_json::Value> = entries
            .iter()
            .map(|entry| serde_json::json!({ "key": entry.key, "value": entry.value }))
            .collect();
        let json = serde_json::to_string_pretty(&rows)
            .map_err(|e| anyhow::anyhow!("JSON serialization error: {}", e))?;
        println!("{}", json);
    } else {
        for (i, entry) in entries.iter().enumerate() {
            println!("[{}] key={:?} value={:?}", i, entry.key, entry.value);
        }
    }

    Ok(())
}
