use anyhow::Result;
use std::path::PathBuf;

#[derive(clap::Args)]
pub struct ViewArgs {
    /// Input file (use '-' or omit for stdin)
    pub file: Option<PathBuf>,
}

pub fn run(_args: ViewArgs) -> Result<()> {
    todo!()
}
