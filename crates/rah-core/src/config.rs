use std::path::Path;

use anyhow::Result;
use rah_config::{RahConfig, find, load};

pub fn load_config(start: impl AsRef<Path>) -> Result<Option<RahConfig>> {
    let Some(path) = find(start) else {
        return Ok(None);
    };

    Ok(Some(load(path)?))
}
