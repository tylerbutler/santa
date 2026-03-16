use anyhow::Result;
use std::path::PathBuf;

#[derive(clap::Args)]
pub struct ConvertArgs {
    /// Input file (use '-' or omit for stdin)
    pub file: Option<PathBuf>,
}

pub fn run(_args: ConvertArgs) -> Result<()> {
    todo!()
}
