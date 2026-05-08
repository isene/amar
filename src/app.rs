//! Application shell — 5-tab TUI built on crust panes.
//!
//! Tab dispatch: number keys 1..=5 jump directly, TAB / S-TAB cycle.
//! Each tab owns its own render method but shares the App's state
//! (Canon, Campaign, GlobalConfig). Idle path is a single blocking
//! `Input::getchr` — no timers, no polling.

use crate::canon::Canon;
use crate::store::{Campaign, GlobalConfig, list_campaigns};
use crust::{Crust, Input, Pane};
use crust::style;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Session,
    Forge,
    Campaign,
    Lore,
    Inspire,
}

impl Tab {
    fn name(self) -> &'static str {
        match self {
            Tab::Session => "Session",
            Tab::Forge => "Forge",
            Tab::Campaign => "Campaign",
            Tab::Lore => "Lore",
            Tab::Inspire => "Inspire",
        }
    }
    fn all() -> [Tab; 5] { [Tab::Session, Tab::Forge, Tab::Campaign, Tab::Lore, Tab::Inspire] }
    fn next(self) -> Tab {
        let all = Tab::all();
        let i = all.iter().position(|t| *t == self).unwrap_or(0);
        all[(i + 1) % all.len()]
    }
    fn prev(self) -> Tab {
        let all = Tab::all();
        let i = all.iter().position(|t| *t == self).unwrap_or(0);
        all[(i + all.len() - 1) % all.len()]
    }
}

pub struct App {
    pub canon: Canon,
    pub config: GlobalConfig,
    pub campaign: Option<Campaign>,
    pub tab: Tab,
    pub cols: u16,
    pub rows: u16,
    pub header: Pane,
    pub body: Pane,
    pub footer: Pane,
    pub status: Option<(String, u8)>,
}

impl App {
    pub fn new() -> Self {
        let canon = Canon::load();
        let config = GlobalConfig::load();
        let campaign = config.active_campaign.as_deref()
            .and_then(|n| Campaign::load(n).ok());
        let (cols, rows) = Crust::terminal_size();
        let mut header = Pane::new(1, 1, cols, 1, 255, 236);
        header.wrap = false;
        header.scroll = false;
        let mut body = Pane::new(1, 2, cols, rows.saturating_sub(2), 252, 0);
        body.wrap = true;
        let mut footer = Pane::new(1, rows, cols, 1, 245, 236);
        footer.wrap = false;
        footer.scroll = false;
        Self {
            canon, config, campaign, tab: Tab::Campaign,
            cols, rows, header, body, footer, status: None,
        }
    }

    pub fn run(&mut self) {
        Crust::clear_screen();
        self.render_all();
        loop {
            let Some(key) = Input::getchr(None) else { continue };
            match key.as_str() {
                "q" | "Q" => {
                    if let Some(ref c) = self.campaign { let _ = c.save(); }
                    let _ = self.config.save();
                    break;
                }
                "1" => { self.tab = Tab::Session;  self.render_all(); }
                "2" => { self.tab = Tab::Forge;    self.render_all(); }
                "3" => { self.tab = Tab::Campaign; self.render_all(); }
                "4" => { self.tab = Tab::Lore;     self.render_all(); }
                "5" => { self.tab = Tab::Inspire;  self.render_all(); }
                "TAB" => { self.tab = self.tab.next(); self.render_all(); }
                "S-TAB" | "BTAB" => { self.tab = self.tab.prev(); self.render_all(); }
                "?" => self.show_help(),
                "C" => { self.campaign_create(); self.render_all(); }
                "L" => { self.campaign_load(); self.render_all(); }
                "r" => self.render_all(),
                "ESC" => { self.status = None; self.render_footer(); }
                other => {
                    self.handle_tab_key(other);
                    self.render_all();
                }
            }
        }
        Crust::clear_screen();
    }

    fn handle_tab_key(&mut self, _key: &str) {
        // Tab-specific keys land here in later versions (Forge sub-nav,
        // Lore tree navigation, Session combat actions, …). v0.1.0
        // tabs are mostly read-only so there's nothing to do yet.
    }

    pub fn render_all(&mut self) {
        self.render_header();
        self.render_body();
        self.render_footer();
    }

    fn render_header(&mut self) {
        let date_str = self.campaign.as_ref()
            .map(|c| c.date.fmt_header())
            .unwrap_or_else(|| "(no campaign)".to_string());
        let camp_str = self.campaign.as_ref()
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "(no campaign)".to_string());

