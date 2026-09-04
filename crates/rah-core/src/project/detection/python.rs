use std::path::Path;

use super::{Detection, DetectorKind, ProjectDetector};

pub struct PythonDetector;

impl ProjectDetector for PythonDetector {
    fn detect(&self, root: &Path) -> Option<Detection> {
        let markers = [
            "pyproject.toml",
            "requirements.txt",
            "setup.py",
            "setup.cfg",
        ];

        markers
            .iter()
            .find(|marker| root.join(marker).is_file())
            .map(|marker| Detection {
                kind: DetectorKind::Python,
                name: "Python",
                reason: format!("{marker} exists"),
            })
    }
}
