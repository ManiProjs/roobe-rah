use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use super::{registry::ToolRegistry, requirement::ToolRequirement, version::ToolVersion};

pub struct ToolInstaller {
    registry: ToolRegistry,
}

impl ToolInstaller {
    pub fn new() -> Self {
        Self {
            registry: ToolRegistry::new(),
        }
    }

    pub async fn install(&self, requirement: &ToolRequirement) -> Result<PathBuf> {
        let backend = self
            .registry
            .resolve_backend(requirement)
            .with_context(|| format!("failed to resolve backend for `{requirement}`"))?;

        let version = backend
            .resolve(requirement)
            .await
            .with_context(|| format!("failed to resolve `{requirement}`"))?;

        let install_dir = install_path(requirement, &version)?;

        if install_dir.exists() {
            return Ok(install_dir);
        }

        // Install into a temporary directory first.
        // This prevents partially-installed toolchains.
        let temp =
            tempfile::tempdir().context("failed to create temporary installation directory")?;

        backend
            .install(requirement, &version, temp.path())
            .await
            .with_context(|| format!("failed to install {}@{}", requirement.name, version))?;

        let bin_dir = install_dir.join("bin");

        fs::create_dir_all(&bin_dir)
            .with_context(|| format!("failed to create {}", bin_dir.display()))?;

        normalize_installation(temp.path(), &bin_dir).with_context(|| {
            format!(
                "failed to normalize installation of {}@{}",
                requirement.name, version
            )
        })?;

        Ok(install_dir)
    }
}

impl Default for ToolInstaller {
    fn default() -> Self {
        Self::new()
    }
}

fn install_path(requirement: &ToolRequirement, version: &ToolVersion) -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;

    Ok(home
        .join(".local")
        .join("share")
        .join("rah")
        .join("installs")
        .join(&requirement.name)
        .join(version.to_string()))
}

/// Normalize a backend installation into:
///
///     <install>/<tool>/<version>/bin/<executable>
///
/// Backends are allowed to extract archives in whatever layout
/// the upstream project uses. Rah converts that layout into its
/// own predictable structure here.
fn normalize_installation(source: &Path, bin_dir: &Path) -> Result<()> {
    let files = collect_files(source)?;

    if files.is_empty() {
        anyhow::bail!("installation archive contained no files");
    }

    for file in files {
        let name = file
            .file_name()
            .and_then(|name| name.to_str())
            .context("installation contains an invalid filename")?;

        // Ignore obvious metadata/documentation files.
        if is_metadata(name) {
            continue;
        }

        let destination = bin_dir.join(name);

        fs::copy(&file, &destination)
            .with_context(|| format!("failed to install {}", destination.display()))?;

        make_executable(&destination)?;
    }

    Ok(())
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    collect_files_recursive(root, &mut files)?;

    Ok(files)
}

fn collect_files_recursive(directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed to read {}", directory.display()))?
    {
        let entry = entry?;

        let path = entry.path();

        if path.is_dir() {
            collect_files_recursive(&path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }

    Ok(())
}

fn is_metadata(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();

    lower == "readme"
        || lower.starts_with("readme.")
        || lower == "license"
        || lower.starts_with("license.")
        || lower == "copying"
        || lower.starts_with("copying.")
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = fs::metadata(path)?;

    let mut permissions = metadata.permissions();

    permissions.set_mode(permissions.mode() | 0o111);

    fs::set_permissions(path, permissions)?;

    Ok(())
}
