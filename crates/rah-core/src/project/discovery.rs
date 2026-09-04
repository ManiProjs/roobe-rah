use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::detection::{
    Detection, ProjectDetector, git::GitDetector, go::GoDetector, node::NodeDetector,
    python::PythonDetector, rust::RustDetector, tauri::TauriDetector,
};

#[derive(Debug, Clone)]
pub struct Project {
    pub root: PathBuf,
    pub detections: Vec<Detection>,
}

impl Project {
    pub fn discover(start: impl AsRef<Path>) -> Result<Self> {
        let start = start.as_ref();

        let mut current = start
            .canonicalize()
            .with_context(|| format!("failed to resolve {}", start.display()))?;

        if current.is_file() {
            current.pop();
        }

        loop {
            let detections = detect_all(&current);

            if !detections.is_empty() {
                return Ok(Self {
                    root: current,
                    detections,
                });
            }

            if !current.pop() {
                break;
            }
        }

        anyhow::bail!("could not find a project starting from {}", start.display());
    }
}

fn detect_all(root: &Path) -> Vec<Detection> {
    let detectors: &[Box<dyn ProjectDetector>] = &[
        Box::new(GitDetector),
        Box::new(RustDetector),
        Box::new(NodeDetector),
        Box::new(PythonDetector),
        Box::new(GoDetector),
        Box::new(TauriDetector),
    ];

    detectors
        .iter()
        .filter_map(|detector| detector.detect(root))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn discovers_project_from_current_directory() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        let project = Project::discover(dir.path()).unwrap();

        assert_eq!(project.root, dir.path().canonicalize().unwrap());
        assert_eq!(project.detections.len(), 1);
        assert_eq!(project.detections[0].name, "Rust");
    }

    #[test]
    fn discovers_project_from_nested_directory() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        let nested = dir.path().join("src").join("components");
        fs::create_dir_all(&nested).unwrap();

        let project = Project::discover(&nested).unwrap();

        assert_eq!(project.root, dir.path().canonicalize().unwrap());
    }

    #[test]
    fn detects_multiple_project_types() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        fs::write(dir.path().join("package.json"), "{}").unwrap();

        fs::create_dir(dir.path().join("src-tauri")).unwrap();

        let project = Project::discover(dir.path()).unwrap();

        assert_eq!(project.detections.len(), 3);

        assert!(project.detections.iter().any(|d| d.name == "Rust"));
        assert!(project.detections.iter().any(|d| d.name == "Node.js"));
        assert!(project.detections.iter().any(|d| d.name == "Tauri"));
    }

    #[test]
    fn discovers_git_project() {
        let dir = tempdir().unwrap();

        fs::create_dir(dir.path().join(".git")).unwrap();

        let project = Project::discover(dir.path()).unwrap();

        assert_eq!(project.detections.len(), 1);
        assert_eq!(project.detections[0].name, "Git");
    }

    #[test]
    fn discovers_python_project() {
        let dir = tempdir().unwrap();

        fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"test\"",
        )
        .unwrap();

        let project = Project::discover(dir.path()).unwrap();

        assert_eq!(project.detections.len(), 1);
        assert_eq!(project.detections[0].name, "Python");
    }

    #[test]
    fn discovers_go_project() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("go.mod"), "module example.com/test").unwrap();

        let project = Project::discover(dir.path()).unwrap();

        assert_eq!(project.detections.len(), 1);
        assert_eq!(project.detections[0].name, "Go");
    }

    #[test]
    fn fails_when_no_project_is_found() {
        let dir = tempdir().unwrap();

        let result = Project::discover(dir.path());

        assert!(result.is_err());
    }
}
