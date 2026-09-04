use std::path::Path;

use super::{Detection, DetectorKind, ProjectDetector};

pub struct TauriDetector;

impl ProjectDetector for TauriDetector {
    fn detect(&self, root: &Path) -> Option<Detection> {
        if root.join("src-tauri").is_dir() {
            Some(Detection {
                kind: DetectorKind::Tauri,
                name: "Tauri",
                reason: "src-tauri exists".into(),
            })
        } else {
            None
        }
    }
}
