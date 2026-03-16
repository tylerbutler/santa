use anyhow::Result;
use std::path::PathBuf;

#[derive(clap::Args)]
pub struct ValidateArgs {
    /// Input file (use '-' or omit for stdin)
    pub file: Option<PathBuf>,
}

pub fn run(_args: ValidateArgs) -> Result<()> {
    todo!()
}
