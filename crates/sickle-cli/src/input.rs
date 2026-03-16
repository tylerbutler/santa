use anyhow::{bail, Result};
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum Format {
    Ccl,
    Json,
    Toml,
}

pub enum InputSource {
    File(PathBuf),
    Stdin,
}

pub struct Input {
    pub content: String,
    pub source_name: String,
}

impl InputSource {
    pub fn from_arg(file: Option<&Path>) -> Self {
        match file {
            Some(p) if p.to_str() != Some("-") => InputSource::File(p.to_path_buf()),
            _ => InputSource::Stdin,
        }
    }

    pub fn read(&self) -> Result<Input> {
        match self {
            InputSource::File(path) => {
                let content = std::fs::read_to_string(path)
                    .map_err(|e| anyhow::anyhow!("{}: {}", path.display(), e))?;
                Ok(Input {
                    content,
                    source_name: path.display().to_string(),
                })
            }
            InputSource::Stdin => {
                let mut content = String::new();
                std::io::stdin().read_to_string(&mut content)?;
                Ok(Input {
                    content,
                    source_name: "<stdin>".to_string(),
                })
            }
        }
    }
}

pub fn detect_format(file: Option<&Path>, explicit: Option<Format>) -> Result<Format> {
    if let Some(fmt) = explicit {
        return Ok(fmt);
    }
    match file {
        Some(p) if p.to_str() != Some("-") => match p.extension().and_then(|e| e.to_str()) {
            Some("ccl") => Ok(Format::Ccl),
            Some("json") => Ok(Format::Json),
            Some("toml") => Ok(Format::Toml),
            Some(ext) => bail!("Unknown extension '.{}'. Use --from to specify format.", ext),
            None => bail!("No file extension. Use --from to specify format."),
        },
        _ => bail!("Reading from stdin requires --from to specify format."),
    }
}
