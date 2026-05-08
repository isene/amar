//! Application shell — 5-tab TUI built on crust panes.
//!
//! Tab dispatch: number keys 1..=5 jump directly, TAB / S-TAB cycle.
//! Each tab owns its own render method but shares the App's state
//! (Canon, Campaign, GlobalConfig). Idle path is a single blocking
//! `Input::getchr` — no timers, no polling.

use crate::canon::Canon;
use crate::lore::{self, Node, Tree};
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

/// Which pane within a multi-pane tab currently owns the cursor.
/// Tabs with a single pane ignore this; tabs with two panes (Lore for
/// now, Campaign / Forge / Inspire later) consult it to route arrow
/// and PgUp/PgDown keys to the correct pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus { Left, Right }

pub struct App {
    pub canon: Canon,
    pub config: GlobalConfig,
    pub campaign: Option<Campaign>,
    pub tab: Tab,
    pub focus: Focus,
    pub cols: u16,
    pub rows: u16,
    pub header: Pane,
    pub body: Pane,
    pub lore_tree_pane: Pane,
    pub lore_content_pane: Pane,
    pub footer: Pane,
    pub status: Option<(String, u8)>,
    /// Lore tab navigation state. `lore_idx` is the cursor in the
    /// flattened tree; the content pane's scroll position lives on the
    /// pane itself (ix), driven by linedown/lineup/pagedown/pageup.
    pub lore_idx: usize,
    pub lore_expanded: Vec<String>,
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

        let body_h = rows.saturating_sub(2);
        let mut body = Pane::new(1, 2, cols, body_h, 252, 0);
        body.wrap = true;

        // Lore panes share the body area: tree on the left (~30 cols),
        // content on the right (rest). Both have scroll markers; the
        // content pane wraps long lines.
        let tree_w: u16 = 30.min(cols.saturating_sub(20));
        let content_w: u16 = cols.saturating_sub(tree_w);
        let mut lore_tree_pane = Pane::new(1, 2, tree_w, body_h, 252, 0);
        lore_tree_pane.wrap = false;
        let mut lore_content_pane = Pane::new(tree_w + 1, 2, content_w, body_h, 252, 0);
        lore_content_pane.wrap = true;

