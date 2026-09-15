use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result};
use serde::Serialize;

use super::{installer::ToolInstaller, requirement::ToolRequirement};

/// The complete environment Rah resolves for a project.
#[derive(Debug, Clone, Serialize)]
pub struct ResolvedEnvironment {
    /// Environment variables to inject into processes.
    pub variables: HashMap<String, String>,

    /// Executables provided by Rah-managed tools.
    ///
    /// The key is the executable name, such as `node` or `rg`.
    /// The value is the absolute executable path.
    pub executables: HashMap<String, String>,
}

impl ResolvedEnvironment {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            executables: HashMap::new(),
        }
    }
}

impl Default for ResolvedEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve the complete Rah environment for the supplied tool requirements.
///
/// This:
/// - installs missing tools
/// - adds their `bin` directories to PATH
/// - maps executables to their absolute paths
/// - preserves the user's existing environment
/// - loads project `[env]` variables
pub async fn resolve_environment(requirements: &[ToolRequirement]) -> Result<ResolvedEnvironment> {
    let installer = ToolInstaller::new();

    let mut environment = ResolvedEnvironment::new();
    let mut tool_paths = Vec::new();

    for requirement in requirements {
        let install_dir = installer
            .install(requirement)
            .await
            .with_context(|| format!("failed to install `{requirement}`"))?;

        let bin = find_bin_directory(&install_dir)?;

        if !bin.is_dir() {
            continue;
        }

        tool_paths.push(bin.clone());

        register_executables(&bin, &mut environment.executables)?;
    }

    // Rah-managed binaries come first.
    //
    // This is what makes:
    //
    //     rah exec -- rg
    //
    // resolve to:
    //
    //     ~/.local/share/rah/installs/.../rg
    //
    // instead of Homebrew's /opt/homebrew/bin/rg.
    if !tool_paths.is_empty() {
        let separator = if cfg!(windows) { ";" } else { ":" };

        let rah_path = tool_paths
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(separator);

        let current_path = std::env::var("PATH").unwrap_or_default();

        environment.variables.insert(
            "PATH".to_string(),
            if current_path.is_empty() {
                rah_path
            } else {
                format!("{rah_path}{separator}{current_path}")
            },
        );
    }

    // Load project environment variables.
    //
    // We deliberately do this last so `[env]` can override an inherited
    // environment variable when executing a project command.
    load_project_environment(&mut environment.variables)?;

    Ok(environment)
}

/// Generate shell-compatible commands for the resolved Rah environment.
pub async fn shell_environment(requirements: &[ToolRequirement], shell: &str) -> Result<String> {
    let environment = resolve_environment(requirements).await?;

    match shell {
        "bash" | "zsh" => render_posix_environment(&environment),

        "fish" => render_fish_environment(&environment),

        "powershell" | "pwsh" => render_powershell_environment(&environment),

        _ => anyhow::bail!("unsupported shell `{shell}`"),
    }
}

/// Locate the `bin` directory created by the Rah installer.
///
/// The normal structure is:
///
///     install/<version>/bin
///
/// Some backends may preserve an upstream directory inside the install
/// directory, so we also search recursively for a directory named `bin`.
fn find_bin_directory(install_dir: &Path) -> Result<std::path::PathBuf> {
    let direct = install_dir.join("bin");

    if direct.is_dir() {
        return Ok(direct);
    }

    find_directory_named(install_dir, "bin")?.with_context(|| {
        format!(
            "could not find a `bin` directory in {}",
            install_dir.display()
        )
    })
}

fn find_directory_named(root: &Path, name: &str) -> Result<Option<std::path::PathBuf>> {
    for entry in
        std::fs::read_dir(root).with_context(|| format!("failed to read {}", root.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        if path.file_name().and_then(|name| name.to_str()) == Some(name) {
            return Ok(Some(path));
        }

        if let Some(found) = find_directory_named(&path, name)? {
            return Ok(Some(found));
        }
    }

    Ok(None)
}

/// Register every executable in a tool's `bin` directory.
fn register_executables(bin: &Path, executables: &mut HashMap<String, String>) -> Result<()> {
    for entry in
        std::fs::read_dir(bin).with_context(|| format!("failed to read {}", bin.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };

        // Don't let an invalid or duplicate executable overwrite an already
        // registered one. The first resolved tool wins.
        executables
            .entry(name.to_string())
            .or_insert_with(|| path.to_string_lossy().into_owned());
    }

    Ok(())
}

/// Load `[env]` from the current project's rah.toml.
fn load_project_environment(variables: &mut HashMap<String, String>) -> Result<()> {
    let Some(config_path) = rah_config::find(".") else {
        return Ok(());
    };

    let config = rah_config::load(&config_path)
        .with_context(|| format!("failed to load {}", config_path.display()))?;

    for (name, value) in config.env {
        variables.insert(name, value);
    }

    Ok(())
}

/// Render environment variables as POSIX shell exports.
fn render_posix_environment(environment: &ResolvedEnvironment) -> Result<String> {
    let mut output = String::new();

    for (key, value) in sorted_variables(&environment.variables) {
        output.push_str("export ");
        output.push_str(key);
        output.push('=');
        output.push_str(&shell_quote(value));
        output.push('\n');
    }

    Ok(output)
}

/// Render environment variables for fish.
fn render_fish_environment(environment: &ResolvedEnvironment) -> Result<String> {
    let mut output = String::new();

    for (key, value) in sorted_variables(&environment.variables) {
        output.push_str("set -gx ");
        output.push_str(key);
        output.push(' ');
        output.push_str(&fish_quote(value));
        output.push('\n');
    }

    Ok(output)
}

/// Render environment variables for PowerShell.
fn render_powershell_environment(environment: &ResolvedEnvironment) -> Result<String> {
    let mut output = String::new();

    for (key, value) in sorted_variables(&environment.variables) {
        output.push_str("$env:");
        output.push_str(key);
        output.push_str(" = ");
        output.push_str(&powershell_quote(value));
        output.push('\n');
    }

    Ok(output)
}

/// Sort variables for deterministic output.
///
/// This makes `rah env` stable instead of depending on HashMap iteration order.
fn sorted_variables(variables: &HashMap<String, String>) -> Vec<(&String, &String)> {
    let mut values = variables.iter().collect::<Vec<_>>();

    values.sort_by(|a, b| a.0.cmp(b.0));

    values
}

/// Basic POSIX single-quote escaping.
fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }

    format!("'{}'", value.replace('\'', "'\\''"))
}

/// Basic fish single-quote escaping.
fn fish_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }

    format!("'{}'", value.replace('\'', "\\'"))
}

/// Basic PowerShell single-quote escaping.
fn powershell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}
