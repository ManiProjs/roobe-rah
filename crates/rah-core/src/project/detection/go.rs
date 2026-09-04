use std::path::Path;

use super::{Detection, DetectorKind, ProjectDetector};

pub struct GoDetector;

impl ProjectDetector for GoDetector {
    fn detect(&self, root: &Path) -> Option<Detection> {
        if root.join("go.mod").is_file() {
            Some(Detection {
                kind: DetectorKind::Go,
                name: "Go",
                reason: "go.mod exists".into(),
            })
        } else {
            None
        }
    }
}
