use std::sync::Arc;

use anyhow::{Result, bail};

use super::{backend::ToolBackend, github::GitHubBackend, requirement::ToolRequirement};

pub struct ToolRegistry {
    backends: Vec<Arc<dyn ToolBackend>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            backends: vec![Arc::new(GitHubBackend::new())],
        }
    }

    pub fn resolve_backend(&self, requirement: &ToolRequirement) -> Result<Arc<dyn ToolBackend>> {
        if let Some(name) = &requirement.backend {
            return self
                .backends
                .iter()
                .find(|backend| backend.name() == name)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("unknown backend `{name}`"));
        }

        for backend in &self.backends {
            if backend.supports(requirement) {
                return Ok(backend.clone());
            }
        }

        bail!("no backend found for `{requirement}`");
    }
}
