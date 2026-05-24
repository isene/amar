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
    /// Path to a file containing the OpenAI API key (one line). The
    /// global default lives at `/home/.safe/openai.txt` per the
    /// user's machine convention. Empty / missing → OpenAI image
    /// generation is unavailable.
    #[serde(default = "default_openai_key_path")]
    pub openai_key_path: String,
    /// Path to a file containing the Gemini API key. Empty / missing
    /// → Gemini image generation is unavailable.
    #[serde(default)]
    pub gemini_key_path: String,
    /// Which provider to hit when the user picks "API" on the portrait
    /// menu. "openai" or "gemini". Defaults to "openai".
    #[serde(default = "default_image_provider")]
    pub image_provider: String,
}

fn default_pane_width() -> u8 { 3 }
fn default_openai_key_path() -> String { "/home/.safe/openai.txt".into() }
fn default_image_provider() -> String { "openai".into() }

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            active_campaign: None,
            pane_width: default_pane_width(),
            openai_key_path: default_openai_key_path(),
            gemini_key_path: String::new(),
            image_provider: default_image_provider(),
        }
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

/// Generic save-wrapper for forge artefacts. Keeps the user-given
/// label, the unix timestamp the roll was kept at, the AI flavour
/// (if `A` was pressed before save), and the artefact itself. The
/// `id` is unique within its vector and lets the Campaign tab refer
/// to an item across deletes / re-orders without depending on
/// position.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Saved<T> {
    pub id: u64,
    pub name: String,
    pub created_at: u64,
    #[serde(default)]
    pub flavour: Option<String>,
    pub item: T,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Campaign {
    pub name: String,
    pub date: AmarDate,
    pub bortle: u8,
    pub pcs: Vec<Character>,
    pub npcs: Vec<Character>,
    /// Forge artefacts persisted into the campaign. Every field is
    /// `#[serde(default)]` so existing campaign.json files from
    /// before the "saved generators" feature still load — empty
    /// vectors fill in on first save.
    #[serde(default)]
    pub saved_encounters: Vec<Saved<crate::forge::encounter::Encounter>>,
    #[serde(default)]
    pub saved_towns: Vec<Saved<crate::forge::town::Town>>,
    #[serde(default)]
    pub saved_weather: Vec<Saved<crate::forge::WeatherDay>>,
    #[serde(default)]
    pub saved_npcs: Vec<Saved<Character>>,
    /// Adventures imported into this campaign. Each one references an
    /// on-disk directory (no copying) and indexes its narrative +
    /// scene / floorplan / NPC-portrait assets.
    #[serde(default)]
    pub adventures: Vec<crate::adventure::Adventure>,
    /// `id` of whichever adventure the GM is currently running. The
    /// app uses this to render a "Active: <name> · §<section>" hint
    /// in the status line so resuming next session is friction-free.
    #[serde(default)]
    pub active_adventure_id: Option<u64>,
    /// Active fight on the Combat tab. `None` between fights; set
    /// when `C` is pressed with a non-empty tag pool, scrubbed when
    /// `C` is pressed on the Combat tab itself with `y` confirm.
    #[serde(default)]
    pub combat: Option<crate::combat::CombatState>,
    /// Cross-source tag pool. Filled by `t` on any browsable row
    /// (PC, NPC, encounter). Drained when `C` launches a combat;
    /// persisted across sessions so a half-built roster survives a
    /// quit.
    #[serde(default)]
    pub tagged: crate::combat::TagPool,
}

/// One participant in the combat HUD. Indexed against
/// `Campaign.pcs` / `Campaign.npcs` (not by ID — neither roster
/// uses stable IDs yet; we patch up on remove instead).
///
/// `EncounterNpc` points into `saved_encounters[enc_idx].item.npcs[npc_idx]`
/// so a rolled batch (e.g. nine giant rats) can be tagged per-instance
/// without copying the stat blocks into the campaign NPC roster.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CombatRef {
    Pc(usize),
    Npc(usize),
    EncounterNpc { enc_idx: usize, npc_idx: usize },
}

impl Campaign {
    pub fn new(name: &str) -> Self {
        let mut c = Campaign::default();
        c.name = name.to_string();
        c.date = AmarDate::default();
        c.bortle = 4;
        c
    }

    /// Promote NPC-portrait assets on the indexed adventure into
    /// stub `Character` rows on `self.npcs`. Idempotent: skips
    /// portraits whose name OR absolute portrait path already
    /// matches an existing NPC. Clears the adventure's
    /// `npc_portraits` list so NPCs render at one place
    /// (campaign-level) instead of duplicated under each adventure.
    /// Returns the count of new NPCs created.
    pub fn promote_adventure_portraits_to_npcs(&mut self, adv_idx: usize) -> usize {
        let portraits: Vec<(String, String)> = {
            let Some(adv) = self.adventures.get(adv_idx) else { return 0; };
            adv.npc_portraits.iter()
                .map(|p| (p.name.clone(),
                          adv.absolute(&p.path).to_string_lossy().to_string()))
                .collect()
        };
        let mut created = 0;
        for (name, abs_path) in portraits {
            let already = self.npcs.iter().any(|n|
                n.name == name
                || (!n.portrait_path.is_empty() && n.portrait_path == abs_path));
            if already { continue; }
            let mut ch = Character::new_blank(&name);
            ch.is_pc = false;
            ch.portrait_path = abs_path;
            self.npcs.push(ch);
            created += 1;
        }
        if let Some(adv) = self.adventures.get_mut(adv_idx) {
            adv.npc_portraits.clear();
        }
        created
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
