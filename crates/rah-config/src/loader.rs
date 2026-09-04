use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use crate::RahConfig;

pub fn load(path: impl AsRef<Path>) -> Result<RahConfig> {
    let path = path.as_ref();

    let contents =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;

    let config: RahConfig =
        toml::from_str(&contents).with_context(|| format!("failed to parse {}", path.display()))?;

    Ok(config)
}

pub fn find(start: impl AsRef<Path>) -> Option<PathBuf> {
    let mut current = start.as_ref().canonicalize().ok()?;

    if current.is_file() {
        current.pop();
    }

    loop {
        let candidate = current.join("rah.toml");

        if candidate.is_file() {
            return Some(candidate);
        }

        if !current.pop() {
            break;
        }
    }

    None
}
