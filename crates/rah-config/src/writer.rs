use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

pub fn init(path: impl AsRef<Path>) -> Result<PathBuf> {
    let path = path.as_ref();

    if path.exists() {
        anyhow::bail!("rah.toml already exists at {}", path.display());
    }

    let contents = r#"# rah.toml

[tools]

[env]

[settings]
"#;

    fs::write(path, contents).with_context(|| format!("failed to create {}", path.display()))?;

    Ok(path.to_path_buf())
}

pub fn add_tool(path: impl AsRef<Path>, name: &str, version: &str) -> Result<()> {
    let path = path.as_ref();

    let contents = if path.exists() {
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?
    } else {
        String::new()
    };

    let mut document: toml::Table = if contents.trim().is_empty() {
        toml::Table::new()
    } else {
        toml::from_str(&contents).with_context(|| format!("failed to parse {}", path.display()))?
    };

    let tools = document
        .entry("tools")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()));

    let tools = tools
        .as_table_mut()
        .ok_or_else(|| anyhow::anyhow!("[tools] must be a table"))?;

    tools.insert(name.to_owned(), toml::Value::String(version.to_owned()));

    let output = toml::to_string_pretty(&document)?;

    fs::write(path, output).with_context(|| format!("failed to write {}", path.display()))?;

    Ok(())
}
