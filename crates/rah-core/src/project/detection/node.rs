use std::path::Path;

use super::{Detection, DetectorKind, ProjectDetector};

pub struct NodeDetector;

impl ProjectDetector for NodeDetector {
    fn detect(&self, root: &Path) -> Option<Detection> {
        if root.join("package.json").is_file() {
            Some(Detection {
                kind: DetectorKind::Node,
                name: "Node.js",
                reason: "package.json exists".into(),
            })
        } else {
            None
        }
    }
}
