use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallScope {
    Project,
    Global,
}

impl InstallScope {
    pub fn root(self) -> anyhow::Result<PathBuf> {
        match self {
            Self::Project => {
                anyhow::bail!("project installation root is not implemented yet")
            }

            Self::Global => {
                let home = dirs::home_dir()
                    .ok_or_else(|| anyhow::anyhow!("could not determine home directory"))?;

                Ok(home.join(".local").join("share").join("rah"))
            }
        }
    }
}
