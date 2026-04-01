use anyhow::Result;
use std::path::PathBuf;

use crate::input::InputSource;

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

    let entries = sickle::parse(&input.content)
        .map_err(|e| anyhow::anyhow!("{}: {}", input.source_name, e))?;

    if args.json {
        let json = serde_json::to_string_pretty(&entries)
            .map_err(|e| anyhow::anyhow!("JSON serialization error: {}", e))?;
        println!("{}", json);
    } else {
        for (i, entry) in entries.iter().enumerate() {
            println!("[{}] key={:?} value={:?}", i, entry.key, entry.value);
        }
    }

    Ok(())
}
