use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde::Deserialize;

use super::{backend::ToolBackend, requirement::ToolRequirement, version::ToolVersion};

const API_VERSION: &str = "2026-03-10";

pub struct GitHubBackend {
    client: Client,
}

impl GitHubBackend {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("rah")
                .build()
                .expect("failed to build GitHub HTTP client"),
        }
    }

    fn repository<'a>(&self, requirement: &'a ToolRequirement) -> Result<(&'a str, &'a str)> {
        let path = &requirement.name;

        let (owner, repo) = path
            .split_once('/')
            .context("GitHub tool must use github:owner/repo")?;

        if owner.is_empty() || repo.is_empty() || repo.contains('/') {
            bail!("invalid GitHub repository `{path}`");
        }

        Ok((owner, repo))
    }

    async fn release(&self, requirement: &ToolRequirement) -> Result<GitHubRelease> {
        let (owner, repo) = self.repository(requirement)?;

        let url = if requirement.version == "latest" {
            format!("https://api.github.com/repos/{owner}/{repo}/releases/latest")
        } else {
            let version = requirement.version.trim_start_matches('v');

            // Try the exact tag first.
            format!("https://api.github.com/repos/{owner}/{repo}/releases/tags/{version}")
        };

        let response = self
            .client
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", API_VERSION)
            .send()
            .await
            .context("failed to contact GitHub")?;

        if response.status() == reqwest::StatusCode::NOT_FOUND && requirement.version != "latest" {
            // Some repositories use v-prefixed tags.
            let version = requirement.version.trim_start_matches('v');

            let fallback_url =
                format!("https://api.github.com/repos/{owner}/{repo}/releases/tags/v{version}");

            let fallback = self
                .client
                .get(&fallback_url)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", API_VERSION)
                .send()
                .await
                .context("failed to contact GitHub")?;

            if !fallback.status().is_success() {
                bail!(
                    "GitHub returned {} while resolving {}",
                    fallback.status(),
                    requirement
                );
            }

            return fallback
                .json::<GitHubRelease>()
                .await
                .context("invalid GitHub release response");
        }

        if !response.status().is_success() {
            bail!(
                "GitHub returned {} while resolving {}",
                response.status(),
                requirement
            );
        }

        response
            .json::<GitHubRelease>()
            .await
            .context("invalid GitHub release response")
    }

    fn select_asset<'a>(&self, release: &'a GitHubRelease) -> Result<&'a GitHubAsset> {
        let target = platform_target();

        release
            .assets
            .iter()
            .filter(|asset| asset.name.contains(target))
            .find(|asset| {
                asset.name.ends_with(".tar.gz")
                    || asset.name.ends_with(".tgz")
                    || asset.name.ends_with(".zip")
            })
            .or_else(|| {
                release.assets.iter().find(|asset| {
                    asset.name.ends_with(".tar.gz")
                        || asset.name.ends_with(".tgz")
                        || asset.name.ends_with(".zip")
                })
            })
            .context("no suitable release asset found")
    }

    async fn download(&self, asset: &GitHubAsset, destination: &Path) -> Result<PathBuf> {
        tokio::fs::create_dir_all(destination).await?;

        let archive = destination.join(&asset.name);

        let mut response = self
            .client
            .get(&asset.browser_download_url)
            .header("Accept", "application/octet-stream")
            .header("X-GitHub-Api-Version", API_VERSION)
            .send()
            .await
            .context("failed to download GitHub release")?;

        if !response.status().is_success() {
            bail!(
                "GitHub returned {} while downloading {}",
                response.status(),
                asset.name
            );
        }

        let total_size = response.content_length();

        let progress = match total_size {
            Some(total) => indicatif::ProgressBar::new(total),
            None => indicatif::ProgressBar::new_spinner(),
        };

        progress.set_style(
            indicatif::ProgressStyle::with_template(
                "{spinner:.green} {msg} {bar:30} {bytes}/{total_bytes} ({percent}%)",
            )?
            .progress_chars("█▓░")
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
        );

        progress.set_message(format!("Downloading {}", asset.name));

        let mut file = tokio::fs::File::create(&archive)
            .await
            .with_context(|| format!("failed to create {}", archive.display()))?;

        while let Some(chunk) = response.chunk().await? {
            tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;

            progress.inc(chunk.len() as u64);

            // For responses without Content-Length, indicatif's spinner
            // still needs to be refreshed.
            if total_size.is_none() {
                progress.set_message(format!(
                    "Downloading {}  {}",
                    asset.name,
                    format_bytes(progress.position()),
                ));

                progress.tick();
            }
        }

        progress.finish_with_message(format!(
            "Downloaded {} ({})",
            asset.name,
            format_bytes(progress.position()),
        ));

        Ok(archive)
    }

    fn extract(archive: &Path, destination: &Path) -> Result<()> {
        let name = archive
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();

        if name.ends_with(".zip") {
            let file = std::fs::File::open(archive)?;
            let mut zip = zip::ZipArchive::new(file)?;

            zip.extract(destination)?;

            return Ok(());
        }

        if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
            let file = std::fs::File::open(archive)?;
            let decoder = flate2::read::GzDecoder::new(file);
            let mut archive = tar::Archive::new(decoder);

            archive.unpack(destination)?;

            return Ok(());
        }

        bail!("unsupported archive format `{name}`");
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];

    let mut size = bytes as f64;
    let mut unit = 0;

    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }

    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{:.1} {}", size, UNITS[unit])
    }
}

#[async_trait::async_trait]
impl ToolBackend for GitHubBackend {
    fn name(&self) -> &'static str {
        "github"
    }

    fn supports(&self, requirement: &ToolRequirement) -> bool {
        requirement.backend.as_deref() == Some("github")
    }

    async fn resolve(&self, requirement: &ToolRequirement) -> Result<ToolVersion> {
        let release = self.release(requirement).await?;

        Ok(ToolVersion::new(release.tag_name.trim_start_matches('v')))
    }

    async fn install(
        &self,
        requirement: &ToolRequirement,
        _version: &ToolVersion,
        destination: &Path,
    ) -> Result<()> {
        let release = self.release(requirement).await?;

        let asset = self.select_asset(&release)?;

        let temp = tempfile::tempdir()?;

        let archive = self.download(asset, temp.path()).await?;

        Self::extract(&archive, destination)?;

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

fn platform_target() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "darwin-arm64",
        ("macos", "x86_64") => "darwin-amd64",
        ("linux", "aarch64") => "linux-arm64",
        ("linux", "x86_64") => "linux-amd64",
        ("windows", "x86_64") => "windows-amd64",
        _ => "unknown",
    }
}
