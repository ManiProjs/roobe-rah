use std::fmt;

use anyhow::{Result, bail};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRequirement {
    pub backend: Option<String>,
    pub name: String,
    pub version: String,
}

impl ToolRequirement {
    pub fn parse(input: &str) -> Result<Self> {
        let (tool, version) = match input.split_once('@') {
            Some((tool, version)) => (tool, version),
            None => (input, "latest"),
        };

        if tool.is_empty() {
            bail!("tool name cannot be empty");
        }

        if version.is_empty() {
            bail!("tool version cannot be empty");
        }

        let (backend, name) = match tool.split_once(':') {
            Some((backend, name)) => {
                if backend.is_empty() || name.is_empty() {
                    bail!("invalid backend specification `{tool}`");
                }

                (Some(backend.to_owned()), name.to_owned())
            }
            None => (None, tool.to_owned()),
        };

        Ok(Self {
            backend,
            name,
            version: version.to_owned(),
        })
    }
}

impl fmt::Display for ToolRequirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(backend) = &self.backend {
            write!(f, "{backend}:")?;
        }

        write!(f, "{}@{}", self.name, self.version)
    }
}
