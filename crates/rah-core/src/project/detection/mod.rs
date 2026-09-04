use std::path::Path;

pub mod git;
pub mod go;
pub mod node;
pub mod python;
pub mod rust;
pub mod tauri;

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

pub trait ProjectDetector {
    fn detect(&self, root: &Path) -> Option<Detection>;
}