        let mut footer = Pane::new(1, rows, cols, 1, 245, 236);
        footer.wrap = false;
        footer.scroll = false;
        Self {
            canon, config, campaign, tab: Tab::Campaign,
            focus: Focus::Left,
            cols, rows, header, body,
            lore_tree_pane, lore_content_pane,
            footer, status: None,
            lore_idx: 0,
            lore_expanded: Vec::new(),
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
                "1" => { self.set_tab(Tab::Session); }
                "2" => { self.set_tab(Tab::Forge); }
                "3" => { self.set_tab(Tab::Campaign); }
                "4" => { self.set_tab(Tab::Lore); }
                "5" => { self.set_tab(Tab::Inspire); }
                "C-RIGHT" => { self.set_tab(self.tab.next()); }
                "C-LEFT"  => { self.set_tab(self.tab.prev()); }
                "TAB" => {
                    // Toggle pane focus on tabs that have two panes.
                    // Single-pane tabs ignore TAB.
                    if self.tab_has_two_panes() {
                        self.focus = match self.focus {
                            Focus::Left  => Focus::Right,
                            Focus::Right => Focus::Left,
                        };
                        self.render_all();
                    }
                }
                "?" => self.show_help(),
                "C" => { self.campaign_create(); self.render_all(); }
                "L" => { self.campaign_load(); self.render_all(); }
                "r" => self.render_all(),
                "ESC" => {
                    // ESC has two cumulative effects: drop focus back to
                    // the left pane (if currently on the right), then
                    // clear any status message.
                    if self.focus == Focus::Right {
                        self.focus = Focus::Left;
                        self.render_all();
                    }
                    self.status = None;
                    self.render_footer();
                }
                other => {
                    self.handle_tab_key(other);
                    self.render_all();
                }
            }
        }
        Crust::clear_screen();
    }

    fn set_tab(&mut self, t: Tab) {
        self.tab = t;
        // Tabs that have only one pane don't make sense with Right focus.
        if !self.tab_has_two_panes() {
            self.focus = Focus::Left;
        }
        self.render_all();
    }

    fn tab_has_two_panes(&self) -> bool {
        // Lore is the only multi-pane tab today. Future: Campaign sub-tabs
        // and Forge will return true here too.
        matches!(self.tab, Tab::Lore)
    }

    fn handle_tab_key(&mut self, key: &str) {
        if self.tab == Tab::Lore {
            self.handle_lore_key(key);
        }
    }

    fn handle_lore_key(&mut self, key: &str) {
        // Right-pane scroll keys work regardless of focus. They mirror
        // kastrup's right-pane bindings, so the muscle memory carries.
        match key {
            "S-DOWN"  => { self.lore_content_pane.linedown(); return; }
            "S-UP"    => { self.lore_content_pane.lineup();   return; }
            "S-RIGHT" => { self.lore_content_pane.pagedown(); return; }
            "S-LEFT"  => { self.lore_content_pane.pageup();   return; }
            _ => {}
        }
        match self.focus {
            Focus::Left  => self.handle_lore_tree_key(key),
            Focus::Right => self.handle_lore_content_key(key),
        }
    }

    fn handle_lore_tree_key(&mut self, key: &str) {
        let tree = Tree::build(&self.canon, &self.lore_expanded);
        match key {
            "j" | "DOWN" => {
                if self.lore_idx + 1 < tree.len() {
                    self.lore_idx += 1;
                    self.lore_content_pane.ix = 0;
                }
            }
            "k" | "UP" => {
                if self.lore_idx > 0 {
                    self.lore_idx -= 1;
                    self.lore_content_pane.ix = 0;
                }
            }
            "g" => { self.lore_idx = 0; self.lore_content_pane.ix = 0; }
            "G" => {
                self.lore_idx = tree.len().saturating_sub(1);
                self.lore_content_pane.ix = 0;
            }
            "ENTER" | "l" | "RIGHT" => {
                if let Some(item) = tree.get(self.lore_idx) {
                    if let Node::CanonCategory { category, .. } = &item.node {
                        if !self.lore_expanded.iter().any(|e| e == category) {
                            self.lore_expanded.push(category.clone());
                        }
                    }
                }
            }
            "h" | "LEFT" => {
                if let Some(item) = tree.get(self.lore_idx) {
                    match &item.node {
                        Node::CanonCategory { category, .. } => {
                            self.lore_expanded.retain(|e| e != category);
                        }
                        Node::CanonEntry { .. } => {
                            let mut i = self.lore_idx;
                            while i > 0 {
                                i -= 1;
                                if let Some(it) = tree.get(i) {
                                    if let Node::CanonCategory { category, .. } = &it.node {
                                        self.lore_expanded.retain(|e| e != category);
                                        self.lore_idx = i;
                                        self.lore_content_pane.ix = 0;
                                        break;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_lore_content_key(&mut self, key: &str) {
        match key {
            "j" | "DOWN" => self.lore_content_pane.linedown(),
            "k" | "UP"   => self.lore_content_pane.lineup(),
            "PgDOWN" | " " | "SPACE" => self.lore_content_pane.pagedown(),
            "PgUP"   | "b" => self.lore_content_pane.pageup(),
            "g" | "HOME" => self.lore_content_pane.ix = 0,
            "G" | "END"  => {
                // Page down repeatedly until we've hit the bottom; cheap
                // because each call is a couple of pointer ops.
                for _ in 0..200 { self.lore_content_pane.pagedown(); }
            }
            _ => {}
        }
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

        // Tab strip — highlight the active tab with bold + bright fg.
        // Avoid changing bg here: SGR 49 (bg-reset) goes to terminal
        // default, NOT back to the pane bg, so any `style::bg` segment
        // in mid-line would leave a black hole for the rest of the row.
        let mut tab_strip = String::new();
        for (i, t) in Tab::all().iter().enumerate() {
            let label = format!(" [{}] {} ", i + 1, t.name());
            if *t == self.tab {
                tab_strip.push_str(&style::bold(&style::fg(&label, 226)));
            } else {
                tab_strip.push_str(&style::fg(&label, 245));
            }
        }
        let line = format!(" {}    {}    {}",
            style::bold("amar"),
            tab_strip,
            style::fg(&format!("{} | {}", camp_str, date_str), 252));
        self.header.say(&line);
    }

    fn render_body(&mut self) {
        if self.tab == Tab::Lore {
            self.render_lore_panes();
            return;
        }
        let lines = match self.tab {
            Tab::Session  => self.render_session(),
            Tab::Forge    => self.render_forge(),
            Tab::Campaign => self.render_campaign(),
            Tab::Lore     => unreachable!(),
            Tab::Inspire  => self.render_inspire(),
        };
        // Wipe the Lore panes' area when switching off Lore so the
        // body content doesn't sit over the old tree contents.
        self.lore_tree_pane.clear();
        self.lore_content_pane.clear();
        self.body.set_text(&lines.join("\n"));
        self.body.full_refresh();
    }

    fn render_lore_panes(&mut self) {
        // Build the tree against the current expanded-set. Cheap (~ms).
        let tree = Tree::build(&self.canon, &self.lore_expanded);
        if self.lore_idx >= tree.len().max(1) {
            self.lore_idx = tree.len().saturating_sub(1);
        }

        // Tree pane: one line per item, expandable categories get +/-.
        // Cursor row: bright yellow + bold when Tree has focus, dim
        // when Content has focus (so the user can see which pane will
        // receive arrow / PgUp / PgDown keys).
        let tree_active = self.focus == Focus::Left;
        let mut tree_lines: Vec<String> = Vec::with_capacity(tree.len());
        for (i, item) in tree.items.iter().enumerate() {
            let cursor = if i == self.lore_idx { "→" } else { " " };
            let indent = "  ".repeat(item.depth as usize);
            let glyph = if item.expandable {
                if item.expanded { "-" } else { "+" }
            } else {
                " "
            };
            let title = item.node.title();
            let row = format!("{} {}{} {}", cursor, indent, glyph, title);
            let line = if i == self.lore_idx {
                if tree_active {
                    style::bold(&style::fg(&row, 226))
                } else {
                    style::fg(&row, 244)
                }
            } else {
                match &item.node {
                    Node::Doc { .. } => row,
                    Node::CanonCategory { .. } => style::fg(&row, 117),
                    Node::CanonEntry { .. } => style::fg(&row, 250),
                }
            };
            tree_lines.push(line);
        }
        self.lore_tree_pane.set_text(&tree_lines.join("\n"));
        self.lore_tree_pane.ix = scroll_offset(self.lore_idx, tree.len(), self.lore_tree_pane.h as usize);
        self.lore_tree_pane.full_refresh();

        // Body pane: render the selected item's content.
        let content = match tree.get(self.lore_idx) {
            Some(item) => match &item.node {
                Node::Doc { body, .. } => lore::render_markdown(body, self.lore_content_pane.w as usize),
                Node::CanonCategory { title, category, .. } => {
                    let mut out = vec![
                        String::new(),
                        style::bold(&style::fg(title, 226)),
                        style::fg(&"-".repeat(title.chars().count()), 244),
                        String::new(),
                        format!("ENTER or l to expand. {} entries.", self.canon.category(category).len()),
                    ];
                    if let Some(extra) = category_blurb(category) {
                        out.push(String::new());
                        out.push(extra.into());
                    }
                    out
                }
                Node::CanonEntry { name } => {
                    if let Some(entry) = self.canon.lookup(name) {
                        lore::render_canon_entry(entry)
                    } else {
                        vec![format!("(entry '{}' not found in canon)", name)]
                    }
                }
            }
            None => vec!["(empty tree)".into()],
        };
        // set_text() doesn't reset ix — the pane keeps its scroll position
        // across selection changes. Cursor moves in the tree explicitly
        // reset ix to 0 in handle_lore_key.
        self.lore_content_pane.set_text(&content.join("\n"));
        self.lore_content_pane.full_refresh();
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
            Tab::Session  => " 1-5:tabs  C-LEFT/RIGHT:tabs  C:new-campaign  L:load  ?:help  q:quit",
            Tab::Forge    => " 1-5:tabs  C-LEFT/RIGHT:tabs  C:new-campaign  L:load  ?:help  q:quit",
            Tab::Campaign => " 1-5:tabs  C-LEFT/RIGHT:tabs  C:new-campaign  L:load-campaign  ?:help  q:quit",
            Tab::Lore     => match self.focus {
                Focus::Left  => " TAB:focus-content  j/k:tree  l/h:expand/collapse  C-LEFT/RIGHT:tabs  ?:help",
                Focus::Right => " TAB:focus-tree  ↑↓:line  PgUp/PgDn:page  g/G:top/end  C-LEFT/RIGHT:tabs  ?:help",
            },
            Tab::Inspire  => " 1-5:tabs  C-LEFT/RIGHT:tabs  C:new-campaign  L:load  ?:help  q:quit",
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
              1-5            Jump to tab\n  \
              C-RIGHT/LEFT   Next / previous tab\n  \
              TAB            Toggle focus between left + right pane (Lore)\n  \
              ESC            Drop focus back to left pane\n  \
              ?              This help\n\n  \
            LORE — TREE FOCUS (left pane)\n  \
              j / k          Tree cursor down / up\n  \
              ENTER / l      Expand a canon category\n  \
              h              Collapse / jump to parent\n  \
              g / G          First / last item\n\n  \
            LORE — CONTENT FOCUS (right pane)\n  \
              UP / DOWN      Line scroll\n  \
              PgUp / PgDn    Page scroll (also SPACE / b)\n  \
              g / HOME       Top of content\n  \
              G / END        End of content\n\n  \
            LORE — ALWAYS\n  \
              S-DOWN / S-UP        Right pane line scroll\n  \
              S-RIGHT / S-LEFT     Right pane page scroll\n\n  \
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

/// Compute the top-of-pane scroll offset that keeps `idx` near the
/// vertical centre of a pane of height `h` rows over a list of `total`
/// items. Returns 0 if the list fits without scrolling.
fn scroll_offset(idx: usize, total: usize, h: usize) -> usize {
    if total <= h { return 0; }
    let half = h / 2;
    if idx < half { 0 }
    else if idx + half >= total { total - h }
    else { idx - half }
}

/// One-line description shown for an unexpanded canon category in the
/// Lore content pane. Plain text — no markdown.
fn category_blurb(cat: &str) -> Option<&'static str> {
    Some(match cat {
        "Spells"  => "Active casting via the Casting attribute. Each spell has a domain (Fire, Water, Earth, Air, Life, Black, Ice, Lava, Magic, Perception, Protection, Summoning), a DR, a Mental Fortitude cost, and an Encumbrance value that limits how many spells can be active at once.",
        "Rituals" => "Slow, ingredient-driven magic resolved with the Magick Rituals skill (under MIND -> Nature Knowledge). The wiki currently lists 11 rituals.",
        "Potions" => "Alchemy: brewed in ~1 hour, last ~1 hour, resolved with the Alchemy skill (under MIND -> Nature Knowledge). The wiki currently lists 9 potions.",
        _ => return None,
    })
}
