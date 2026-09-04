use std::{env, path::PathBuf};

use anyhow::{Context, Result};

use super::{installer::ToolInstaller, requirement::ToolRequirement};

pub async fn build_path(requirements: &[ToolRequirement]) -> Result<String> {
    let installer = ToolInstaller::new();

    let mut paths: Vec<PathBuf> = Vec::new();

    for requirement in requirements {
        let install_dir = installer
            .install(requirement)
            .await
            .with_context(|| format!("failed to prepare `{requirement}`"))?;

        let bin_dir = install_dir.join("bin");

        if bin_dir.is_dir() {
            paths.push(bin_dir);
        }
    }

    let current_path = env::var_os("PATH").unwrap_or_default();

    paths.extend(env::split_paths(&current_path));

    let result = env::join_paths(paths)?;

    Ok(result.to_string_lossy().into_owned())
}
