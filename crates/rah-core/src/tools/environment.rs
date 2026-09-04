use std::collections::HashMap;

use anyhow::Result;

use super::{installer::ToolInstaller, requirement::ToolRequirement};

pub async fn resolve_environment(
    requirements: &[ToolRequirement],
) -> Result<HashMap<String, String>> {
    let installer = ToolInstaller::new();

    let mut environment = HashMap::new();

    let mut tool_paths = Vec::new();

    for requirement in requirements {
        let install_dir = installer.install(requirement).await?;

        let bin = install_dir.join("bin");

        if bin.is_dir() {
            tool_paths.push(bin);
        }
    }

    if !tool_paths.is_empty() {
        let separator = if cfg!(windows) { ";" } else { ":" };

        let rah_path = tool_paths
            .iter()
            .map(|path| path.to_string_lossy())
            .collect::<Vec<_>>()
            .join(separator);

        let current_path = std::env::var("PATH").unwrap_or_default();

        environment.insert(
            "PATH".into(),
            format!("{rah_path}{separator}{current_path}"),
        );
    }

    Ok(environment)
}
