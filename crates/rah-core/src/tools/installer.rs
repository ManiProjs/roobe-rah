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

    /// Find an already-installed version without contacting a backend.
    ///
    /// This is intentionally synchronous and cheap because it is used by
    /// `rah env` and shell hooks.
    pub fn installed_path(&self, requirement: &ToolRequirement) -> Result<Option<PathBuf>> {
        let home = dirs::home_dir().context("could not determine home directory")?;

        let root = home
            .join(".local")
            .join("share")
            .join("rah")
            .join("installs")
            .join(&requirement.name);

        if !root.is_dir() {
            return Ok(None);
        }

        // Exact version:
        //
        // github:BurntSushi/ripgrep@14.1.1
        //
        // becomes:
        //
        // installs/BurntSushi/ripgrep/14.1.1
        if requirement.version != "latest" {
            let exact = root.join(&requirement.version);

            if exact.is_dir() {
                return Ok(Some(exact));
            }

            return Ok(None);
        }

        // `latest` means:
        // "find the newest version that Rah has already installed."
        //
        // This MUST NOT contact the backend.
        let mut versions = Vec::new();

        for entry in
            fs::read_dir(&root).with_context(|| format!("failed to read {}", root.display()))?
        {
            let entry = entry?;

            if !entry.path().is_dir() {
                continue;
            }

            versions.push(entry.path());
        }

        versions.sort();

        Ok(versions.pop())
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
    fs::create_dir_all(bin_dir)
        .with_context(|| format!("failed to create {}", bin_dir.display()))?;

    let files = collect_files(source)?;

    if files.is_empty() {
        anyhow::bail!("installation archive contained no files");
    }

    let mut installed = 0;

    for file in files {
        let name = file
            .file_name()
            .and_then(|name| name.to_str())
            .context("installation contains an invalid filename")?;

        if is_metadata(name) {
            continue;
        }

        // On Unix, only install files that already have an
        // executable bit. This prevents README/config/data files
        // from being placed into bin/.
        #[cfg(unix)]
        if !is_executable(&file)? {
            continue;
        }

        let destination = bin_dir.join(name);

        fs::copy(&file, &destination)
            .with_context(|| format!("failed to install {}", destination.display()))?;

        make_executable(&destination)?;

        installed += 1;
    }

    if installed == 0 {
        anyhow::bail!("installation archive contained no executable files");
    }

    Ok(())
}

#[cfg(unix)]
fn is_executable(path: &Path) -> Result<bool> {
    use std::os::unix::fs::PermissionsExt;

    let metadata = fs::metadata(path)?;

    Ok(metadata.permissions().mode() & 0o111 != 0)
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
