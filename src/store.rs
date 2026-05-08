//! Persistence: ~/.amar/ layout, campaign load/save, autosave.

use crate::calendar::AmarDate;
use crate::pc::Character;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub fn root_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".amar")
}

pub fn config_path() -> PathBuf { root_dir().join("config.toml") }

pub fn campaigns_dir() -> PathBuf { root_dir().join("campaigns") }

pub fn campaign_dir(name: &str) -> PathBuf {
    campaigns_dir().join(sanitize(name))
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    pub active_campaign: Option<String>,
    /// Width of the left pane in two-pane tabs, on a 1-6 scale (kastrup
    /// convention). Computed as `(cols - 4) × width / 10`. 3 ≈ 30%
    /// of width — a comfortable default that matches the original
    /// fixed-30-col layout.
    #[serde(default = "default_pane_width")]
    pub pane_width: u8,
}

fn default_pane_width() -> u8 { 3 }

impl Default for GlobalConfig {
    fn default() -> Self {
        Self { active_campaign: None, pane_width: default_pane_width() }
    }
}

impl GlobalConfig {
    pub fn load() -> Self {
        std::fs::read_to_string(config_path())
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }
    pub fn save(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(root_dir())?;
        let s = toml::to_string_pretty(self).unwrap_or_default();
        std::fs::write(config_path(), s)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Campaign {
    pub name: String,
    pub date: AmarDate,
    pub bortle: u8,
    pub pcs: Vec<Character>,
    pub npcs: Vec<Character>,
}

impl Campaign {
    pub fn new(name: &str) -> Self {
        let mut c = Campaign::default();
        c.name = name.to_string();
        c.date = AmarDate::default();
        c.bortle = 4;
        c
    }

    pub fn save(&self) -> std::io::Result<()> {
        let dir = campaign_dir(&self.name);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("campaign.json");
        let s = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, s)
    }

    pub fn load(name: &str) -> std::io::Result<Self> {
        let path = campaign_dir(name).join("campaign.json");
        let s = std::fs::read_to_string(path)?;
        serde_json::from_str(&s)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
    }
}

/// List all campaign directories under ~/.amar/campaigns/.
pub fn list_campaigns() -> Vec<String> {
    let dir = campaigns_dir();
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new(); };
    let mut names: Vec<String> = entries.flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let manifest = e.path().join("campaign.json");
            if manifest.exists() {
                std::fs::read_to_string(manifest).ok()
                    .and_then(|s| serde_json::from_str::<Campaign>(&s).ok())
                    .map(|c| c.name)
            } else {
                None
            }
        })
        .collect();
    names.sort();
    names
}
