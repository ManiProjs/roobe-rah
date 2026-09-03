use std::path::{Path, PathBuf};

use crate::detection::{Detection, detect};

#[derive(Debug, Clone)]
pub struct Project {
    root: PathBuf,
}

impl Project {
    pub fn discover(start: impl AsRef<Path>) -> Option<Self> {
        let mut current = start.as_ref().canonicalize().ok()?;

        loop {
            if Self::looks_like_project(&current) {
                return Some(Self { root: current });
            }

            if !current.pop() {
                return None;
            }
        }
    }

    fn looks_like_project(path: &Path) -> bool {
        path.join(".git").exists()
            || path.join("Cargo.toml").is_file()
            || path.join("package.json").is_file()
            || path.join("pyproject.toml").is_file()
            || path.join("requirements.txt").is_file()
            || path.join("setup.py").is_file()
            || path.join("go.mod").is_file()
            || path.join("src-tauri").is_dir()
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn detect(&self) -> Vec<Detection> {
        detect(&self.root)
    }

    pub fn has_rah_config(&self) -> bool {
        self.root.join("rah.toml").is_file()
    }
}
