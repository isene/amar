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

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// One diary line attached to an in-world day. Several entries may share a
/// date (the GM jots more than one as play unfolds); they render in the
/// order written. `created_at` is the real-world unix time it was written.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiaryEntry {
    pub date: AmarDate,
    pub text: String,
    #[serde(default)]
    pub created_at: u64,
}

/// A place the party knows: city, keep, region, inn, ruin… `image` is
/// an absolute path to a map / illustration rendered inline under the
/// text (and openable externally with →). Sparse-friendly: everything
/// but the name may be omitted in injected JSON.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Location {
    pub name: String,
    /// Free-form kind: "Dwarven mountain-capital", "Inn", "Region", …
    pub kind: String,
    pub description: String,
    /// Absolute path to a map / illustration (optional).
    pub image: String,
    pub notes: String,
    pub created_at: u64,
}

/// The shared WORLD: locations and the major NPCs that exist across
/// every campaign (royals, barons, famous adventurers…). Campaigns are
/// time-boxed groups of players moving through this world; what they
/// meet that is theirs alone (lesser NPCs, PCs, adventures) lives in
/// the campaign instead. Stored at `~/.amar/world.json`, edited live
/// by the companion session under the same contract as campaign.json.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct World {
    pub locations: Vec<Location>,
    pub npcs: Vec<crate::pc::Character>,
}

pub fn world_path() -> PathBuf { root_dir().join("world.json") }

impl World {
    pub fn load() -> World {
        let mut w: World = std::fs::read_to_string(world_path())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        for n in w.npcs.iter_mut() { n.normalize(); }
        w
    }

    pub fn save(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(root_dir())?;
        let s = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let tmp = world_path().with_extension("json.tmp");
        std::fs::write(&tmp, s)?;
        std::fs::rename(&tmp, world_path())
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
    /// Places the party knows — towns, keeps, regions, inns. Browsable
    /// under the Locations section; searchable with `/`.
    #[serde(default)]
    pub locations: Vec<Location>,
    /// The campaign diary: free-form lines the GM writes against in-world
    /// days from the Calendar. This is the running record of the campaign.
    #[serde(default)]
    pub diary: Vec<DiaryEntry>,
    /// Rolling weather run that always covers the current day plus a week
    /// ahead. Past days stay immutable so the diary's weather never changes
    /// retroactively; only the tail is extended as the current day advances.
    #[serde(default)]
    pub forecast: Vec<crate::forge::WeatherDay>,
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

    fn lin(d: AmarDate) -> i64 {
        d.year as i64 * crate::calendar::DAYS_PER_YEAR as i64 + d.day_of_year as i64
    }

    /// Diary lines recorded on `date`, in the order they were written.
    pub fn diary_for(&self, date: AmarDate) -> Vec<&DiaryEntry> {
        self.diary.iter().filter(|e| e.date == date).collect()
    }

    /// Append a diary line to `date`.
    pub fn add_diary(&mut self, date: AmarDate, text: &str) {
        self.diary.push(DiaryEntry { date, text: text.to_string(), created_at: now_unix() });
    }

    /// The generated weather for `date`, if it falls inside the current run.
    pub fn weather_for(&self, date: AmarDate) -> Option<&crate::forge::WeatherDay> {
        self.forecast.iter().find(|w| w.date == date)
    }

    /// Make sure the weather run covers `self.date ..= self.date + horizon-1`.
    /// Generates only the missing tail (or reseeds if the run is empty or
    /// starts after today), so past days stay stable and the cost is zero
    /// once the week ahead already exists.
    pub fn ensure_forecast(&mut self, horizon: u32) {
        let horizon = horizon.max(1);
        let target_end = self.date.advance(horizon as i64 - 1);
        let starts_after_today = self.forecast.first()
            .map(|w| Self::lin(w.date) > Self::lin(self.date))
            .unwrap_or(true);
        if starts_after_today {
            self.forecast = crate::forge::generate_weather(self.date, horizon);
        }
        if let Some(last) = self.forecast.last().map(|w| w.date) {
            let gap = Self::lin(target_end) - Self::lin(last);
            if gap > 0 {
                let more = crate::forge::generate_weather(last.advance(1), gap as u32);
                self.forecast.extend(more);
            }
        }
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
        let mut c: Campaign = serde_json::from_str(&s)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        // Normalize every character on the way in: a companion session
        // may inject sparse "short form" JSON (name, level, a few skills,
        // weapons); this fills the gaps so it behaves like a full sheet.
        // Idempotent for fully-populated characters.
        for p in c.pcs.iter_mut() { p.normalize(); }
        for n in c.npcs.iter_mut() { n.normalize(); }
        for s in c.saved_npcs.iter_mut() { s.item.normalize(); }
        for e in c.saved_encounters.iter_mut() {
            for n in e.item.npcs.iter_mut() { n.normalize(); }
        }
        Ok(c)
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
