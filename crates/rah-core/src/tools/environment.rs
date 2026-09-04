use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Serialize;

use super::{installer::ToolInstaller, requirement::ToolRequirement};

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedEnvironment {
    pub variables: HashMap<String, String>,
    pub executables: HashMap<String, PathBuf>,
}

/// Resolve the environment using ONLY tools that are already installed.
///
/// This function:
/// - does not contact the network
/// - does not resolve versions
/// - does not install anything
///
/// It is therefore safe to call from shell hooks.
pub fn resolve_environment(requirements: &[ToolRequirement]) -> Result<ResolvedEnvironment> {
    let installer = ToolInstaller::new();

    let variables = HashMap::new();
    let mut executables = HashMap::new();

    for requirement in requirements {
        let Some(install_dir) = installer.installed_path(requirement)? else {
            continue;
        };

        let bin_dir = install_dir.join("bin");

        if !bin_dir.is_dir() {
            continue;
        }

        for entry in fs::read_dir(&bin_dir)
            .with_context(|| format!("failed to read {}", bin_dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };

            executables.insert(name.to_owned(), path);
        }
    }

    Ok(ResolvedEnvironment {
        variables,
        executables,
    })
}

/// Ensure that every requested tool is installed, then resolve the
/// resulting environment.
///
/// This is intentionally separate from `resolve_environment()` because
/// installation may involve network requests and filesystem operations.
pub async fn ensure_environment(requirements: &[ToolRequirement]) -> Result<ResolvedEnvironment> {
    let installer = ToolInstaller::new();

    for requirement in requirements {
        installer
            .install(requirement)
            .await
            .with_context(|| format!("failed to install `{requirement}`"))?;
    }

    resolve_environment(requirements)
}

/// Generate shell commands that apply the current Rah environment.
///
/// This function is read-only. Missing tools are simply omitted from the
/// generated environment.
pub fn shell_environment(requirements: &[ToolRequirement], shell: &str) -> Result<String> {
    let environment = resolve_environment(requirements)?;

    let mut paths: Vec<PathBuf> = environment
        .executables
        .values()
        .filter_map(|path| path.parent().map(Path::to_path_buf))
        .collect();

    paths.sort();
    paths.dedup();

    let mut output = String::new();

    match shell {
        "bash" | "zsh" => {
            if paths.is_empty() {
                output.push_str(
                    r#"export PATH="${__RAH_ORIG_PATH:-$PATH}"
"#,
                );
            } else {
                let joined = paths
                    .iter()
                    .map(|path| shell_escape(&path.to_string_lossy()))
                    .collect::<Vec<_>>()
                    .join(":");

                output.push_str(&format!(
                    "export PATH=\"{}:${{__RAH_ORIG_PATH:-$PATH}}\"\n",
                    joined
                ));
            }

            for (key, value) in &environment.variables {
                output.push_str(&format!("export {}={}\n", key, shell_escape(value)));
            }
        }

        "fish" => {
            output.push_str("set -gx PATH");

            if paths.is_empty() {
                output.push_str(" $__RAH_ORIG_PATH\n");
            } else {
                for path in &paths {
                    output.push(' ');
                    output.push_str(&fish_escape(&path.to_string_lossy()));
                }

                output.push_str(" $__RAH_ORIG_PATH\n");
            }

            for (key, value) in &environment.variables {
                output.push_str(&format!("set -gx {} {}\n", key, fish_escape(value)));
            }
        }

        "powershell" => {
            if paths.is_empty() {
                output.push_str("$env:PATH = $env:__RAH_ORIG_PATH\n");
            } else {
                let joined = paths
                    .iter()
                    .map(|path| path.to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join(";");

                output.push_str(&format!(
                    "$env:PATH = \"{};$env:__RAH_ORIG_PATH\"\n",
                    powershell_escape(&joined)
                ));
            }

            for (key, value) in &environment.variables {
                output.push_str(&format!(
                    "$env:{} = \"{}\"\n",
                    key,
                    powershell_escape(value)
                ));
            }
        }

        _ => {
            anyhow::bail!("unsupported shell `{shell}`");
        }
    }

    Ok(output)
}

fn shell_escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn fish_escape(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}

fn powershell_escape(value: &str) -> String {
    value.replace('`', "``").replace('"', "`\"")
}
