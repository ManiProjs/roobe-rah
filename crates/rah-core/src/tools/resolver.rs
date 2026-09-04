use std::path::Path;

use super::{requirement::ToolRequirement, version::ToolVersion};

#[derive(Debug, Clone)]
pub struct ResolvedTool {
    pub requirement: ToolRequirement,
    pub version: ToolVersion,
    pub executable: String,
    pub path: Option<std::path::PathBuf>,
}

pub trait ToolResolver {
    fn resolve(&self, requirement: &ToolRequirement, root: &Path) -> anyhow::Result<ResolvedTool>;
}
