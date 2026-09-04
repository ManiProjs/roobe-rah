use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolVersion {
    pub version: String,
}

impl ToolVersion {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
        }
    }
}

impl fmt::Display for ToolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.version)
    }
}
