use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_ram_threshold")]
    pub ram_threshold_gb: f64,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    #[serde(default = "default_notify_enabled")]
    pub notify_enabled: bool,
}

fn default_ram_threshold() -> f64 {
    8.0
}
fn default_poll_interval() -> u64 {
    30
}
fn default_notify_enabled() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            ram_threshold_gb: default_ram_threshold(),
            poll_interval_secs: default_poll_interval(),
            notify_enabled: default_notify_enabled(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            toml::from_str(&content).unwrap_or_default()
        } else {
            let config = Self::default();
            config.save();
            config
        }
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(content) = toml::to_string_pretty(self) {
            let _ = std::fs::write(&path, content);
        }
    }

    fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".claude-monitor")
            .join("config.toml")
    }
}
