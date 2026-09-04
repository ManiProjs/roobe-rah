use std::path::Path;

use super::{Detection, DetectorKind, ProjectDetector};

pub struct GitDetector;

impl ProjectDetector for GitDetector {
    fn detect(&self, root: &Path) -> Option<Detection> {
        let marker = root.join(".git");

        if marker.exists() {
            Some(Detection {
                kind: DetectorKind::Git,
                name: "Git",
                reason: ".git exists".into(),
            })
        } else {
            None
        }
    }
}
