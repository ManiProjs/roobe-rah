use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct RahConfig {
    #[serde(default)]
    pub tools: BTreeMap<String, String>,

    #[serde(default)]
    pub env: BTreeMap<String, String>,

    #[serde(default)]
    pub tasks: BTreeMap<String, Task>,

    #[serde(default)]
    pub settings: Settings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Task {
    pub run: Option<String>,

    #[serde(default)]
    pub depends: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub experimental: bool,
}