        // Tab strip: highlight active.
        let mut tab_strip = String::new();
        for (i, t) in Tab::all().iter().enumerate() {
            let label = format!(" [{}] {} ", i + 1, t.name());
            if *t == self.tab {
                tab_strip.push_str(&style::bold(&style::bg(&label, 24)));
            } else {
                tab_strip.push_str(&style::fg(&label, 245));
            }
        }
        let line = format!(" {}    {}    {}", style::bold("amar"), tab_strip, style::fg(&format!("{} | {}", camp_str, date_str), 252));
        self.header.say(&line);
    }

    fn render_body(&mut self) {
        let lines = match self.tab {
            Tab::Session  => self.render_session(),
            Tab::Forge    => self.render_forge(),
            Tab::Campaign => self.render_campaign(),
            Tab::Lore     => self.render_lore(),
            Tab::Inspire  => self.render_inspire(),
        };
        self.body.set_text(&lines.join("\n"));
        self.body.full_refresh();
    }

    fn render_footer(&mut self) {
        if let Some((ref msg, color)) = self.status {
            let pad = self.cols.saturating_sub(crust::display_width(msg) as u16 + 12 + 1) as usize;
            let right = format!("amar v{}", VERSION);
            let line = format!("{}{}{}", style::fg(msg, color), " ".repeat(pad), style::fg(&right, 244));
            self.footer.say(&line);
            return;
        }
        let hint = match self.tab {
            Tab::Session  => " 1-5:tabs  TAB:next  C:new-campaign  L:load  ?:help  q:quit",
            Tab::Forge    => " 1-5:tabs  TAB:next  C:new-campaign  L:load  ?:help  q:quit",
            Tab::Campaign => " 1-5:tabs  TAB:next  C:new-campaign  L:load-campaign  ?:help  q:quit",
            Tab::Lore     => " 1-5:tabs  TAB:next  C:new-campaign  L:load  ?:help  q:quit",
            Tab::Inspire  => " 1-5:tabs  TAB:next  C:new-campaign  L:load  ?:help  q:quit",
        };
        // Right-align the version. Pad with spaces between hint and version.
        let right = format!("amar v{} ", VERSION);
        let hw = crust::display_width(hint);
        let rw = crust::display_width(&right);
        let pad = (self.cols as usize).saturating_sub(hw + rw);
        let line = format!("{}{}{}", style::fg(hint, 244), " ".repeat(pad), style::fg(&right, 244));
        self.footer.say(&line);
    }

    // --- Tab body renderers ---

    fn render_session(&self) -> Vec<String> {
        let mut out = Vec::new();
        out.push(String::new());
        out.push(style::bold("  Session").to_string());
        out.push(String::new());
        match &self.campaign {
            Some(_) => {
                out.push("  No active adventure. Use Campaign tab to create or load one.".into());
                out.push(String::new());
                out.push(style::fg("  (Initiative tracker, party block, in-game clock, weather rolls, and session log land in v0.5.0.)", 245).to_string());
            }
            None => {
                out.push("  No campaign loaded. Press C to create one or L to load an existing campaign.".into());
            }
        }
        out
    }

    fn render_forge(&self) -> Vec<String> {
        let mut out = Vec::new();
        out.push(String::new());
        out.push(style::bold("  Forge").to_string());
        out.push(String::new());
        out.push(format!("  Canon: {} entries — {} spells, {} rituals, {} potions",
            self.canon.entries.len(),
            self.canon.spell_count(),
            self.canon.ritual_count(),
            self.canon.potion_count()));
        out.push(String::new());
        out.push(style::fg("  Generators arrive in v0.2.0+:", 245).to_string());
        out.push(style::fg("    NPC + Name + Weather (v0.2.0)", 245).to_string());
        out.push(style::fg("    Encounter + Treasure (v0.3.0)", 245).to_string());
        out.push(style::fg("    Town (v0.4.0)", 245).to_string());
        out.push(style::fg("    Adventure (v0.6.0, AI-driven via Inspire)", 245).to_string());
        out
    }

    fn render_campaign(&self) -> Vec<String> {
        let mut out = Vec::new();
        out.push(String::new());
        out.push(style::bold("  Campaign").to_string());
        out.push(String::new());
        match &self.campaign {
            Some(c) => {
                out.push(format!("  Name: {}", c.name));
                out.push(format!("  Date: {}", c.date.fmt_header()));
                out.push(format!("  Bortle: {}", c.bortle));
                out.push(format!("  PCs: {}", c.pcs.len()));
                out.push(format!("  NPCs: {}", c.npcs.len()));
                out.push(String::new());
                out.push(style::fg("  PC entry, NPC roster, locations, adventures, factions land in v0.4.0.", 245).to_string());
                out.push(style::fg("  Calendar advance + weather hookup land in v0.5.0.", 245).to_string());
            }
            None => {
                out.push("  No campaign loaded.".into());
                out.push(String::new());
                out.push("  C — create a new campaign".into());
                out.push("  L — load an existing campaign".into());
                let existing = list_campaigns();
                if !existing.is_empty() {
                    out.push(String::new());
                    out.push(style::fg("  Existing campaigns:", 245).to_string());
                    for n in existing.iter().take(20) {
                        out.push(format!("    - {}", n));
                    }
                }
            }
        }
        out
    }

    fn render_lore(&self) -> Vec<String> {
        let mut out = Vec::new();
        out.push(String::new());
        out.push(style::bold("  Lore").to_string());
        out.push(String::new());
        out.push("  Three sources, all available offline:".into());
        out.push(String::new());
        out.push(format!("    Wiki canon  — {} entries scraped from d6gaming.org",
            self.canon.entries.len()));
        out.push("    Setting     — Mythology, Kingdom of Amar, World, Calendar".into());
        out.push("    Author      — Death spells, magic items (filling wiki gaps)".into());
        out.push(String::new());
        out.push(style::fg("  Tree navigation + content viewer + search land in v0.1.x", 245).to_string());
        out.push(String::new());
        out.push(style::bold("  Quick reference — d6gaming.org canon").to_string());
        let mut cats: Vec<_> = self.canon.domain_index.iter().collect();
        cats.sort_by_key(|(k, _)| (*k).clone());
        for (cat, list) in cats {
            out.push(format!("    {:<22} {} entries", cat, list.len()));
        }
        out
    }

    fn render_inspire(&self) -> Vec<String> {
        let mut out = Vec::new();
        out.push(String::new());
        out.push(style::bold("  Inspire").to_string());
        out.push(String::new());
        out.push("  AI-assisted brainstorming via the Claude CLI (claude -p).".into());
        out.push(String::new());
        out.push(style::fg("  Modes (v0.6.0):", 245).to_string());
        out.push(style::fg("    Adventure        — generate a full adventure (with deterministic fallback)", 245).to_string());
        out.push(style::fg("    NPC voice        — flesh out a saved NPC's personality + speech", 245).to_string());
        out.push(style::fg("    Location desc    — vivid description of a saved location", 245).to_string());
        out.push(style::fg("    Session recap    — turn the session log into prose", 245).to_string());
        out.push(style::fg("    Plot threads     — what could happen next?", 245).to_string());
        out.push(style::fg("    Free-form        — your own prompt with full canon context", 245).to_string());
        out.push(String::new());
        out.push(style::fg("  Each call is gated behind a key press; no background polling.", 245).to_string());
        out
    }

    // --- Campaign lifecycle ---

    fn campaign_create(&mut self) {
        let name = self.footer.ask(" New campaign name: ", "");
        let name = name.trim().to_string();
        if name.is_empty() {
            self.status = Some(("Cancelled.".into(), 208));
            return;
        }
        let c = Campaign::new(&name);
        if let Err(e) = c.save() {
            self.status = Some((format!("Save failed: {}", e), 196));
            return;
        }
        self.config.active_campaign = Some(name.clone());
        let _ = self.config.save();
        self.campaign = Some(c);
        self.status = Some((format!("Created campaign '{}'.", name), 46));
    }

    fn campaign_load(&mut self) {
        let existing = list_campaigns();
        if existing.is_empty() {
            self.status = Some(("No campaigns yet — press C to create one.".into(), 208));
            return;
        }
        let initial = existing[0].clone();
        let name = self.footer.ask(
            &format!(" Load campaign ({}): ", existing.join(", ")),
            &initial,
        );
        let name = name.trim().to_string();
        match Campaign::load(&name) {
            Ok(c) => {
                self.config.active_campaign = Some(name.clone());
                let _ = self.config.save();
                self.campaign = Some(c);
                self.status = Some((format!("Loaded '{}'.", name), 46));
            }
            Err(e) => {
                self.status = Some((format!("Load failed: {}", e), 196));
            }
        }
    }

    fn show_help(&mut self) {
        let help = format!("\n  \
            amar v{} - Amar RPG companion\n  \
            5-tab TUI honoring d6gaming.org canon.\n\n  \
            TABS\n  \
              1   Session    Live in-game tools (combat, party, log)\n  \
              2   Forge      Generators (NPC, encounter, town, weather, …)\n  \
              3   Campaign   Persistent state — PCs, NPCs, locations, adventures\n  \
              4   Lore       Browsable canon (wiki + setting + author additions)\n  \
              5   Inspire    AI-assisted brainstorming (claude -p)\n\n  \
            NAVIGATION\n  \
              1-5     Jump to tab\n  \
              TAB     Next tab\n  \
              S-TAB   Previous tab\n  \
              ?       This help\n\n  \
            CAMPAIGN\n  \
              C       Create a new campaign\n  \
              L       Load an existing campaign\n\n  \
            OTHER\n  \
              r       Redraw\n  \
              ESC     Clear status line\n  \
              q / Q   Quit (saves campaign + config)\n\n  \
            Data: ~/.amar/campaigns/<name>/\n  \
            Canon: bundled, scraped from d6gaming.org\n  \
            ESC closes this popup.", VERSION);
        let (cols, rows) = Crust::terminal_size();
        let w = cols.saturating_sub(8).min(76);
        let h = rows.saturating_sub(4).min(28);
        let mut popup = crust::Popup::centered(w, h, 252, 234);
        let _ = popup.modal(&help);
        Crust::clear_screen();
        self.render_all();
    }
}
