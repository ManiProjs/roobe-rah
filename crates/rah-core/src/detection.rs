use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectorKind {
    Git,
    Rust,
    Node,
    Python,
    Go,
    Tauri,
}

#[derive(Debug, Clone)]
pub struct Detection {
    pub kind: DetectorKind,
    pub name: &'static str,
    pub reason: String,
}

impl Detection {
    pub fn new(kind: DetectorKind, name: &'static str, reason: impl Into<String>) -> Self {
        Self {
            kind,
            name,
            reason: reason.into(),
        }
    }
}

pub fn detect(root: &Path) -> Vec<Detection> {
    let mut detections = Vec::new();

    if root.join(".git").exists() {
        detections.push(Detection::new(
            DetectorKind::Git,
            "Git",
            "Found .git directory",
        ));
    }

    if root.join("Cargo.toml").is_file() {
        detections.push(Detection::new(
            DetectorKind::Rust,
            "Rust",
            "Found Cargo.toml",
        ));
    }

    if root.join("package.json").is_file() {
        detections.push(Detection::new(
            DetectorKind::Node,
            "Node.js",
            "Found package.json",
        ));
    }

    if root.join("pyproject.toml").is_file()
        || root.join("requirements.txt").is_file()
        || root.join("setup.py").is_file()
    {
        detections.push(Detection::new(
            DetectorKind::Python,
            "Python",
            "Found Python project metadata",
        ));
    }

    if root.join("go.mod").is_file() {
        detections.push(Detection::new(DetectorKind::Go, "Go", "Found go.mod"));
    }

    if root.join("src-tauri").is_dir() {
        detections.push(Detection::new(
            DetectorKind::Tauri,
            "Tauri",
            "Found src-tauri directory",
        ));
    }

    detections
}
