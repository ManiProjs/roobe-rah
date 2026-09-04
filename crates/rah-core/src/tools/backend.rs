use std::path::Path;

use anyhow::Result;

use super::{requirement::ToolRequirement, version::ToolVersion};

#[async_trait::async_trait]
pub trait ToolBackend: Send + Sync {
    fn name(&self) -> &'static str;

    fn supports(&self, requirement: &ToolRequirement) -> bool;

    async fn resolve(&self, requirement: &ToolRequirement) -> Result<ToolVersion>;

    async fn install(
        &self,
        requirement: &ToolRequirement,
        version: &ToolVersion,
        destination: &Path,
    ) -> Result<()>;
}
