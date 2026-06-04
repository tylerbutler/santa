use anyhow::{bail, Result};
use std::path::PathBuf;

use crate::input::InputSource;

#[derive(clap::Args)]
pub(crate) struct FmtArgs {
    /// Input file (reads from stdin if omitted or -)
    pub file: Option<PathBuf>,

    /// Overwrite the file in place
    #[clap(short = 'i', long = "in-place")]
    pub in_place: bool,
}

pub(crate) fn run(args: FmtArgs) -> Result<()> {
    if args.in_place && args.file.is_none() {
        bail!("--in-place requires a file argument (cannot overwrite stdin)");
    }

    let source = InputSource::from_arg(args.file.as_deref());
    let input = source.read()?;

    // Structure-preserving format: parse to entries, print back
    let entries = sickle::parse(&input.content)
        .map_err(|e| anyhow::anyhow!("{}: {}", input.source_name, e))?;
    let formatted = sickle::printer::print(&entries);

    if args.in_place {
        let path = args.file.as_ref().unwrap();
        std::fs::write(path, &formatted)
            .map_err(|e| anyhow::anyhow!("{}: {}", path.display(), e))?;
    } else {
        print!("{}", formatted);
    }

    Ok(())
}
