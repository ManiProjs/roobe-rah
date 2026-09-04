use std::path::Path;

use super::{Detection, DetectorKind, ProjectDetector};

pub struct RustDetector;

impl ProjectDetector for RustDetector {
    fn detect(&self, root: &Path) -> Option<Detection> {
        if root.join("Cargo.toml").is_file() {
            Some(Detection {
                kind: DetectorKind::Rust,
                name: "Rust",
                reason: "Cargo.toml exists".into(),
            })
        } else {
            None
        }
    }
}
