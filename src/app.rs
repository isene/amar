//! Application shell — 5-tab TUI built on crust panes.
//!
//! Tab dispatch: number keys 1..=5 jump directly, TAB / S-TAB cycle.
//! Each tab owns its own render method but shares the App's state
//! (Canon, Campaign, GlobalConfig). Idle path is a single blocking
//! `Input::getchr` — no timers, no polling.

use crate::canon::Canon;
use crate::lore::{self, Node, Tree};
use crate::store::{Campaign, CombatRef, GlobalConfig, list_campaigns};
use crate::theme as t;
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
    fn all() -> [Tab; 5] { [Tab::Inspire, Tab::Forge, Tab::Campaign, Tab::Session, Tab::Lore] }
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

/// Discriminator for the Forge tab's generator list. Each entry maps
/// to a method on `App` that produces the right-pane output. O6 rolls
/// are NOT here — they're bound globally to `o` (status-line output).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeGen {
    WeatherToday,
    WeatherMonth,
    Names,
    Npc,
    Encounter,
    Town,
    /// Placeholder — shows a "not yet ported" message. The static
    /// label argument is the human-readable name.
    NotYet(&'static str),
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
    /// Generic two-pane layout shared by tabs that need a list-on-left,
    /// detail-on-right view (Lore, Campaign PCs, Forge later). Each
    /// tab fills these panes with its own content; the marker panes
    /// track which side has focus.
    pub left_marker: Pane,
    pub left_pane: Pane,
    pub right_marker: Pane,
    pub right_pane: Pane,
    pub footer: Pane,
    pub status: Option<(String, u8)>,
    /// Lore tab navigation state.
    pub lore_idx: usize,
    pub lore_expanded: Vec<String>,
    /// Forge tab navigation state. `forge_idx` is the cursor in the
    /// fixed list of generators; `forge_output` holds the lines from
    /// the most-recently-run generator (rendered into right_pane).
    /// `forge_npc` keeps the most-recently-generated NPC around so
    /// the right pane re-renders it via `render_pc_sheet` and so
    /// "Add to campaign" can pick it up later without regenerating.
    pub forge_idx: usize,
    pub forge_output: Vec<String>,
    pub forge_npc: Option<crate::pc::Character>,
    /// Chartype string ("Warrior", "Mage", …) the user picked for
    /// the last-generated NPC. Stashed alongside `forge_npc` so the
    /// AI enrichment prompt can name the role precisely instead of
    /// inferring it from the skill block (which is lossy — a Hunter
    /// with a longsword looks the same as a Warrior with one).
    pub forge_npc_chartype: Option<String>,
    /// Last encounter rolled in the Forge tab. Kept so the user can
    /// press `A` to ask Claude for AI flavour (backstory / purpose /
    /// scenery / opening line) over the deterministic roll, without
    /// re-rolling and losing the existing stat block.
    pub forge_encounter: Option<crate::forge::encounter::Encounter>,
    /// Last town generated in the Forge tab. Kept so the user can flip
    /// to the relationship-map image view (key `r`) without having to
    /// regenerate the same town.
    pub forge_town: Option<crate::forge::town::Town>,
    /// Last weather batch rolled in the Forge tab (1 day for "today",
    /// 28 for "month"). Kept so `A` enriches a stable roll instead
    /// of re-rolling and surprising the GM with different weather.
    pub forge_weather: Option<Vec<crate::forge::WeatherDay>>,
    /// True when the Forge right pane is currently showing the
    /// relations PNG via glow rather than text. The next text repaint
    /// flips this back to false (and clears the image).
    pub forge_town_image: bool,
    /// Glow display handle. Lazy-init: created once when the first
    /// image render is requested so amar's cold-start stays fast.
    /// `None` means glow hasn't been touched yet.
    pub image_display: Option<glow::Display>,
    /// Path the next `render_campaign_panes` should overlay on the
    /// right pane after the text is written. Cleared on every render
    /// cycle. Set by `request_image_display` from the
    /// `render_adventure_asset` codepath.
    pub pending_image: Option<std::path::PathBuf>,
    /// True while an adventure-asset image is on screen. Lets a
    /// subsequent navigation key clear it (same dismiss-on-scroll
    /// pattern as the town relations map).
    pub adv_image_shown: bool,
    /// Campaign tab navigation state. `camp_idx` is the cursor in the
    /// flattened tree (Sections + items: PCs, Adventures, …).
    /// `camp_expanded` holds the expanded-sections set.
    pub camp_idx: usize,
    pub camp_expanded: Vec<String>,
    /// Left-pane width on a 1-6 scale (kastrup convention). Persisted
    /// in GlobalConfig.
    pub pane_width: u8,
    /// PC sheet editable-field cursor. Indexes into `self.edits`;
    /// ENTER on the right pane edits the field at this index.
    pub sheet_idx: usize,
    /// Cached list of editable fields for the currently-rendered PC
    /// sheet. Refreshed on every render_campaign_panes() call.
    pub edits: Vec<EditableField>,
}

/// One editable position on a rendered PC sheet. The cursor moves
/// between these (j/k on the right pane); ENTER opens an edit prompt
/// pre-filled with `current` and dispatches the result via
/// Character::set_field.
#[derive(Debug, Clone)]
pub struct EditableField {
    pub line: usize,
    pub field_id: String,
    pub label: String,
    pub current: String,
}

/// Sections in the Campaign tree, in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CampSection {
    Pcs,
    Adventures,
    Npcs,
    Locations,
    Calendar,
    Factions,
    /// Forge artefacts the user pressed `S` to save (encounters,
    /// towns, weather, NPCs). One section so they're grouped at the
    /// bottom of the tree; the kind of each leaf is shown by glyph.
    SavedForge,
}

impl CampSection {
    fn id(self) -> &'static str {
        match self {
            CampSection::Pcs        => "PCs",
            CampSection::Adventures => "Adventures",
            CampSection::Npcs       => "NPCs",
            CampSection::Locations  => "Locations",
            CampSection::Calendar   => "Calendar",
            CampSection::Factions   => "Factions",
            CampSection::SavedForge => "Forge log",
        }
    }
    fn all() -> [CampSection; 7] {
        [CampSection::Pcs, CampSection::Adventures, CampSection::Npcs,
         CampSection::Locations, CampSection::Calendar, CampSection::Factions,
         CampSection::SavedForge]
    }
}

/// One row in the Campaign tree. Either a section header (expandable)
/// or a leaf belonging to a section.
#[derive(Debug, Clone)]
enum CampNode {
    Section(CampSection),
    Pc(usize),
    Adventure(usize),
    Npc(usize),
    Location(usize),
    /// Saved forge artefact. The kind tag picks which campaign vector
    /// to dereference; the usize is the index into that vector.
    SavedForge(SavedKind, usize),
    /// Sub-section header inside an expanded adventure ("Sections",
    /// "Scenes", "Floorplans", "NPC Portraits", "NPC Docs"). The
    /// adventure index + the kind tag let the render dispatcher pull
    /// the right list.
    AdventureGroup(usize, AdventureGroupKind),
    /// One parsed `##`/`###` heading inside an adventure's narrative.
    AdventureSection(usize, usize),
    /// One asset (scene / floorplan / portrait / npc-doc) inside an
    /// adventure. The tag picks which vector on the Adventure.
    AdventureAsset(usize, AdventureAssetKind, usize),
    /// Placeholder shown under an expanded section that has no items.
    Placeholder { section: CampSection, msg: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SavedKind { Encounter, Town, Weather, Npc }

/// Which sub-list inside an expanded adventure we're looking at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdventureGroupKind { Sections, Scenes, Floorplans, NpcPortraits, NpcDocs }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdventureAssetKind { Scene, Floorplan, NpcPortrait, NpcDoc }

/// Internal "what is the user trying to delete" tag used by
/// `try_delete_under_cursor` to thread the deletion decision through
/// the confirmation prompt without re-resolving the cursor's
/// `CampNode`.
enum DeleteTarget {
    Pc(usize),
    Npc(usize),
    Adventure(usize),
    SavedForge(SavedKind, usize),
}

/// Which roster (PC or NPC) holds the Character at the cursor.
/// Same struct underneath — only the campaign vector differs.
#[derive(Debug, Clone, Copy)]
enum CharCursor {
    Pc(usize),
    Npc(usize),
}

/// Best human-readable name for a saved-forge entry — used by the
/// delete prompt so the user sees what they're about to remove
/// instead of just "encounter #3".
fn saved_forge_display_name(camp: &Campaign, kind: SavedKind, idx: usize) -> String {
    match kind {
        SavedKind::Encounter => camp.saved_encounters.get(idx).map(|s| s.name.clone()),
        SavedKind::Town      => camp.saved_towns.get(idx).map(|s| s.name.clone()),
        SavedKind::Weather   => camp.saved_weather.get(idx).map(|s| s.name.clone()),
        SavedKind::Npc       => camp.saved_npcs.get(idx).map(|s| s.name.clone()),
    }.unwrap_or_else(|| "?".to_string())
}

#[derive(Debug, Clone)]
struct CampTreeItem {
    node: CampNode,
    depth: u8,
    expandable: bool,
    expanded: bool,
}

impl App {
    pub fn new() -> Self {
        let canon = Canon::load();
        let config = GlobalConfig::load();
        let campaign = config.active_campaign.as_deref()
            .and_then(|n| Campaign::load(n).ok())
            .map(|mut c| {
                // Catch up any campaigns that were imported under
                // the old layout (NPC portraits on the adventure
                // tree) — promote into c.npcs in-place. Idempotent
                // and no-op when there's nothing left to promote.
                let n = c.adventures.len();
                let mut moved = 0;
                for i in 0..n {
                    moved += c.promote_adventure_portraits_to_npcs(i);
                }
                if moved > 0 { let _ = c.save(); }
                c
            });
        let (cols, rows) = Crust::terminal_size();
        let mut header = Pane::new(1, 1, cols, 1, t::FG_BRIGHT as u16, t::BG_BAR as u16);
        header.wrap = false;
        header.scroll = false;

        let body_h = rows.saturating_sub(2);
        let mut body = Pane::new(1, 2, cols, body_h, t::FG as u16, 0);
        body.wrap = true;

        // Two-pane layout (used by Lore and Campaign): a 2-col marker
        // pane sits flush against the left edge of each content pane.
        // Col 1 of the marker holds a thin `▏` glyph (one-eighth block)
        // in bright yellow when active, dim grey when inactive; col 2
        // is blank to give the bar a little breathing room from the
        // text on its right.
        //
        // Left pane width follows kastrup's 1-6 cycle persisted in
        // GlobalConfig: left ≈ (cols - 4) × width / 10, clamped so
        // both sides retain a minimum of 20 cols.
        let pane_width = config.pane_width.clamp(1, 6);
        let (left_total, right_total) = compute_left_right(cols, pane_width);
        let left_pane_w: u16 = left_total.saturating_sub(2);
        let right_pane_w: u16 = right_total.saturating_sub(2);

        let mut left_marker = Pane::new(1, 2, 2, body_h, t::FG_FAINT as u16, 0);
        left_marker.wrap = false;
        left_marker.scroll = false;
        let mut left_pane = Pane::new(3, 2, left_pane_w, body_h, t::FG as u16, 0);
        left_pane.wrap = false;

        let mut right_marker = Pane::new(left_total + 1, 2, 2, body_h, t::FG_FAINT as u16, 0);
        right_marker.wrap = false;
        right_marker.scroll = false;
        let mut right_pane = Pane::new(left_total + 3, 2, right_pane_w, body_h, t::FG as u16, 0);
        right_pane.wrap = true;

        let mut footer = Pane::new(1, rows, cols, 1, t::FG_MUTED as u16, t::BG_BAR as u16);
        footer.wrap = false;
        footer.scroll = false;
        Self {
            canon, config, campaign, tab: Tab::Campaign,
            focus: Focus::Left,
            cols, rows, header, body,
            left_marker, left_pane,
            right_marker, right_pane,
            footer, status: None,
            lore_idx: 0,
            lore_expanded: Vec::new(),
            forge_idx: 0,
            forge_output: Vec::new(),
            forge_npc: None,
            forge_npc_chartype: None,
            forge_encounter: None,
            forge_town: None,
            forge_weather: None,
            forge_town_image: false,
            image_display: None,
            pending_image: None,
            adv_image_shown: false,
            camp_idx: 0,
            camp_expanded: vec!["PCs".to_string()],  // PCs auto-expanded on first run
            pane_width,
            sheet_idx: 0,
            edits: Vec::new(),
        }
    }

    pub fn run(&mut self) {
        Crust::clear_screen();
        self.render_all();
        // Warm up glow's PNG cache for the active campaign's adventure
        // images so the first ENTER on a scene / floorplan / NPC
        // portrait lands instantly. Background thread — no blocking.
        self.preconvert_active_adventure_images();
        loop {
            let Some(key) = Input::getchr(None) else { continue };
            // One-shot status messages: as soon as the user presses
            // any key, the previous status is cleared. The current
            // key's handler may set a new status; that one shows
            // until the user's *next* keypress.
            self.status = None;
            match key.as_str() {
                "q" | "Q" => {
                    if let Some(ref c) = self.campaign { let _ = c.save(); }
                    let _ = self.config.save();
                    break;
                }
                "1" => { self.set_tab(Tab::Inspire); }
                "2" => { self.set_tab(Tab::Forge); }
                "3" => { self.set_tab(Tab::Campaign); }
                "4" => { self.set_tab(Tab::Session); }
                "5" => { self.set_tab(Tab::Lore); }
                "C-RIGHT" => { self.set_tab(self.tab.next()); }
                "C-LEFT"  => { self.set_tab(self.tab.prev()); }
                "TAB" => {
                    if self.tab_has_two_panes() {
                        self.focus = match self.focus {
                            Focus::Left  => Focus::Right,
                            Focus::Right => Focus::Left,
                        };
                        self.render_all();
                    } else {
                        self.render_all();
                    }
                }
                "w" => { self.cycle_width(false); }
                "W" => { self.cycle_width(true); }
                "?" => self.show_help(),
                "C" => { self.campaign_create(); self.render_all(); }
                "L" => { self.campaign_load(); self.render_all(); }
                "X" => { self.campaign_delete(); self.render_all(); }
                "r" => {
                    // Forge uses `r` for the relations map of the
                    // last-generated town. The focus side doesn't
                    // matter — the right pane is where the graph
                    // lands either way, so `r` works whether the
                    // cursor is on the generator list (Left) or
                    // already on the output (Right). Everywhere
                    // else `r` is just a redraw shortcut.
                    if matches!(self.tab, Tab::Forge) && self.forge_town.is_some() {
                        self.show_town_relations();
                    } else {
                        self.render_all();
                    }
                }
                "C-l" | "C-L" => {
                    // Hard refresh: wipe any image overlay, clear
                    // the screen, rebuild panes (picks up a
                    // terminal resize), and repaint everything.
                    // Pointer / kastrup / scribe all bind C-l to
                    // this — muscle memory carries across the suite.
                    self.clear_overlay_image();
                    crust::Crust::clear_screen();
                    self.rebuild_panes();
                    self.render_all();
                }
                "o" => { self.roll_status(false); self.render_all(); }
                "O" => { self.roll_status(true);  self.render_all(); }
                "A" => {
                    // Forge: hand the last-generated artefact (encounter
                    // for now; NPC / town / weather hooks slot in later)
                    // to `claude -p` for AI flavour. Everywhere else
                    // `A` is unused. Result is appended to the right
                    // pane's existing text so the user keeps both the
                    // deterministic stat block and the prose.
                    if matches!(self.tab, Tab::Forge) {
                        self.ai_enrich_forge();
                    }
                }
                "S" => {
                    // Forge: snapshot the current artefact (with any
                    // AI flavour already produced) into the active
                    // campaign's `saved_*` vector and write the
                    // campaign to disk. Everywhere else `S` falls
                    // through to the per-tab key handler.
                    if matches!(self.tab, Tab::Forge) {
                        self.save_forge_artefact();
                    } else {
                        self.handle_tab_key("S");
                        self.render_all();
                    }
                }
                "ESC" => {
                    if self.focus == Focus::Right {
                        self.focus = Focus::Left;
                    }
                    self.render_all();
                }
                other => {
                    self.handle_tab_key(other);
                    self.render_all();
                }
            }
        }
        Crust::clear_screen();
    }

    /// Roll an open-ended d6 and stash the result in the status line.
    /// Two flavors share this implementation:
    ///
    ///   - `o` → skill roll (`combat = false`).
    ///   - `O` → combat roll (`combat = true`). On critical / fumble,
    ///     the framing reads "Combat" and table results biased toward
    ///     the typical combat categories are surfaced.
    ///
    /// On Critical or Fumble, the wiki critical/fumble tables are
    /// rolled (category 1-6 + entry 1-6) and the description is
    /// appended to the status line. Sequence is rendered in dim
    /// gray so the total + outcome stand out.
    fn roll_status(&mut self, combat: bool) {
        use crate::dice;
        let mut rng = dice::StdRng::from_time();
        let r = dice::o6(&mut rng);
        let trail = r.sequence.iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let label = if combat { "Combat" } else { "Skill " };
        let mut parts = Vec::<String>::new();
        // Header label.
        parts.push(style::fg(label, t::STEEL).to_string());
        // Total — bold-bright, color-coded by outcome.
        let (total_color, tag, status_color) = match r.outcome {
            dice::Outcome::Critical => (46u8,  " CRITICAL", 46u8),
            dice::Outcome::Fumble   => (196u8, " FUMBLE",   196u8),
            dice::Outcome::Normal   => (255u8, "",          252u8),
        };
        parts.push(style::bold(&style::fg(&format!("O6 → {}", r.total), total_color))
            .to_string());
        if !tag.is_empty() {
            parts.push(style::bold(&style::fg(tag.trim(), total_color))
                .to_string());
        }
        // Sequence in dim gray.
        parts.push(style::fg(&format!("({})", trail), t::FG_DIM).to_string());
        // Critical / fumble table outcome. When the original
        // category roll lands on the recursive entry (Cat 6 on
        // Critical, Cat 1 on Fumble), surface that explicitly so
        // the GM sees why two sub-rolls follow.
        if matches!(r.outcome, dice::Outcome::Critical) {
            let table = dice::roll_critical(&mut rng);
            if table.recursive {
                parts.push(style::fg(
                    "→ 6 Roll twice (no 6s), +1 mark", t::AMBER).to_string());
            }
            for h in table.hits {
                parts.push(style::fg(
                    &format!("→ {}/{} {}: {}", h.category, h.entry,
                        h.category_name, h.description),
                    220).to_string());
            }
        } else if matches!(r.outcome, dice::Outcome::Fumble) {
            let table = dice::roll_fumble(&mut rng);
            if table.recursive {
                parts.push(style::fg(
                    "→ 1 Roll twice (no 1s), -1 mark", t::WARN).to_string());
            }
            for h in table.hits {
                parts.push(style::fg(
                    &format!("→ {}/{} {}: {}", h.category, h.entry,
                        h.category_name, h.description),
                    208).to_string());
            }
        }
        self.status = Some((parts.join("  "), status_color));
    }

    fn cycle_width(&mut self, reverse: bool) {
        self.pane_width = if reverse {
            if self.pane_width <= 1 { 6 } else { self.pane_width - 1 }
        } else {
            if self.pane_width >= 6 { 1 } else { self.pane_width + 1 }
        };
        self.config.pane_width = self.pane_width;
        let _ = self.config.save();
        self.rebuild_panes();
        self.status_msg(&format!("Pane width: {} / 6", self.pane_width), t::STEEL);
        self.render_all();
    }

    /// Reposition + resize the two-pane layout based on `self.pane_width`.
    /// Called whenever `w`/`W` cycles the width.
    fn rebuild_panes(&mut self) {
        let (cols, rows) = (self.cols, self.rows);
        let body_h = rows.saturating_sub(2);
        let (left_total, right_total) = compute_left_right(cols, self.pane_width);
        let left_pane_w = left_total.saturating_sub(2);
        let right_pane_w = right_total.saturating_sub(2);
        self.left_marker.x = 1;
        self.left_marker.w = 2;
        self.left_marker.h = body_h;
        self.left_pane.x = 3;
        self.left_pane.w = left_pane_w;
        self.left_pane.h = body_h;
        self.right_marker.x = left_total + 1;
        self.right_marker.w = 2;
        self.right_marker.h = body_h;
        self.right_pane.x = left_total + 3;
        self.right_pane.w = right_pane_w;
        self.right_pane.h = body_h;
        // Wipe so old content from the previous (wider/narrower) layout
        // doesn't linger in the now-uncovered area.
        Crust::clear_screen();
    }

    fn set_tab(&mut self, t: Tab) {
        // If we were showing the town relations PNG, the new tab will
        // paint over its area but the kitty-graphics overlay survives
        // a plain SGR repaint. Tear it down explicitly so we don't
        // leak the image into Lore / Campaign / Inspire.
        if self.forge_town_image {
            self.hide_town_relations();
        }
        // Same fix for the adventure-asset overlay (scene image /
        // floorplan / NPC portrait). Kitty placements outlive the
        // pane text repaint, so without explicit teardown the old
        // image stays on top of whatever the next tab paints.
        if self.adv_image_shown {
            self.clear_overlay_image();
        }
        self.tab = t;
        // Tabs that have only one pane don't make sense with Right focus.
        if !self.tab_has_two_panes() {
            self.focus = Focus::Left;
        }
        self.render_all();
    }

    fn tab_has_two_panes(&self) -> bool {
        matches!(self.tab, Tab::Lore | Tab::Campaign | Tab::Forge)
    }

    fn handle_tab_key(&mut self, key: &str) {
        match self.tab {
            Tab::Lore     => self.handle_lore_key(key),
            Tab::Campaign => self.handle_campaign_key(key),
            Tab::Forge    => self.handle_forge_key(key),
            Tab::Inspire  => self.handle_inspire_key(key),
            Tab::Session  => self.handle_session_key(key),
            _ => {}
        }
    }

    /// Combat-HUD key handler. Operates on `camp.combatants` /
    /// `camp.combat_idx` + the bp_current / mf_current of the selected
    /// PC or NPC. Persists every change via Campaign::save so the
    /// fight survives a quit-relaunch mid-session.
    fn handle_session_key(&mut self, key: &str) {
        match key {
            "j" | "DOWN" => {
                if let Some(c) = self.campaign.as_mut() {
                    if c.combat_idx + 1 < c.combatants.len() { c.combat_idx += 1; }
                }
            }
            "k" | "UP" => {
                if let Some(c) = self.campaign.as_mut() {
                    if c.combat_idx > 0 { c.combat_idx -= 1; }
                }
            }
            "+" => { self.combat_hp_delta(1); }
            "-" => { self.combat_hp_delta(-1); }
            "M" => { self.combat_mf_delta(1); }
            "m" => { self.combat_mf_delta(-1); }
            "A" => { self.combat_add_all_pcs(); }
            "a" => { self.combat_add_by_name(); }
            "d" => { self.combat_remove_selected(); }
            "c" => { self.combat_clear(); }
            "N" => { self.append_section_note(); }
            "E" => { self.end_current_session(); }
            _ => {}
        }
    }

    fn combat_hp_delta(&mut self, delta: i32) {
        let Some(c) = self.campaign.as_mut() else { return };
        let Some(r) = c.combatants.get(c.combat_idx).copied() else { return };
        let ch_opt = match r {
            crate::store::CombatRef::Pc(i)  => c.pcs.get_mut(i),
            crate::store::CombatRef::Npc(i) => c.npcs.get_mut(i),
        };
        if let Some(ch) = ch_opt {
            let new = (ch.bp_current + delta).max(0).min(ch.bp_max());
            ch.bp_current = new;
        }
        let _ = c.save();
    }

    fn combat_mf_delta(&mut self, delta: i32) {
        let Some(c) = self.campaign.as_mut() else { return };
        let Some(r) = c.combatants.get(c.combat_idx).copied() else { return };
        let ch_opt = match r {
            crate::store::CombatRef::Pc(i)  => c.pcs.get_mut(i),
            crate::store::CombatRef::Npc(i) => c.npcs.get_mut(i),
        };
        if let Some(ch) = ch_opt {
            let new = (ch.mf_current + delta).max(0).min(ch.mf_max());
            ch.mf_current = new;
        }
        let _ = c.save();
    }

    fn combat_add_all_pcs(&mut self) {
        let n_pcs;
        if let Some(c) = self.campaign.as_mut() {
            n_pcs = c.pcs.len();
            for i in 0..n_pcs {
                let r = crate::store::CombatRef::Pc(i);
                if !c.combatants.contains(&r) {
                    c.combatants.push(r);
                }
            }
            // Reset current_bp / mf to max if they're 0 (typically
            // means stale state from a previous fight that ended).
            for r in c.combatants.clone() {
                if let crate::store::CombatRef::Pc(i) = r {
                    if let Some(pc) = c.pcs.get_mut(i) {
                        if pc.bp_current == 0 { pc.bp_current = pc.bp_max(); }
                        if pc.mf_current == 0 { pc.mf_current = pc.mf_max(); }
                    }
                }
            }
            let _ = c.save();
        } else {
            return;
        }
        self.status_msg(
            &format!("Added all {} PCs to the fight.", n_pcs),
            t::OK);
    }

    fn combat_add_by_name(&mut self) {
        if self.campaign.is_none() { return; }
        let needle = self.footer.ask(" Add to combat — name substring: ", "");
        let needle = needle.trim().to_lowercase();
        if needle.is_empty() { return; }
        let mut added = 0;
        let Some(c) = self.campaign.as_mut() else { return };
        for i in 0..c.pcs.len() {
            if c.pcs[i].name.to_lowercase().contains(&needle) {
                let r = crate::store::CombatRef::Pc(i);
                if !c.combatants.contains(&r) {
                    c.combatants.push(r);
                    added += 1;
                }
            }
        }
        for i in 0..c.npcs.len() {
            if c.npcs[i].name.to_lowercase().contains(&needle) {
                let r = crate::store::CombatRef::Npc(i);
                if !c.combatants.contains(&r) {
                    c.combatants.push(r);
                    added += 1;
                }
            }
        }
        let _ = c.save();
        self.status_msg(
            &format!("Added {} combatant{}.", added,
                if added == 1 { "" } else { "s" }),
            if added > 0 { t::OK } else { t::WARN });
    }

    fn combat_remove_selected(&mut self) {
        let Some(c) = self.campaign.as_mut() else { return };
        if c.combat_idx < c.combatants.len() {
            c.combatants.remove(c.combat_idx);
            if c.combat_idx > 0 && c.combat_idx >= c.combatants.len() {
                c.combat_idx -= 1;
            }
        }
        let _ = c.save();
    }

    fn combat_clear(&mut self) {
        let answer = self.footer.ask(" Clear combat HUD? (y/N): ", "");
        if answer.trim() != "y" && answer.trim() != "Y" { return; }
        if let Some(c) = self.campaign.as_mut() {
            c.combatants.clear();
            c.combat_idx = 0;
            let _ = c.save();
        }
        self.status_msg("Combat HUD cleared.", t::OK);
    }

    fn handle_inspire_key(&mut self, key: &str) {
        // ENTER or `i` hands the terminal off to Claude. Everything
        // else is a no-op — the Inspire pane is a static brief, so
        // there's nothing to scroll or navigate.
        if key == "ENTER" || key == "i" {
            self.launch_inspire_claude();
        }
    }

    fn handle_campaign_key(&mut self, key: &str) {
        // Right-pane scroll keys work regardless of focus (kastrup-style).
        match key {
            "S-DOWN"  => { self.right_pane.linedown(); return; }
            "S-UP"    => { self.right_pane.lineup();   return; }
            "S-RIGHT" => { self.right_pane.pagedown(); return; }
            "S-LEFT"  => { self.right_pane.pageup();   return; }
            _ => {}
        }
        match self.focus {
            Focus::Left  => self.handle_camp_tree_key(key),
            Focus::Right => self.handle_camp_content_key(key),
        }
    }

    fn handle_camp_tree_key(&mut self, key: &str) {
        // Universal "add a PC" / "add an adventure" — work even when
        // the cursor is on a section header or on the wrong section.
        // Weapon / spell / portrait add shortcuts also work from the
        // tree pane when a PC is the cursor target, so the user
        // doesn't need to TAB into the right pane just to add their
        // first weapon or generate a portrait.
        match key {
            "n" => { self.pc_new(); return; }
            "I" => { self.adventure_import(); return; }
            "N" => {
                // On the Campaign tree, N has two meanings depending
                // on the cursor target: appending a note to a section
                // (already wired below) OR scaffolding a brand-new
                // adventure when the cursor is on the Adventures
                // section header / its placeholder.
                let on_adventures_section = self.campaign.as_ref().map(|c| {
                    let tree = build_camp_tree(c, &self.camp_expanded);
                    matches!(tree.get(self.camp_idx).map(|i| &i.node),
                        Some(CampNode::Section(CampSection::Adventures))
                        | Some(CampNode::Placeholder { section: CampSection::Adventures, .. }))
                }).unwrap_or(false);
                if on_adventures_section {
                    self.adventure_scaffold();
                } else {
                    self.append_section_note();
                }
                return;
            }
            "a" => { self.adventure_set_active(); return; }
            "R" => { self.adventure_rescan(); return; }
            "V" => { self.push_image_to_player(); return; }
            "E" => { self.end_current_session(); return; }
            "G" => { self.generate_scene_image(); return; }
            "c" => { self.rename_under_cursor(); return; }
            "D" => { self.try_delete_under_cursor(); return; }
            "+" => { self.try_promote_under_cursor(); return; }
            "M" => { self.add_weapon(crate::pc::WeaponKind::Melee);   return; }
            "I" => { self.add_weapon(crate::pc::WeaponKind::Missile); return; }
            "S" => { self.add_spell();                                return; }
            "P" => { self.generate_portrait();                        return; }
            _ => {}
        }
        let Some(camp) = self.campaign.as_ref() else {
            // No campaign yet → only a few keys make sense.
            if key == "C" { /* handled at top of run() */ }
            return;
        };
        let tree = build_camp_tree(camp, &self.camp_expanded);
        let n = tree.len();
        match key {
            "j" | "DOWN" => {
                if self.camp_idx + 1 < n { self.camp_idx += 1; self.right_pane.ix = 0; }
            }
            "k" | "UP" => {
                if self.camp_idx > 0 { self.camp_idx -= 1; self.right_pane.ix = 0; }
            }
            "PgDOWN" => {
                // Pointer-style page-down: jump the cursor by a
                // viewport's worth (left-pane height minus a row of
                // overlap so the user keeps context).
                let step = (self.left_pane.h as usize).saturating_sub(2).max(1);
                self.camp_idx = (self.camp_idx + step).min(n.saturating_sub(1));
                self.right_pane.ix = 0;
            }
            "PgUP" => {
                let step = (self.left_pane.h as usize).saturating_sub(2).max(1);
                self.camp_idx = self.camp_idx.saturating_sub(step);
                self.right_pane.ix = 0;
            }
            "g" | "HOME" => { self.camp_idx = 0; self.right_pane.ix = 0; }
            "G" | "END"  => { self.camp_idx = n.saturating_sub(1); self.right_pane.ix = 0; }
            // Pointer-style nav:
            //   l / RIGHT  → expand (no-op if already open or leaf)
            //   h / LEFT   → collapse if open; else jump to parent
            //   SPACE      → toggle (open ↔ close)
            //   ENTER      → activate: set current_section on a
            //                section leaf, else toggle as a fallback
            "l" | "RIGHT" => {
                if let Some(item) = tree.get(self.camp_idx) {
                    if let Some(k) = self.expand_key_for_node(&item.node, camp) {
                        if !self.camp_expanded.iter().any(|e| e == &k) {
                            self.camp_expanded.push(k);
                        }
                    }
                }
            }
            " " | "SPACE" => {
                if let Some(item) = tree.get(self.camp_idx) {
                    if let Some(k) = self.expand_key_for_node(&item.node, camp) {
                        if let Some(pos) = self.camp_expanded.iter().position(|e| e == &k) {
                            self.camp_expanded.remove(pos);
                        } else {
                            self.camp_expanded.push(k);
                        }
                    }
                }
            }
            "ENTER" => {
                if let Some(item) = tree.get(self.camp_idx) {
                    match &item.node {
                        CampNode::AdventureSection(adv_idx, sec_idx) => {
                            self.adventure_jump_to_section(*adv_idx, *sec_idx);
                        }
                        other => {
                            if let Some(k) = self.expand_key_for_node(other, camp) {
                                if let Some(pos) = self.camp_expanded.iter().position(|e| e == &k) {
                                    self.camp_expanded.remove(pos);
                                } else {
                                    self.camp_expanded.push(k);
                                }
                            }
                        }
                    }
                }
            }
            "h" | "LEFT" => {
                if let Some(item) = tree.get(self.camp_idx) {
                    let node = item.node.clone();
                    // First try: collapse the current node if it's open.
                    let collapsed = self.expand_key_for_node(&node, camp)
                        .and_then(|k| {
                            let pos = self.camp_expanded.iter().position(|e| e == &k)?;
                            self.camp_expanded.remove(pos);
                            Some(())
                        })
                        .is_some();
                    if !collapsed {
                        // Not open (or leaf): jump to parent. For
                        // markdown sub-sections, parent is the
                        // enclosing ## section; for everything else,
                        // it's the top-level CampSection.
                        if let Some(target_idx) = self.parent_tree_index(&node) {
                            self.camp_idx = target_idx;
                            self.right_pane.ix = 0;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_camp_content_key(&mut self, key: &str) {
        // Navigation:
        //   l / RIGHT  → next field (+1)
        //   h / LEFT   → prev field (-1)
        //   j / DOWN   → +10 fields (page-style jump)
        //   k / UP     → -10 fields
        // PgUp/PgDn still page-scroll the pane visually so very long
        // sheets are still browsable without moving the edit cursor.
        let editable = !self.edits.is_empty();
        match key {
            "l" | "RIGHT" => {
                if editable && self.sheet_idx + 1 < self.edits.len() {
                    self.sheet_idx += 1;
                    self.scroll_active_field_into_view();
                }
            }
            "h" | "LEFT" => {
                if editable && self.sheet_idx > 0 {
                    self.sheet_idx -= 1;
                    self.scroll_active_field_into_view();
                }
            }
            "j" | "DOWN" => {
                if editable {
                    let last = self.edits.len().saturating_sub(1);
                    self.sheet_idx = (self.sheet_idx + 10).min(last);
                    self.scroll_active_field_into_view();
                } else {
                    self.right_pane.linedown();
                }
            }
            "k" | "UP" => {
                if editable {
                    self.sheet_idx = self.sheet_idx.saturating_sub(10);
                    self.scroll_active_field_into_view();
                } else {
                    self.right_pane.lineup();
                }
            }
            "PgDOWN" | " " | "SPACE" => self.right_pane.pagedown(),
            "PgUP"   | "b" => self.right_pane.pageup(),
            "g" | "HOME" => {
                if editable { self.sheet_idx = 0; }
                self.right_pane.ix = 0;
            }
            "G" | "END" => {
                if editable {
                    self.sheet_idx = self.edits.len().saturating_sub(1);
                }
                for _ in 0..200 { self.right_pane.pagedown(); }
            }
            "ENTER" => {
                if editable {
                    self.edit_focused_field();
                }
            }
            "+" => {
                if editable {
                    self.add_custom_skill();
                }
            }
            // Weapons / spells / portrait — context-free shortcuts so
            // the user doesn't have to navigate to a specific section.
            "M" => self.add_weapon(crate::pc::WeaponKind::Melee),
            "I" => self.add_weapon(crate::pc::WeaponKind::Missile),
            "S" => self.add_spell(),
            "P" => self.generate_portrait(),
            _ => {}
        }
    }

    /// Add a melee or missile weapon to the focused PC. Prompts in
    /// sequence: name, hands (1H/2H, melee only), Init, ±O, ±D (melee
    /// only) or shots/round (missile only), Damage, Range (missile
    /// only), HP. Defaults are sensible for a knife / short sword.
    fn add_weapon(&mut self, kind: crate::pc::WeaponKind) {
        let cursor = match self.current_character_target() {
            Some(c) => c,
            None => return,
        };
        let kind_name = match kind {
            crate::pc::WeaponKind::Melee   => "melee",
            crate::pc::WeaponKind::Missile => "missile",
        };
        let Some(name) = self.footer.ask_or_cancel(
            &format!(" New {} weapon name: ", kind_name), "") else {
            self.status_msg("Cancelled.", t::WARN);
            return;
        };
        let name = name.trim().to_string();
        if name.is_empty() {
            self.status_msg("Cancelled.", t::WARN);
            return;
        }
        // Quick add — just ask for the weapon's name and skill (e.g.
        // "Sword", "Bow"). Every other field defaults to 0; the user
        // walks the editable rows on the sheet to fill them in.
        let Some(skill) = self.footer.ask_or_cancel(
            " Weapon skill (e.g. Sword, Bow): ", "") else {
            self.status_msg("Cancelled.", t::WARN);
            return;
        };
        let skill = skill.trim().to_string();

        if let Some(c) = self.campaign.as_mut() {
            if let Some(ch) = Self::cursor_character_mut(c, cursor) {
                ch.weapons.push(crate::pc::Weapon {
                    name: name.clone(),
                    kind: kind.clone(),
                    skill_name: skill,
                    two_handed: false,
                    init: 0,
                    off_mod: 0,
                    def_mod: 0,
                    shots_per_round: if matches!(kind, crate::pc::WeaponKind::Missile) { 1 } else { 0 },
                    damage: 0,
                    hp: 8,
                    range_m: if matches!(kind, crate::pc::WeaponKind::Missile) { 30 } else { 0 },
                    xp: 0,
                });
            }
            let _ = c.save();
        }
        self.status_msg(&format!("Added {} weapon '{}'.", kind_name, name), t::OK);
    }

    /// Add a spell to the focused PC. The spell name is matched
    /// against the wiki canon — known spells display their full stat
    /// block on the sheet. Unknown names are accepted but flagged
    /// "(not in canon)" until the canon is regenerated.
    fn add_spell(&mut self) {
        let cursor = match self.current_character_target() {
            Some(c) => c,
            None => return,
        };
        let name = self.footer.ask(" Spell name (canon entry): ", "");
        let name = name.trim().to_string();
        if name.is_empty() {
            self.status_msg("Cancelled.", t::WARN);
            return;
        }
        // Pre-fill stats from canon if the name matches a known spell.
        let canon_entry = self.canon.lookup(&name).cloned();
        let in_canon = canon_entry.is_some();
        let mut sp = crate::pc::Spell { name: name.clone(), ..Default::default() };
        if let Some(entry) = canon_entry {
            // Wiki canon stores cost as e.g. "3 Mental Fortitude" — the
            // parse picks up the leading integer and ignores the rest.
            let parse_lead_int = |s: &str| -> i32 {
                s.split_whitespace().next()
                    .and_then(|t| t.parse::<i32>().ok())
                    .unwrap_or(0)
            };
            sp.domain         = entry.fields.get("domain").cloned().unwrap_or_default();
            sp.active_passive = entry.fields.get("active_passive").cloned().unwrap_or_default();
            sp.dr             = entry.fields.get("dr")
                .map(|s| parse_lead_int(s)).unwrap_or(0);
            sp.cost           = entry.fields.get("cost")
                .map(|s| parse_lead_int(s)).unwrap_or(0);
            sp.casting_time   = entry.fields.get("casting_time").cloned().unwrap_or_default();
            sp.distance       = entry.fields.get("distance").cloned().unwrap_or_default();
            sp.duration       = entry.fields.get("duration").cloned().unwrap_or_default();
            // Canon uses "area_of_effect" for the area field.
            sp.area           = entry.fields.get("area_of_effect")
                .or_else(|| entry.fields.get("area"))
                .cloned().unwrap_or_default();
            sp.cooldown       = entry.fields.get("cooldown").cloned().unwrap_or_default();
            sp.effects        = entry.fields.get("effects").cloned().unwrap_or_default();
        }
        if let Some(c) = self.campaign.as_mut() {
            if let Some(ch) = Self::cursor_character_mut(c, cursor) {
                ch.spells.push(sp);
            }
            let _ = c.save();
        }
        let suffix = if in_canon { " (canon stats pre-filled)" } else { " (not in canon)" };
        self.status_msg(&format!("Added spell '{}'.{}", name, suffix), t::OK);
    }

    /// Generate (or import) a portrait for the focused PC. Footer
    /// menu offers two paths:
    ///   1 = clipboard: copy the prompt to xclip / wl-copy and ask
    ///       the user for the path of the resulting image. Useful for
    ///       a manual round-trip via ChatGPT.
    ///   2 = API: hit DALL-E 3 (OpenAI) or Imagen (Gemini) per
    ///       `config.image_provider` and save the image directly.
    /// Either way the saved path is stored on `pc.portrait_path` so
    /// the sheet's portrait box can render it via kitty graphics.
    fn generate_portrait(&mut self) {
        let cursor = match self.current_character_target() {
            Some(c) => c,
            None => {
                self.status_msg("Move the cursor onto a PC or NPC first.", t::WARN);
                return;
            }
        };
        // Pull a snapshot of the character + campaign name so we don't
        // hold a mutable borrow during the footer prompts (which need
        // immutable access for redraw).
        let (pc_name, prompt) = {
            let camp = match self.campaign.as_ref() {
                Some(c) => c, None => return,
            };
            let ch = match cursor {
                CharCursor::Pc(i)  => camp.pcs.get(i),
                CharCursor::Npc(i) => camp.npcs.get(i),
            };
            let ch = match ch {
                Some(c) => c, None => return,
            };
            (ch.name.clone(), crate::portrait::build_prompt(ch))
        };
        let choice = self.footer.ask(
            &format!(" Portrait for {} — 1=clipboard, 2=API: ", pc_name),
            "1");
        let saved_path: Result<std::path::PathBuf, String> = match choice.trim() {
            "1" | "" => {
                // Clipboard flow: copy prompt → user pastes into ChatGPT
                // → user gives us back the image path → we import.
                match crate::portrait::copy_to_clipboard(&prompt) {
                    Ok(tool) => self.status_msg(
                        &format!("Prompt copied via {} — paste into ChatGPT.", tool), t::OK),
                    Err(e) => {
                        self.status_msg(&format!("Clipboard failed: {}", e), t::ERR);
                        return;
                    }
                }
                let path_str = self.footer.ask(
                    " Image path (after generating + saving): ", "");
                let path_str = path_str.trim();
                if path_str.is_empty() {
                    self.status_msg("Cancelled — no image path.", t::WARN);
                    return;
                }
                let src = std::path::PathBuf::from(shellexpand_simple(path_str));
                if !src.exists() {
                    Err(format!("file not found: {}", src.display()))
                } else {
                    let camp = self.campaign.as_ref().unwrap();
                    crate::portrait::import_image(camp, &pc_name, &src)
                }
            }
            "2" => {
                let provider = self.config.image_provider.clone();
                self.status_msg(&format!("Calling {} — this can take ~20s…", provider), t::AMBER);
                self.footer.full_refresh();
                let camp = self.campaign.as_ref().unwrap();
                match provider.as_str() {
                    "openai" => crate::portrait::generate_openai(
                        &self.config, camp, &pc_name, &prompt),
                    "gemini" => crate::portrait::generate_gemini(
                        &self.config, camp, &pc_name, &prompt),
                    other => Err(format!("unknown image_provider: {}", other)),
                }
            }
            _ => {
                self.status_msg("Cancelled.", t::WARN);
                return;
            }
        };
        match saved_path {
            Ok(path) => {
                if let Some(camp) = self.campaign.as_mut() {
                    if let Some(ch) = Self::cursor_character_mut(camp, cursor) {
                        ch.portrait_path = path.to_string_lossy().to_string();
                    }
                    let _ = camp.save();
                }
                self.status_msg(&format!("Portrait saved → {}", path.display()), t::OK);
            }
            Err(e) => {
                self.status_msg(&format!("Portrait failed: {}", e), t::ERR);
            }
        }
    }

    /// Index of the PC the cursor is currently pointing at (in the
    /// Campaign tree). Returns None if the cursor is on a non-PC
    /// node, or no campaign is loaded.
    fn current_pc_idx(&self) -> Option<usize> {
        let camp = self.campaign.as_ref()?;
        let tree = build_camp_tree(camp, &self.camp_expanded);
        match tree.get(self.camp_idx).map(|t| t.node.clone()) {
            Some(CampNode::Pc(i)) => Some(i),
            _ => None,
        }
    }

    /// Either-roster character cursor. Used by the field-edit dispatch
    /// so NPC sheets are editable too — same Character type, just a
    /// different vector on the campaign.
    fn current_character_target(&self) -> Option<CharCursor> {
        let camp = self.campaign.as_ref()?;
        let tree = build_camp_tree(camp, &self.camp_expanded);
        match tree.get(self.camp_idx).map(|t| t.node.clone()) {
            Some(CampNode::Pc(i))  => Some(CharCursor::Pc(i)),
            Some(CampNode::Npc(i)) => Some(CharCursor::Npc(i)),
            _ => None,
        }
    }

    /// Mutable borrow of whatever character the tree cursor points at
    /// (PC or NPC). Returns None if the cursor isn't on a character row.
    fn cursor_character_mut<'a>(camp: &'a mut Campaign, cur: CharCursor)
        -> Option<&'a mut crate::pc::Character>
    {
        match cur {
            CharCursor::Pc(i)  => camp.pcs.get_mut(i),
            CharCursor::Npc(i) => camp.npcs.get_mut(i),
        }
    }

    /// Add a non-canonical skill (e.g. Drawing, Singing, Cooking) under
    /// the attribute the cursor is currently on. Works whether the
    /// cursor is on the attribute row itself or on any of its skill
    /// rows — the parent attribute is parsed out of the field id.
    fn add_custom_skill(&mut self) {
        let Some(field) = self.edits.get(self.sheet_idx).cloned() else { return; };
        // Parent attribute: from "attr/X" or "skill/X/Y" → "X".
        let attr_name: Option<String> = if let Some(a) = field.field_id.strip_prefix("attr/") {
            Some(a.to_string())
        } else if let Some(rest) = field.field_id.strip_prefix("skill/") {
            rest.split('/').next().map(|s| s.to_string())
        } else {
            None
        };
        let Some(attr) = attr_name else {
            self.status_msg("Move the cursor onto an attribute or skill row first.", t::WARN);
            return;
        };
        let Some(name) = self.footer.ask_or_cancel(
            &format!(" New skill under {} (name): ", attr), "") else {
            self.status_msg("Cancelled.", t::WARN);
            return;
        };
        let name = name.trim().to_string();
        if name.is_empty() {
            self.status_msg("Cancelled.", t::WARN);
            return;
        }
        let Some(rank_str) = self.footer.ask_or_cancel(" Initial rank [0]: ", "0") else {
            self.status_msg("Cancelled.", t::WARN);
            return;
        };
        let rank: i32 = rank_str.trim().parse().unwrap_or(0);

        // Find the active character (PC or NPC) and add the skill.
        let cursor = match self.current_character_target() {
            Some(c) => c,
            None => return,
        };
        if let Some(c) = self.campaign.as_mut() {
            if let Some(ch) = Self::cursor_character_mut(c, cursor) {
                ch.skills.entry(attr.clone())
                    .or_default()
                    .insert(name.clone(), rank);
            }
            let _ = c.save();
        }
        self.status_msg(&format!("Added skill '{}' under {}.", name, attr), t::OK);
    }

    /// Scroll the right pane so the line of `edits[sheet_idx]` sits
    /// roughly in the middle of the visible area.
    fn scroll_active_field_into_view(&mut self) {
        let Some(field) = self.edits.get(self.sheet_idx) else { return; };
        let h = self.right_pane.h as usize;
        let half = h / 2;
        let new_ix = field.line.saturating_sub(half);
        self.right_pane.ix = new_ix;
    }

    /// Open a footer prompt for the currently-focused editable field
    /// and dispatch the result via Character::set_field. Auto-saves
    /// the campaign on success. Special "action" ids
    /// (weapon_add_melee, weapon_add_missile, spell_add) bypass the
    /// prompt and call the corresponding add handler instead — they
    /// give the user a navigable place to ENTER on to add the first
    /// weapon / spell, since empty rows have no edit fields to land
    /// on otherwise.
    fn edit_focused_field(&mut self) {
        let Some(field) = self.edits.get(self.sheet_idx).cloned() else { return; };
        match field.field_id.as_str() {
            "weapon_add_melee"   => { self.add_weapon(crate::pc::WeaponKind::Melee);   return; }
            "weapon_add_missile" => { self.add_weapon(crate::pc::WeaponKind::Missile); return; }
            "spell_add"          => { self.add_spell();                                return; }
            _ => {}
        }
        // For slot fields, append the valid choices inline so the user
        // knows what to type without needing a TAB-completion popup.
        // Slot's `attribute` choices depend on which char the slot
        // already targets (or all attributes if char isn't picked yet).
        let prompt = if let Some(rest) = field.field_id.strip_prefix("slot/") {
            let mut parts = rest.splitn(2, '/');
            let idx: Option<usize> = parts.next().and_then(|s| s.parse().ok());
            let kind = parts.next().unwrap_or("");
            match kind {
                "char" => format!("{} (BODY|MIND|SPIRIT, or B/M/S): ", field.label),
                "attribute" => {
                    use crate::pc::ATTRIBUTES;
                    // Look up the slot's current parent_char so we can
                    // narrow the attribute list — saves the user from
                    // wading through 14 attributes.
                    let parent = idx.and_then(|i| {
                        let camp = self.campaign.as_ref()?;
                        let ch = match self.current_character_target()? {
                            CharCursor::Pc(j)  => camp.pcs.get(j)?,
                            CharCursor::Npc(j) => camp.npcs.get(j)?,
                        };
                        ch.open_skills.get(i).map(|s| s.parent_char.clone())
                    }).unwrap_or_default();
                    let names: Vec<&str> = ATTRIBUTES.iter()
                        .filter(|(c, _)| parent.is_empty() ||
                            (parent == "BODY"   && *c == crate::pc::Char::Body)   ||
                            (parent == "MIND"   && *c == crate::pc::Char::Mind)   ||
                            (parent == "SPIRIT" && *c == crate::pc::Char::Spirit))
                        .map(|(_, n)| *n)
                        .collect();
                    format!("{} ({}): ", field.label, names.join("|"))
                }
                _ => format!("{}: ", field.label),
            }
        } else {
            format!("{}: ", field.label)
        };
        let Some(value) = self.footer.ask_or_cancel(&prompt, &field.current) else {
            self.status_msg("Cancelled.", t::WARN);
            return;
        };
        // Resolve the cursor to either a PC or NPC, then commit.
        // PCs + NPCs share the Character struct so the set_field
        // codepath is identical — only the campaign vector differs.
        let cursor = match self.current_character_target() {
            Some(c) => c,
            None => return,
        };
        let result = if let Some(camp) = self.campaign.as_mut() {
            if let Some(ch) = Self::cursor_character_mut(camp, cursor) {
                ch.set_field(&field.field_id, &value)
            } else { Err("Character not found".into()) }
        } else { Err("No campaign loaded".into()) };
        match result {
            Ok(_) => {
                if let Some(c) = self.campaign.as_ref() { let _ = c.save(); }
                self.status_msg(&format!("Updated {}.", field.label.trim()), t::OK);
            }
            Err(e) => self.status_msg(&format!("Edit failed: {}", e), t::ERR),
        }
    }

    /// New PC — prompts only for the name; everything else gets a
    /// sensible default (Human, 70 kg → SIZE 3) and the user edits
    /// the rest inline by pressing ENTER on individual fields.
    fn pc_new(&mut self) {
        if self.campaign.is_none() {
            self.status_msg("No campaign loaded — press C to create one first.", t::WARN);
            return;
        }
        let name = self.footer.ask(" PC name: ", "");
        let name = name.trim().to_string();
        if name.is_empty() {
            self.status_msg("Cancelled.", t::WARN);
            return;
        }

        let mut pc = crate::pc::Character::new_blank(&name);
        pc.is_pc = true;

        if let Some(c) = self.campaign.as_mut() {
            c.pcs.push(pc);
            let _ = c.save();
        }
        // Make sure PCs section is expanded and cursor lands on the
        // freshly added PC.
        if !self.camp_expanded.iter().any(|e| e == "PCs") {
            self.camp_expanded.push("PCs".into());
        }
        if let Some(camp) = self.campaign.as_ref() {
            let tree = build_camp_tree(camp, &self.camp_expanded);
            let new_pc_idx = camp.pcs.len() - 1;
            if let Some(pos) = tree.iter().position(|it| matches!(&it.node,
                CampNode::Pc(i) if *i == new_pc_idx))
            {
                self.camp_idx = pos;
            }
        }
        self.status_msg(&format!("Added '{}'.", name), t::OK);
    }

    /// Delete whatever the cursor is currently on (PC for now).
    fn try_delete_under_cursor(&mut self) {
        let Some(camp) = self.campaign.as_ref() else { return; };
        let tree = build_camp_tree(camp, &self.camp_expanded);
        let Some(item) = tree.get(self.camp_idx) else { return; };
        // What's the cursor on? Resolve to a (label, deletion closure) pair.
        // The closure runs after the confirmation prompt; it mutates the
        // campaign in place. Keeps the prompt logic in one place instead
        // of duplicating it per variant.
        let (label, kind): (String, _) = match item.node.clone() {
            CampNode::Pc(idx) => {
                let n = camp.pcs.get(idx).map(|p| p.name.clone()).unwrap_or_default();
                (format!("PC '{}'", n), DeleteTarget::Pc(idx))
            }
            CampNode::Npc(idx) => {
                let n = camp.npcs.get(idx).map(|p| p.name.clone()).unwrap_or_default();
                (format!("NPC '{}'", n), DeleteTarget::Npc(idx))
            }
            CampNode::Adventure(idx) => {
                let n = camp.adventures.get(idx).map(|a| a.name.clone()).unwrap_or_default();
                (format!("adventure '{}' (on-disk files left intact)", n),
                 DeleteTarget::Adventure(idx))
            }
            CampNode::SavedForge(kind, idx) => {
                let display = saved_forge_display_name(camp, kind, idx);
                let kind_word = match kind {
                    SavedKind::Encounter => "encounter",
                    SavedKind::Town      => "town",
                    SavedKind::Weather   => "weather day",
                    SavedKind::Npc       => "saved NPC",
                };
                (format!("{} '{}'", kind_word, display), DeleteTarget::SavedForge(kind, idx))
            }
            _ => {
                self.status_msg(
                    "Move cursor onto a PC or saved-forge entry to delete it (D).",
                    t::WARN);
                return;
            }
        };
        let answer = self.footer.ask(&format!(" Delete {}? (y/N): ", label), "");
        if answer.trim() != "y" && answer.trim() != "Y" { return; }
        if let Some(c) = self.campaign.as_mut() {
            match kind {
                DeleteTarget::Pc(i)  => { c.pcs.remove(i); }
                DeleteTarget::Npc(i) => { c.npcs.remove(i); }
                DeleteTarget::Adventure(i) => {
                    let removed_id = c.adventures.get(i).map(|a| a.id);
                    c.adventures.remove(i);
                    if c.active_adventure_id == removed_id {
                        c.active_adventure_id = None;
                    }
                }
                DeleteTarget::SavedForge(SavedKind::Encounter, i) => { c.saved_encounters.remove(i); }
                DeleteTarget::SavedForge(SavedKind::Town,      i) => { c.saved_towns.remove(i); }
                DeleteTarget::SavedForge(SavedKind::Weather,   i) => { c.saved_weather.remove(i); }
                DeleteTarget::SavedForge(SavedKind::Npc,       i) => { c.saved_npcs.remove(i); }
            }
            let _ = c.save();
        }
        // Re-anchor cursor — the previous tree may have shortened.
        if let Some(camp) = self.campaign.as_ref() {
            let tree = build_camp_tree(camp, &self.camp_expanded);
            if self.camp_idx >= tree.len() {
                self.camp_idx = tree.len().saturating_sub(1);
            }
        }
        self.status_msg(&format!("Deleted {}.", label), t::OK);
    }

    /// "+" on the Campaign tab: promote whatever is under the cursor
    /// into the active campaign's NPC roster (default) or PC roster.
    ///   * Saved encounter → ask which NPC (#1..count), then which
    ///     roster. The encounter record stays intact.
    ///   * Saved NPC       → ask which roster. The saved-NPC entry
    ///     stays so there's still a historical record of the
    ///     pre-recruitment stat block.
    /// Anything else → status hint.
    fn try_promote_under_cursor(&mut self) {
        let Some(camp) = self.campaign.as_ref() else { return; };
        let tree = build_camp_tree(camp, &self.camp_expanded);
        let Some(item) = tree.get(self.camp_idx) else { return; };
        let node = item.node.clone();
        match node {
            CampNode::SavedForge(SavedKind::Encounter, idx) => {
                self.promote_encounter_npc(idx);
            }
            CampNode::SavedForge(SavedKind::Npc, idx) => {
                let Some(saved) = camp.saved_npcs.get(idx) else { return; };
                let name = saved.item.name.clone();
                let character = saved.item.clone();
                self.prompt_roster_and_promote(character, &name);
            }
            _ => {
                self.status_msg(
                    "Move cursor onto a saved encounter or saved NPC, then + to promote.",
                    t::WARN);
            }
        }
    }

    /// "+" on the Forge tab while an encounter is on the right pane.
    /// Same idea as the Campaign-tab path but sources the encounter
    /// from `self.forge_encounter` (the last roll) so the user can
    /// recruit without going through Save → Campaign-tab first.
    fn try_promote_from_forge(&mut self) {
        if self.campaign.is_none() {
            self.status_msg(
                "Load a campaign first (Campaign tab → C / L).", t::WARN);
            return;
        }
        let Some(ref enc) = self.forge_encounter else {
            self.status_msg(
                "Roll an encounter first, then press + to promote one of its NPCs.",
                t::WARN);
            return;
        };
        if enc.npcs.is_empty() {
            self.status_msg(
                "This encounter has no NPCs to promote (NO ENCOUNTER / event).",
                t::WARN);
            return;
        }
        self.promote_one_of(&enc.npcs.clone());
    }

    fn promote_encounter_npc(&mut self, enc_idx: usize) {
        let Some(camp) = self.campaign.as_ref() else { return; };
        let Some(saved) = camp.saved_encounters.get(enc_idx) else { return; };
        if saved.item.npcs.is_empty() {
            self.status_msg(
                "This encounter has no NPCs to promote (event or empty roll).",
                t::WARN);
            return;
        }
        let npcs = saved.item.npcs.clone();
        self.promote_one_of(&npcs);
    }

    /// Prompt for "which NPC?" from a Vec, then for which roster.
    /// Shared by the Forge-tab and Campaign-tab entrypoints.
    fn promote_one_of(&mut self, npcs: &[crate::pc::Character]) {
        let listing: String = npcs.iter().enumerate()
            .map(|(i, n)| format!("{}={}", i + 1, n.name))
            .collect::<Vec<_>>().join(", ");
        let Some(answer) = self.footer.ask_or_cancel(
            &format!(" Promote NPC # ({}): ", listing), "1") else {
            self.status_msg("Cancelled.", t::WARN);
            return;
        };
        let n: usize = match answer.trim().parse::<usize>() {
            Ok(n) if n >= 1 && n <= npcs.len() => n,
            _ => {
                self.status_msg("Promotion cancelled (invalid index).", t::WARN);
                return;
            }
        };
        let character = npcs[n - 1].clone();
        let name = character.name.clone();
        self.prompt_roster_and_promote(character, &name);
    }

    /// Ask whether the promoted character lands in the NPC roster
    /// (default) or the PC roster. Anything other than `p`/`P` →
    /// NPC, since recruiting an encounter NPC into the tracked-NPC
    /// list is the usual case; promotion to a playable PC is rarer.
    fn prompt_roster_and_promote(&mut self, character: crate::pc::Character, name: &str) {
        let Some(answer) = self.footer.ask_or_cancel(
            &format!(" Promote '{}' to (n)PC / (p)C [n]: ", name), "n") else {
            self.status_msg("Cancelled.", t::WARN);
            return;
        };
        let to_pc = matches!(answer.trim(), "p" | "P");
        self.promote_character(character, name, to_pc);
    }

    /// Final step shared by all promotion paths: set is_pc, push onto
    /// the chosen roster, save, status line.
    fn promote_character(&mut self, mut character: crate::pc::Character,
        name: &str, to_pc: bool)
    {
        character.is_pc = to_pc;
        let dest = if to_pc { "PC" } else { "NPC" };
        if let Some(c) = self.campaign.as_mut() {
            if to_pc { c.pcs.push(character); }
            else     { c.npcs.push(character); }
            let _ = c.save();
        }
        self.status_msg(
            &format!("Promoted '{}' to the {} roster.", name, dest), t::OK);
    }

    // ---- Adventure management ------------------------------------------

    /// Import an on-disk adventure into the active campaign. Prompts
    /// for a directory path (with ~ expansion). The directory must
    /// exist; it'll be walked once for narrative + assets via
    /// `adventure::import_from_dir`. After import, the new adventure
    /// is appended to `campaign.adventures` and saved.
    fn adventure_import(&mut self) {
        if self.campaign.is_none() {
            self.status_msg(
                "Load a campaign first (Campaign tab → C / L).", t::WARN);
            return;
        }
        let raw = self.footer.ask(" Import from directory: ", "");
        let raw = raw.trim();
        if raw.is_empty() {
            self.status_msg("Cancelled.", t::WARN);
            return;
        }
        let path = std::path::PathBuf::from(shellexpand_simple(raw));
        let next_id = self.campaign.as_ref()
            .map(|c| c.adventures.iter().map(|a| a.id).max().map(|n| n + 1).unwrap_or(1))
            .unwrap_or(1);
        match crate::adventure::import_from_dir(&path, next_id) {
            Ok(adv) => {
                let name = adv.name.clone();
                let n_sec = adv.sections.len();
                let n_assets = adv.scenes.len() + adv.floorplans.len()
                    + adv.npc_portraits.len() + adv.npc_docs.len();
                let n_new_npcs;
                if let Some(c) = self.campaign.as_mut() {
                    let new_id = adv.id;
                    c.adventures.push(adv);
                    let new_idx = c.adventures.len() - 1;
                    if c.active_adventure_id.is_none() {
                        c.active_adventure_id = Some(new_id);
                    }
                    // Promote NPC portraits into campaign.npcs (one
                    // stub Character per portrait). The portraits
                    // subtree on the adventure is cleared so NPCs
                    // live at one place (Campaign tab → NPCs).
                    n_new_npcs = c.promote_adventure_portraits_to_npcs(new_idx);
                    let _ = c.save();
                } else {
                    n_new_npcs = 0;
                }
                if !self.camp_expanded.iter().any(|e| e == "Adventures") {
                    self.camp_expanded.push("Adventures".into());
                }
                // Kick the glow PNG cache so subsequent ENTERs land
                // instantly instead of paying the decode tax per nav.
                self.preconvert_active_adventure_images();
                self.status_msg(
                    &format!("Imported '{}' — {} sections, {} assets, {} new NPCs.",
                        name, n_sec, n_assets, n_new_npcs), t::OK);
            }
            Err(e) => self.status_msg(&format!("Import failed: {}", e), t::ERR),
        }
    }

    /// Generate a scene image for the section under the cursor.
    /// Reuses the portrait pipeline: clipboard mode (copy a prompt
    /// for ChatGPT then ingest the user-saved file) or API mode
    /// (DALL-E / Imagen direct). Writes the result into the
    /// adventure's `Scenes/<safe-name>.png`, then runs rescan so
    /// the new image attaches to its section.
    fn generate_scene_image(&mut self) {
        // Resolve to an adventure-section target.
        let target: Option<(usize, usize, String, String, std::path::PathBuf)>;
        {
            let camp = match self.campaign.as_ref() {
                Some(c) => c,
                None => { self.status_msg("Load a campaign first.", t::WARN); return; }
            };
            let tree = build_camp_tree(camp, &self.camp_expanded);
            match tree.get(self.camp_idx).map(|i| i.node.clone()) {
                Some(CampNode::AdventureSection(adv_idx, sec_idx)) => {
                    let Some(adv) = camp.adventures.get(adv_idx) else { return };
                    let Some(sec) = adv.sections.get(sec_idx) else { return };
                    // Snip the first ~600 chars of the section body
                    // as inspiration for the prompt; gives the model
                    // setting cues without burying the heading.
                    let body = crate::adventure::section_body(adv, sec_idx);
                    let mut snippet = body.join(" ");
                    snippet.truncate(snippet.char_indices().nth(600).map(|(i, _)| i).unwrap_or(snippet.len()));
                    let prompt = format!(
                        "Fantasy RPG scene illustration for an Amar RPG adventure. \
                         Scene: \"{}\". Atmosphere from the description: {}. \
                         Painterly, cinematic, atmospheric lighting, no text, no UI, \
                         16:9. Avoid modern elements; medieval-fantasy aesthetic.",
                        sec.heading, snippet);
                    let safe = sec.heading.chars().map(|c|
                        if c.is_alphanumeric() { c } else { '_' }).collect::<String>();
                    let dst = std::path::PathBuf::from(&adv.root_dir)
                        .join("Scenes")
                        .join(format!("{}.png", safe));
                    target = Some((adv_idx, sec_idx, sec.heading.clone(), prompt, dst));
                }
                _ => {
                    self.status_msg(
                        "Move cursor onto a section (not the header) to generate a scene image.",
                        t::WARN);
                    return;
                }
            }
        }
        let Some((adv_idx, _sec_idx, sec_name, prompt, dst)) = target else { return };

        let choice = self.footer.ask(
            &format!(" Scene image for '{}' — 1=clipboard, 2=API: ", sec_name),
            "1");
        let result: Result<std::path::PathBuf, String> = match choice.trim() {
            "1" | "" => {
                match crate::portrait::copy_to_clipboard(&prompt) {
                    Ok(tool) => self.status_msg(
                        &format!("Prompt copied via {} — paste into ChatGPT.", tool), t::OK),
                    Err(e) => {
                        self.status_msg(&format!("Clipboard failed: {}", e), t::ERR);
                        return;
                    }
                }
                let path_str = self.footer.ask(
                    " Image path (after generating + saving): ", "");
                let path_str = path_str.trim();
                if path_str.is_empty() {
                    self.status_msg("Cancelled — no image path.", t::WARN);
                    return;
                }
                let src = std::path::PathBuf::from(shellexpand_simple(path_str));
                if !src.exists() {
                    Err(format!("file not found: {}", src.display()))
                } else {
                    if let Some(parent) = dst.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    std::fs::copy(&src, &dst).map(|_| dst.clone())
                        .map_err(|e| format!("copy: {}", e))
                }
            }
            "2" => {
                let provider = self.config.image_provider.clone();
                self.status_msg(
                    &format!("Calling {} — this can take ~20s…", provider), t::AMBER);
                self.footer.full_refresh();
                match provider.as_str() {
                    "openai" => crate::portrait::generate_openai_to_path(
                        &self.config, &prompt, &dst),
                    "gemini" => crate::portrait::generate_gemini_to_path(
                        &self.config, &prompt, &dst),
                    other => Err(format!("unknown image_provider: {}", other)),
                }
            }
            _ => {
                self.status_msg("Cancelled.", t::WARN);
                return;
            }
        };
        match result {
            Ok(path) => {
                // Rescan picks up the new file and runs the
                // section-attachment matcher so the image appears
                // under the section in the tree on next render.
                self.adventure_rescan_idx(adv_idx);
                self.status_msg(
                    &format!("Scene image saved → {}", path.display()),
                    t::OK);
            }
            Err(e) => self.status_msg(&format!("Scene gen failed: {}", e), t::ERR),
        }
    }

    /// Rescan a specific adventure (used by `generate_scene_image`
    /// after writing a new file — the cursor doesn't have to be on
    /// the adventure header for this to work).
    fn adventure_rescan_idx(&mut self, adv_idx: usize) {
        let (root, id) = match self.campaign.as_ref() {
            Some(c) => match c.adventures.get(adv_idx) {
                Some(a) => (a.root_dir.clone(), a.id),
                None => return,
            },
            None => return,
        };
        match crate::adventure::import_from_dir(std::path::Path::new(&root), id) {
            Ok(mut new_adv) => {
                if let Some(c) = self.campaign.as_mut() {
                    if let Some(old) = c.adventures.get(adv_idx) {
                        new_adv.current_section = old.current_section;
                        new_adv.notes = old.notes.clone();
                        for new_sec in new_adv.sections.iter_mut() {
                            if let Some(old_sec) = old.sections.iter()
                                .find(|s| s.heading == new_sec.heading)
                            {
                                new_sec.notes = old_sec.notes.clone();
                            }
                        }
                    }
                    c.adventures[adv_idx] = new_adv;
                    c.promote_adventure_portraits_to_npcs(adv_idx);
                    let _ = c.save();
                }
                self.preconvert_active_adventure_images();
            }
            Err(_) => {}
        }
    }

    /// Scaffold a fresh adventure on disk: prompt for name + root
    /// dir, create the canonical directory layout
    /// (`Scenes/Floorplans/NPCs`), drop a skeleton markdown file
    /// with the section structure ThePortal uses, then import the
    /// new directory so the tree shows it immediately.
    fn adventure_scaffold(&mut self) {
        if self.campaign.is_none() {
            self.status_msg(
                "Load a campaign first (Campaign tab → C / L).", t::WARN);
            return;
        }
        let Some(name) = self.footer.ask_or_cancel(" New adventure name: ", "") else {
            self.status_msg("Cancelled.", t::WARN);
            return;
        };
        let name = name.trim().to_string();
        if name.is_empty() {
            self.status_msg("Cancelled.", t::WARN);
            return;
        }
        // Default root dir mirrors the layout the user already uses
        // for ThePortal: ~/Main/G/AMAR/<camp>/<name>/. The user can
        // override by typing any path; `~` expansion supported.
        let camp_name = self.campaign.as_ref().unwrap().name.clone();
        let default_root = format!("~/Main/G/AMAR/{}/{}", camp_name, name);
        let Some(root) = self.footer.ask_or_cancel(
            &format!(" Root dir [{}]: ", default_root),
            &default_root) else {
            self.status_msg("Cancelled.", t::WARN);
            return;
        };
        let root = root.trim();
        let root = if root.is_empty() { &default_root } else { root };
        let root_path = std::path::PathBuf::from(shellexpand_simple(root));
        for sub in &["", "Scenes", "Floorplans", "NPCs"] {
            let p = if sub.is_empty() { root_path.clone() } else { root_path.join(sub) };
            if let Err(e) = std::fs::create_dir_all(&p) {
                self.status_msg(&format!("mkdir failed ({}): {}", p.display(), e), t::ERR);
                return;
            }
        }
        let md_path = root_path.join(format!("{}.md", name));
        if !md_path.exists() {
            let skeleton = format!(
                "# {}\n\
                 *An Amar RPG Adventure*\n\n\
                 ---\n\n\
                 ## Adventure Overview\n\n\
                 ### The Hook\n\nWhy do the PCs care?\n\n\
                 ### Synopsis\n\nThe central premise in 2-3 paragraphs.\n\n\
                 ---\n\n\
                 ## Major NPCs\n\n\
                 ### NPC Name (gender, age) - Race: Type [Level X]\n\
                 Stat block + personality + motivation + connection.\n\n\
                 ---\n\n\
                 ## Key Locations\n\n\
                 ### 1. Location Name\n\nWhat's here, what can happen.\n\n\
                 ---\n\n\
                 ## Scene-by-Scene Breakdown\n\n\
                 ### Scene 1: Opening\n\nDescription read aloud.\n\n\
                 ---\n\n\
                 ## Complications & Twists\n\n\
                 ### 1. ...\n\n\
                 ---\n\n\
                 ## Future Hooks\n\n\
                 ### 1. ...\n\n\
                 ---\n\n\
                 ## GM Notes & Tips\n\n",
                name);
            if let Err(e) = std::fs::write(&md_path, skeleton) {
                self.status_msg(&format!("write {} failed: {}", md_path.display(), e), t::ERR);
                return;
            }
        }
        // Run the importer to wire it into the campaign.
        let next_id = self.campaign.as_ref()
            .map(|c| c.adventures.iter().map(|a| a.id).max().map(|n| n + 1).unwrap_or(1))
            .unwrap_or(1);
        match crate::adventure::import_from_dir(&root_path, next_id) {
            Ok(adv) => {
                let new_name = adv.name.clone();
                if let Some(c) = self.campaign.as_mut() {
                    let new_id = adv.id;
                    c.adventures.push(adv);
                    if c.active_adventure_id.is_none() {
                        c.active_adventure_id = Some(new_id);
                    }
                    let _ = c.save();
                }
                if !self.camp_expanded.iter().any(|e| e == "Adventures") {
                    self.camp_expanded.push("Adventures".into());
                }
                self.status_msg(
                    &format!("Scaffolded '{}' at {}. Edit {}.md in scribe to flesh it out, press R here to rescan.",
                        new_name, root_path.display(), name),
                    t::OK);
            }
            Err(e) => self.status_msg(&format!("Scaffold-import failed: {}", e), t::ERR),
        }
    }

    /// Mark the adventure under the cursor as the campaign's active
    /// adventure (the one whose name + current section show in the
    /// header so the GM can resume next session). Works on the
    /// adventure node itself OR any sub-node belonging to it.
    fn adventure_set_active(&mut self) {
        let adv_idx = self.cursor_adventure_idx();
        let Some(adv_idx) = adv_idx else {
            self.status_msg(
                "Move cursor onto an adventure (or one of its sub-rows).",
                t::WARN);
            return;
        };
        if let Some(c) = self.campaign.as_mut() {
            let id = c.adventures.get(adv_idx).map(|a| a.id);
            if let Some(id) = id {
                c.active_adventure_id = Some(id);
                let _ = c.save();
                let name = c.adventures[adv_idx].name.clone();
                self.status_msg(&format!("Active adventure: '{}'", name), t::OK);
            }
        }
    }

    /// Re-walk the on-disk root for the adventure under the cursor.
    /// Picks up new images / NPC docs / heading changes without
    /// requiring re-import.
    fn adventure_rescan(&mut self) {
        let adv_idx = self.cursor_adventure_idx();
        let Some(adv_idx) = adv_idx else {
            self.status_msg("Move cursor onto an adventure first.", t::WARN);
            return;
        };
        let (root, id, name) = match self.campaign.as_ref() {
            Some(c) => match c.adventures.get(adv_idx) {
                Some(a) => (a.root_dir.clone(), a.id, a.name.clone()),
                None => return,
            },
            None => return,
        };
        match crate::adventure::import_from_dir(std::path::Path::new(&root), id) {
            Ok(mut new_adv) => {
                // Preserve the GM's bookkeeping that doesn't come
                // from disk: current_section + per-adventure notes
                // + per-section notes (matched by heading so notes
                // survive a section being reordered in the .md).
                if let Some(c) = self.campaign.as_mut() {
                    if let Some(old) = c.adventures.get(adv_idx) {
                        new_adv.current_section = old.current_section;
                        new_adv.notes = old.notes.clone();
                        for new_sec in new_adv.sections.iter_mut() {
                            if let Some(old_sec) = old.sections.iter()
                                .find(|s| s.heading == new_sec.heading)
                            {
                                new_sec.notes = old_sec.notes.clone();
                            }
                        }
                    }
                    c.adventures[adv_idx] = new_adv;
                    // Re-run NPC-portrait promotion: any new portrait
                    // files dropped into NPCs/ since last import
                    // become campaign.npcs stubs. Existing entries
                    // are de-duped by name and portrait path.
                    c.promote_adventure_portraits_to_npcs(adv_idx);
                    let _ = c.save();
                }
                self.preconvert_active_adventure_images();
                self.status_msg(&format!("Re-scanned '{}'.", name), t::OK);
            }
            Err(e) => self.status_msg(&format!("Rescan failed: {}", e), t::ERR),
        }
    }

    /// Walk the active adventure's image assets and spawn a
    /// background pre-convert pass so glow's PNG cache is warm by
    /// the time the user navigates onto a scene / floorplan / NPC.
    /// Mirrors pointer's adjacent-image preconvert pattern. Cheap on
    /// idle (nothing happens if there's no adventure / no display).
    fn preconvert_active_adventure_images(&mut self) {
        if self.image_display.is_none() {
            self.image_display = Some(glow::Display::new());
        }
        let display = match self.image_display.as_ref() {
            Some(d) if d.supported() => d,
            _ => return,
        };
        let (cell_w, cell_h) = glow::get_cell_size();
        if cell_w == 0 || cell_h == 0 { return; }
        let pixel_w = self.right_pane.w as u32 * cell_w as u32;
        let pixel_h = self.right_pane.h as u32 * cell_h as u32;

        let mut paths: Vec<String> = Vec::new();
        if let Some(camp) = self.campaign.as_ref() {
            for adv in &camp.adventures {
                // Active adventure first (priority warm-up), then
                // the rest so cross-adventure browsing also feels
                // snappy.
                if camp.active_adventure_id == Some(adv.id) {
                    for a in adv.scenes.iter()
                        .chain(adv.floorplans.iter())
                        .chain(adv.npc_portraits.iter())
                    {
                        paths.push(adv.absolute(&a.path).to_string_lossy().to_string());
                    }
                }
            }
            for adv in &camp.adventures {
                if camp.active_adventure_id != Some(adv.id) {
                    for a in adv.scenes.iter()
                        .chain(adv.floorplans.iter())
                        .chain(adv.npc_portraits.iter())
                    {
                        paths.push(adv.absolute(&a.path).to_string_lossy().to_string());
                    }
                }
            }
        }
        if paths.is_empty() { return; }
        let cache = display.png_cache.clone();
        std::thread::spawn(move || {
            glow::preconvert_images(&paths, pixel_w, pixel_h, &cache);
        });
    }

    /// Resolve the cursor to its parent Adventure index, regardless
    /// of which sub-node (group / section / asset) it's on. Lets
    /// `a` / `R` / `D` work without forcing the GM to navigate back
    /// to the adventure header row.
    /// Map a `CampNode` to its `camp_expanded` key (the stable string
    /// that records "this node is open"). Returns None for nodes
    /// that can't be expanded (Pc, Npc, asset rows, placeholders).
    fn expand_key_for_node(&self, node: &CampNode, camp: &Campaign) -> Option<String> {
        match node {
            CampNode::Section(sec) => Some(sec.id().to_string()),
            CampNode::Adventure(i) => camp.adventures.get(*i)
                .map(|a| format!("adv:{}", a.id)),
            CampNode::AdventureGroup(i, kind) => camp.adventures.get(*i)
                .map(|a| format!("adv:{}:{:?}", a.id, kind)),
            CampNode::AdventureSection(adv_idx, sec_idx) => {
                let adv = camp.adventures.get(*adv_idx)?;
                let sec = adv.sections.get(*sec_idx)?;
                // Section is expandable iff it has children: a
                // following heading of deeper level before the next
                // heading at same or higher level. Same rule as the
                // tree builder uses.
                let has_children = section_has_children(adv, *sec_idx);
                if !has_children { return None; }
                Some(format!("advsec:{}:{}", adv.id, sec.line_start))
            }
            _ => None,
        }
    }

    /// For a given CampNode, find the tree index of its visual
    /// parent (used by h/LEFT when the current node is already
    /// collapsed or is a leaf). Section-level nodes → the
    /// top-level CampSection header. Sub-section nodes → the
    /// enclosing ## section if any, else the Sections group, else
    /// the Adventure itself.
    fn parent_tree_index(&self, node: &CampNode) -> Option<usize> {
        let camp = self.campaign.as_ref()?;
        let tree = build_camp_tree(camp, &self.camp_expanded);
        let target_section: Option<CampSection> = match node {
            CampNode::Section(_) => None,
            CampNode::Pc(_)        => Some(CampSection::Pcs),
            CampNode::Adventure(_) => Some(CampSection::Adventures),
            CampNode::AdventureGroup(_, _)    => Some(CampSection::Adventures),
            CampNode::AdventureSection(adv_idx, sec_idx) => {
                // Try to find the enclosing ## (level-2) section
                // earlier in the same adventure. If we ARE a level-2
                // ourselves, fall back to the Sections group.
                let adv = camp.adventures.get(*adv_idx)?;
                let me = adv.sections.get(*sec_idx)?;
                if me.level > 2 {
                    let parent_h2 = adv.sections[..*sec_idx].iter()
                        .rposition(|s| s.level < me.level);
                    if let Some(p) = parent_h2 {
                        // Find that section in the rendered tree.
                        let pos = tree.iter().position(|it|
                            matches!(&it.node,
                                CampNode::AdventureSection(a, s) if *a == *adv_idx && *s == p));
                        if pos.is_some() { return pos; }
                    }
                }
                Some(CampSection::Adventures)
            }
            CampNode::AdventureAsset(_, _, _) => Some(CampSection::Adventures),
            CampNode::Npc(_) => Some(CampSection::Npcs),
            CampNode::Location(_) => Some(CampSection::Locations),
            CampNode::SavedForge(_, _) => Some(CampSection::SavedForge),
            CampNode::Placeholder { section, .. } => Some(*section),
        };
        let target_section = target_section?;
        tree.iter().position(|it| matches!(&it.node,
            CampNode::Section(s) if *s == target_section))
    }

    /// Append a timestamped session note. Works on:
    ///   * An adventure section (note attaches to that section)
    ///   * An adventure header (note attaches to the current_section)
    /// In addition to the in-campaign-json persistence, every note
    /// is appended to `~/.amar/campaigns/<camp>/session.log` so the
    /// running history grows in plain text and Inspire's context dump
    /// picks it up automatically next time Claude is consulted.
    fn append_section_note(&mut self) {
        let cursor_target = self.note_target();
        let Some((adv_idx, sec_idx)) = cursor_target else {
            self.status_msg(
                "Cursor on a section or its parent adventure first, then N.",
                t::WARN);
            return;
        };
        let note = self.footer.ask(" Session note: ", "");
        let note = note.trim().to_string();
        if note.is_empty() {
            self.status_msg("Cancelled (empty note).", t::WARN);
            return;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs()).unwrap_or(0);
        // Plain-text session log: human-readable, append-only,
        // Inspire-fodder. One line per note, timestamped + tagged
        // with the section so the GM can grep back through it.
        let (camp_name, adv_name, sec_heading) = {
            let camp = match self.campaign.as_ref() { Some(c) => c, None => return };
            let adv = match camp.adventures.get(adv_idx) { Some(a) => a, None => return };
            let sec_heading = adv.sections.get(sec_idx)
                .map(|s| s.heading.clone()).unwrap_or_default();
            (camp.name.clone(), adv.name.clone(), sec_heading)
        };
        let log_path = crate::store::campaign_dir(&camp_name).join("session.log");
        let line = format!("[{}] {} § {}: {}\n",
            fmt_ts(now), adv_name, sec_heading, note);
        let _ = std::fs::OpenOptions::new()
            .create(true).append(true).open(&log_path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()));
        // Persist into the campaign.json under the section.
        if let Some(c) = self.campaign.as_mut() {
            if let Some(adv) = c.adventures.get_mut(adv_idx) {
                if let Some(sec) = adv.sections.get_mut(sec_idx) {
                    sec.notes.push(crate::adventure::TimestampedNote {
                        at: now, text: note,
                    });
                }
            }
            let _ = c.save();
        }
        self.status_msg("Note added.", t::OK);
    }

    /// Returns (adventure-index, section-index) for note attachment.
    /// On a section row → that section. On an adventure header or
    /// group → the adventure's current_section (so quick-jot from
    /// the overview works without drilling).
    fn note_target(&self) -> Option<(usize, usize)> {
        let camp = self.campaign.as_ref()?;
        let tree = build_camp_tree(camp, &self.camp_expanded);
        match tree.get(self.camp_idx)?.node.clone() {
            CampNode::AdventureSection(a, s) => Some((a, s)),
            CampNode::Adventure(a)
            | CampNode::AdventureGroup(a, _) => {
                let adv = camp.adventures.get(a)?;
                Some((a, adv.current_section?))
            }
            _ => None,
        }
    }

    /// "End the current session" — writes a marker line to
    /// `session.log` and advances `current_section` to the next
    /// section in document order. Asks for an optional one-line
    /// session title so the log is searchable later.
    fn end_current_session(&mut self) {
        let (camp_name, adv_idx, current_sec) = {
            let camp = match self.campaign.as_ref() {
                Some(c) => c, None => {
                    self.status_msg("No campaign loaded.", t::WARN);
                    return;
                }
            };
            let adv_idx = match camp.active_adventure_id {
                Some(id) => camp.adventures.iter().position(|a| a.id == id),
                None => None,
            };
            let Some(adv_idx) = adv_idx else {
                self.status_msg(
                    "No active adventure. Press a on one first.", t::WARN);
                return;
            };
            let cur = camp.adventures.get(adv_idx)
                .and_then(|a| a.current_section);
            (camp.name.clone(), adv_idx, cur)
        };
        let title = self.footer.ask(" Session title (one-liner, ENTER for default): ", "");
        let title = title.trim().to_string();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs()).unwrap_or(0);
        let (adv_name, current_heading, next_idx) = {
            let camp = self.campaign.as_ref().unwrap();
            let adv = &camp.adventures[adv_idx];
            let cur_h = current_sec.and_then(|i| adv.sections.get(i))
                .map(|s| s.heading.clone()).unwrap_or_default();
            // "Next" = the section immediately after current_section
            // in document order. If we were at the last, stay put.
            let next = match current_sec {
                Some(i) if i + 1 < adv.sections.len() => Some(i + 1),
                _ => current_sec,
            };
            (adv.name.clone(), cur_h, next)
        };
        let label = if title.is_empty() {
            format!("(closed at § {})", current_heading)
        } else {
            title.clone()
        };
        let log_path = crate::store::campaign_dir(&camp_name).join("session.log");
        let banner = format!("\n===== SESSION END [{}] · {} · {} =====\n",
            fmt_ts(now), adv_name, label);
        let _ = std::fs::OpenOptions::new()
            .create(true).append(true).open(&log_path)
            .and_then(|mut f| std::io::Write::write_all(&mut f, banner.as_bytes()));
        if let Some(c) = self.campaign.as_mut() {
            if let Some(adv) = c.adventures.get_mut(adv_idx) {
                adv.current_section = next_idx;
            }
            let _ = c.save();
        }
        let next_heading = self.campaign.as_ref()
            .and_then(|c| c.adventures.get(adv_idx))
            .and_then(|a| a.current_section.and_then(|i| a.sections.get(i)))
            .map(|s| s.heading.clone()).unwrap_or_default();
        self.status_msg(
            &format!("Session closed. Next: {}", next_heading),
            t::OK);
    }

    /// Resolve the cursor to an absolute image path, regardless of
    /// whether it's on a section row (first attached image), an
    /// asset row, or an NPC node with a portrait. Returns None for
    /// other targets. Used by `V` (push to player display).
    fn cursor_image_path(&self) -> Option<std::path::PathBuf> {
        let camp = self.campaign.as_ref()?;
        let tree = build_camp_tree(camp, &self.camp_expanded);
        let node = tree.get(self.camp_idx)?.node.clone();
        match node {
            CampNode::AdventureSection(adv_idx, sec_idx) => {
                let adv = camp.adventures.get(adv_idx)?;
                let rel = adv.sections.get(sec_idx)?.attached_images.first()?.clone();
                Some(adv.absolute(&rel))
            }
            CampNode::AdventureAsset(adv_idx, kind, asset_idx) => {
                let adv = camp.adventures.get(adv_idx)?;
                let asset = match kind {
                    AdventureAssetKind::Scene       => adv.scenes.get(asset_idx),
                    AdventureAssetKind::Floorplan   => adv.floorplans.get(asset_idx),
                    AdventureAssetKind::NpcPortrait => adv.npc_portraits.get(asset_idx),
                    AdventureAssetKind::NpcDoc      => None,
                }?;
                Some(adv.absolute(&asset.path))
            }
            CampNode::Npc(i) => {
                let n = camp.npcs.get(i)?;
                if n.portrait_path.is_empty() { return None; }
                Some(std::path::PathBuf::from(&n.portrait_path))
            }
            CampNode::Pc(i) => {
                let p = camp.pcs.get(i)?;
                if p.portrait_path.is_empty() { return None; }
                Some(std::path::PathBuf::from(&p.portrait_path))
            }
            _ => None,
        }
    }

    /// Rename the on-disk file behind the cursor. Currently
    /// supports adventure scene / floorplan / NPC-portrait /
    /// NPC-doc rows. Other targets (PC, NPC sheets, adventures
    /// themselves) get a hint and a no-op for now.
    ///
    /// The new name is taken as the user types it; we keep the
    /// existing extension. After the rename, we kick a rescan on
    /// the parent adventure so the new filename re-runs through
    /// the section-attachment matcher (e.g. rename `1.png` →
    /// `OpeningTrust.png` and it auto-attaches to a "Trust"
    /// section, or just lands cleanly with its new label).
    fn rename_under_cursor(&mut self) {
        // Resolve target via a small closure so `?` returns to it,
        // not the outer fn.
        let target: Option<(usize, std::path::PathBuf)> = (|| {
            let camp = self.campaign.as_ref()?;
            let tree = build_camp_tree(camp, &self.camp_expanded);
            match tree.get(self.camp_idx).map(|i| i.node.clone()) {
                Some(CampNode::AdventureAsset(adv_idx, kind, asset_idx)) => {
                    let adv = camp.adventures.get(adv_idx)?;
                    let asset = match kind {
                        AdventureAssetKind::Scene       => adv.scenes.get(asset_idx),
                        AdventureAssetKind::Floorplan   => adv.floorplans.get(asset_idx),
                        AdventureAssetKind::NpcPortrait => adv.npc_portraits.get(asset_idx),
                        AdventureAssetKind::NpcDoc      => adv.npc_docs.get(asset_idx),
                    }?;
                    Some((adv_idx, adv.absolute(&asset.path)))
                }
                _ => None,
            }
        })();
        let Some((adv_idx, old_path)) = target else {
            self.status_msg(
                "Cursor on a scene / floorplan / NPC image to rename it.",
                t::WARN);
            return;
        };
        let old_name = old_path.file_name().and_then(|n| n.to_str())
            .map(|s| s.to_string()).unwrap_or_default();
        let ext = old_path.extension().and_then(|e| e.to_str())
            .map(|s| s.to_string()).unwrap_or_default();
        let stem = old_path.file_stem().and_then(|s| s.to_str())
            .map(|s| s.to_string()).unwrap_or_default();
        let answer = self.footer.ask(
            &format!(" Rename '{}' → ", old_name),
            &stem);
        let answer = answer.trim().to_string();
        if answer.is_empty() || answer == stem {
            self.status_msg("Rename cancelled.", t::WARN);
            return;
        }
        // Preserve the extension if the user didn't supply one.
        let new_filename = if answer.contains('.') {
            answer
        } else if ext.is_empty() {
            answer
        } else {
            format!("{}.{}", answer, ext)
        };
        let new_path = old_path.with_file_name(&new_filename);
        if new_path == old_path {
            self.status_msg("Rename cancelled (same name).", t::WARN);
            return;
        }
        if new_path.exists() {
            self.status_msg(
                &format!("Target already exists: {}", new_path.display()),
                t::ERR);
            return;
        }
        if let Err(e) = std::fs::rename(&old_path, &new_path) {
            self.status_msg(&format!("Rename failed: {}", e), t::ERR);
            return;
        }
        // Rescan the parent adventure so the new filename gets
        // re-indexed + re-run through the attachment matcher.
        self.adventure_rescan_idx(adv_idx);
        // Wipe any image overlay — the old image-id cache entry
        // is now stale and would re-render the deleted-path file.
        self.clear_overlay_image();
        self.status_msg(
            &format!("Renamed → {}", new_filename), t::OK);
    }

    /// Push the cursor's image to the external player display by
    /// launching feh with a known WM-class. Detached + non-blocking
    /// so amar stays responsive. The user's tile WM places windows
    /// with class "amar-player" on whichever workspace / monitor
    /// they've configured. Falls back to default feh placement if
    /// no rule matches.
    fn push_image_to_player(&mut self) {
        let Some(path) = self.cursor_image_path() else {
            self.status_msg(
                "No image to push (move cursor onto a scene / portrait / section with image).",
                t::WARN);
            return;
        };
        // Re-use the same feh instance per session by closing the old
        // one first — kill any process whose argv contains our marker.
        // Cheap pkill; failure is fine (no instance yet → no-op).
        let _ = std::process::Command::new("pkill")
            .args(["-f", "feh --class amar-player"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        let spawn_res = std::process::Command::new("feh")
            .arg("--class").arg("amar-player")
            .arg("--auto-zoom")
            .arg("--fullscreen")
            .arg(&path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        match spawn_res {
            Ok(_) => self.status_msg(
                &format!("→ player display: {}", path.display()), t::OK),
            Err(e) => self.status_msg(
                &format!("feh launch failed: {} (is feh installed?)", e), t::ERR),
        }
    }

    fn cursor_adventure_idx(&self) -> Option<usize> {
        let camp = self.campaign.as_ref()?;
        let tree = build_camp_tree(camp, &self.camp_expanded);
        match tree.get(self.camp_idx)?.node.clone() {
            CampNode::Adventure(i) => Some(i),
            CampNode::AdventureGroup(i, _) => Some(i),
            CampNode::AdventureSection(i, _) => Some(i),
            CampNode::AdventureAsset(i, _, _) => Some(i),
            _ => None,
        }
    }

    fn status_msg(&mut self, msg: &str, color: u8) {
        self.status = Some((msg.to_string(), color));
        self.render_footer();
    }

    fn handle_lore_key(&mut self, key: &str) {
        // Right-pane scroll keys work regardless of focus. They mirror
        // kastrup's right-pane bindings, so the muscle memory carries.
        match key {
            "S-DOWN"  => { self.right_pane.linedown(); return; }
            "S-UP"    => { self.right_pane.lineup();   return; }
            "S-RIGHT" => { self.right_pane.pagedown(); return; }
            "S-LEFT"  => { self.right_pane.pageup();   return; }
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
                    self.right_pane.ix = 0;
                }
            }
            "k" | "UP" => {
                if self.lore_idx > 0 {
                    self.lore_idx -= 1;
                    self.right_pane.ix = 0;
                }
            }
            "PgDOWN" => {
                let step = (self.left_pane.h as usize).saturating_sub(2).max(1);
                self.lore_idx = (self.lore_idx + step).min(tree.len().saturating_sub(1));
                self.right_pane.ix = 0;
            }
            "PgUP" => {
                let step = (self.left_pane.h as usize).saturating_sub(2).max(1);
                self.lore_idx = self.lore_idx.saturating_sub(step);
                self.right_pane.ix = 0;
            }
            "g" | "HOME" => { self.lore_idx = 0; self.right_pane.ix = 0; }
            "G" | "END" => {
                self.lore_idx = tree.len().saturating_sub(1);
                self.right_pane.ix = 0;
            }
            // Pointer-style: l = expand, h = collapse-or-parent,
            // SPACE = toggle, ENTER = open (== toggle in the lore
            // case since there's nothing else to activate).
            "l" | "RIGHT" => {
                if let Some(item) = tree.get(self.lore_idx) {
                    if let Node::CanonCategory { category, .. } = &item.node {
                        if !self.lore_expanded.iter().any(|e| e == category) {
                            self.lore_expanded.push(category.clone());
                        }
                    }
                }
            }
            " " | "SPACE" | "ENTER" => {
                if let Some(item) = tree.get(self.lore_idx) {
                    if let Node::CanonCategory { category, .. } = &item.node {
                        if let Some(pos) = self.lore_expanded.iter().position(|e| e == category) {
                            self.lore_expanded.remove(pos);
                        } else {
                            self.lore_expanded.push(category.clone());
                        }
                    }
                }
            }
            "h" | "LEFT" => {
                if let Some(item) = tree.get(self.lore_idx) {
                    match &item.node {
                        Node::CanonCategory { category, .. } => {
                            // Collapse-if-open, else jump to a parent
                            // (no parents at the top level of lore,
                            // so just collapse).
                            self.lore_expanded.retain(|e| e != category);
                        }
                        Node::CanonEntry { .. } => {
                            // Leaf — jump up to enclosing category +
                            // collapse it. Same behaviour as before.
                            let mut i = self.lore_idx;
                            while i > 0 {
                                i -= 1;
                                if let Some(it) = tree.get(i) {
                                    if let Node::CanonCategory { category, .. } = &it.node {
                                        self.lore_expanded.retain(|e| e != category);
                                        self.lore_idx = i;
                                        self.right_pane.ix = 0;
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
            "j" | "DOWN" => self.right_pane.linedown(),
            "k" | "UP"   => self.right_pane.lineup(),
            "PgDOWN" | " " | "SPACE" => self.right_pane.pagedown(),
            "PgUP"   | "b" => self.right_pane.pageup(),
            "g" | "HOME" => self.right_pane.ix = 0,
            "G" | "END"  => {
                // Page down repeatedly until we've hit the bottom; cheap
                // because each call is a couple of pointer ops.
                for _ in 0..200 { self.right_pane.pagedown(); }
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
        for (i, tab) in Tab::all().iter().enumerate() {
            let label = format!(" [{}] {} ", i + 1, tab.name());
            if *tab == self.tab {
                tab_strip.push_str(&style::bold(&style::fg(&label, t::ACCENT)));
            } else {
                tab_strip.push_str(&style::fg(&label, t::FG_MUTED));
            }
        }
        let line = format!(" {}    {}    {}",
            style::bold(&style::fg("amar", t::TAN)),
            tab_strip,
            style::fg(&format!("{} | {}", camp_str, date_str), t::FG));
        // full_refresh (rather than say()'s diff-refresh) so the bar
        // survives a clear_screen() — kicked off by rebuild_panes when
        // w/W cycles the layout.
        self.header.set_text(&line);
        self.header.full_refresh();
    }

    fn render_body(&mut self) {
        if self.tab_has_two_panes() {
            // Paint focus markers once for both two-pane tabs, then
            // delegate left+right content to the per-tab renderer.
            self.paint_focus_markers();
            match self.tab {
                Tab::Lore     => self.render_lore_panes(),
                Tab::Campaign => self.render_campaign_panes(),
                Tab::Forge    => self.render_forge_panes(),
                _ => {}
            }
            return;
        }
        let lines = match self.tab {
            Tab::Session  => self.render_session(),
            Tab::Inspire  => self.render_inspire(),
            _ => unreachable!(),
        };
        // Wipe the two-pane layout when switching to a single-pane tab.
        self.left_marker.clear();
        self.left_pane.clear();
        self.right_marker.clear();
        self.right_pane.clear();
        self.body.set_text(&lines.join("\n"));
        self.body.full_refresh();
    }

    /// Paint the bright-yellow / dim-grey ▏ stripe for the active and
    /// inactive panes. Centralised so Lore and Campaign render the same
    /// way; the per-tab content renderers just fill left_pane and
    /// right_pane afterwards.
    fn paint_focus_markers(&mut self) {
        let left_active = self.focus == Focus::Left;
        let right_active = self.focus == Focus::Right;
        let h = self.left_marker.h as usize;
        let stripe = vec!["\u{258F}"; h].join("\n");
        self.left_marker.fg = if left_active { t::ACCENT as u16 } else { t::FG_FAINT as u16 };
        self.left_marker.set_text(&stripe);
        self.left_marker.full_refresh();
        self.right_marker.fg = if right_active { t::ACCENT as u16 } else { t::FG_FAINT as u16 };
        self.right_marker.set_text(&stripe);
        self.right_marker.full_refresh();
    }

    fn render_lore_panes(&mut self) {
        // Build the tree against the current expanded-set. Cheap (~ms).
        let tree = Tree::build(&self.canon, &self.lore_expanded);
        if self.lore_idx >= tree.len().max(1) {
            self.lore_idx = tree.len().saturating_sub(1);
        }

        let tree_active = self.focus == Focus::Left;
        // Tree pane: one line per item, expandable categories get +/-.
        // Cursor row: bright yellow + bold when Tree has focus, dim
        // when Content has focus.
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
                    style::bold(&style::fg(&row, t::ACCENT))
                } else {
                    style::fg(&row, t::FG_DIM)
                }
            } else {
                match &item.node {
                    Node::Doc { .. } => row,
                    Node::CanonCategory { .. } => style::fg(&row, t::STEEL),
                    Node::CanonEntry { .. } => style::fg(&row, t::FG),
                }
            };
            tree_lines.push(line);
        }
        self.left_pane.set_text(&tree_lines.join("\n"));
        self.left_pane.ix = scroll_offset(self.lore_idx, tree.len(), self.left_pane.h as usize);
        self.left_pane.full_refresh();

        // Body pane: render the selected item's content.
        let content = match tree.get(self.lore_idx) {
            Some(item) => match &item.node {
                Node::Doc { body, .. } => lore::render_markdown(body, self.right_pane.w as usize),
                Node::CanonCategory { title, category, .. } => {
                    let mut out = vec![
                        String::new(),
                        style::bold(&style::fg(title, t::ACCENT)),
                        style::fg(&"-".repeat(title.chars().count()), t::FG_DIM),
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
        self.right_pane.set_text(&content.join("\n"));
        self.right_pane.full_refresh();
    }

    fn render_footer(&mut self) {
        if let Some((ref msg, color)) = self.status {
            let pad = self.cols.saturating_sub(crust::display_width(msg) as u16 + 12 + 1) as usize;
            let right = format!("amar v{}", VERSION);
            let line = format!("{}{}{}", style::fg(msg, color), " ".repeat(pad), style::fg(&right, t::FG_DIM));
            self.footer.set_text(&line);
            self.footer.full_refresh();
            return;
        }
        let hint = match self.tab {
            Tab::Session  => " j/k:select  +/-:HP  M/m:MF up/down  a:add  A:add-all-PCs  d:remove  c:clear  N:note  E:end-session  o:skill  O:combat",
            Tab::Forge    => match self.focus {
                Focus::Left  => " TAB:focus-output  j/k:list  ENTER:run  C-LEFT/RIGHT:tabs  ?:help",
                Focus::Right => " TAB:focus-list  ↑↓:line  PgUp/PgDn:page  g/G:top/end  C-LEFT/RIGHT:tabs",
            },
            Tab::Campaign => match self.focus {
                Focus::Left  => " TAB:focus  j/k/PgDn/PgUp:tree  l/h/SPACE:expand  V:player  N:note/new-adv  G:gen-img  c:rename  E:end-session  +:promote  I:import  a:active  R:rescan  D:delete  C-l:refresh  ?:help",
                Focus::Right => " l/h:±1  j/k:±10  ENTER:edit  +:skill  M:melee  I:missile  S:spell  TAB:focus",
            },
            Tab::Lore     => match self.focus {
                Focus::Left  => " TAB:focus-content  j/k:tree  l/h:expand/collapse  C-LEFT/RIGHT:tabs  ?:help",
                Focus::Right => " TAB:focus-tree  ↑↓:line  PgUp/PgDn:page  g/G:top/end  C-LEFT/RIGHT:tabs  ?:help",
            },
            Tab::Inspire  => " 1-5:tabs  C-LEFT/RIGHT:tabs  o:skill  O:combat  C:new-camp  L:load  ?:help  q:quit",
        };
        // Right-align the version. Pad with spaces between hint and version.
        let right = format!("amar v{} ", VERSION);
        let hw = crust::display_width(hint);
        let rw = crust::display_width(&right);
        let pad = (self.cols as usize).saturating_sub(hw + rw);
        let line = format!("{}{}{}", style::fg(hint, t::FG_DIM), " ".repeat(pad), style::fg(&right, t::FG_DIM));
        self.footer.set_text(&line);
        self.footer.full_refresh();
    }

    // --- Tab body renderers ---

    fn render_session(&self) -> Vec<String> {
        const LBL: u8 = 245;
        let mut out: Vec<String> = Vec::new();
        out.push(String::new());
        let Some(camp) = self.campaign.as_ref() else {
            out.push(style::bold("  Session").to_string());
            out.push(String::new());
            out.push("  No campaign loaded. Press C to create one or L to load.".into());
            return out;
        };
        // Header with active-adventure pointer so the GM keeps
        // their place even without flipping tabs.
        let mut header = format!("  Session — Combat HUD");
        if let Some(id) = camp.active_adventure_id {
            if let Some(adv) = camp.adventures.iter().find(|a| a.id == id) {
                header.push_str(&format!("           {}", adv.name));
                if let Some(s) = adv.current_section.and_then(|i| adv.sections.get(i)) {
                    header.push_str(&format!(" § {}", s.heading));
                }
            }
        }
        out.push(style::bold(&style::fg(&header, t::ACCENT)).to_string());
        out.push(String::new());

        if camp.combatants.is_empty() {
            out.push(style::fg("  Combat HUD empty.", t::FG_MUTED).to_string());
            out.push(String::new());
            out.push(style::fg("  A      add ALL PCs to the fight", LBL).to_string());
            out.push(style::fg("  a      add one PC or NPC by name", LBL).to_string());
            out.push(String::new());
            out.push(style::fg("  Also useful:", t::FG_MUTED).to_string());
            out.push(style::fg("  o      private skill O6 (roll, status line)", LBL).to_string());
            out.push(style::fg("  O      private combat O6", LBL).to_string());
            out.push(style::fg("  N      jot a session note onto the current section", LBL).to_string());
            return out;
        }

        let cursor_idx = camp.combat_idx.min(camp.combatants.len().saturating_sub(1));
        // Build the table. Pre-compute the widest name so the columns
        // line up regardless of who's in the fight.
        let name_w = camp.combatants.iter().filter_map(|r| {
            let n = match r {
                CombatRef::Pc(i)  => camp.pcs.get(*i).map(|c| c.name.as_str()),
                CombatRef::Npc(i) => camp.npcs.get(*i).map(|c| c.name.as_str()),
            }?;
            Some(crust::display_width(n))
        }).max().unwrap_or(8).max(8).min(20);

        let mut emit_section = |out: &mut Vec<String>, label: &str, want_pc: bool| {
            let mut emitted = false;
            for (i, r) in camp.combatants.iter().enumerate() {
                let is_pc = matches!(r, CombatRef::Pc(_));
                if is_pc != want_pc { continue; }
                if !emitted {
                    out.push(style::fg(&format!("  {}", label), t::TAN).to_string());
                    emitted = true;
                }
                let ch = match r {
                    CombatRef::Pc(idx)  => camp.pcs.get(*idx),
                    CombatRef::Npc(idx) => camp.npcs.get(*idx),
                };
                let Some(ch) = ch else {
                    out.push(format!("  (missing roster entry)"));
                    continue;
                };
                let bp_max = ch.bp_max();
                let mf_max = ch.mf_max();
                let bp_cur = ch.bp_current;
                let mf_cur = ch.mf_current;
                let bp_ok = bp_cur > 0;
                let mf_ok = mf_cur > 0;
                let status = if bp_cur <= 0 {
                    style::fg("DOWN", t::ERR).to_string()
                } else if (bp_cur as f32) / (bp_max.max(1) as f32) < 0.34 {
                    style::fg("LOW", t::WARN).to_string()
                } else {
                    style::fg("ok", t::OK).to_string()
                };
                let cursor = if i == cursor_idx { style::fg("→", t::ACCENT).to_string() } else { " ".to_string() };
                let bp_tick = if bp_ok { "\u{2713}" } else { "\u{2717}" };
                let mf_tick = if mf_ok { "\u{2713}" } else { "\u{2717}" };
                out.push(format!("  {} {:<width$}  BP {:>3}/{:<3} {}   MF {:>2}/{:<2} {}   DB {}  MD {}  React {}   {}",
                    cursor,
                    ch.name,
                    bp_cur, bp_max, bp_tick,
                    mf_cur, mf_max, mf_tick,
                    ch.db(),
                    ch.md(),
                    ch.skill_total(crate::pc::Char::Mind, "Awareness", "Reaction Speed"),
                    status,
                    width = name_w));
            }
            if emitted {
                out.push(String::new());
            }
        };

        emit_section(&mut out, "Player characters", true);
        emit_section(&mut out, "NPCs / Encounter", false);

        out.push(style::fg("  j/k:select  +/-:HP  M/m:MF up/down  a:add  A:add all PCs  d:remove  c:clear", LBL).to_string());
        out.push(style::fg("  N:note    o:skill O6    O:combat O6    E:end session", LBL).to_string());
        out
    }

    /// Fixed list of generators offered by the Forge tab. Ported one
    /// at a time from Amar-Tools. The "(soon)" entries are placeholders
    /// — pressing ENTER on them just shows a "not yet ported" line.
    /// O6 dice rolls don't live here — they're bound globally to the
    /// `o` key and surface in the status line.
    const FORGE_LIST: &'static [(&'static str, ForgeGen)] = &[
        ("Weather — today",          ForgeGen::WeatherToday),
        ("Weather — current month",  ForgeGen::WeatherMonth),
        ("Names — pick category",    ForgeGen::Names),
        ("NPC — pick chartype",      ForgeGen::Npc),
        ("Encounter — by terrain",   ForgeGen::Encounter),
        ("Town — castle/village/city",ForgeGen::Town),
    ];

    fn handle_forge_key(&mut self, key: &str) {
        match key {
            "S-DOWN"  => { self.right_pane.linedown(); return; }
            "S-UP"    => { self.right_pane.lineup();   return; }
            "S-RIGHT" => { self.right_pane.pagedown(); return; }
            "S-LEFT"  => { self.right_pane.pageup();   return; }
            // Promote an encounter NPC straight into the active campaign's
            // PC roster. Works regardless of focus so the user doesn't
            // have to TAB through panes mid-table.
            "+" => { self.try_promote_from_forge(); return; }
            _ => {}
        }
        match self.focus {
            Focus::Left  => self.handle_forge_list_key(key),
            Focus::Right => self.handle_forge_output_key(key),
        }
    }

    fn handle_forge_list_key(&mut self, key: &str) {
        let n = Self::FORGE_LIST.len();
        match key {
            "j" | "DOWN" => {
                if self.forge_idx + 1 < n { self.forge_idx += 1; self.right_pane.ix = 0; }
            }
            "k" | "UP" => {
                if self.forge_idx > 0 { self.forge_idx -= 1; self.right_pane.ix = 0; }
            }
            "g" => { self.forge_idx = 0; self.right_pane.ix = 0; }
            "G" => { self.forge_idx = n.saturating_sub(1); self.right_pane.ix = 0; }
            "ENTER" | "l" | "RIGHT" | " " | "SPACE" => self.run_forge(),
            _ => {}
        }
    }

    fn handle_forge_output_key(&mut self, key: &str) {
        // Note: the relations-map binding (`r`) is intercepted by the
        // global key dispatcher (so it works regardless of focus) and
        // never reaches this handler. Any other navigation key flips
        // the image overlay back to text so scrolling the building
        // list isn't blocked by a stale picture.
        if self.forge_town_image {
            self.hide_town_relations();
        }
        match key {
            "j" | "DOWN" => self.right_pane.linedown(),
            "k" | "UP"   => self.right_pane.lineup(),
            "PgDOWN" | " " | "SPACE" => self.right_pane.pagedown(),
            "PgUP"   | "b" => self.right_pane.pageup(),
            "g" | "HOME" => { self.right_pane.ix = 0; }
            "G" | "END" => {
                for _ in 0..200 { self.right_pane.pagedown(); }
            }
            _ => {}
        }
    }

    /// Render the last-generated town's relationship graph as a PNG
    /// (via `dot -Tpng`) and paint it inline in the right pane using
    /// the glow terminal image protocol (kitty / sixel / braille).
    /// Idempotent: a second press refreshes the image on the same pane.
    fn show_town_relations(&mut self) {
        let Some(town) = self.forge_town.clone() else {
            self.status_msg("No town yet. Generate one first (Forge → Town).", t::WARN);
            return;
        };
        if town.relations.persons.len() < 2 {
            self.status_msg("Town has too few residents for a relationship map.", t::WARN);
            return;
        }
        let png_path = match crate::forge::town::render_png(&town) {
            Ok(p) => p,
            Err(e) => {
                self.status_msg(
                    &format!("Relations render failed: {} (is graphviz `dot` installed?)", e),
                    t::ERR,
                );
                return;
            }
        };
        // Lazy-init the glow display the first time someone asks for
        // an image. This keeps cold-start cheap for users who only
        // ever use the text-only generators.
        if self.image_display.is_none() {
            self.image_display = Some(glow::Display::new());
        }
        // Clear the underlying right-pane text first so the image
        // sits on a clean background, not on top of the building list.
        self.right_pane.set_text("");
        self.right_pane.full_refresh();
        let shown = self.image_display.as_mut().map(|d| {
            d.show(
                &png_path.to_string_lossy(),
                self.right_pane.x,
                self.right_pane.y,
                self.right_pane.w,
                self.right_pane.h,
            )
        }).unwrap_or(false);
        if shown {
            self.forge_town_image = true;
            let truncated_hint = if town.relations.truncated {
                format!("  (first {} residents — town is larger; press any nav key to dismiss)",
                    town.relations.persons.len())
            } else {
                "  (press any nav key to dismiss)".to_string()
            };
            self.status_msg(
                &format!("{} — relationships{}", town.name, truncated_hint),
                t::INFO,
            );
        } else {
            self.status_msg(
                "Image protocol not supported by this terminal (need kitty / sixel).",
                t::WARN,
            );
            self.forge_town_image = false;
            self.right_pane.set_text(&self.forge_output.join("\n"));
            self.right_pane.full_refresh();
        }
    }

    /// Tear down the relations image overlay and restore the text
    /// report in the right pane. Called when the user navigates after
    /// viewing the image (scroll keys, tab switch, etc.).
    fn hide_town_relations(&mut self) {
        if let Some(d) = self.image_display.as_mut() {
            d.clear(
                self.right_pane.x,
                self.right_pane.y,
                self.right_pane.w,
                self.right_pane.h,
                self.cols,
                self.rows,
            );
        }
        self.forge_town_image = false;
        self.right_pane.set_text(&self.forge_output.join("\n"));
        self.right_pane.full_refresh();
    }

    /// Run the generator under the cursor and stash its output for
    /// the right pane. All generators are pure-random so each ENTER
    /// produces a fresh result.
    fn run_forge(&mut self) {
        let gen = match Self::FORGE_LIST.get(self.forge_idx) {
            Some((_, g)) => *g,
            None => return,
        };
        let new_output = match gen {
            ForgeGen::WeatherToday  => self.forge_weather_today(),
            ForgeGen::WeatherMonth  => self.forge_weather_month(),
            ForgeGen::Names         => self.forge_names_prompt(),
            ForgeGen::Npc           => self.forge_npc_prompt(),
            ForgeGen::Encounter     => self.forge_encounter_prompt(),
            ForgeGen::Town          => self.forge_town_prompt(),
            ForgeGen::NotYet(label) => vec![
                style::bold(&style::fg(label, t::AMBER)).to_string(),
                String::new(),
                style::fg("  Not yet ported from Amar-Tools.", t::FG_MUTED).to_string(),
                style::fg("  This is a Phase-3 generator (large data tables).", t::FG_MUTED).to_string(),
            ],
        };
        // Empty Vec = the prompt was cancelled with ESC. Keep the
        // previous forge_output so the screen doesn't blank out.
        if !new_output.is_empty() {
            self.forge_output = new_output;
            self.right_pane.ix = 0;
        }
    }


    fn forge_weather_today(&mut self) -> Vec<String> {
        let date = match self.campaign.as_ref() {
            Some(c) => c.date,
            None => crate::calendar::AmarDate::default(),
        };
        let days = crate::forge::generate_weather(date, 1);
        let d = days[0].clone();
        let mut out = vec![
            style::bold(&style::fg(
                &format!("Weather — {}", d.date.fmt_long()), t::ACCENT)).to_string(),
            String::new(),
        ];
        if !d.special.is_empty() {
            out.push(format!("  {}: {}",
                style::fg("feast", t::AMBER),
                style::fg(&d.special, t::FG)));
        }
        let sky_col = d.weather_color();
        out.push(format!("  {}  {}: {}",
            d.weather_emoji(),
            style::fg("sky",  t::FG_MUTED),
            style::bold(&style::fg(d.weather_text(), sky_col))));
        let wind_col = d.wind_color();
        out.push(format!("  {}  {}: {}",
            style::fg(d.wind_arrow(), wind_col),
            style::fg("wind", t::FG_MUTED),
            style::fg(&d.wind_text(), wind_col)));
        out.push(String::new());
        out.push(style::fg(
            "  Press 'A' for AI flavour — atmosphere over today's sky.",
            t::AMBER,
        ).to_string());
        self.forge_weather = Some(days);
        out
    }

    fn forge_weather_month(&mut self) -> Vec<String> {
        let date = match self.campaign.as_ref() {
            Some(c) => c.date,
            None => crate::calendar::AmarDate::default(),
        };
        // Start at day 1 of the current month.
        let m = date.month();
        let start = crate::calendar::AmarDate::from_ymd(date.year, m, 1);
        let days = crate::forge::generate_weather(start, 28);
        // Single format spec used for both the header and each data
        // row so the columns can't drift the way they did when the two
        // had different paddings.
        //   "  {day:>3}  {special:22}  {sky:38}  {wind}"
        // 2-cell emoji prefix inside the sky cell counts cleanly via
        // crust::display_width.
        const DAY_W: usize = 3;
        const SP_W: usize = 22;
        const SKY_W: usize = 38;
        let row = |day: String, sp: String, sky: String, wind: String| {
            format!("  {}  {}  {}  {}",
                pad_visible(&day, DAY_W),
                pad_visible(&sp,  SP_W),
                pad_visible(&sky, SKY_W),
                wind)
        };
        let mut out = vec![
            style::bold(&style::fg(&format!(
                "Weather — {} {} (Year-{})",
                m, crate::forge::month_name(m), date.year), t::ACCENT)).to_string(),
            String::new(),
            row(
                style::fg("Day",     t::FG_MUTED),
                style::fg("Special", t::FG_MUTED),
                style::fg("Sky",     t::FG_MUTED),
                style::fg("Wind",    t::FG_MUTED),
            ),
        ];
        for d in &days {
            let dom = format!("{:>3}", d.date.day_of_month());
            let sp  = if d.special.is_empty() { "—".to_string() } else { d.special.to_string() };
            // Sky cell: emoji (2 cells) + space (1 cell) + styled
            // description. pad_visible strips ANSI before measuring,
            // so colour codes don't inflate the cell width.
            let sky_col = d.weather_color();
            let sky_cell = format!("{} {}",
                d.weather_emoji(),
                style::fg(d.weather_text(), sky_col));
            let wind_col = d.wind_color();
            let wind_cell = format!("{} {}",
                style::fg(d.wind_arrow(), wind_col),
                style::fg(&d.wind_text(), wind_col));
            out.push(row(
                style::fg(&dom, t::FG_DIM),
                style::fg(&sp, t::AMBER),
                sky_cell,
                wind_cell,
            ));
        }
        out.push(String::new());
        out.push(style::fg(
            "  Press 'A' for AI flavour — atmosphere over a notable day this month.",
            t::AMBER,
        ).to_string());
        self.forge_weather = Some(days);
        out
    }

    fn forge_names_prompt(&mut self) -> Vec<String> {
        // Step 1: render the numbered category list to the right pane
        // first — the list is too long for the footer. 3-col layout.
        let items: Vec<(usize, &str)> = crate::forge::NAME_CATEGORIES.iter()
            .enumerate().map(|(i, (n, _, _))| (i, *n)).collect();
        let pane_w = self.right_pane.w as usize;
        let mut picker = vec![
            style::bold(&style::fg("Names — pick a category", t::ACCENT)).to_string(),
            String::new(),
        ];
        picker.extend(format_picker_columns(&items, 3, pane_w));
        picker.push(String::new());
        picker.push(style::fg("  Type the number on the prompt below.", t::FG_DIM).to_string());
        self.forge_output = picker;
        self.right_pane.set_text(&self.forge_output.join("\n"));
        self.right_pane.ix = 0;
        self.right_pane.full_refresh();

        // Step 2: footer prompts now ask only for the small inputs
        // (number + count), so they fit on the status line cleanly.
        // ESC at any step cancels the whole flow.
        let Some(cat_str) = self.footer.ask_or_cancel(" Category #: ", "0") else {
            self.status_msg("Cancelled.", t::WARN);
            return Vec::new();
        };
        let cat: usize = cat_str.trim().parse().unwrap_or(0);
        let Some(n_str) = self.footer.ask_or_cancel(" How many? ", "10") else {
            self.status_msg("Cancelled.", t::WARN);
            return Vec::new();
        };
        let n: usize = n_str.trim().parse().unwrap_or(10).clamp(1, 200);

        // Step 3: build the result. Caller assigns this back into
        // self.forge_output (overwriting the picker), so the right
        // pane re-renders with the names.
        let names = crate::forge::generate_names(cat, n);
        let cat_label = crate::forge::NAME_CATEGORIES.get(cat)
            .map(|t| t.0).unwrap_or("?");
        let mut out = vec![
            style::bold(&style::fg(&format!("Names — {} (×{})", cat_label, n), t::ACCENT)).to_string(),
            String::new(),
        ];
        for n in names {
            out.push(format!("  {}", n));
        }
        out
    }

    /// NPC generator. Two-step flow:
    ///
    ///   1. Render the chartype list (3-col grid) to the right pane.
    ///   2. Ask for chartype # / level / sex on the status line.
    ///
    /// Output: the **same full character sheet** the Campaign tab
    /// uses — produced by `render_pc_sheet` on the generated
    /// `pc::Character`. No shortcuts, no compact summary; the NPC
    /// gets every cell of the PC sheet (identity rows, hit-location
    /// box, characteristics, attributes, every canonical skill,
    /// open slots, melee + missile + spell tables, equipment).
    fn forge_npc_prompt(&mut self) -> Vec<String> {
        // Step 1 — chartype picker, 3 columns to keep the list short.
        let items: Vec<(usize, &str)> = crate::forge::data::CHARTYPES.iter()
            .enumerate().map(|(i, c)| (i, c.name)).collect();
        let pane_w = self.right_pane.w as usize;
        let mut picker = vec![
            style::bold(&style::fg("NPC — pick a chartype", t::ACCENT)).to_string(),
            String::new(),
        ];
        picker.extend(format_picker_columns(&items, 3, pane_w));
        picker.push(String::new());
        picker.push(style::fg(
            "  Type the number on the prompt below. Level 1-6 (3 average). Sex M/F.",
            244).to_string());
        self.forge_output = picker;
        self.right_pane.set_text(&self.forge_output.join("\n"));
        self.right_pane.ix = 0;
        self.right_pane.full_refresh();

        // Step 2 — small footer prompts. ESC at any step cancels.
        let Some(idx_str) = self.footer.ask_or_cancel(" Chartype #: ", "0") else {
            self.status_msg("Cancelled.", t::WARN);
            return Vec::new();
        };
        let idx: usize = idx_str.trim().parse().unwrap_or(0);
        let cname: &str = crate::forge::data::CHARTYPES.get(idx)
            .map(|c| c.name).unwrap_or("Commoner");
        let Some(lvl_str) = self.footer.ask_or_cancel(" Level (1-6): ", "3") else {
            self.status_msg("Cancelled.", t::WARN);
            return Vec::new();
        };
        let level: u8 = lvl_str.trim().parse().unwrap_or(3).clamp(1, 9);
        let Some(sex_str) = self.footer.ask_or_cancel(" Sex (M/F or empty for random): ", "") else {
            self.status_msg("Cancelled.", t::WARN);
            return Vec::new();
        };
        let sex = sex_str.trim();
        let sex = if sex.eq_ignore_ascii_case("m") { "M" }
                  else if sex.eq_ignore_ascii_case("f") { "F" }
                  else { "" };

        // Step 3 — build the NPC. Use the canon-aware variant so
        // magic-user NPCs get fully-populated wiki spells (with DR,
        // cost, distance, duration, area, effects) instead of
        // placeholder names.
        let npc = crate::forge::npc::build_npc_with_canon(cname, level, sex, &self.canon);
        let (mut lines, _edits) = self.render_pc_sheet(&npc, None);
        // Remember the chartype so the AI prompt can mention it
        // without re-deriving from skills + weapons (which is
        // possible but lossy — a "Hunter" rolled with a Longsword
        // looks identical to a "Warrior" rolled with a Longsword).
        self.forge_npc = Some(npc);
        self.forge_npc_chartype = Some(cname.to_string());
        // AI-enrichment hint, only on the Forge path. The shared
        // `render_pc_sheet` is also used by the Campaign tab's PC
        // viewer where we DON'T want the prompt, so the hint is
        // appended here instead of inside the renderer.
        lines.push(String::new());
        lines.push(style::fg(
            "  Press 'A' for AI flavour — appearance, voice, hook.",
            t::AMBER,
        ).to_string());
        lines
    }

    /// Encounter generator. Picks a terrain + day/night, optional
    /// level modifier; rolls the encounter table and builds N NPCs.
    fn forge_encounter_prompt(&mut self) -> Vec<String> {
        // Step 1 — terrain picker shown on the right pane (3 col).
        let items: Vec<(usize, &str)> = crate::forge::data::TERRAIN_NAMES.iter()
            .enumerate().map(|(i, n)| (i, *n)).collect();
        let pane_w = self.right_pane.w as usize;
        let mut picker = vec![
            style::bold(&style::fg("Encounter — pick terrain", t::ACCENT)).to_string(),
            String::new(),
        ];
        picker.extend(format_picker_columns(&items, 3, pane_w));
        picker.push(String::new());
        picker.push(style::fg(
            "  Type the terrain number below. Day = d (default), Night = n.",
            244).to_string());
        self.forge_output = picker;
        self.right_pane.set_text(&self.forge_output.join("\n"));
        self.right_pane.ix = 0;
        self.right_pane.full_refresh();

        // Step 2 — small footer prompts. ESC at any step cancels.
        let Some(t_str) = self.footer.ask_or_cancel(" Terrain # (0-7): ", "2") else {
            self.status_msg("Cancelled.", t::WARN);
            return Vec::new();
        };
        let t: usize = t_str.trim().parse().unwrap_or(2).clamp(0, 7);
        let Some(dn) = self.footer.ask_or_cancel(" Day or night (d/n): ", "d") else {
            self.status_msg("Cancelled.", t::WARN);
            return Vec::new();
        };
        let day = !dn.trim().eq_ignore_ascii_case("n");
        let Some(lm_str) = self.footer.ask_or_cancel(" Level modifier (e.g. 0, +1, -1): ", "0") else {
            self.status_msg("Cancelled.", t::WARN);
            return Vec::new();
        };
        let lm: i32 = lm_str.trim().parse().unwrap_or(0);

        // Step 3 — roll + format. The encounter is stashed on the App
        // so pressing `A` afterwards can hand the same roll to Claude
        // for atmospheric flavour without re-rolling stat blocks.
        let enc = crate::forge::encounter::build_encounter(t, day, lm);
        let lines = format_encounter(&enc);
        self.forge_encounter = Some(enc);
        lines
    }

    /// Town generator. Two prompts: optional name override, then
    /// target size (number of buildings). Output is the populated
    /// town report.
    fn forge_town_prompt(&mut self) -> Vec<String> {
        // Step 1 — guide. The picker explains the size brackets.
        let guide = vec![
            style::bold(&style::fg("Town — castle / village / town / city", t::ACCENT)).to_string(),
            String::new(),
            style::fg("  Target size = number of buildings.", t::FG_MUTED).to_string(),
            String::new(),
            format!("  {}  {}", style::fg("1-5",    245), style::fg("Castle / small stronghold", t::FG)),
            format!("  {}  {}", style::fg("6-25",   245), style::fg("Village",                  252)),
            format!("  {}  {}", style::fg("26-99",  245), style::fg("Town",                     252)),
            format!("  {}  {}", style::fg("100+",   245), style::fg("City",                     252)),
            String::new(),
            style::fg("  Leave the name blank to auto-pick from the bundled lists.", t::FG_DIM).to_string(),
        ];
        self.forge_output = guide;
        self.right_pane.set_text(&self.forge_output.join("\n"));
        self.right_pane.ix = 0;
        self.right_pane.full_refresh();

        // Step 2 — small footer prompts. ESC at either step cancels.
        let Some(name) = self.footer.ask_or_cancel(" Town name (blank = auto): ", "") else {
            self.status_msg("Cancelled.", t::WARN);
            return Vec::new();
        };
        let Some(size_str) = self.footer.ask_or_cancel(" Target size (number of buildings): ", "15") else {
            self.status_msg("Cancelled.", t::WARN);
            return Vec::new();
        };
        let size: u32 = size_str.trim().parse().unwrap_or(15).clamp(1, 500);

        // Step 3 — build + format. The Town is stashed on the App so
        // the user can press `r` (on the Forge right pane) to flip
        // between the text report and the relationship-map PNG without
        // re-rolling everything.
        let town = crate::forge::town::build_town(name.trim(), size);
        let lines = format_town(&town);
        self.forge_town = Some(town);
        self.forge_town_image = false;
        lines
    }

    fn render_forge_panes(&mut self) {
        // Left: list of generators; cursor row highlighted.
        let left_active = self.focus == Focus::Left;
        let mut left_lines: Vec<String> = Vec::new();
        left_lines.push(String::new());
        left_lines.push(style::bold(&style::fg("  Forge", t::ACCENT)).to_string());
        left_lines.push(String::new());
        for (i, (label, _)) in Self::FORGE_LIST.iter().enumerate() {
            let cursor = if i == self.forge_idx { "→" } else { " " };
            let line = if i == self.forge_idx {
                if left_active {
                    style::bold(&style::fg(&format!("{} {}", cursor, label), t::ACCENT))
                        .to_string()
                } else {
                    style::fg(&format!("{} {}", cursor, label), t::FG).to_string()
                }
            } else {
                style::fg(&format!("{} {}", cursor, label), t::FG_MUTED).to_string()
            };
            left_lines.push(line);
        }
        left_lines.push(String::new());
        left_lines.push(style::fg("  j/k:list  ENTER:run", t::FG_DIM).to_string());
        self.left_pane.set_text(&left_lines.join("\n"));
        self.left_pane.full_refresh();

        // Right: most-recent output, or a placeholder.
        let content = if self.forge_output.is_empty() {
            vec![
                String::new(),
                style::bold(&style::fg("  Forge", t::ACCENT)).to_string(),
                String::new(),
                style::fg("  Pick a generator on the left and press ENTER.", t::FG_MUTED).to_string(),
                String::new(),
                format!("  Canon: {} entries — {} spells, {} rituals, {} potions",
                    self.canon.entries.len(),
                    self.canon.spell_count(),
                    self.canon.ritual_count(),
                    self.canon.potion_count()),
            ].join("\n")
        } else {
            self.forge_output.join("\n")
        };
        self.right_pane.set_text(&content);
        self.right_pane.full_refresh();
    }

    fn render_campaign_panes(&mut self) {
        // No campaign loaded → spell out the load/create flow on the
        // right pane, leave the left blank. Both markers stay dim.
        let Some(camp) = self.campaign.as_ref() else {
            self.left_pane.set_text("");
            self.left_pane.full_refresh();
            let mut hint = vec![
                String::new(),
                style::bold(&style::fg("  No campaign loaded", t::ACCENT)).to_string(),
                String::new(),
                "  C — create a new campaign".into(),
                "  L — load an existing campaign".into(),
            ];
            let existing = list_campaigns();
            if !existing.is_empty() {
                hint.push(String::new());
                hint.push(style::fg("  Existing campaigns:", t::FG_MUTED).to_string());
                for n in existing.iter().take(20) {
                    hint.push(format!("    - {}", n));
                }
            }
            self.right_pane.set_text(&hint.join("\n"));
            self.right_pane.full_refresh();
            return;
        };

        let tree = build_camp_tree(camp, &self.camp_expanded);
        if self.camp_idx >= tree.len().max(1) {
            self.camp_idx = tree.len().saturating_sub(1);
        }
        let tree_active = self.focus == Focus::Left;

        // Left pane: section headers + their items.
        let mut left_lines: Vec<String> = Vec::new();
        left_lines.push(style::bold(&format!(" {}", camp.name)));
        left_lines.push(style::fg(&format!(" {}", camp.date.fmt_header()), t::FG_MUTED));
        left_lines.push(String::new());
        for (i, item) in tree.iter().enumerate() {
            let cursor = if i == self.camp_idx { "→" } else { " " };
            let indent = "  ".repeat(item.depth as usize);
            let glyph = if item.expandable {
                if item.expanded { "-" } else { "+" }
            } else {
                " "
            };
            let title = camp_node_title(camp, &item.node);
            let row = format!("{} {}{} {}", cursor, indent, glyph, title);
            let line = if i == self.camp_idx {
                if tree_active {
                    style::bold(&style::fg(&row, t::ACCENT))
                } else {
                    style::fg(&row, t::FG_DIM)
                }
            } else {
                match &item.node {
                    CampNode::Section(_) => style::fg(&row, t::STEEL),
                    CampNode::Placeholder { .. } => style::fg(&row, t::FG_MUTED),
                    _ => row,
                }
            };
            left_lines.push(line);
        }
        self.left_pane.set_text(&left_lines.join("\n"));
        self.left_pane.ix = scroll_offset(self.camp_idx + 3, // +3 for header lines
            tree.len() + 3, self.left_pane.h as usize);
        self.left_pane.full_refresh();

        // Right pane: content for the selected node. PC nodes return
        // both the displayed lines AND the editable-field map (used by
        // ENTER on the right pane to dispatch inline edits); other
        // nodes return display lines only and we leave self.edits
        // unchanged.
        //
        // Snapshot the previous render's active field id BEFORE we
        // wipe self.edits. render_pc_sheet uses this to bg-highlight
        // the value cell of the field the cursor is currently on; if
        // we cleared first, active_id would always be None and no
        // highlight would ever show.
        let active_id: Option<String> = self.edits.get(self.sheet_idx)
            .map(|e| e.field_id.clone());
        self.edits.clear();
        // Adventure-asset rendering can request a glow image overlay
        // after the text is written. Capture it here, apply post-match.
        let mut pending_img: Option<std::path::PathBuf> = None;
        let content = match tree.get(self.camp_idx).map(|t| t.node.clone()) {
            Some(CampNode::Section(sec)) => self.render_camp_section(camp, sec),
            Some(CampNode::Pc(idx)) => {
                if let Some(pc) = camp.pcs.get(idx) {
                    let (lines, edits) = self.render_pc_sheet(pc, active_id.as_deref());
                    self.edits = edits;
                    if self.sheet_idx >= self.edits.len().max(1) {
                        self.sheet_idx = self.edits.len().saturating_sub(1);
                    }
                    lines
                } else {
                    vec!["(PC not found)".into()]
                }
            }
            Some(CampNode::Adventure(idx)) => {
                self.render_adventure_overview(camp, idx)
            }
            Some(CampNode::AdventureGroup(idx, kind)) => {
                self.render_adventure_group(camp, idx, kind)
            }
            Some(CampNode::AdventureSection(adv_idx, sec_idx)) => {
                let (lines, img) = self.render_adventure_section(camp, adv_idx, sec_idx);
                pending_img = img;
                lines
            }
            Some(CampNode::AdventureAsset(adv_idx, kind, asset_idx)) => {
                let (lines, img) = self.render_adventure_asset(camp, adv_idx, kind, asset_idx);
                pending_img = img;
                lines
            }
            Some(CampNode::Npc(idx)) => {
                // Same renderer as PC — Character is one type, the
                // is_pc flag just tags the roster. Reuses the editable
                // sheet so the GM can tweak stats post-promotion.
                if let Some(npc) = camp.npcs.get(idx) {
                    let (lines, edits) = self.render_pc_sheet(npc, active_id.as_deref());
                    self.edits = edits;
                    if self.sheet_idx >= self.edits.len().max(1) {
                        self.sheet_idx = self.edits.len().saturating_sub(1);
                    }
                    lines
                } else {
                    vec!["(NPC not found)".into()]
                }
            }
            Some(CampNode::Location(_)) => {
                vec![style::fg("(Coming in a later version.)", t::FG_MUTED).to_string()]
            }
            Some(CampNode::SavedForge(kind, idx)) => {
                let (lines, edits) = self.render_saved_forge(camp, kind, idx, active_id.as_deref());
                if let Some(e) = edits {
                    self.edits = e;
                    if self.sheet_idx >= self.edits.len().max(1) {
                        self.sheet_idx = self.edits.len().saturating_sub(1);
                    }
                }
                lines
            }
            Some(CampNode::Placeholder { msg, .. }) => {
                vec![String::new(), style::fg(&format!("  {}", msg), t::FG_MUTED).to_string()]
            }
            None => vec![],
        };
        self.right_pane.set_text(&content.join("\n"));
        self.right_pane.full_refresh();
        // Adventure-asset image overlay: stash from the match arm
        // above, then act on it now that the immutable camp borrow
        // is released and we can mutate self again.
        self.pending_image = pending_img;
        if let Some(path) = self.pending_image.take() {
            self.overlay_image(&path);
        } else if self.adv_image_shown {
            // We had an image up but the new render doesn't want one
            // — clear it so the new text shows through.
            self.clear_overlay_image();
        }
    }

    /// Queue an image to be drawn on the right pane once the current
    /// render cycle finishes laying down its text. Used by
    /// `render_adventure_asset` (scenes / floorplans / portraits).
    fn request_image_display(&mut self, path: std::path::PathBuf) {
        self.pending_image = Some(path);
    }

    fn overlay_image(&mut self, path: &std::path::Path) {
        if self.image_display.is_none() {
            self.image_display = Some(glow::Display::new());
        }
        // Always clear any prior image first — kitty placements
        // don't auto-replace when a new image is drawn at the
        // same coords (each `show` allocates a fresh image-id
        // and stacks on top). Without this, navigating between
        // scenes paints the new one *over* the old one,
        // producing the visible overlap.
        if let Some(d) = self.image_display.as_mut() {
            d.clear(self.right_pane.x, self.right_pane.y,
                self.right_pane.w, self.right_pane.h,
                self.cols, self.rows);
        }
        let shown = self.image_display.as_mut().map(|d| {
            d.show(&path.to_string_lossy(),
                self.right_pane.x, self.right_pane.y,
                self.right_pane.w, self.right_pane.h)
        }).unwrap_or(false);
        self.adv_image_shown = shown;
        if !shown {
            self.status_msg(
                "Image protocol not supported by this terminal (need kitty / sixel).",
                t::WARN);
        }
    }

    fn clear_overlay_image(&mut self) {
        if let Some(d) = self.image_display.as_mut() {
            d.clear(self.right_pane.x, self.right_pane.y,
                self.right_pane.w, self.right_pane.h,
                self.cols, self.rows);
        }
        self.adv_image_shown = false;
    }

    fn render_camp_section(&self, camp: &Campaign, sec: CampSection) -> Vec<String> {
        const LBL: u8 = 245;
        let mut out = vec![String::new()];
        match sec {
            CampSection::Pcs => {
                out.push(style::bold(&style::fg("Player characters", t::ACCENT)));
                out.push(String::new());
                out.push(format!("  {} PC{} in {}.",
                    camp.pcs.len(),
                    if camp.pcs.len() == 1 { "" } else { "s" },
                    camp.name));
                out.push(String::new());
                out.push(style::fg("  l / ENTER  expand the section", LBL).to_string());
                out.push(style::fg("  n          add a new PC", LBL).to_string());
                out.push(style::fg("  D          delete the PC under the cursor", LBL).to_string());
            }
            CampSection::Adventures => {
                out.push(style::bold(&style::fg("Adventures", t::ACCENT)));
                out.push(String::new());
                out.push(format!("  {} adventure{} in {}.",
                    camp.adventures.len(),
                    if camp.adventures.len() == 1 { "" } else { "s" },
                    camp.name));
                if let Some(id) = camp.active_adventure_id {
                    if let Some(a) = camp.adventures.iter().find(|a| a.id == id) {
                        let where_at = a.current_section
                            .and_then(|i| a.sections.get(i))
                            .map(|s| format!(" § {}", s.heading))
                            .unwrap_or_default();
                        out.push(String::new());
                        out.push(format!("  {} {}{}",
                            style::fg("Active:", t::FG_MUTED),
                            style::bold(&style::fg(&a.name, t::AMBER)),
                            style::fg(&where_at, t::AMBER)));
                    }
                }
                out.push(String::new());
                out.push(style::fg("  l / ENTER  expand the section", LBL).to_string());
                out.push(style::fg("  I          import an adventure (point at a directory)", LBL).to_string());
                out.push(style::fg("  a          mark cursor adventure as ACTIVE", LBL).to_string());
                out.push(style::fg("  R          re-scan an adventure's directory", LBL).to_string());
                out.push(style::fg("  D          remove the adventure from this campaign", LBL).to_string());
            }
            CampSection::Npcs => {
                out.push(style::bold(&style::fg("NPCs", t::ACCENT)));
                out.push(String::new());
                out.push(format!("  {} NPC{} in the campaign roster.",
                    camp.npcs.len(),
                    if camp.npcs.len() == 1 { "" } else { "s" }));
                out.push(String::new());
                out.push(style::fg("  Expand this section (l / ENTER) to browse.", LBL).to_string());
                out.push(style::fg("  Add NPCs by promoting an encounter/saved NPC with + on", LBL).to_string());
                out.push(style::fg("  the Forge tab or here.", LBL).to_string());
            }
            CampSection::Locations => {
                out.push(style::bold(&style::fg("Locations", t::ACCENT)));
                out.push(String::new());
                out.push("  Towns + landmarks visited or known to the party.".into());
                out.push("  Land in v0.4.0 alongside the Forge → Town generator.".into());
            }
            CampSection::Calendar => {
                out.push(style::bold(&style::fg("Calendar", t::ACCENT)));
                out.push(String::new());
                out.push(field_row(LBL, "Today", &camp.date.fmt_header()));
                out.push(field_row(LBL, "Bortle", &camp.bortle.to_string()));
                out.push(String::new());
                out.push(style::fg("  Calendar advance + weather hookup land in v0.5.0.", LBL).to_string());
            }
            CampSection::Factions => {
                out.push(style::bold(&style::fg("Factions", t::ACCENT)));
                out.push(String::new());
                out.push("  Faction reputation tracks (King's court, the Calah,".into());
                out.push("  the Cloaks, Dark Dagger, Magick Circle, the gods…)".into());
                out.push("  land in v0.5+.".into());
            }
            CampSection::SavedForge => {
                out.push(style::bold(&style::fg("Forge log", t::ACCENT)));
                out.push(String::new());
                let n_enc  = camp.saved_encounters.len();
                let n_town = camp.saved_towns.len();
                let n_wx   = camp.saved_weather.len();
                let n_npc  = camp.saved_npcs.len();
                let total = n_enc + n_town + n_wx + n_npc;
                out.push(format!("  {} saved artefact{}.", total,
                    if total == 1 { "" } else { "s" }));
                out.push(String::new());
                out.push(format!("    \u{2694}  {} encounter{}",
                    n_enc, if n_enc == 1 { "" } else { "s" }));
                out.push(format!("    \u{263B}  {} NPC{}",
                    n_npc, if n_npc == 1 { "" } else { "s" }));
                out.push(format!("    \u{2302}  {} town{}",
                    n_town, if n_town == 1 { "" } else { "s" }));
                out.push(format!("    \u{2600}  {} weather day{}",
                    n_wx, if n_wx == 1 { "" } else { "s" }));
                out.push(String::new());
                out.push(style::fg("  l / ENTER  expand the section", LBL).to_string());
                out.push(style::fg("  ENTER on a leaf  show the saved artefact", LBL).to_string());
                out.push(style::fg("  D          delete the saved artefact under the cursor", LBL).to_string());
            }
        }
        out
    }

    /// Render the right-pane content for a saved Forge artefact:
    /// the artefact rendered through its existing formatter, the
    /// AI flavour stored alongside it (if any), and a small footer
    /// with metadata (name, when saved).
    fn render_saved_forge(
        &self, camp: &Campaign, kind: SavedKind, idx: usize,
        active_id: Option<&str>,
    ) -> (Vec<String>, Option<Vec<EditableField>>) {
        let mut out: Vec<String> = Vec::new();
        let mut edits: Option<Vec<EditableField>> = None;
        let (display_name, saved_at, flavour): (String, u64, Option<String>);
        match kind {
            SavedKind::Encounter => {
                let Some(s) = camp.saved_encounters.get(idx) else {
                    return (vec!["(missing saved encounter)".into()], None);
                };
                display_name = s.name.clone();
                saved_at = s.created_at;
                flavour = s.flavour.clone();
                out.extend(format_encounter(&s.item));
            }
            SavedKind::Town => {
                let Some(s) = camp.saved_towns.get(idx) else {
                    return (vec!["(missing saved town)".into()], None);
                };
                display_name = s.name.clone();
                saved_at = s.created_at;
                flavour = s.flavour.clone();
                out.extend(format_town(&s.item));
            }
            SavedKind::Weather => {
                let Some(s) = camp.saved_weather.get(idx) else {
                    return (vec!["(missing saved weather)".into()], None);
                };
                display_name = s.name.clone();
                saved_at = s.created_at;
                flavour = s.flavour.clone();
                let d = &s.item;
                out.push(style::bold(&style::fg(
                    &format!("Weather — {}", d.date.fmt_long()), t::ACCENT)).to_string());
                out.push(String::new());
                if !d.special.is_empty() {
                    out.push(format!("  {}: {}",
                        style::fg("feast", t::AMBER),
                        style::fg(&d.special, t::FG)));
                }
                let sky_col = d.weather_color();
                out.push(format!("  {}  {}: {}",
                    d.weather_emoji(),
                    style::fg("sky",  t::FG_MUTED),
                    style::bold(&style::fg(d.weather_text(), sky_col))));
                let wind_col = d.wind_color();
                out.push(format!("  {}  {}: {}",
                    style::fg(d.wind_arrow(), wind_col),
                    style::fg("wind", t::FG_MUTED),
                    style::fg(&d.wind_text(), wind_col)));
            }
            SavedKind::Npc => {
                let Some(s) = camp.saved_npcs.get(idx) else {
                    return (vec!["(missing saved NPC)".into()], None);
                };
                display_name = s.name.clone();
                saved_at = s.created_at;
                flavour = s.flavour.clone();
                let (lines, e) = self.render_pc_sheet(&s.item, active_id);
                edits = Some(e);
                out.extend(lines);
            }
        }
        if let Some(text) = flavour {
            out.push(String::new());
            out.push(style::bold(&style::fg("  AI flavour", t::ACCENT)).to_string());
            out.push(String::new());
            for line in text.lines() {
                out.push(format!("  {}", line));
            }
        }
        out.push(String::new());
        out.push(style::fg(
            &format!("  saved as “{}” at {}", display_name, fmt_unix(saved_at)),
            t::FG_DIM,
        ).to_string());
        (out, edits)
    }

    // ---- Adventure rendering ------------------------------------------

    /// Overview pane shown when the cursor is on an Adventure node
    /// itself (depth-1, before expansion). Reads the live count of
    /// scenes / NPC docs / etc. and the current-section marker, so
    /// the GM can land on the adventure row and see "where am I" at
    /// a glance.
    fn render_adventure_overview(&self, camp: &Campaign, idx: usize) -> Vec<String> {
        const LBL: u8 = 245;
        let Some(adv) = camp.adventures.get(idx) else {
            return vec!["(adventure not found)".into()];
        };
        let mut out: Vec<String> = vec![String::new()];
        out.push(style::bold(&style::fg(&adv.name, t::ACCENT)).to_string());
        out.push(String::new());
        if camp.active_adventure_id == Some(adv.id) {
            out.push(style::fg("  ● ACTIVE adventure", t::OK).to_string());
        } else {
            out.push(style::fg("  press a to set as active adventure", t::FG_MUTED).to_string());
        }
        out.push(String::new());
        if let Some(idx) = adv.current_section {
            if let Some(s) = adv.sections.get(idx) {
                out.push(format!("  {} {}",
                    style::fg("Current section:", t::FG_MUTED),
                    style::bold(&style::fg(&s.heading, t::AMBER))));
            }
        } else {
            out.push(style::fg("  No current section — drill into Sections and ENTER one.", t::FG_MUTED).to_string());
        }
        out.push(String::new());
        out.push(style::fg("  Root:    ", t::FG_MUTED).to_string() + &adv.root_dir);
        if !adv.narrative_md.is_empty() {
            out.push(style::fg("  Markdown:", t::FG_MUTED).to_string() + &format!(" {}", adv.narrative_md));
        }
        out.push(String::new());
        out.push(format!("  {} sections", adv.sections.len()));
        out.push(format!("  {} scenes / {} floorplans / {} NPC portraits / {} NPC docs",
            adv.scenes.len(), adv.floorplans.len(),
            adv.npc_portraits.len(), adv.npc_docs.len()));
        out.push(String::new());
        out.push(style::fg(
            "  l / ENTER to expand. a = set active. r = re-scan from disk. D = remove.",
            LBL).to_string());
        out
    }

    /// Sub-group header view (Sections / Scenes / …). Short and dim
    /// — the real content is one level deeper.
    fn render_adventure_group(&self, camp: &Campaign, idx: usize, kind: AdventureGroupKind) -> Vec<String> {
        let Some(adv) = camp.adventures.get(idx) else {
            return vec!["(adventure not found)".into()];
        };
        let (label, count) = match kind {
            AdventureGroupKind::Sections     => ("Sections", adv.sections.len()),
            AdventureGroupKind::Scenes       => ("Scenes", adv.scenes.len()),
            AdventureGroupKind::Floorplans   => ("Floorplans", adv.floorplans.len()),
            AdventureGroupKind::NpcPortraits => ("NPC portraits", adv.npc_portraits.len()),
            AdventureGroupKind::NpcDocs      => ("NPC docs", adv.npc_docs.len()),
        };
        vec![
            String::new(),
            style::bold(&style::fg(&format!("{} — {}", adv.name, label), t::ACCENT)).to_string(),
            String::new(),
            format!("  {} item{} — l / ENTER to expand.",
                count, if count == 1 { "" } else { "s" }),
        ]
    }

    /// Render the markdown lines belonging to one section. Re-reads
    /// the file from disk on every render so the GM can edit the
    /// .md in scribe and just press `r` to refresh. Minimal
    /// formatting: bolds the heading, dims block-quote lines,
    /// highlights `**bold**` and `*italic*` inline.
    fn render_adventure_section(&self, camp: &Campaign,
        adv_idx: usize, sec_idx: usize)
        -> (Vec<String>, Option<std::path::PathBuf>)
    {
        let Some(adv) = camp.adventures.get(adv_idx) else {
            return (vec!["(adventure not found)".into()], None);
        };
        let Some(sec) = adv.sections.get(sec_idx) else {
            return (vec!["(section not found)".into()], None);
        };
        let body = crate::adventure::section_body(adv, sec_idx);
        let img = sec.attached_images.first().map(|p| adv.absolute(p));
        let mut out: Vec<String> = vec![String::new()];
        out.push(style::bold(&style::fg(&sec.heading, t::ACCENT)).to_string());
        if camp.active_adventure_id == Some(adv.id) && adv.current_section == Some(sec_idx) {
            out.push(style::fg("  ● current section", t::OK).to_string());
        }
        if !sec.attached_images.is_empty() {
            let n = sec.attached_images.len();
            out.push(style::fg(
                &format!("  📷 {} attached image{} (press j/k to dismiss + read)",
                    n, if n == 1 { "" } else { "s" }),
                t::AMBER).to_string());
        }
        out.push(String::new());
        // Pass body lines through with extremely light markdown styling
        // — headings, blockquotes, list bullets. Anything more
        // ambitious lives behind a richer renderer if the user wants
        // one (currently keeps the .md feel verbatim).
        for line in &body {
            if line.starts_with("### ") {
                let body = line.strip_prefix("### ").unwrap_or(line);
                out.push(style::bold(&style::fg(body, t::AMBER)).to_string());
            } else if line.starts_with("## ") {
                let body = line.strip_prefix("## ").unwrap_or(line);
                out.push(style::bold(&style::fg(body, t::ACCENT)).to_string());
            } else if line.starts_with("> ") {
                out.push(style::fg(&inline_md(line), t::FG_MUTED).to_string());
            } else if line.starts_with("---") {
                out.push(style::fg(line, t::FG_DIM).to_string());
            } else {
                out.push(inline_md(line));
            }
        }
        if !sec.notes.is_empty() {
            out.push(String::new());
            out.push(style::fg("  ── Session notes ──", t::FG_MUTED).to_string());
            for n in &sec.notes {
                out.push(format!("  {} {}",
                    style::fg(&format!("[{}]", fmt_ts(n.at)), t::FG_MUTED),
                    n.text));
            }
        }
        (out, img)
    }

    /// Render an asset row. For images, returns the text scaffold +
    /// an absolute path the caller will overlay via glow. For NPC
    /// docs (.npc text files), inlines the file contents and returns
    /// None for the image path.
    fn render_adventure_asset(&self, camp: &Campaign,
        adv_idx: usize, kind: AdventureAssetKind, asset_idx: usize)
        -> (Vec<String>, Option<std::path::PathBuf>)
    {
        let Some(adv) = camp.adventures.get(adv_idx) else {
            return (vec!["(adventure not found)".into()], None);
        };
        let asset = match kind {
            AdventureAssetKind::Scene       => adv.scenes.get(asset_idx),
            AdventureAssetKind::Floorplan   => adv.floorplans.get(asset_idx),
            AdventureAssetKind::NpcPortrait => adv.npc_portraits.get(asset_idx),
            AdventureAssetKind::NpcDoc      => adv.npc_docs.get(asset_idx),
        };
        let Some(asset) = asset else { return (vec!["(asset not found)".into()], None); };
        let abs = adv.absolute(&asset.path);
        match kind {
            AdventureAssetKind::NpcDoc => {
                let mut out = vec![
                    String::new(),
                    style::bold(&style::fg(&asset.name, t::ACCENT)).to_string(),
                    style::fg(&format!("  {}", asset.path), t::FG_MUTED).to_string(),
                    String::new(),
                ];
                match std::fs::read_to_string(&abs) {
                    Ok(text) => out.extend(text.lines().map(|s| s.to_string())),
                    Err(e) => out.push(format!("  (read failed: {})", e)),
                }
                (out, None)
            }
            _ => {
                let lines = vec![
                    String::new(),
                    style::bold(&style::fg(&asset.name, t::ACCENT)).to_string(),
                    style::fg(&format!("  {}", asset.path), t::FG_MUTED).to_string(),
                    String::new(),
                    style::fg("  (image loading via glow…)", t::FG_MUTED).to_string(),
                ];
                (lines, Some(abs))
            }
        }
    }

    /// Mark a section as "current" + make the parent adventure
    /// active. Saves the campaign so the next launch resumes here.
    fn adventure_jump_to_section(&mut self, adv_idx: usize, sec_idx: usize) {
        if let Some(c) = self.campaign.as_mut() {
            let adv_id = match c.adventures.get_mut(adv_idx) {
                Some(adv) => {
                    adv.current_section = Some(sec_idx);
                    adv.id
                }
                None => return,
            };
            c.active_adventure_id = Some(adv_id);
            let _ = c.save();
        }
        self.status_msg("Section marked current.", t::OK);
    }

    /// Render one PC's full character sheet. Mirrors
    /// CharacterSheet-new.xml: Identity, Derived stats, Status, Hit
    /// locations, 3-tier Characteristics + attributes + skills (in
    /// three side-by-side columns when the pane is ≥ 96 cols wide,
    /// stacked vertically otherwise), Melee + Missile weapons,
    /// Spells, Equipment, Notes. Returns the displayed lines plus a
    /// Vec<EditableField> mapping line indices to the field id the
    /// inline editor should target on ENTER.
    fn render_pc_sheet(&self, pc: &crate::pc::Character, active_id: Option<&str>) -> (Vec<String>, Vec<EditableField>) {
        use crate::pc::{ATTRIBUTES, SKILLS, Char, HIT_LOCATIONS, bp_for_location};
        const LBL_ID:    u8 = 245;
        const LBL_PHYS:  u8 = 174;
        const LBL_HIT:   u8 = 117;
        const DICE:      u8 = 220;
        const TITLE:     u8 = 226;
        const PLAYER:    u8 = 244;
        const STATUS_OK: u8 = 46;
        const STATUS_W:  u8 = 220;
        const STATUS_HW: u8 = 208;
        const STATUS_X:  u8 = 196;
        const LBL: u8 = LBL_ID;
        let mut out: Vec<String> = Vec::new();
        let mut edits: Vec<EditableField> = Vec::new();
        let pane_w = self.right_pane.w as usize;

        // --- Top section: title + identity + stats + hit locations.
        // Built into `out` first, then post-processed to overlay a
        // portrait placeholder box. Portrait sits 3 cols right of the
        // hit-location box (which lives below the identity rows in
        // the left half of the top section).
        //
        // hit-location row width = stat_w (14) + hit_text (~28) +
        //   "  BP " (5) + bp_value (1-2) ≈ 49 cols.
        // Portrait left col = 49 + 3 gap = 52 (with the portrait's
        // own 1-col leading space taking col 52, frame on 53+).
        // 19-col cells line the 3rd cell's value start (col 38 + 9
        // chars of "{ } {label}: ") up with the BP-value column on
        // the hit-location row at col 47. So Age and SIZE values
        // visually align with the per-location BP column.
        let id_cell_w: usize = 19;
        let hit_section_w: usize = 49;
        // The portrait inserts itself at column `port_left_col_target`
        // once the top section is built. 17-col gutter past the hit-
        // location box gives the description plenty of room to wrap
        // before the portrait frame starts. Width threshold bumps in
        // step with the new column position so we don't overflow
        // narrow panes.
        let port_left_col_target: usize = hit_section_w + 17;
        // Portrait width: aim for landscape (3:2 .. 16:9). 36 cols at
        // ~12 rows tall gives a comfortable 16:9 aspect. The vertical
        // size is whatever the top section ends up tall — adding a
        // second description row grows the frame by one row for free.
        let port_w: usize = if pane_w >= port_left_col_target + 36 { 36 } else { 0 };
        let top_start = out.len();

        // Title
        let name_disp = if pc.name.is_empty() { "(unnamed)".to_string() } else { pc.name.clone() };
        let bp_max = pc.bp_max().max(1);
        let (state_text, state_color, wound_penalty) =
            if pc.bp_current <= 0          { ("Helpless", STATUS_X,  None) }
            else if pc.bp_current <= bp_max / 4 { ("Heavily Wounded", STATUS_HW, Some(-4)) }
            else if pc.bp_current <= bp_max / 2 { ("Wounded",         STATUS_W,  Some(-2)) }
            else                                { ("Healthy",         STATUS_OK, Some(0))  };
        // Encumbrance: armor m_mod offset by Wield Weapon total
        // (Body + Strength + Strength/Wield Weapon skill). Capped
        // at 0 so a strong fighter in light armor doesn't get a
        // bonus from being light-on-their-feet. Chain mail (-4)
        // with WW total 3 → net -1, matching the Amar table.
        let encumbrance = encumbrance_penalty(pc);
        let status_penalty = wound_penalty.map(|w| w + encumbrance);
        let title = if pc.player.is_empty() {
            style::bold(&style::fg(&name_disp, TITLE))
        } else {
            format!("{}  {}",
                style::bold(&style::fg(&name_disp, TITLE)),
                style::fg(&format!("({})", pc.player), PLAYER))
        };
        out.push(format!(" {}", title));

        // Identity rows — 3 cells × `id_cell_w` (set above). Tight
        // enough that the right portion stays free for the portrait.
        let id_row1: &[(&str, &str, String)] = &[
            ("race",   "Race",   pc.race.clone()),
            ("sex",    "Sex",    pc.gender.clone()),
            ("age",    "Age",    if pc.age == 0 { String::new() } else { pc.age.to_string() }),
        ];
        let id_row2: &[(&str, &str, String)] = &[
            ("height", "Height", if pc.height_cm == 0 { String::new() } else { pc.height_cm.to_string() }),
            ("weight", "Weight", pc.weight_kg.to_string()),
            ("",       "SIZE",   fmt_size(pc.size)),
        ];
        for row in [id_row1, id_row2] {
            let mut cells: Vec<String> = Vec::with_capacity(3);
            for (id, label, value) in row {
                let active = !id.is_empty() && active_id == Some(*id);
                if !id.is_empty() {
                    edits.push(EditableField {
                        line: out.len(),
                        field_id: (*id).to_string(),
                        label: format!(" {}", label),
                        current: value.clone(),
                    });
                }
                let label_styled = style::fg(&format!("{}:", label), LBL_ID);
                cells.push(format!(" {} {}",
                    pad_visible(&label_styled, 7),
                    value_cell(value, 5, active)));
            }
            // Pad first two cells to id_cell_w; leave the third
            // unpadded so the row content stops at col 52 (well
            // before the portrait at col 54). Otherwise trailing
            // whitespace pad on cell 3 would push the portrait right.
            out.push(format!("{}{}{}",
                pad_visible(&cells[0], id_cell_w),
                pad_visible(&cells[1], id_cell_w),
                cells[2]));
        }

        // Birthplace
        let bp_active = active_id == Some("birthplace");
        edits.push(EditableField { line: out.len(),
            field_id: "birthplace".into(),
            label: " Birthplace".into(),
            current: pc.birthplace.clone() });
        out.push(format!(" {} {}",
            style::fg("Birthplace:", LBL_ID),
            value_cell(&pc.birthplace, 12, bp_active)));

        // Description — always reserves TWO rows so the portrait
        // frame sits one row taller on a freshly-rolled PC. Line 1 is
        // `description`, line 2 is `description2`; set_field joins
        // them with `\n` so pc.description stays a single string.
        // Long descriptions still wrap onto extra rows below the
        // editable pair, but those are display-only — the user edits
        // them by pressing ENTER on either of the first two lines.
        let desc_lines: Vec<&str> = pc.description.lines().collect();
        let desc_l1 = desc_lines.first().copied().unwrap_or("");
        let desc_l2 = desc_lines.get(1).copied().unwrap_or("");
        let desc_active  = active_id == Some("description");
        let desc2_active = active_id == Some("description2");
        edits.push(EditableField { line: out.len(),
            field_id: "description".into(),
            label: " Description".into(),
            current: desc_l1.to_string() });
        out.push(format!(" {} {}",
            style::fg("Description:", LBL_ID),
            if desc_active || !desc_l1.is_empty() {
                value_cell(desc_l1, desc_l1.chars().count().max(1), desc_active)
            } else {
                value_cell("", 8, desc_active)
            }));
        edits.push(EditableField { line: out.len(),
            field_id: "description2".into(),
            label: " Description (line 2)".into(),
            current: desc_l2.to_string() });
        // The continuation line is indented under the value column —
        // 13 leading spaces match the visual offset of "Description: ".
        out.push(format!("             {}",
            if desc2_active || !desc_l2.is_empty() {
                value_cell(desc_l2, desc_l2.chars().count().max(1), desc2_active)
            } else {
                value_cell("", 8, desc2_active)
            }));
        // Any extra wrapped lines (line 3+) sit below — not editable
        // through the sheet, just shown for context.
        for cont in desc_lines.iter().skip(2) {
            out.push(format!("             {}", cont));
        }
        out.push(String::new());

        // --- Stats + Hit Locations side by side ---
        // Six rows, each with a left "stat:value" cell and a right
        // hit-location cell. Per-location BP comes from the wiki rule
        // ("50% in head+arms, 80% in body+legs").
        let bp_curr_active = active_id == Some("bp_current");
        let mf_curr_active = active_id == Some("mf_current");
        let bp_max_total = pc.bp_max();
        let stat_cells: Vec<(Option<&str>, &str, String, bool)> = vec![
            (None,             "Status",  status_penalty.map(|p| format!("{:+}", p)).unwrap_or_else(|| state_text.to_string()), false),
            (Some("bp_current"), "BP",     format!("{}/{}", pc.bp_current.max(0), bp_max_total), bp_curr_active),
            (None,             "DB",      pc.db().to_string(),       false),
            (None,             "MD",      pc.md().to_string(),       false),
            (Some("mf_current"), "M.Fort", format!("{}/{}", pc.mf_current.max(0), pc.mf_max()), mf_curr_active),
            (None,             "React.",  pc.reaction().to_string(), false),
        ];
        let dice = ["⚅", "⚄", "⚃", "⚂", "⚁", "⚀"];
        // Stats column on the left, hit-locations on the right.
        // Sized to fit inside the top-left area so the portrait
        // placeholder box on the right has room.
        let stat_w = 14;
        let _stats_total = stat_w;
        for (stat, (loc, die)) in stat_cells.iter().zip(HIT_LOCATIONS.iter().zip(dice.iter())) {
            let (id_opt, label, value, active) = stat;
            let value: &str = value;
            // Left: stat
            if let Some(id) = id_opt {
                edits.push(EditableField {
                    line: out.len(),
                    field_id: (*id).to_string(),
                    label: format!(" {}", label),
                    current: value.to_string(),
                });
            }
            let label_styled = style::fg(&format!("{}:", label), LBL_PHYS);
            let stat_text = format!(" {} {}",
                pad_visible(&label_styled, 7),
                if *active {
                    value_cell(value, value.chars().count().max(3), true)
                } else if id_opt.is_none() && *label == "Status" {
                    // Color the wound-state cell.
                    style::fg(value, state_color)
                } else {
                    value.to_string()
                });

            // Right: hit location
            let hl = pc.hit_locations.get(*loc).cloned().unwrap_or_default();
            let armor_id = format!("hit/{}/armor", loc);
            let ap_id    = format!("hit/{}/ap", loc);
            let armor_active = active_id == Some(&armor_id);
            let ap_active    = active_id == Some(&ap_id);
            edits.push(EditableField { line: out.len(),
                field_id: armor_id.clone(),
                label: format!(" {} armor", loc), current: hl.armor.clone() });
            edits.push(EditableField { line: out.len(),
                field_id: ap_id.clone(),
                label: format!(" {} AP", loc), current: hl.ap.to_string() });
            let loc_bp = bp_for_location(bp_max_total, loc);
            let hit_text = format!(" {} {} {} {} {}",
                style::fg(die, DICE),
                pad_visible(&style::fg(loc, LBL_HIT), 8),
                pad_visible(&value_cell(&hl.armor, 10, armor_active), 10),
                style::fg("AP", LBL_ID),
                value_cell(&hl.ap.to_string(), 2, ap_active));
            // Append BP at the right edge.
            let combined = format!("{}{}  {}  {}",
                pad_visible(&stat_text, stat_w),
                hit_text,
                style::fg("BP", LBL_ID),
                style::fg(&loc_bp.to_string(), t::FG));
            out.push(combined);
        }

        // Push the gap row BEFORE recording top_end so the portrait
        // frame extends one row past the stats/hit-locations area.
        // The bottom border `└──┘` therefore sits in what would
        // otherwise be empty whitespace, giving the portrait a more
        // natural landscape height without consuming any extra space.
        out.push(String::new());

        // Post-process the top section: overlay the portrait
        // placeholder box. Portrait sits 3 cols right of the
        // hit-location table.
        let top_end = out.len();
        let port_left_col = port_left_col_target;
        let port_h = top_end - top_start;
        let img_path: Option<&str> = if pc.portrait_path.is_empty() {
            None
        } else {
            Some(pc.portrait_path.as_str())
        };
        if port_w >= 16 && port_h >= 4 {
            for i in 0..port_h {
                let row_idx = top_start + i;
                let original = out[row_idx].clone();
                let right = portrait_row(i, port_w, port_h, img_path);
                // Pad the original content to start the portrait at
                // `port_left_col` (3 cols right of the hit-location
                // box).
                out[row_idx] = format!("{}{}",
                    pad_visible(&original, port_left_col),
                    right);
            }
        }
        // `id_cell_w` is still used by the identity-rows layout below.
        let _ = id_cell_w;

        // --- 3-tier Attributes & Skills (no header — obvious from
        // the BODY/MIND/SPIRIT column titles) ---
        // The generic open-skill slots are appended to the SPIRIT
        // column so the freed Attunement-skills vertical space is
        // reused. Slot count is dynamic: enough to balance the
        // column heights with BODY / MIND.
        let three_col = pane_w >= 96;
        if three_col {
            let body_col   = render_char_column(pc, Char::Body,   ATTRIBUTES, SKILLS, active_id);
            let mind_col   = render_char_column(pc, Char::Mind,   ATTRIBUTES, SKILLS, active_id);
            let mut spirit_col = render_char_column(pc, Char::Spirit, ATTRIBUTES, SKILLS, active_id);
            // Pad SPIRIT with a header + N slots, where N is chosen to
            // make SPIRIT column at least as tall as the longer of
            // BODY / MIND. Floor of 8 slots so the section is always
            // visibly useful. The slot section starts with 2 lead
            // rows (empty separator + label header) and 1 row per
            // slot.
            let target_h = body_col.lines.len().max(mind_col.lines.len());
            let avail = target_h.saturating_sub(spirit_col.lines.len() + 2);
            let n_slots = avail.max(8).max(pc.open_skills.len());
            let line_offset = spirit_col.lines.len();
            let (slot_lines, slot_edits) = render_open_slots(pc, n_slots, line_offset, active_id);
            spirit_col.lines.extend(slot_lines);
            spirit_col.edits.extend(slot_edits);
            // 3-col gap between BODY / MIND / SPIRIT.
            let col_w = (pane_w / 3).min(36).max(33);
            let max_rows = body_col.lines.len()
                .max(mind_col.lines.len())
                .max(spirit_col.lines.len());
            let merge_start = out.len();
            for i in 0..max_rows {
                let b = body_col.lines.get(i).cloned().unwrap_or_default();
                let m = mind_col.lines.get(i).cloned().unwrap_or_default();
                let s = spirit_col.lines.get(i).cloned().unwrap_or_default();
                out.push(format!("{}{}{}",
                    pad_visible(&b, col_w),
                    pad_visible(&m, col_w),
                    s));
            }
            for col in [&body_col, &mind_col, &spirit_col] {
                for e in &col.edits {
                    edits.push(EditableField {
                        line: merge_start + e.line,
                        field_id: e.field_id.clone(),
                        label: e.label.clone(),
                        current: e.current.clone(),
                    });
                }
            }
        } else {
            for ch in [Char::Body, Char::Mind, Char::Spirit] {
                let col = render_char_column(pc, ch, ATTRIBUTES, SKILLS, active_id);
                let base = out.len();
                for line in &col.lines { out.push(line.clone()); }
                for e in &col.edits {
                    edits.push(EditableField {
                        line: base + e.line,
                        field_id: e.field_id.clone(),
                        label: e.label.clone(),
                        current: e.current.clone(),
                    });
                }
                out.push(String::new());
            }
            // Single-column fallback: render slots as a tail block.
            let line_offset = out.len();
            let n_slots = pc.open_skills.len().max(8);
            let (slot_lines, slot_edits) = render_open_slots(pc, n_slots, line_offset, active_id);
            for ln in slot_lines { out.push(ln); }
            edits.extend(slot_edits);
        }
        out.push(String::new());

        // --- Melee weapons (editable, full character-sheet columns) ---
        // Columns: Name (16) | Skill (10) | H (2) | Init (4) | ±O (4)
        //          | ±D (4) | OFF (4) | DEF (4) | Dam (4) | HP (3)
        out.push(style::bold(&style::fg("Melee weapons", 173)));
        // Numeric-total columns (OFF / DEF) get right-aligned headers
        // so the F-of-OFF lines up with the 1s digit of the value
        // beneath it. Everything else is left-aligned (matches how
        // value_cell / `{:+}` pad).
        out.push(format!("  {} {} {} {} {} {} {} {} {} {}",
            pad_visible(&style::fg("Name",  LBL), 16),
            pad_visible(&style::fg("Skill", LBL), 10),
            pad_visible(&style::fg("H",     LBL), 2),
            pad_visible(&style::fg("Init",  LBL), 4),
            pad_visible(&style::fg("±O",    LBL), 4),
            pad_visible(&style::fg("±D",    LBL), 4),
            style::fg(&format!("{:>4}", "OFF"), LBL),
            style::fg(&format!("{:>4}", "DEF"), LBL),
            pad_visible(&style::fg("Dam",   LBL), 4),
            pad_visible(&style::fg("HP",    LBL), 3)));
        for (idx, w) in pc.weapons.iter().enumerate()
            .filter(|(_, w)| matches!(w.kind, crate::pc::WeaponKind::Melee))
        {
            let line = out.len();
            push_weapon_row(&mut out, &mut edits, pc, idx, w, active_id, line);
        }
        // Always-present add row, navigable as an editable field. ENTER
        // dispatches to add_weapon(Melee). Field id is special so the
        // editor knows to call the add handler instead of set_field.
        push_add_row(&mut out, &mut edits, "weapon_add_melee",
            " Add melee weapon", "(+ add melee weapon — press ENTER)", active_id);
        out.push(String::new());

        // --- Missile weapons (editable) ---
        // Columns: Name (16) | Skill (10) | Init (4) | ±O (4) | s/r (4)
        //          | OFF (4) | Rng (5) | Dam (4) | HP (3)
        out.push(style::bold(&style::fg("Missile weapons", 130)));
        out.push(format!("  {} {} {} {} {} {} {} {} {}",
            pad_visible(&style::fg("Name",  LBL), 16),
            pad_visible(&style::fg("Skill", LBL), 10),
            pad_visible(&style::fg("Init",  LBL), 4),
            pad_visible(&style::fg("±O",    LBL), 4),
            pad_visible(&style::fg("s/r",   LBL), 4),
            style::fg(&format!("{:>4}", "OFF"), LBL),
            pad_visible(&style::fg("Rng",   LBL), 5),
            pad_visible(&style::fg("Dam",   LBL), 4),
            pad_visible(&style::fg("HP",    LBL), 3)));
        for (idx, w) in pc.weapons.iter().enumerate()
            .filter(|(_, w)| matches!(w.kind, crate::pc::WeaponKind::Missile))
        {
            let line = out.len();
            push_weapon_row(&mut out, &mut edits, pc, idx, w, active_id, line);
        }
        push_add_row(&mut out, &mut edits, "weapon_add_missile",
            " Add missile weapon", "(+ add missile weapon — press ENTER)", active_id);
        out.push(String::new());

        // --- Spells (editable, full character-sheet columns) ---
        // Columns: Name (16) | Domain (8) | A/P (3) | DR (3) | Cost (4)
        //          | Cast (8) | Dist (10) | Dur (10) | Area (10)
        //          | Cooldown (10) | Effects (rest of line)
        out.push(style::bold(&style::fg("Spells", 139)));
        out.push(format!("  {} {} {} {} {} {} {} {} {} {} {}",
            pad_visible(&style::fg("Name",     LBL), 16),
            pad_visible(&style::fg("Domain",   LBL), 8),
            pad_visible(&style::fg("A/P",      LBL), 3),
            pad_visible(&style::fg("DR",       LBL), 3),
            pad_visible(&style::fg("Cost",     LBL), 4),
            pad_visible(&style::fg("Cast",     LBL), 8),
            pad_visible(&style::fg("Dist",     LBL), 10),
            pad_visible(&style::fg("Dur",      LBL), 10),
            pad_visible(&style::fg("Area",     LBL), 10),
            pad_visible(&style::fg("Cooldown", LBL), 10),
            style::fg("Effects",                LBL)));
        for (idx, sp) in pc.spells.iter().enumerate() {
            let line = out.len();
            push_spell_row(&mut out, &mut edits, idx, sp, active_id, line);
        }
        push_add_row(&mut out, &mut edits, "spell_add",
            " Add spell", "(+ add spell — press ENTER)", active_id);
        out.push(String::new());

        // Equipment + money
        out.push(style::bold(&style::fg("Equipment", t::TAN)));
        let cloth_active = active_id == Some("clothing");
        edits.push(EditableField { line: out.len(), field_id: "clothing".into(),
            label: " Clothing".into(), current: pc.clothing.clone() });
        out.push(emit_cell(LBL, "Clothing", &pc.clothing, cloth_active));
        for item in &pc.equipment {
            out.push(format!("  • {}", item));
        }
        let money_active = active_id == Some("money");
        edits.push(EditableField { line: out.len(), field_id: "money".into(),
            label: " Money (sp)".into(), current: pc.money_sp.to_string() });
        out.push(emit_cell(LBL, "Money",
            &format!("{} sp", pc.money_sp), money_active));
        out.push(String::new());

        // Notes
        out.push(style::bold(&style::fg("Notes", t::FG_MUTED)));
        let notes_active = active_id == Some("notes");
        edits.push(EditableField { line: out.len(), field_id: "notes".into(),
            label: " Notes".into(), current: pc.notes.clone() });
        if pc.notes.is_empty() {
            // Show the empty-value placeholder, bg-highlighted when active.
            out.push(format!("  {}", value_cell("(none — press ENTER to add)", 32, notes_active)));
        } else {
            // Bg-highlight the first line when active; further lines stay
            // plain so multi-line notes still wrap nicely.
            let mut first = true;
            for line in pc.notes.lines() {
                if first && notes_active {
                    out.push(format!("  {}", value_cell(line,
                        crust::display_width(line).max(1), true)));
                } else {
                    out.push(format!("  {}", line));
                }
                first = false;
            }
        }

        // Sort edits by line so j/k advances row-major across the
        // 3-column 3-tier section (BODY-row1, MIND-row1, SPIRIT-row1,
        // BODY-row2, …). Stable sort preserves the within-row order
        // we appended in (BODY → MIND → SPIRIT, armor → AP → BP, etc).
        edits.sort_by_key(|e| e.line);

        (out, edits)
    }

    fn render_inspire(&self) -> Vec<String> {
        let mut out = Vec::new();
        out.push(String::new());
        out.push(style::bold(&style::fg("  Inspire", t::ACCENT)).to_string());
        out.push(String::new());
        out.push(style::fg(
            "  Hand off to Claude for adventure ideas, NPC voice, plot threads,",
            t::FG).to_string());
        out.push(style::fg(
            "  session recap prose, or any free-form brainstorm. The active",
            t::FG).to_string());
        out.push(style::fg(
            "  campaign — its date, PC roster, and recent session log — plus",
            t::FG).to_string());
        out.push(style::fg(
            "  a pointer to the bundled Amar canon skill is baked into the",
            t::FG).to_string());
        out.push(style::fg(
            "  opening prompt so you can start asking immediately.",
            t::FG).to_string());
        out.push(String::new());
        out.push(style::fg("  Context that will be sent:", t::FG_MUTED).to_string());
        let camp_line = match &self.campaign {
            Some(c) => format!(
                "    • Campaign: {} — date {} — {} PC{} — {} NPC{}",
                c.name, c.date.fmt_header(),
                c.pcs.len(),  if c.pcs.len()  == 1 { "" } else { "s" },
                c.npcs.len(), if c.npcs.len() == 1 { "" } else { "s" }),
            None => "    • Campaign: (none loaded — Claude will get a bare amar context)".into(),
        };
        out.push(style::fg(&camp_line, t::FG_MUTED).to_string());
        out.push(style::fg(
            "    • Canon skill: ~/.claude/skills/amar (mythology / kingdom / world / game system)",
            t::FG_MUTED).to_string());
        out.push(style::fg(
            "    • Session log: last 200 lines from the campaign's session.log (if present)",
            t::FG_MUTED).to_string());
        out.push(String::new());
        out.push(style::bold(&style::fg(
            "  Press ENTER (or i) to start a Claude session. /exit returns you here.",
            t::AMBER)).to_string());
        out.push(String::new());
        out.push(style::fg(
            "  Tip: every call is gated on a keypress — no background polling, no",
            t::FG_DIM).to_string());
        out.push(style::fg(
            "  surprise spend. Claude runs in the foreground; amar resumes on /exit.",
            t::FG_DIM).to_string());
        out
    }

    /// Hand the terminal off to the Claude CLI, seeded with the
    /// active campaign's context. Mirrors kastrup's `:chat` flow:
    /// `Crust::cleanup` → spawn `claude <initial prompt>` → on
    /// return, `Crust::init` + `handle_resize` to repaint.
    fn launch_inspire_claude(&mut self) {
        // Build the context blob. The point isn't to dump every byte
        // we have into the prompt — it's to give Claude enough to skip
        // the "tell me about your campaign" round-trip on the first
        // question.
        let mut ctx = String::new();
        ctx.push_str("You're helping me run a tabletop session of Amar RPG (d6 system, ");
        ctx.push_str("home of the Kingdom of Amar). The mythology, geography, ");
        ctx.push_str("pantheon, and 3-tier game mechanics are bundled as a Claude ");
        ctx.push_str("skill at ~/.claude/skills/amar/SKILL.md and its references/*.md — ");
        ctx.push_str("load them when you need canonical details rather than inventing them.\n\n");
        if let Some(c) = &self.campaign {
            ctx.push_str(&format!("Active campaign: \"{}\"\n", c.name));
            ctx.push_str(&format!("In-game date: {}\n", c.date.fmt_header()));
            if !c.pcs.is_empty() {
                ctx.push_str("\nPlayer characters:\n");
                for pc in &c.pcs {
                    let label = if pc.name.is_empty() { "(unnamed)".to_string() } else { pc.name.clone() };
                    let race = if pc.race.is_empty() { "Human".to_string() } else { pc.race.clone() };
                    ctx.push_str(&format!("  - {} ({} {}, lvl {})\n",
                        label, pc.gender, race, pc.level));
                }
            }
            if !c.npcs.is_empty() {
                ctx.push_str(&format!("\nNPCs in this campaign ({}):\n", c.npcs.len()));
                // List up to 30 by name + race + level so Claude can
                // reference them by name in the response. Beyond 30
                // we summarise — keeps the prompt tight.
                let cap = 30usize.min(c.npcs.len());
                for n in c.npcs.iter().take(cap) {
                    let race = if n.race.is_empty() { "Human".to_string() } else { n.race.clone() };
                    ctx.push_str(&format!("  - {} ({} {}, lvl {})\n",
                        n.name, n.gender, race, n.level));
                }
                if c.npcs.len() > cap {
                    ctx.push_str(&format!("  (+{} more — ask if you need any of them by name)\n",
                        c.npcs.len() - cap));
                }
            }
            if !c.adventures.is_empty() {
                ctx.push_str(&format!("\nAdventures in this campaign ({}):\n", c.adventures.len()));
                for a in &c.adventures {
                    let active = c.active_adventure_id == Some(a.id);
                    let cur = a.current_section.and_then(|i| a.sections.get(i))
                        .map(|s| format!(" § {}", s.heading))
                        .unwrap_or_default();
                    ctx.push_str(&format!("  {} {}{}\n",
                        if active { "● ACTIVE" } else { "  " },
                        a.name, cur));
                }
                // For the active adventure, also pull the immediate
                // context (current section heading + a few surrounding
                // sections) so Claude knows where we are in the story.
                if let Some(active_id) = c.active_adventure_id {
                    if let Some(adv) = c.adventures.iter().find(|a| a.id == active_id) {
                        if let Some(cur_idx) = adv.current_section {
                            ctx.push_str(&format!(
                                "\nCurrent context — adventure \"{}\":\n", adv.name));
                            let start = cur_idx.saturating_sub(2);
                            let end = (cur_idx + 3).min(adv.sections.len());
                            for i in start..end {
                                if let Some(s) = adv.sections.get(i) {
                                    let marker = if i == cur_idx { "→" } else { " " };
                                    ctx.push_str(&format!("  {} {}\n", marker, s.heading));
                                }
                            }
                        }
                    }
                }
            }
            let log_path = crate::store::campaign_dir(&c.name).join("session.log");
            if let Ok(log) = std::fs::read_to_string(&log_path) {
                let tail: Vec<&str> = log.lines().rev().take(200).collect();
                if !tail.is_empty() {
                    ctx.push_str("\nRecent session log (newest first, ~200 lines):\n");
                    for ln in tail.iter().rev() {
                        ctx.push_str("  ");
                        ctx.push_str(ln);
                        ctx.push('\n');
                    }
                }
            }
        } else {
            ctx.push_str("No campaign is currently loaded in amar. Treat me as a GM ");
            ctx.push_str("planning Amar content in general (no specific party state).\n");
        }
        ctx.push_str("\nReady for your first question. /exit when we're done — that returns me to amar.\n");

        // Drop the alt-screen + bracketed-paste so claude has a clean
        // terminal. Same handshake kastrup uses for :chat.
        use std::io::Write as _;
        print!("\x1b[?2004l");
        let _ = std::io::stdout().flush();
        crust::Crust::cleanup();
        crust::Crust::clear_screen();

        let status = std::process::Command::new("claude").arg(&ctx).status();

        crust::Crust::init();
        print!("\x1b[?2004h");
        let _ = std::io::stdout().flush();
        // Force a full repaint. Rebuild the pane layout in case the
        // user resized the terminal during the Claude session, then
        // re-run every render path so headers + tabs + body come back.
        crust::Crust::clear_screen();
        self.rebuild_panes();
        self.render_all();
        match status {
            Ok(s) if s.success() => self.status_msg("Back from Claude.", t::OK),
            Ok(s)                => self.status_msg(&format!("Claude exited with {}", s), t::WARN),
            Err(e)               => self.status_msg(&format!("Could not launch claude: {}", e), t::ERR),
        }
    }

    /// Dispatch the global `A` key on the Forge tab. Picks the most
    /// recently generated artefact and routes to its enricher. The
    /// `ForgeGen` of the currently selected generator decides which
    /// stashed artefact wins — so `A` enriches whatever the user is
    /// looking at, not "whichever was rolled last in any tab".
    fn ai_enrich_forge(&mut self) {
        if !std::process::Command::new("claude").arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            self.status_msg(
                "AI enrich needs `claude` on PATH — not found.",
                t::WARN,
            );
            return;
        }
        let gen = Self::FORGE_LIST.get(self.forge_idx).map(|(_, g)| *g);
        match gen {
            Some(ForgeGen::Npc) => {
                if let (Some(npc), chartype) =
                    (self.forge_npc.clone(), self.forge_npc_chartype.clone())
                {
                    let ct = chartype.unwrap_or_else(|| "Commoner".to_string());
                    self.ai_enrich_npc(&npc, &ct);
                } else {
                    self.status_msg(
                        "No NPC to enrich — roll one first (NPC generator).",
                        t::WARN,
                    );
                }
            }
            Some(ForgeGen::Encounter) => {
                if let Some(enc) = self.forge_encounter.clone() {
                    self.ai_enrich_encounter(&enc);
                } else {
                    self.status_msg(
                        "No encounter to enrich — roll one first.",
                        t::WARN,
                    );
                }
            }
            Some(ForgeGen::Town) => {
                if let Some(town) = self.forge_town.clone() {
                    self.ai_enrich_town(&town);
                } else {
                    self.status_msg(
                        "No town to enrich — generate one first.",
                        t::WARN,
                    );
                }
            }
            Some(ForgeGen::WeatherToday) | Some(ForgeGen::WeatherMonth) => {
                self.ai_enrich_weather();
            }
            _ => {
                self.status_msg(
                    "AI enrich isn't wired for this generator yet.",
                    t::WARN,
                );
            }
        }
    }

    /// Hand the NPC stat block to `claude -p` for character flavour.
    /// Appended below the existing PC-sheet render so the stats and
    /// the prose stay visible together. Same opt-in pattern as
    /// `ai_enrich_encounter` — short prompt, no headers in the
    /// response, canon tone via the bundled skill.
    fn ai_enrich_npc(&mut self, npc: &crate::pc::Character, chartype: &str) {
        let prompt = "You are helping me run a tabletop session of Amar RPG, \
            a d6-based medieval fantasy (Kingdom of Amar — see d6gaming.org \
            and ~/.claude/skills/amar for canon: mythology, geography, \
            pantheon, the wizard school in the capital, faeries and \
            dragons in the surrounding lands). The NPC below was just \
            rolled. Write a SHORT character flavour package the GM can \
            drop on the table:\n\n\
            * APPEARANCE — one or two sentences: face, build, marks, dress.\n\
            * VOICE & MANNERISM — one sentence: how they speak, body \
            language, a tic or habit.\n\
            * HOOK — one sentence: a thread the GM could pull (a debt, a \
            secret, a feud, a missing relative, a stake in something \
            happening).\n\n\
            Continuous prose, paragraphs separated by blank lines. No \
            headers, no bullets, no markdown. Let the canon tone come \
            through — don't impose grim-dark or whimsical defaults.";

        let mut ctx = String::new();
        ctx.push_str(&format!("Name: {}\n", npc.name));
        ctx.push_str(&format!("Race: {}\n", npc.race));
        ctx.push_str(&format!("Sex: {}\n", npc.gender));
        if npc.age > 0 { ctx.push_str(&format!("Age: {}\n", npc.age)); }
        ctx.push_str(&format!("Chartype / role: {}\n", chartype));
        ctx.push_str(&format!("Level: {}\n", npc.level));
        if !npc.birthplace.is_empty() {
            ctx.push_str(&format!("Birthplace: {}\n", npc.birthplace));
        }
        if !npc.description.is_empty() {
            ctx.push_str(&format!("Existing description: {}\n", npc.description));
        }
        // Weapons (names only; stats add noise to the prompt).
        let weapons: Vec<String> = npc.weapons.iter()
            .filter(|w| w.name != "Unarmed")
            .map(|w| w.name.clone())
            .collect();
        if !weapons.is_empty() {
            ctx.push_str(&format!("Weapons: {}\n", weapons.join(", ")));
        }
        // Worship — handy for picking the right "by Walmaer!" oath
        // in the opening line / mannerism.
        if let Some(worship) = npc.skills.get("Worship") {
            let gods: Vec<&str> = worship.keys().map(|s| s.as_str()).collect();
            if !gods.is_empty() {
                ctx.push_str(&format!("Worships: {}\n", gods.join(", ")));
            }
        }
        // Top skills — anything ranked 4+ is distinctive enough to
        // hint at how this NPC has spent their time.
        let mut top: Vec<(String, i32)> = Vec::new();
        for (_attr, m) in &npc.skills {
            for (skill, rank) in m {
                if *rank >= 4 { top.push((skill.clone(), *rank)); }
            }
        }
        top.sort_by(|a, b| b.1.cmp(&a.1));
        top.truncate(6);
        if !top.is_empty() {
            let parts: Vec<String> = top.iter()
                .map(|(k, v)| format!("{} {}", k, v))
                .collect();
            ctx.push_str(&format!("Notable skills: {}\n", parts.join(", ")));
        }

        self.status_msg(
            &format!("Asking Claude about {} — ~5-30s…", npc.name),
            t::INFO,
        );
        self.render_footer();
        use std::io::Write as _;
        let _ = std::io::stdout().flush();

        let answer = match claude_pipe(prompt, &ctx) {
            Ok(s) => s.trim().to_string(),
            Err(e) => {
                self.status_msg(&format!("claude failed: {}", e), t::ERR);
                return;
            }
        };
        if answer.is_empty() {
            self.status_msg("Claude returned empty response.", t::WARN);
            return;
        }

        self.forge_output.retain(|ln| !ln.contains("Press 'A' for AI flavour"));
        self.forge_output.push(String::new());
        self.forge_output.push(
            style::bold(&style::fg("  AI flavour", t::ACCENT)).to_string()
        );
        self.forge_output.push(String::new());
        for line in answer.lines() {
            self.forge_output.push(format!("  {}", line));
        }
        self.right_pane.set_text(&self.forge_output.join("\n"));
        self.right_pane.full_refresh();
        self.status_msg("AI flavour added.", t::OK);
    }

    /// Hand the encounter to `claude -p` for atmospheric flavour.
    /// Returns to amar with the response appended below the existing
    /// stat block — both stay visible in the right pane.
    fn ai_enrich_encounter(&mut self, enc: &crate::forge::encounter::Encounter) {
        let prompt = "You are helping me run a tabletop session of Amar RPG, \
            a d6-based medieval fantasy (Kingdom of Amar — see \
            d6gaming.org and ~/.claude/skills/amar for the canon: \
            mythology, geography, pantheon, the wizard school in the \
            capital, faeries and dragons in the surrounding lands, etc.). \
            The encounter below was just rolled randomly. Write a SHORT \
            atmospheric package the GM can read at the table:\n\n\
            * one sentence on who they are / why they're here (backstory),\n\
            * one sentence on what they want right now (purpose),\n\
            * two or three short sensory beats matching the terrain + time \
            (scenery — sight, sound, smell),\n\
            * one quotable opening line spoken by the leader (or, for \
            non-speaking beasts, one observable behaviour).\n\n\
            Continuous prose, paragraphs separated by blank lines. No \
            headers, no bullets, no markdown. Let the canon tone come \
            through — don't impose grim-dark or whimsical defaults.";

        let mut ctx = String::new();
        ctx.push_str(&format!("Terrain: {}\n", enc.terrain_name()));
        ctx.push_str(&format!("Time of day: {}\n", enc.time_of_day()));
        ctx.push_str(&format!("Category: {}\n", enc.category));
        ctx.push_str(&format!("Spec: {}\n", enc.spec));
        if enc.count > 0 { ctx.push_str(&format!("Count: {}\n", enc.count)); }
        ctx.push_str(&format!("Attitude: {}\n", enc.attitude));
        if !enc.npcs.is_empty() {
            ctx.push_str("\nNPCs (stat-block summary, not for the prose):\n");
            for (i, npc) in enc.npcs.iter().enumerate() {
                ctx.push_str(&format!(
                    "  [{}] {} — {} {}, lvl {}, BP {}\n",
                    i + 1, npc.name, npc.gender, npc.race, npc.level, npc.bp_max(),
                ));
                if let Some(w) = npc.weapons.iter().find(|w| {
                    matches!(w.kind, crate::pc::WeaponKind::Melee) && w.name != "Unarmed"
                }) {
                    ctx.push_str(&format!("      melee: {}\n", w.name));
                }
                if let Some(w) = npc.weapons.iter().find(|w|
                    matches!(w.kind, crate::pc::WeaponKind::Missile))
                {
                    ctx.push_str(&format!("      missile: {}\n", w.name));
                }
            }
        }

        self.status_msg(
            &format!("Asking Claude about {} — ~5-30s…", enc.spec),
            t::INFO,
        );
        self.render_footer();
        use std::io::Write as _;
        let _ = std::io::stdout().flush();

        let answer = match claude_pipe(prompt, &ctx) {
            Ok(s) => s.trim().to_string(),
            Err(e) => {
                self.status_msg(&format!("claude failed: {}", e), t::ERR);
                return;
            }
        };
        if answer.is_empty() {
            self.status_msg("Claude returned empty response.", t::WARN);
            return;
        }

        // Append the prose to the existing forge_output so both the
        // stat block and the AI flavour stay readable. Strip the
        // "Press 'A'" hint line first — once we've enriched, the
        // hint is stale.
        self.forge_output.retain(|ln| !ln.contains("Press 'A' for AI flavour"));
        self.forge_output.push(String::new());
        self.forge_output.push(
            style::bold(&style::fg("  AI flavour", t::ACCENT)).to_string()
        );
        self.forge_output.push(String::new());
        for line in answer.lines() {
            self.forge_output.push(format!("  {}", line));
        }
        self.right_pane.set_text(&self.forge_output.join("\n"));
        self.right_pane.full_refresh();
        self.status_msg("AI flavour added.", t::OK);
    }

    /// Town vignette — give Claude the size class, building roster,
    /// temple gods, and a sample of named residents, ask for a short
    /// overall feel plus a sentence each for the keep / inn / temple
    /// when present. Appended below the existing town report.
    fn ai_enrich_town(&mut self, town: &crate::forge::town::Town) {
        let prompt = "You are helping me run a tabletop session of Amar RPG, \
            a d6-based medieval fantasy (Kingdom of Amar — see d6gaming.org \
            and ~/.claude/skills/amar). The town below was just rolled. \
            Write a SHORT location flavour package the GM can drop on the \
            table:\n\n\
            * OVERALL FEEL — two or three sentences: the smell of the place, \
            the soundscape on the main approach, a recurring sight, the mood \
            of the people.\n\
            * KEEP / STRONGHOLD — one sentence (only if the town has one): \
            who holds it and a small detail.\n\
            * INN — one sentence (only if there is one): the keeper, the \
            atmosphere, who you'd find drinking there.\n\
            * TEMPLE — one sentence (only if there is one): which god, the \
            condition of the building, the priest's reputation.\n\n\
            Continuous prose, paragraphs separated by blank lines. No \
            headers, no bullets, no markdown. Let the canon tone come \
            through — don't impose grim-dark or whimsical defaults.";

        let mut ctx = String::new();
        ctx.push_str(&format!("Name: {}\n", town.name));
        ctx.push_str(&format!("Size class: {} ({} target buildings, {} residents)\n",
            town.size_class, town.target_size, town.total_residents));

        // Building roster grouped by base type.
        use std::collections::BTreeMap;
        let mut counts: BTreeMap<String, u32> = BTreeMap::new();
        let mut temple_gods: Vec<String> = Vec::new();
        for b in &town.buildings {
            if let Some(god) = b.name.strip_prefix("Temple: ") {
                temple_gods.push(god.to_string());
                *counts.entry("Temple".into()).or_insert(0) += 1;
            } else {
                let base = b.name.split(':').next().unwrap_or(&b.name).to_string();
                *counts.entry(base).or_insert(0) += 1;
            }
        }
        ctx.push_str("Buildings:\n");
        let mut entries: Vec<(String, u32)> = counts.into_iter().collect();
        entries.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        for (name, n) in entries.iter().take(12) {
            ctx.push_str(&format!("  - {} ×{}\n", name, n));
        }
        if !temple_gods.is_empty() {
            ctx.push_str(&format!("Temple gods worshipped here: {}\n",
                temple_gods.join(", ")));
        }

        // Sample of named residents — head of each interesting
        // building so Claude has handles for the inn keeper / smith
        // / etc. Cap at ~8 so the prompt stays terse.
        let mut sample: Vec<String> = Vec::new();
        for b in &town.buildings {
            if sample.len() >= 8 { break; }
            let role = b.name.split(':').next().unwrap_or(&b.name).trim();
            if let Some(p) = b.people.first() {
                sample.push(format!(
                    "  - {} ({}, {}/{} · {}) — {}",
                    p.name, role, p.sex, p.age, p.personality, role,
                ));
            }
        }
        if !sample.is_empty() {
            ctx.push_str("Notable named residents (heads of households):\n");
            for line in &sample { ctx.push_str(line); ctx.push('\n'); }
        }

        self.status_msg(
            &format!("Asking Claude about {} — ~5-30s…", town.name),
            t::INFO,
        );
        self.render_footer();
        use std::io::Write as _;
        let _ = std::io::stdout().flush();

        let answer = match claude_pipe(prompt, &ctx) {
            Ok(s) => s.trim().to_string(),
            Err(e) => {
                self.status_msg(&format!("claude failed: {}", e), t::ERR);
                return;
            }
        };
        if answer.is_empty() {
            self.status_msg("Claude returned empty response.", t::WARN);
            return;
        }

        self.forge_output.retain(|ln| !ln.contains("Press 'A' for AI flavour"));
        self.forge_output.push(String::new());
        self.forge_output.push(
            style::bold(&style::fg("  AI flavour", t::ACCENT)).to_string()
        );
        self.forge_output.push(String::new());
        for line in answer.lines() {
            self.forge_output.push(format!("  {}", line));
        }
        self.right_pane.set_text(&self.forge_output.join("\n"));
        self.right_pane.full_refresh();
        self.status_msg("AI flavour added.", t::OK);
    }

    /// Weather mood — pick the most interesting day from the stashed
    /// roll (a feast day wins; otherwise the first day) and ask
    /// Claude for two or three atmospheric sentences fitting the sky
    /// + wind + (optional) feast. Useful at session start to set
    /// the scene before the first scene.
    fn ai_enrich_weather(&mut self) {
        let Some(days) = self.forge_weather.clone() else {
            self.status_msg(
                "No weather to enrich — roll one first (Weather generator).",
                t::WARN,
            );
            return;
        };
        if days.is_empty() {
            self.status_msg("Empty weather batch.", t::WARN);
            return;
        }
        // For month-rolls, prefer a feast day over a plain one — that
        // gives Claude more to grab onto. Falls back to the first day
        // when nothing's notable.
        let day = days.iter()
            .find(|d| !d.special.is_empty())
            .unwrap_or(&days[0])
            .clone();

        let prompt = "You are helping me run a tabletop session of Amar RPG, \
            a d6-based medieval fantasy (Kingdom of Amar — see d6gaming.org \
            and ~/.claude/skills/amar). The weather below was just rolled \
            for a session day. Write TWO OR THREE short atmospheric \
            sentences the GM can read to set the scene: what the sky looks \
            like, how the wind moves through the setting, one sensory \
            detail tied to the time of year, and — if a feast is named — \
            how the day's mood differs from an ordinary one. Continuous \
            prose, no headers, no bullets, no markdown. Match the canon \
            tone — don't impose grim-dark or whimsical defaults.";

        let mut ctx = String::new();
        ctx.push_str(&format!("Date: {}\n", day.date.fmt_long()));
        ctx.push_str(&format!("Sky: {}\n", day.weather_text()));
        ctx.push_str(&format!("Wind: {}\n", day.wind_text()));
        if !day.special.is_empty() {
            ctx.push_str(&format!("Feast / notable day: {}\n", day.special));
        }

        self.status_msg(
            &format!("Asking Claude about the {} sky — ~5-30s…",
                day.date.fmt_long()),
            t::INFO,
        );
        self.render_footer();
        use std::io::Write as _;
        let _ = std::io::stdout().flush();

        let answer = match claude_pipe(prompt, &ctx) {
            Ok(s) => s.trim().to_string(),
            Err(e) => {
                self.status_msg(&format!("claude failed: {}", e), t::ERR);
                return;
            }
        };
        if answer.is_empty() {
            self.status_msg("Claude returned empty response.", t::WARN);
            return;
        }

        self.forge_output.retain(|ln| !ln.contains("Press 'A' for AI flavour"));
        self.forge_output.push(String::new());
        self.forge_output.push(
            style::bold(&style::fg("  AI flavour", t::ACCENT)).to_string()
        );
        self.forge_output.push(String::new());
        for line in answer.lines() {
            self.forge_output.push(format!("  {}", line));
        }
        self.right_pane.set_text(&self.forge_output.join("\n"));
        self.right_pane.full_refresh();
        self.status_msg("AI flavour added.", t::OK);
    }

    /// Snapshot the Forge artefact under the cursor (encounter / npc /
    /// town / weather) into the active campaign's `saved_*` vector
    /// and write the campaign to disk. Pulls any AI flavour that was
    /// already produced for the artefact out of `forge_output` so it
    /// rides along on the saved item — no need to press `A` again
    /// after restart.
    fn save_forge_artefact(&mut self) {
        let Some(_) = self.campaign.as_ref() else {
            self.status_msg(
                "Load a campaign first (Campaign tab → C / L).",
                t::WARN,
            );
            return;
        };
        let gen = Self::FORGE_LIST.get(self.forge_idx).map(|(_, g)| *g);
        let suggested: String = match gen {
            Some(ForgeGen::Encounter) => self.forge_encounter.as_ref()
                .map(|e| format!("{} ({})", e.spec, e.terrain_name()))
                .unwrap_or_default(),
            Some(ForgeGen::Npc) => self.forge_npc.as_ref()
                .map(|n| n.name.clone())
                .unwrap_or_default(),
            Some(ForgeGen::Town) => self.forge_town.as_ref()
                .map(|t| format!("{} ({})", t.name, t.size_class))
                .unwrap_or_default(),
            Some(ForgeGen::WeatherToday) | Some(ForgeGen::WeatherMonth) =>
                self.forge_weather.as_ref()
                    .and_then(|days| days.first())
                    .map(|d| format!("Weather — {}", d.date.fmt_long()))
                    .unwrap_or_default(),
            _ => String::new(),
        };
        if suggested.is_empty() {
            self.status_msg(
                "Nothing to save — generate something first.",
                t::WARN,
            );
            return;
        }
        let name = self.footer.ask(" Save as (Enter for suggested): ", &suggested);
        let name = name.trim().to_string();
        let name = if name.is_empty() { suggested } else { name };

        // Pull AI flavour out of the right pane if it's there. It
        // lives under a "  AI flavour" line we wrote in
        // `ai_enrich_*`, so everything from that line to the next
        // empty-prefix block is the prose to keep.
        let flavour = extract_ai_flavour(&self.forge_output);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let saved_msg: String;
        if let Some(camp) = self.campaign.as_mut() {
            match gen {
                Some(ForgeGen::Encounter) => {
                    let Some(enc) = self.forge_encounter.clone() else {
                        self.status_msg("No encounter to save.", t::WARN);
                        return;
                    };
                    let id = next_id(&camp.saved_encounters);
                    camp.saved_encounters.push(crate::store::Saved {
                        id, name: name.clone(), created_at: now,
                        flavour, item: enc,
                    });
                    saved_msg = format!("Saved encounter “{}”.", name);
                }
                Some(ForgeGen::Npc) => {
                    let Some(npc) = self.forge_npc.clone() else {
                        self.status_msg("No NPC to save.", t::WARN);
                        return;
                    };
                    let id = next_id(&camp.saved_npcs);
                    camp.saved_npcs.push(crate::store::Saved {
                        id, name: name.clone(), created_at: now,
                        flavour, item: npc,
                    });
                    saved_msg = format!("Saved NPC “{}”.", name);
                }
                Some(ForgeGen::Town) => {
                    let Some(town) = self.forge_town.clone() else {
                        self.status_msg("No town to save.", t::WARN);
                        return;
                    };
                    let id = next_id(&camp.saved_towns);
                    camp.saved_towns.push(crate::store::Saved {
                        id, name: name.clone(), created_at: now,
                        flavour, item: town,
                    });
                    saved_msg = format!("Saved town “{}”.", name);
                }
                Some(ForgeGen::WeatherToday) | Some(ForgeGen::WeatherMonth) => {
                    let Some(days) = self.forge_weather.clone() else {
                        self.status_msg("No weather to save.", t::WARN);
                        return;
                    };
                    // Save the first day of the batch — the AI
                    // flavour was generated for that day anyway.
                    let Some(day) = days.into_iter().next() else {
                        self.status_msg("Empty weather batch.", t::WARN);
                        return;
                    };
                    let id = next_id(&camp.saved_weather);
                    camp.saved_weather.push(crate::store::Saved {
                        id, name: name.clone(), created_at: now,
                        flavour, item: day,
                    });
                    saved_msg = format!("Saved weather “{}”.", name);
                }
                _ => {
                    self.status_msg("Save isn't wired for this generator yet.", t::WARN);
                    return;
                }
            }
            if let Err(e) = camp.save() {
                self.status_msg(&format!("Save failed: {}", e), t::ERR);
                return;
            }
        } else {
            return;
        }
        self.status_msg(&saved_msg, t::OK);
    }

    // --- Campaign lifecycle ---

    fn campaign_create(&mut self) {
        let name = self.footer.ask(" New campaign name: ", "");
        let name = name.trim().to_string();
        if name.is_empty() {
            self.status = Some(("Cancelled.".into(), t::WARN));
            return;
        }
        let c = Campaign::new(&name);
        if let Err(e) = c.save() {
            self.status = Some((format!("Save failed: {}", e), t::ERR));
            return;
        }
        self.config.active_campaign = Some(name.clone());
        let _ = self.config.save();
        self.campaign = Some(c);
        self.status = Some((format!("Created campaign '{}'.", name), t::OK));
    }

    fn campaign_load(&mut self) {
        let existing = list_campaigns();
        if existing.is_empty() {
            self.status = Some(("No campaigns yet — press C to create one.".into(), t::WARN));
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
                self.status = Some((format!("Loaded '{}'.", name), t::OK));
            }
            Err(e) => {
                self.status = Some((format!("Load failed: {}", e), t::ERR));
            }
        }
    }

    /// Permanently remove a campaign — deletes the entire
    /// `~/.amar/campaigns/<name>/` directory (campaign.json + any
    /// generated assets). Two confirmation steps because this is
    /// destructive and irreversible:
    ///   1. Pick which campaign from the existing list.
    ///   2. Type "DELETE" to confirm — guards against fat-fingered y.
    /// If the deleted campaign was the active one, clears
    /// `self.campaign` and `config.active_campaign`.
    fn campaign_delete(&mut self) {
        let existing = list_campaigns();
        if existing.is_empty() {
            self.status = Some(("No campaigns to delete.".into(), t::WARN));
            return;
        }
        let initial = existing[0].clone();
        let name = self.footer.ask(
            &format!(" Delete which campaign? ({}): ", existing.join(", ")),
            &initial,
        );
        let name = name.trim().to_string();
        if name.is_empty() { return; }
        if !existing.iter().any(|n| n == &name) {
            self.status = Some((format!("No such campaign: '{}'.", name), t::WARN));
            return;
        }
        let confirm = self.footer.ask(
            &format!(" Type DELETE to permanently remove '{}': ", name), "");
        if confirm.trim() != "DELETE" {
            self.status = Some(("Delete cancelled.".into(), t::WARN));
            return;
        }
        let dir = crate::store::campaign_dir(&name);
        match std::fs::remove_dir_all(&dir) {
            Ok(_) => {
                if self.config.active_campaign.as_deref() == Some(name.as_str()) {
                    self.campaign = None;
                    self.config.active_campaign = None;
                    let _ = self.config.save();
                }
                self.status = Some((format!("Deleted campaign '{}'.", name), t::OK));
            }
            Err(e) => {
                self.status = Some((format!("Delete failed: {}", e), t::ERR));
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
              TAB            Toggle focus between left + right pane\n  \
              ESC            Drop focus back to left pane\n  \
              w / W          Cycle left-pane width (kastrup-style 1-6)\n  \
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
              L       Load an existing campaign\n  \
              X       Delete a campaign (asks twice — type DELETE to confirm)\n  \
              D       Delete the PC or saved-forge entry under the cursor\n  \
              +       Promote NPC → roster (default: NPC list; press p\n  \
                      at the second prompt to send to the PC list instead).\n  \
                      Works on a saved encounter, a saved NPC, or a\n  \
                      freshly-rolled Forge encounter on the right pane.\n  \
              I       Import an adventure directory into the campaign\n  \
                      (markdown + Scenes/ + Floorplans/ + NPCs/ get indexed).\n  \
              N       On Adventures section header → scaffold a NEW adventure\n  \
                      (creates root dir, Scenes/Floorplans/NPCs subdirs,\n  \
                      skeleton .md). On a section row → append a session note.\n  \
              a       Mark cursor adventure as ACTIVE (persists between sessions).\n  \
              R       Re-scan an adventure's on-disk root to pick up\n  \
                      newly-added scenes, NPC images, or .md edits.\n  \
              V       Push the cursor's image to the player display via feh\n  \
                      (window class amar-player so your WM rules can place it).\n  \
              G       Generate a scene image for the cursor section\n  \
                      (clipboard → ChatGPT, or direct API). Saves to\n  \
                      <adv-root>/Scenes/ and auto-attaches on rescan.\n  \
              E       End the current session — writes a banner to\n  \
                      ~/.amar/campaigns/<camp>/session.log, advances the\n  \
                      current_section pointer to the next section.\n  \
                      ENTER on a section row sets it as the current\n  \
                      section so the GM can resume next session there.\n\n  \
            SESSION TAB (Combat HUD)\n  \
              j/k     select combatant\n  \
              +/-     damage/heal current HP\n  \
              M/m     MF up/down (mental fortitude — spell cost, willpower hits)\n  \
              A       add ALL PCs to the fight (resets HP/MF if zero'd)\n  \
              a       add one PC or NPC by name substring\n  \
              d       remove selected combatant\n  \
              c       clear the HUD\n  \
              o/O     private skill/combat O6 rolls (status line only)\n\n  \
            OTHER\n  \
              o       Roll a SKILL O6 (status line; crit/fumble → table)\n  \
              O       Roll a COMBAT O6 (status line; crit/fumble → table)\n  \
              r       Redraw\n  \
              ESC     Clear status line\n  \
              q / Q   Quit (saves campaign + config)\n\n  \
            Data: ~/.amar/campaigns/<name>/\n  \
            Canon: bundled, scraped from d6gaming.org\n  \
            ESC closes this popup.", VERSION);
        let (cols, rows) = Crust::terminal_size();
        let w = cols.saturating_sub(8).min(76);
        let h = rows.saturating_sub(4).min(28);
        let mut popup = crust::Popup::centered(w, h, t::FG as u16, 234);
        let _ = popup.modal(&help);
        Crust::clear_screen();
        self.render_all();
    }
}

/// One characteristic's vertical column for the 3-tier section. Holds
/// the rendered lines (attribute headers + skills under each) and the
/// editable-field map for those lines, all relative to line 0 of the
/// column. The caller offsets these into the merged-row index.
struct CharColumn {
    lines: Vec<String>,
    edits: Vec<EditableField>,
}

fn render_char_column(
    pc: &crate::pc::Character,
    ch: crate::pc::Char,
    attributes: &[(crate::pc::Char, &'static str)],
    skills:     &[(&'static str, &'static [&'static str])],
    active_id: Option<&str>,
) -> CharColumn {
    use crust::style;
    const LBL: u8 = 245;
    let mut col = CharColumn { lines: Vec::new(), edits: Vec::new() };

    // Per-characteristic 3-shade colour gradient: header darkest,
    // attributes a lighter shade of the same hue, skills lightest.
    // Lets the eye skim the visual hierarchy without reading.
    use crate::pc::Char as Ch;
    let (head_color, attr_color, skill_color): (u8, u8, u8) = match ch {
        Ch::Body   => (124, 167, 217),  // dark red   → indian red → light pink
        Ch::Mind   => (24,  67,  117),  // dark blue  → sky blue   → light sky
        Ch::Spirit => (90,  134, 183),  // dark purple→ med purple → thistle
    };
    // Rank values across the column render in dim gray (245); the
    // skill total stands out in bold bright (255). Same hue for
    // characteristic / attribute / skill rank cells so the eye can
    // skim the gray "input" column and see the "computed" totals.
    const VAL_DIM: u8 = 245;
    const TOTAL:   u8 = 255;
    let dim = |s: String, active: bool| -> String {
        if active { s } else { style::fg(&s, VAL_DIM) }
    };

    let char_id = format!("char/{}", ch.name());
    let char_active = active_id == Some(char_id.as_str());
    col.edits.push(EditableField {
        line: col.lines.len(),
        field_id: char_id,
        label: format!(" {} rank", ch.name()),
        current: pc.ch(ch).to_string(),
    });
    col.lines.push(format!("  {} {}",
        style::bold(&style::fg(ch.name(), head_color)),
        dim(value_cell(&format!("({})", pc.ch(ch)), 4, char_active), char_active)));
    let _ = LBL;

    for (_, attr) in attributes.iter().filter(|(c, _)| *c == ch) {
        let attr: &str = attr;
        let av = pc.attr(attr);
        let attr_id = format!("attr/{}", attr);
        let attr_active = active_id == Some(attr_id.as_str());
        col.edits.push(EditableField {
            line: col.lines.len(),
            field_id: attr_id,
            label: format!(" {} rank", attr),
            current: av.to_string(),
        });
        // Attribute name in the lighter hue. Pad with pad_visible so
        // ANSI escapes don't throw off the rank column alignment.
        let attr_styled = style::fg(attr, attr_color);
        col.lines.push(format!("   {} {}",
            pad_visible(&attr_styled, 19),
            dim(value_cell(&format!("{:>2}", av), 3, attr_active), attr_active)));

        // Skills — even lighter shade of the same hue. Custom skills
        // (anything in pc.skills not matching the canonical list) are
        // NOT shown inline anymore; they live in the open-slots
        // section below the column.
        let canonical: &[&str] = skills.iter()
            .find(|(a, _)| *a == attr)
            .map(|(_, s)| *s)
            .unwrap_or(&[]);
        for skill in canonical {
            let rank = pc.skill(attr, skill);
            let total = pc.skill_total(ch, attr, skill);
            let skill_id = format!("skill/{}/{}", attr, skill);
            let skill_active = active_id == Some(skill_id.as_str());
            col.edits.push(EditableField {
                line: col.lines.len(),
                field_id: skill_id,
                label: format!(" {} (rank)", skill),
                current: rank.to_string(),
            });
            let skill_styled = style::fg(skill, skill_color);
            // Rank in dim gray, total butted directly against it
            // (value_cell's trailing space + {:>3}'s leading spaces
            // give enough visual separation).
            col.lines.push(format!("     {} {}{}",
                pad_visible(&skill_styled, 19),
                dim(value_cell(&format!("{:>2}", rank), 3, skill_active), skill_active),
                style::bold(&style::fg(&format!("{:>3}", total), TOTAL))));
        }
    }

    // The generic open / free-form skill slots live in their own
    // section rendered alongside the SPIRIT column in the PC sheet
    // (see `render_pc_sheet`). Each char column here just shows the
    // canonical attribute / skill rows.
    col
}

/// Render the generic open-skill slot rows. Lives below the SPIRIT
/// column's canonical rows so the freed-up Attunement vertical space
/// is reused. Each slot row has 4 editable cells: char (B/M/S),
/// attribute, skill name, rank. Total = char + attr + rank, computed
/// live. Empty slots show three empty cells the user can fill in any
/// order. `n_slots` is the number of slots to render.
///
/// Slot row colors track the slot's `parent_char` so a BODY slot
/// renders in the BODY hue gradient (red), MIND in blue, SPIRIT in
/// purple. Attribute + skill name are truncated to fixed cell widths
/// so a long name (e.g. "Practical Knowledge") never pushes the rank
/// + total columns off-line.
fn render_open_slots(
    pc: &crate::pc::Character,
    n_slots: usize,
    line_offset: usize,
    active_id: Option<&str>,
) -> (Vec<String>, Vec<EditableField>) {
    use crust::style;
    const LBL: u8 = 245;
    const ATTR_W: usize = 12;
    const NAME_W: usize = 14;
    let mut lines: Vec<String> = Vec::new();
    let mut edits: Vec<EditableField> = Vec::new();
    // Empty row sets the slot section apart from the SPIRIT column's
    // canonical attribute/skill rows above.
    lines.push(String::new());
    lines.push(format!("  {}", style::fg("open skills (any char + attr)", LBL)));
    for i in 0..n_slots {
        let s = pc.open_skills.get(i).cloned().unwrap_or_default();
        let id_char = format!("slot/{}/char", i);
        let id_attr = format!("slot/{}/attribute", i);
        let id_name = format!("slot/{}/name", i);
        let id_rank = format!("slot/{}/rank", i);
        let active_char = active_id == Some(id_char.as_str());
        let active_attr = active_id == Some(id_attr.as_str());
        let active_name = active_id == Some(id_name.as_str());
        let active_rank = active_id == Some(id_rank.as_str());
        let line = line_offset + lines.len();
        edits.push(EditableField {
            line, field_id: id_char,
            label: format!(" Slot {} char (BODY/MIND/SPIRIT)", i + 1),
            current: s.parent_char.clone(),
        });
        edits.push(EditableField {
            line, field_id: id_attr,
            label: format!(" Slot {} attribute", i + 1),
            current: s.attribute.clone(),
        });
        edits.push(EditableField {
            line, field_id: id_name,
            label: format!(" Slot {} skill name", i + 1),
            current: s.name.clone(),
        });
        edits.push(EditableField {
            line, field_id: id_rank,
            label: format!(" Slot {} rank", i + 1),
            current: s.rank.to_string(),
        });
        let parent_char = match s.parent_char.as_str() {
            "BODY"   => Some(crate::pc::Char::Body),
            "MIND"   => Some(crate::pc::Char::Mind),
            "SPIRIT" => Some(crate::pc::Char::Spirit),
            _ => None,
        };
        let total: i32 = parent_char
            .map(|c| pc.ch(c) + pc.attr(&s.attribute) + s.rank)
            .unwrap_or(0);
        // Per-char gradient: same hues as the BODY/MIND/SPIRIT columns
        // so a slot visually "belongs" to its parent characteristic.
        // The head_color is the darkest of the three shades — used
        // for the leading B/M/S letter; attr_color is a touch
        // lighter; skill_color is lightest. Falls back to gray for
        // unfilled slots.
        let (head_color, attr_color, skill_color): (u8, u8, u8) = match s.parent_char.as_str() {
            "BODY"   => (124, 167, 217),
            "MIND"   => (24,  67,  117),
            "SPIRIT" => (90,  134, 183),
            _ => (245, 245, t::FG_MUTED),
        };
        // Char column — single letter B/M/S keeps it tight.
        let ch_disp = match s.parent_char.as_str() {
            "BODY"   => "B",
            "MIND"   => "M",
            "SPIRIT" => "S",
            _ => "",
        };
        let ch_cell = if active_char {
            value_cell(ch_disp, 2, true)
        } else if ch_disp.is_empty() {
            value_cell("", 2, false)
        } else {
            pad_visible(&style::bold(&style::fg(ch_disp, head_color)), 2)
        };
        // Truncate-with-ellipsis so over-long attr / skill names
        // can't push the rank + total columns off alignment.
        let attr_disp = truncate_or_pad(&s.attribute, ATTR_W);
        let name_disp = truncate_or_pad(&s.name, NAME_W);
        let attr_cell = if active_attr {
            value_cell(&attr_disp, ATTR_W, true)
        } else if s.attribute.is_empty() {
            value_cell("", ATTR_W, false)
        } else {
            pad_visible(&style::fg(&attr_disp, attr_color), ATTR_W)
        };
        let name_cell = if active_name {
            value_cell(&name_disp, NAME_W, true)
        } else if s.name.is_empty() {
            value_cell("", NAME_W, false)
        } else {
            pad_visible(&style::fg(&name_disp, skill_color), NAME_W)
        };
        let filled = !s.parent_char.is_empty() || !s.attribute.is_empty() || !s.name.is_empty();
        let rank_str = if filled || active_rank { format!("{:>2}", s.rank) } else { "  ".into() };
        let rank_cell_raw = value_cell(&rank_str, 3, active_rank);
        let rank_cell = if active_rank { rank_cell_raw } else { style::fg(&rank_cell_raw, t::FG_MUTED) };
        let total_cell = if filled {
            style::bold(&style::fg(&format!("{:>3}", total), t::FG_BRIGHT))
        } else {
            "   ".to_string()
        };
        lines.push(format!("  {} {} {} {}{}",
            ch_cell, attr_cell, name_cell, rank_cell, total_cell));
    }
    (lines, edits)
}

/// Render one "Label: value" cell with optional bg-highlight on the
/// value (not the label). Used by Identity / Equipment / Body Points
/// rows where a single field sits per logical cell.
fn emit_cell(label_color: u8, label: &str, value: &str, active: bool) -> String {
    let v_disp = if value.is_empty() && !active {
        crust::style::fg("—", label_color).to_string()
    } else {
        value.to_string()
    };
    let v_part = if active {
        // Always highlight at least one cell — use a single space when
        // the value is empty so the cursor is visible.
        let inner = if value.is_empty() { " ".to_string() } else { v_disp };
        crust::style::bold(&crust::style::bg(&inner, 24))
    } else {
        v_disp
    };
    format!("  {} {}",
        crust::style::fg(&format!("{}:", label), label_color),
        v_part)
}

/// Bg-highlight a value when active, otherwise return the value as-is
/// padded to at least `min_w` chars. Used by inline cells (Hit
/// locations row, Characteristics row, attribute / skill rank cell
/// in the 3-tier section) where the label is rendered separately.
fn value_cell(value: &str, min_w: usize, active: bool) -> String {
    let padded = if value.is_empty() && active {
        " ".repeat(min_w)
    } else {
        let w = crust::display_width(value);
        if w >= min_w { value.to_string() }
        else { format!("{}{}", value, " ".repeat(min_w - w)) }
    };
    if active {
        crust::style::bold(&crust::style::bg(&padded, 24))
    } else if value.is_empty() {
        format!("{:<width$}", "—", width = min_w)
    } else {
        padded
    }
}

/// Render an "(+ add ...)" placeholder row that's navigable as an
/// EditableField. ENTER on this row dispatches to the right add
/// handler via `edit_focused_field`'s special-case for action ids
/// like "weapon_add_melee", "weapon_add_missile", "spell_add".
fn push_add_row(
    out: &mut Vec<String>,
    edits: &mut Vec<EditableField>,
    field_id: &str,
    label: &str,
    visible: &str,
    active_id: Option<&str>,
) {
    use crust::style;
    let active = active_id == Some(field_id);
    edits.push(EditableField {
        line: out.len(),
        field_id: field_id.into(),
        label: label.into(),
        current: String::new(),
    });
    let cell = if active {
        style::bold(&style::bg(visible, 24))
    } else {
        style::fg(visible, t::FG_DIM)
    };
    out.push(format!("  {}", cell));
}

/// Render one melee/missile weapon row with every field individually
/// editable. The row sits on a single output line; each field becomes
/// an EditableField pointing back to the same line so j/k navigation
/// walks across the row column-by-column. OFF/DEF totals are derived
/// from char + attr + weapon_skill_rank + mod and shown un-editable.
fn push_weapon_row(
    out: &mut Vec<String>,
    edits: &mut Vec<EditableField>,
    pc: &crate::pc::Character,
    idx: usize,
    w: &crate::pc::Weapon,
    active_id: Option<&str>,
    line: usize,
) {
    use crust::style;
    let id_name   = format!("weapon/{}/name", idx);
    let id_skill  = format!("weapon/{}/skill", idx);
    let id_two    = format!("weapon/{}/two_handed", idx);
    let id_init   = format!("weapon/{}/init", idx);
    let id_off    = format!("weapon/{}/off_mod", idx);
    let id_def    = format!("weapon/{}/def_mod", idx);
    let id_shots  = format!("weapon/{}/shots", idx);
    let id_range  = format!("weapon/{}/range", idx);
    let id_damage = format!("weapon/{}/damage", idx);
    let id_hp     = format!("weapon/{}/hp", idx);
    let melee = matches!(w.kind, crate::pc::WeaponKind::Melee);
    let attr = if melee { "Melee Combat" } else { "Missile Combat" };
    // Per-weapon skill rank (e.g. "Sword" rank under "Melee Combat").
    let weap_skill_rank = pc.skill(attr, &w.skill_name);
    let base = pc.ch(crate::pc::Char::Body) + pc.attr(attr) + weap_skill_rank;
    // Wiki combat rules (data/lore/combat.md):
    //   OFF total = weapon off + Wield-Weapon total + weapon.off_mod
    //   DEF total = weapon def + skill total + floor(Dodge / 5)
    //   Init      = weapon Init + Reaction Speed (skill total) + O6
    // O6 is rolled at the table — the sheet shows the static portion.
    let dodge_total = pc.skill_total(crate::pc::Char::Body, "Athletics", "Dodge");
    let dodge_bonus = dodge_total / 5;
    let react_total = pc.skill_total(crate::pc::Char::Mind, "Awareness", "Reaction Speed");
    let off_total = base + w.off_mod;
    let def_total = base + w.def_mod + dodge_bonus;
    let init_total = w.init + react_total;
    let h_str = if w.two_handed { "2H" } else { "1H" };

    // helper closures for each editable field
    let push_edit = |edits: &mut Vec<EditableField>, id: &str, label: &str, current: String| {
        edits.push(EditableField {
            line,
            field_id: id.into(),
            label: label.into(),
            current,
        });
    };
    let active = |id: &str| active_id == Some(id);

    push_edit(edits, &id_name,   " Weapon name",       w.name.clone());
    push_edit(edits, &id_skill,  " Weapon skill",      w.skill_name.clone());
    if melee {
        push_edit(edits, &id_two, " Two-handed (y/n)", h_str.into());
    }
    push_edit(edits, &id_init,   " Init",              w.init.to_string());
    push_edit(edits, &id_off,    " ±O (offence mod)",  w.off_mod.to_string());
    if melee {
        push_edit(edits, &id_def, " ±D (defence mod)", w.def_mod.to_string());
    } else {
        push_edit(edits, &id_shots, " Shots/round",    w.shots_per_round.to_string());
        push_edit(edits, &id_range, " Range (m)",      w.range_m.to_string());
    }
    push_edit(edits, &id_damage, " Damage",            w.damage.to_string());
    push_edit(edits, &id_hp,     " HP",                w.hp.to_string());

    // Init column: show the combat-ready total (weapon Init +
    // Reaction Speed) when displayed; show just w.init when active
    // for editing (since that's what's stored — the base weapon
    // bonus, not the live total).
    let init_cell = if active(&id_init) {
        value_cell(&format!("{:+}", w.init), 4, true)
    } else {
        // Bold-bright like other "derived total" columns (OFF/DEF).
        pad_visible(&style::bold(&style::fg(&format!("{:+}", init_total), t::FG_BRIGHT)), 4)
    };

    // Skill column: when not under the field editor, show the
    // weapon's skill NAME plus its live RANK (the number that's
    // already folded into OFF/DEF). Without the rank visible,
    // there's nowhere else on the row that exposes how high the
    // character's skill with this weapon actually is — the value
    // is what makes the row useful at the table. Editing the cell
    // still opens the skill-NAME prompt (the rank is governed by
    // the skill section above; this just looks it up).
    let skill_cell = if active(&id_skill) {
        value_cell(&w.skill_name, 10, true)
    } else {
        let rank = style::bold(&style::fg(&weap_skill_rank.to_string(), t::FG_BRIGHT));
        pad_visible(&format!("{} {}", w.skill_name, rank), 10)
    };

    // Render the row. Layout differs slightly between melee + missile:
    // melee gets ±D + DEF, missile gets s/r + Rng instead.
    if melee {
        out.push(format!("  {} {} {} {} {} {} {} {} {} {}",
            pad_visible(&value_cell(&w.name, 16, active(&id_name)), 16),
            skill_cell,
            pad_visible(&value_cell(h_str, 2, active(&id_two)), 2),
            init_cell,
            pad_visible(&value_cell(&format!("{:+}", w.off_mod), 4, active(&id_off)), 4),
            pad_visible(&value_cell(&format!("{:+}", w.def_mod), 4, active(&id_def)), 4),
            pad_visible(&style::bold(&style::fg(&format!("{:>4}", off_total), t::FG_BRIGHT)), 4),
            pad_visible(&style::bold(&style::fg(&format!("{:>4}", def_total), t::FG_BRIGHT)), 4),
            pad_visible(&value_cell(&format!("{:+}", w.damage), 4, active(&id_damage)), 4),
            pad_visible(&value_cell(&w.hp.to_string(), 3, active(&id_hp)), 3)));
    } else {
        out.push(format!("  {} {} {} {} {} {} {} {} {}",
            pad_visible(&value_cell(&w.name, 16, active(&id_name)), 16),
            skill_cell,
            init_cell,
            pad_visible(&value_cell(&format!("{:+}", w.off_mod), 4, active(&id_off)), 4),
            pad_visible(&value_cell(&w.shots_per_round.to_string(), 4, active(&id_shots)), 4),
            pad_visible(&style::bold(&style::fg(&format!("{:>4}", off_total), t::FG_BRIGHT)), 4),
            pad_visible(&value_cell(&w.range_m.to_string(), 5, active(&id_range)), 5),
            pad_visible(&value_cell(&format!("{:+}", w.damage), 4, active(&id_damage)), 4),
            pad_visible(&value_cell(&w.hp.to_string(), 3, active(&id_hp)), 3)));
    }
}

/// Render one spell row with every field individually editable. Same
/// pattern as `push_weapon_row`: a single line, one EditableField per
/// column, all pointing at the same line.
fn push_spell_row(
    out: &mut Vec<String>,
    edits: &mut Vec<EditableField>,
    idx: usize,
    sp: &crate::pc::Spell,
    active_id: Option<&str>,
    line: usize,
) {
    let id_name    = format!("spell/{}/name", idx);
    let id_domain  = format!("spell/{}/domain", idx);
    let id_ap      = format!("spell/{}/active_passive", idx);
    let id_dr      = format!("spell/{}/dr", idx);
    let id_cost    = format!("spell/{}/cost", idx);
    let id_cast    = format!("spell/{}/casting_time", idx);
    let id_dist    = format!("spell/{}/distance", idx);
    let id_dur     = format!("spell/{}/duration", idx);
    let id_area    = format!("spell/{}/area", idx);
    let id_cd      = format!("spell/{}/cooldown", idx);
    let id_effects = format!("spell/{}/effects", idx);

    let push_edit = |edits: &mut Vec<EditableField>, id: &str, label: &str, current: String| {
        edits.push(EditableField {
            line,
            field_id: id.into(),
            label: label.into(),
            current,
        });
    };
    let active = |id: &str| active_id == Some(id);

    push_edit(edits, &id_name,    " Spell name",          sp.name.clone());
    push_edit(edits, &id_domain,  " Domain",              sp.domain.clone());
    push_edit(edits, &id_ap,      " Active/Passive",      sp.active_passive.clone());
    push_edit(edits, &id_dr,      " DR",                  sp.dr.to_string());
    push_edit(edits, &id_cost,    " Cost (MD)",           sp.cost.to_string());
    push_edit(edits, &id_cast,    " Casting time",        sp.casting_time.clone());
    push_edit(edits, &id_dist,    " Distance",            sp.distance.clone());
    push_edit(edits, &id_dur,     " Duration",            sp.duration.clone());
    push_edit(edits, &id_area,    " Area of effect",      sp.area.clone());
    push_edit(edits, &id_cd,      " Cooldown",            sp.cooldown.clone());
    push_edit(edits, &id_effects, " Effects",             sp.effects.clone());

    // Active/Passive shows just the first letter (A/P) so the cell
    // stays narrow without losing the meaning.
    let ap_short: String = sp.active_passive.chars().next()
        .map(|c| c.to_ascii_uppercase().to_string())
        .unwrap_or_default();
    // Every cell is hard-clipped to its column width before the bg
    // highlight wraps it — without the clip a long value (e.g. a 16-
    // char distance string in the 10-cell Dist column) spills into
    // the next column and the rest of the table drifts. `value_cell`
    // pads up to width but doesn't trim; `truncate_or_pad` handles
    // the trim with an ellipsis. `pad_visible` is then a no-op for
    // these cells but stays in place so the formatting stays uniform
    // alongside the trailing Effects column.
    let effects_short: String = sp.effects.chars().take(40).collect();
    out.push(format!("  {} {} {} {} {} {} {} {} {} {} {}",
        pad_visible(&value_cell(&truncate_or_pad(&sp.name, 16),   16, active(&id_name)),   16),
        pad_visible(&value_cell(&truncate_or_pad(&sp.domain, 8),   8, active(&id_domain)),  8),
        pad_visible(&value_cell(&ap_short,                         3, active(&id_ap)),     3),
        pad_visible(&value_cell(&truncate_or_pad(&sp.dr.to_string(),   3), 3, active(&id_dr)),   3),
        pad_visible(&value_cell(&truncate_or_pad(&sp.cost.to_string(), 4), 4, active(&id_cost)), 4),
        pad_visible(&value_cell(&truncate_or_pad(&sp.casting_time, 8), 8, active(&id_cast)), 8),
        pad_visible(&value_cell(&truncate_or_pad(&sp.distance,    10), 10, active(&id_dist)), 10),
        pad_visible(&value_cell(&truncate_or_pad(&sp.duration,    10), 10, active(&id_dur)),  10),
        pad_visible(&value_cell(&truncate_or_pad(&sp.area,        10), 10, active(&id_area)), 10),
        pad_visible(&value_cell(&truncate_or_pad(&sp.cooldown,    10), 10, active(&id_cd)),   10),
        value_cell(&effects_short, effects_short.chars().count().max(1), active(&id_effects))));
}

/// Build one row of the portrait area. Two modes:
///
///  - **Image mode** (path is Some + file exists): the first row
///    emits a kitty graphics escape that paints the image into the
///    `w × total` cell rectangle starting at the cursor. Remaining
///    rows return empty so nothing else lands on top of the image.
///    Works in wezterm + kitty + ghostty.
///
///  - **Placeholder mode** (no path): a dim Unicode-box frame with
///    "(no portrait)" / "press P to add" labels.
fn portrait_row(row: usize, w: usize, total: usize, image_path: Option<&str>) -> String {
    use crust::style;
    if w < 8 || total < 4 { return String::new(); }
    if let Some(path) = image_path {
        if !path.is_empty() && std::path::Path::new(path).exists() {
            if row == 0 {
                // Kitty graphics: a=T transmit-and-display,
                // f=100 PNG (kitty also auto-detects JPEG/etc),
                // t=f payload is a base64-encoded filename,
                // c/r size the image to this many terminal cells.
                let b64 = crate::portrait::base64_encode(path.as_bytes());
                return format!(" \x1b_Ga=T,f=100,t=f,c={},r={};{}\x1b\\",
                    w, total, b64);
            }
            return String::new();
        }
    }
    // Placeholder mode — original dim frame.
    let dim: u8 = 240;
    let inner = w.saturating_sub(2);
    let frame = if row == 0 {
        format!("┌{}┐", "─".repeat(inner))
    } else if row == total - 1 {
        format!("└{}┘", "─".repeat(inner))
    } else if row == total / 3 {
        format!("│{:^iw$}│", "(no portrait)", iw = inner)
    } else if row == 2 * total / 3 {
        format!("│{:^iw$}│", "press P to add", iw = inner)
    } else {
        format!("│{}│", " ".repeat(inner))
    };
    format!(" {}", style::fg(&frame, dim))
}

/// Pad a string with trailing spaces to reach the given visible width.
/// `crust::display_width` is ANSI-aware so embedded escape sequences
/// don't throw off the alignment.
/// Minimal inline-markdown styling for adventure narrative lines.
/// Spans handled:
///   * `**bold**`   → ANSI bold
///   * `*italic*`   → ANSI italic
///   * `` `code` `` → dim/grey accent
/// Anything that doesn't pair properly is left literal. Greedy left-
/// to-right scan; nested `**bold *and italic*** ` won't unwind but
/// nothing in published adventures does that anyway.
fn inline_md(line: &str) -> String {
    use crust::style;
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < bytes.len() {
        // Delimiter probes work in bytes — safe because `*` and `` ` ``
        // are ASCII so they only ever land on a UTF-8 char boundary.
        if i + 1 < bytes.len() && &bytes[i..i + 2] == b"**" {
            if let Some(end) = find_close(bytes, i + 2, b"**") {
                let inner = &line[i + 2..end];
                out.push_str(&style::bold(inner));
                i = end + 2;
                continue;
            }
        }
        if bytes[i] == b'*'
            && i + 1 < bytes.len()
            && !bytes[i + 1].is_ascii_whitespace()
        {
            if let Some(end) = find_close(bytes, i + 1, b"*") {
                let inner = &line[i + 1..end];
                out.push_str(&style::italic(inner));
                i = end + 1;
                continue;
            }
        }
        if bytes[i] == b'`' {
            if let Some(end) = find_close(bytes, i + 1, b"`") {
                let inner = &line[i + 1..end];
                out.push_str(&style::fg(inner, t::AMBER));
                i = end + 1;
                continue;
            }
        }
        // Passthrough — walk to the next UTF-8 char boundary and
        // copy that one codepoint verbatim. Avoids the
        // `bytes[i] as char` Latin-1 trap for Nordic chars etc.
        let next = next_char_boundary(line, i);
        out.push_str(&line[i..next]);
        i = next;
    }
    out
}

/// Find the next `char`-boundary byte index at or after `from`.
/// Returns `line.len()` if `from` is already at the end. Uses
/// std's `is_char_boundary` so combining marks etc. stay grouped.
fn next_char_boundary(line: &str, from: usize) -> usize {
    let mut j = from + 1;
    while j < line.len() && !line.is_char_boundary(j) { j += 1; }
    j.min(line.len())
}

/// Helper for `inline_md`: scan `bytes` starting at `from` for the
/// next occurrence of `needle`. Returns the byte index of the start
/// of the match. None if the delimiter never closes on this line.
fn find_close(bytes: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    let mut i = from;
    while i + needle.len() <= bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn pad_visible(s: &str, width: usize) -> String {
    let w = crust::display_width(s);
    if w >= width { s.to_string() }
    else { format!("{}{}", s, " ".repeat(width - w)) }
}

/// Translate the kastrup-style 1-6 width slider into actual left /
/// right column counts. Mirrors kastrup's formula:
/// `left = (cols - 4) × width / 10`, then clamp so both sides keep
/// at least ~20 cols of breathing room.
fn compute_left_right(cols: u16, width: u8) -> (u16, u16) {
    let raw = (cols.saturating_sub(4) as u32 * width as u32 / 10) as u16;
    let left = raw.max(20).min(cols.saturating_sub(20));
    let right = cols.saturating_sub(left);
    (left, right)
}

/// Build the Campaign tree (sections + their items) against the
/// expanded-set. Section order: PCs · Adventures · NPCs · Locations
/// · Calendar · Factions. The first four are expandable; the last
/// two are leaves whose detail renders directly when selected.
fn build_camp_tree(camp: &Campaign, expanded: &[String]) -> Vec<CampTreeItem> {
    let mut out: Vec<CampTreeItem> = Vec::new();
    for sec in CampSection::all() {
        let id = sec.id().to_string();
        let is_expandable = matches!(sec,
            CampSection::Pcs | CampSection::Adventures
            | CampSection::Npcs | CampSection::Locations
            | CampSection::SavedForge);
        let is_expanded = is_expandable && expanded.iter().any(|e| e == &id);
        out.push(CampTreeItem {
            node: CampNode::Section(sec),
            depth: 0,
            expandable: is_expandable,
            expanded: is_expanded,
        });
        if !is_expanded { continue; }
        match sec {
            CampSection::Pcs => {
                if camp.pcs.is_empty() {
                    out.push(CampTreeItem {
                        node: CampNode::Placeholder { section: sec,
                            msg: "(no PCs yet — press n to add)".into() },
                        depth: 1,
                        expandable: false, expanded: false,
                    });
                } else {
                    for i in 0..camp.pcs.len() {
                        out.push(CampTreeItem {
                            node: CampNode::Pc(i),
                            depth: 1,
                            expandable: false, expanded: false,
                        });
                    }
                }
            }
            CampSection::Adventures => {
                if camp.adventures.is_empty() {
                    out.push(CampTreeItem {
                        node: CampNode::Placeholder { section: sec,
                            msg: "(no adventures — press I to import a directory)".into() },
                        depth: 1,
                        expandable: false, expanded: false,
                    });
                } else {
                    // Each adventure is depth-1 and expandable; the
                    // expansion key is "adv:<id>" so it survives
                    // re-ordering.
                    for (i, adv) in camp.adventures.iter().enumerate() {
                        let adv_id_key = format!("adv:{}", adv.id);
                        let adv_expanded = expanded.iter().any(|e| e == &adv_id_key);
                        out.push(CampTreeItem {
                            node: CampNode::Adventure(i),
                            depth: 1,
                            expandable: true, expanded: adv_expanded,
                        });
                        if !adv_expanded { continue; }
                        // Adventure sub-groups: Sections / Scenes /
                        // Floorplans / NPC Portraits / NPC Docs.
                        // Each group is itself expandable so the tree
                        // stays scannable when an adventure has 30+
                        // assets in one bucket.
                        for (kind, count, label) in [
                            (AdventureGroupKind::Sections,     adv.sections.len(),       "Sections"),
                            (AdventureGroupKind::Scenes,       adv.scenes.len(),         "Scenes"),
                            (AdventureGroupKind::Floorplans,   adv.floorplans.len(),     "Floorplans"),
                            (AdventureGroupKind::NpcPortraits, adv.npc_portraits.len(),  "NPC portraits"),
                            (AdventureGroupKind::NpcDocs,      adv.npc_docs.len(),       "NPC docs"),
                        ] {
                            let _ = label;
                            if count == 0 { continue; }
                            let grp_key = format!("adv:{}:{:?}", adv.id, kind);
                            let grp_expanded = expanded.iter().any(|e| e == &grp_key);
                            out.push(CampTreeItem {
                                node: CampNode::AdventureGroup(i, kind),
                                depth: 2,
                                expandable: true, expanded: grp_expanded,
                            });
                            if !grp_expanded { continue; }
                            // Leaves.
                            match kind {
                                AdventureGroupKind::Sections => {
                                    // Nest sub-sections by markdown
                                    // heading level: ## sections sit
                                    // at depth 3, ### at depth 4,
                                    // #### at depth 5, etc. A parent
                                    // section can collapse to hide
                                    // its children — same key shape
                                    // as everything else:
                                    // "advsec:<adv_id>:<line_start>".
                                    // Visible-stack tracks which
                                    // levels are currently open so
                                    // we skip emitting deeper rows
                                    // under a collapsed parent.
                                    //
                                    // The cursor-on-a-closed-row
                                    // gracefully stops it growing.
                                    let mut visible_stack: Vec<(u8, bool)> = Vec::new();
                                    for j in 0..adv.sections.len() {
                                        let sec = &adv.sections[j];
                                        let level = sec.level.max(2);
                                        // Pop stack down to anything
                                        // higher than current level.
                                        while visible_stack.last()
                                            .map(|(l, _)| *l >= level).unwrap_or(false)
                                        {
                                            visible_stack.pop();
                                        }
                                        let parent_visible = visible_stack.iter().all(|(_, e)| *e);
                                        if !parent_visible {
                                            // Still push self onto
                                            // stack with expanded=false
                                            // so deeper levels stay
                                            // hidden too.
                                            visible_stack.push((level, false));
                                            continue;
                                        }
                                        let has_children = section_has_children(adv, j);
                                        let key = format!("advsec:{}:{}",
                                            adv.id, sec.line_start);
                                        let expanded = expanded.iter().any(|e| e == &key);
                                        // Map markdown level to tree
                                        // depth. Anchor ## (level 2)
                                        // at depth 3 (under the
                                        // Sections group at depth 2).
                                        let depth = (level - 2 + 3) as u8;
                                        out.push(CampTreeItem {
                                            node: CampNode::AdventureSection(i, j),
                                            depth,
                                            expandable: has_children,
                                            expanded,
                                        });
                                        visible_stack.push((level, expanded));
                                    }
                                }
                                AdventureGroupKind::Scenes => {
                                    for j in 0..adv.scenes.len() {
                                        out.push(CampTreeItem {
                                            node: CampNode::AdventureAsset(i, AdventureAssetKind::Scene, j),
                                            depth: 3,
                                            expandable: false, expanded: false,
                                        });
                                    }
                                }
                                AdventureGroupKind::Floorplans => {
                                    for j in 0..adv.floorplans.len() {
                                        out.push(CampTreeItem {
                                            node: CampNode::AdventureAsset(i, AdventureAssetKind::Floorplan, j),
                                            depth: 3,
                                            expandable: false, expanded: false,
                                        });
                                    }
                                }
                                AdventureGroupKind::NpcPortraits => {
                                    for j in 0..adv.npc_portraits.len() {
                                        out.push(CampTreeItem {
                                            node: CampNode::AdventureAsset(i, AdventureAssetKind::NpcPortrait, j),
                                            depth: 3,
                                            expandable: false, expanded: false,
                                        });
                                    }
                                }
                                AdventureGroupKind::NpcDocs => {
                                    for j in 0..adv.npc_docs.len() {
                                        out.push(CampTreeItem {
                                            node: CampNode::AdventureAsset(i, AdventureAssetKind::NpcDoc, j),
                                            depth: 3,
                                            expandable: false, expanded: false,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            CampSection::Npcs => {
                if camp.npcs.is_empty() {
                    out.push(CampTreeItem {
                        node: CampNode::Placeholder { section: sec,
                            msg: "(persistent NPC roster — v0.4.0)".into() },
                        depth: 1,
                        expandable: false, expanded: false,
                    });
                } else {
                    for i in 0..camp.npcs.len() {
                        out.push(CampTreeItem {
                            node: CampNode::Npc(i),
                            depth: 1,
                            expandable: false, expanded: false,
                        });
                    }
                }
            }
            CampSection::Locations => {
                out.push(CampTreeItem {
                    node: CampNode::Placeholder { section: sec,
                        msg: "(locations land in v0.4.0)".into() },
                    depth: 1,
                    expandable: false, expanded: false,
                });
            }
            CampSection::SavedForge => {
                let n_enc  = camp.saved_encounters.len();
                let n_town = camp.saved_towns.len();
                let n_wx   = camp.saved_weather.len();
                let n_npc  = camp.saved_npcs.len();
                if n_enc + n_town + n_wx + n_npc == 0 {
                    out.push(CampTreeItem {
                        node: CampNode::Placeholder { section: sec,
                            msg: "(empty — press S on a Forge result to save)".into() },
                        depth: 1,
                        expandable: false, expanded: false,
                    });
                } else {
                    // Order: encounters → npcs → towns → weather.
                    // Lets the GM scan combat threats first, then
                    // people, then places, then mood.
                    for i in 0..n_enc {
                        out.push(CampTreeItem {
                            node: CampNode::SavedForge(SavedKind::Encounter, i),
                            depth: 1,
                            expandable: false, expanded: false,
                        });
                    }
                    for i in 0..n_npc {
                        out.push(CampTreeItem {
                            node: CampNode::SavedForge(SavedKind::Npc, i),
                            depth: 1,
                            expandable: false, expanded: false,
                        });
                    }
                    for i in 0..n_town {
                        out.push(CampTreeItem {
                            node: CampNode::SavedForge(SavedKind::Town, i),
                            depth: 1,
                            expandable: false, expanded: false,
                        });
                    }
                    for i in 0..n_wx {
                        out.push(CampTreeItem {
                            node: CampNode::SavedForge(SavedKind::Weather, i),
                            depth: 1,
                            expandable: false, expanded: false,
                        });
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Title shown in the left pane for one tree node.
fn camp_node_title(camp: &Campaign, node: &CampNode) -> String {
    match node {
        CampNode::Section(sec) => match sec {
            CampSection::Pcs        => format!("PCs ({})", camp.pcs.len()),
            CampSection::Adventures => format!("Adventures ({})", camp.adventures.len()),
            CampSection::Npcs       => format!("NPCs ({})", camp.npcs.len()),
            CampSection::Locations  => "Locations (0)".to_string(),
            CampSection::Calendar   => "Calendar".to_string(),
            CampSection::Factions   => "Factions".to_string(),
            CampSection::SavedForge => {
                let n = camp.saved_encounters.len()
                    + camp.saved_npcs.len()
                    + camp.saved_towns.len()
                    + camp.saved_weather.len();
                format!("Forge log ({})", n)
            }
        },
        CampNode::Pc(idx) => {
            camp.pcs.get(*idx)
                .map(|p| format!("{}  L{}", p.name, p.level))
                .unwrap_or_else(|| "(missing PC)".to_string())
        }
        CampNode::Adventure(idx) => {
            camp.adventures.get(*idx).map(|a| {
                let active = camp.active_adventure_id == Some(a.id);
                let marker = if active { "\u{25CF} " } else { "" };
                format!("{}{}", marker, a.name)
            }).unwrap_or_else(|| "(missing adventure)".to_string())
        }
        CampNode::AdventureGroup(adv_idx, kind) => {
            let count = camp.adventures.get(*adv_idx).map(|a| match kind {
                AdventureGroupKind::Sections     => a.sections.len(),
                AdventureGroupKind::Scenes       => a.scenes.len(),
                AdventureGroupKind::Floorplans   => a.floorplans.len(),
                AdventureGroupKind::NpcPortraits => a.npc_portraits.len(),
                AdventureGroupKind::NpcDocs      => a.npc_docs.len(),
            }).unwrap_or(0);
            let label = match kind {
                AdventureGroupKind::Sections     => "Sections",
                AdventureGroupKind::Scenes       => "Scenes",
                AdventureGroupKind::Floorplans   => "Floorplans",
                AdventureGroupKind::NpcPortraits => "NPC portraits",
                AdventureGroupKind::NpcDocs      => "NPC docs",
            };
            format!("{} ({})", label, count)
        }
        CampNode::AdventureSection(adv_idx, sec_idx) => {
            // Tree depth is already set by the builder per markdown
            // level — we don't need any extra in-string indent here.
            // Just the current-section marker + heading.
            camp.adventures.get(*adv_idx)
                .and_then(|a| {
                    let s = a.sections.get(*sec_idx)?;
                    let marker = if a.current_section == Some(*sec_idx) {
                        "\u{2192} "  // → arrow on the current section
                    } else { "" };
                    Some(format!("{}{}", marker, s.heading))
                })
                .unwrap_or_else(|| "(missing section)".to_string())
        }
        CampNode::AdventureAsset(adv_idx, kind, asset_idx) => {
            let glyph = match kind {
                AdventureAssetKind::Scene       => "\u{1F3DE}",  // 🏞
                AdventureAssetKind::Floorplan   => "\u{1F5FA}",  // 🗺
                AdventureAssetKind::NpcPortrait => "\u{1F464}",  // 👤
                AdventureAssetKind::NpcDoc      => "\u{1F4DC}",  // 📜
            };
            camp.adventures.get(*adv_idx)
                .and_then(|a| {
                    let asset = match kind {
                        AdventureAssetKind::Scene       => a.scenes.get(*asset_idx),
                        AdventureAssetKind::Floorplan   => a.floorplans.get(*asset_idx),
                        AdventureAssetKind::NpcPortrait => a.npc_portraits.get(*asset_idx),
                        AdventureAssetKind::NpcDoc      => a.npc_docs.get(*asset_idx),
                    }?;
                    Some(format!("{} {}", glyph, asset.name))
                })
                .unwrap_or_else(|| "(missing asset)".to_string())
        }
        CampNode::Npc(idx) => {
            camp.npcs.get(*idx).map(|n| n.name.clone())
                .unwrap_or_else(|| "(missing NPC)".to_string())
        }
        CampNode::Location(idx) => format!("Location #{}", idx + 1),
        CampNode::SavedForge(kind, idx) => {
            // Leaf row in the Forge-log section. Glyph indicates
            // type at a glance: ⚔ encounter, ☻ NPC, ⌂ town, ☀ weather.
            let (glyph, name) = match kind {
                SavedKind::Encounter => ("\u{2694}",
                    camp.saved_encounters.get(*idx).map(|s| s.name.as_str())),
                SavedKind::Npc => ("\u{263B}",
                    camp.saved_npcs.get(*idx).map(|s| s.name.as_str())),
                SavedKind::Town => ("\u{2302}",
                    camp.saved_towns.get(*idx).map(|s| s.name.as_str())),
                SavedKind::Weather => ("\u{2600}",
                    camp.saved_weather.get(*idx).map(|s| s.name.as_str())),
            };
            match name {
                Some(n) => format!("{}  {}", glyph, n),
                None    => "(missing saved item)".to_string(),
            }
        }
        CampNode::Placeholder { msg, .. } => msg.clone(),
    }
}

/// Render one "Label: value" line for the PC sheet, with the label
/// dim-grey-coloured (style fg 245) and the value in default fg.
fn field_row(lbl_color: u8, label: &str, value: &str) -> String {
    let v = if value.is_empty() { "—" } else { value };
    format!("  {:<14} {}",
        crust::style::fg(&format!("{}:", label), lbl_color),
        v)
}

/// Format SIZE as `3` for whole, `3.5` for half-step.
/// Encumbrance penalty for the character's worn armor, after the
/// Wield Weapon offset. `m_mod` from the ARMOUR table is the raw
/// movement-cost of the armor (Chain mail -4, Cuir-boullie -2, …);
/// Wield Weapon total (BODY characteristic + Strength attribute +
/// Strength/Wield Weapon skill) bleeds that off so a trained
/// fighter handles their kit. Capped at 0 — armor never gives a
/// positive Status bonus.
fn encumbrance_penalty(pc: &crate::pc::Character) -> i32 {
    // All hit locations carry the same armor name; pick any.
    let armor_name = pc.hit_locations.values()
        .find(|h| !h.armor.is_empty() && h.armor != "None")
        .map(|h| h.armor.as_str());
    let Some(name) = armor_name else { return 0; };
    let row = crate::forge::data::ARMOUR.iter().find(|r| r.name == name);
    let m_mod = row.map(|r| r.m_mod).unwrap_or(0);
    if m_mod >= 0 { return 0; }
    let ww_total = pc.skill_total(
        crate::pc::Char::Body, "Strength", "Wield Weapon");
    (m_mod + ww_total).min(0)
}

/// Does the given section index in `adv` have at least one
/// deeper-level child heading before the next sibling-or-shallower?
/// Used by the tree builder and the navigation helpers to decide
/// whether a section is expandable.
fn section_has_children(adv: &crate::adventure::Adventure, sec_idx: usize) -> bool {
    let Some(me) = adv.sections.get(sec_idx) else { return false };
    adv.sections[sec_idx + 1..]
        .iter()
        .take_while(|s| s.level > me.level)
        .any(|s| s.level > me.level)
}

/// Render a unix timestamp as `YYYY-MM-DD HH:MM` for session-log
/// + note display. Civil-from-days arithmetic — no chrono dep.
fn fmt_ts(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let secs_of_day = secs % 86_400;
    let hh = secs_of_day / 3_600;
    let mm = (secs_of_day % 3_600) / 60;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02} {:02}:{:02}", y, m, d, hh, mm)
}

fn fmt_size(size: f32) -> String {
    if (size - size.floor()).abs() < 0.05 {
        format!("{}", size as i32)
    } else {
        format!("{:.1}", size)
    }
}

/// Format an optional numeric value with a unit suffix, blank for 0.
fn fmt_opt_num(n: u32, unit: &str) -> String {
    if n == 0 { String::new() } else { format!("{} {}", n, unit) }
}

/// Truncate to char-count `n` (with an ellipsis when shortened) or
/// return the original. format!'s `{:<N}` uses byte width, so a
/// 14-char telescope name with no multibyte chars is fine — but the
/// ellipsis-on-truncate is still nice when a future entry is wider
/// than the column.
/// Minimal `~` expansion for user-typed paths. Doesn't pull in the
/// `shellexpand` crate just for one prompt. Replaces a leading `~`
/// or `~/` with `$HOME`.
fn shellexpand_simple(s: &str) -> String {
    if let Some(rest) = s.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_default();
        format!("{}/{}", home, rest)
    } else if s == "~" {
        std::env::var("HOME").unwrap_or_default()
    } else {
        s.to_string()
    }
}

/// Render an indexed item list as N-column rows that fit in the
/// pane. Used by the Forge tab's chartype / names / terrain pickers
/// so a long list (35+ chartypes) stays readable instead of running
/// off the top of the pane. Items wrap column-major: column 0 holds
/// the first ceil(N/cols) entries, column 1 the next batch, etc.
fn format_picker_columns(items: &[(usize, &str)], cols: usize, pane_w: usize) -> Vec<String> {
    if items.is_empty() { return Vec::new(); }
    let cols = cols.max(1);
    let rows = items.len().div_ceil(cols);
    let inner = pane_w.saturating_sub(2);
    let col_w = inner / cols;
    let mut lines: Vec<String> = vec![String::new(); rows];
    for (i, (idx, label)) in items.iter().enumerate() {
        let row = i % rows;
        let cell = format!("{}  {}",
            crust::style::fg(&format!("{:>2}", idx), t::FG_MUTED),
            crust::style::fg(label, t::FG));
        lines[row].push_str(&pad_visible(&cell, col_w));
    }
    // Add a 2-space leading indent so the columns sit inset from the
    // pane edge like other Forge content.
    lines.into_iter().map(|l| format!("  {}", l)).collect()
}

/// Compact summary of an NPC produced by `forge::npc::build_npc`.
/// Shows the same basic stat block the user expects from
/// Amar-Tools: name + identity, BP/DB/MD/SIZE, characteristics +
/// attributes, primary weapon and armor, top non-zero skills.
fn format_npc_summary(npc: &crate::pc::Character) -> Vec<String> {
    use crate::pc::{ATTRIBUTES, SKILLS, Char};
    use crust::style;
    let mut out: Vec<String> = Vec::new();
    let title = format!("{} ({} {}, age {}, lvl {}) — {}",
        npc.name, npc.gender, npc.race, npc.age, npc.level, "NPC");
    out.push(style::bold(&style::fg(&title, t::ACCENT)).to_string());
    out.push(format!("  H/W: {} cm / {} kg   SIZE: {}   BP: {}   DB: {}   MD: {}",
        if npc.height_cm == 0 { "—".into() } else { npc.height_cm.to_string() },
        npc.weight_kg, fmt_size(npc.size), npc.bp_max(), npc.db(), npc.md()));
    out.push(String::new());
    out.push(style::fg("Characteristics", t::TAN).to_string());
    out.push(format!("  BODY {:>2}    MIND {:>2}    SPIRIT {:>2}",
        npc.ch(Char::Body), npc.ch(Char::Mind), npc.ch(Char::Spirit)));
    out.push(String::new());

    // Attributes column per characteristic.
    out.push(style::fg("Attributes", t::TAN).to_string());
    for ch in [Char::Body, Char::Mind, Char::Spirit] {
        let ch_label = match ch {
            Char::Body   => style::fg("BODY",   124),
            Char::Mind   => style::fg("MIND",   24),
            Char::Spirit => style::fg("SPIRIT", 90),
        };
        let attrs: Vec<String> = ATTRIBUTES.iter()
            .filter(|(c, _)| *c == ch)
            .map(|(_, name)| format!("{} {}",
                style::fg(name, t::FG_MUTED),
                style::bold(&style::fg(&format!("{}", npc.attr(name)), t::FG_BRIGHT))))
            .collect();
        out.push(format!("  {}  {}", ch_label, attrs.join("  ")));
    }
    out.push(String::new());

    // Top non-zero skills (limit to 12 to keep the block compact).
    let mut top_skills: Vec<(String, i32, i32)> = Vec::new();
    for (attr, skills) in SKILLS {
        let parent = match crate::pc::attribute_parent(attr) {
            Some(p) => p, None => continue,
        };
        for skill in *skills {
            let rank = npc.skill(attr, skill);
            if rank == 0 { continue; }
            let total = npc.skill_total(parent, attr, skill);
            top_skills.push((format!("{} ({})", skill, attr), rank, total));
        }
    }
    if let Some(mc) = npc.skills.get("Melee Combat") {
        for (s, r) in mc {
            if *r > 0 {
                let total = npc.skill_total(Char::Body, "Melee Combat", s);
                top_skills.push((format!("{} (Melee)", s), *r, total));
            }
        }
    }
    if let Some(mc) = npc.skills.get("Missile Combat") {
        for (s, r) in mc {
            if *r > 0 {
                let total = npc.skill_total(Char::Body, "Missile Combat", s);
                top_skills.push((format!("{} (Missile)", s), *r, total));
            }
        }
    }
    top_skills.sort_by_key(|(_, _, t)| -*t);
    out.push(style::fg("Top skills (rank/total)", t::TAN).to_string());
    for (name, rank, total) in top_skills.iter().take(12) {
        out.push(format!("  {} — {}/{}",
            style::fg(name, t::FG),
            style::fg(&rank.to_string(), t::FG_MUTED),
            style::bold(&style::fg(&total.to_string(), t::FG_BRIGHT))));
    }
    out.push(String::new());

    // Equipment.
    out.push(style::fg("Equipment", t::TAN).to_string());
    for w in &npc.weapons {
        let kind = match w.kind {
            crate::pc::WeaponKind::Melee   => "melee",
            crate::pc::WeaponKind::Missile => "missile",
        };
        out.push(format!("  {} {} — Init {:+}, ±O {:+}, ±D {:+}, Dam {:+}, HP {}",
            style::fg(kind, t::FG_MUTED),
            style::fg(&w.name, t::FG),
            w.init, w.off_mod, w.def_mod, w.damage, w.hp));
    }
    if let Some(loc) = npc.hit_locations.values().next() {
        out.push(format!("  {} {} (AP {})",
            style::fg("armor", t::FG_MUTED),
            style::fg(&loc.armor, t::FG),
            loc.ap));
    }
    out
}

/// Pretty-print an encounter (header + per-NPC mini block).
fn format_encounter(enc: &crate::forge::encounter::Encounter) -> Vec<String> {
    use crust::style;
    let mut out = Vec::new();
    let header = format!("Encounter — {} ({})",
        enc.terrain_name(), enc.time_of_day());
    out.push(style::bold(&style::fg(&header, t::ACCENT)).to_string());
    if enc.is_no_encounter() {
        out.push(style::fg("  NO ENCOUNTER — the party travels in peace.", t::FG_MUTED).to_string());
        out.push(String::new());
        out.push(style::fg(
            "  Press 'A' for AI flavour (atmosphere over a quiet stretch of road).",
            t::AMBER,
        ).to_string());
        return out;
    }
    out.push(format!("  {}  {}  attitude: {}",
        style::fg(&format!("{}× {}", enc.count, enc.spec), t::FG),
        style::fg(&format!("(category: {})", enc.category), t::FG_MUTED),
        style::bold(&style::fg(&enc.attitude, attitude_color(&enc.attitude)))));
    out.push(String::new());
    if enc.is_event() { return out; }
    for (i, npc) in enc.npcs.iter().enumerate() {
        out.push(style::bold(&style::fg(
            &format!("[{}] {} ({} {}, lvl {})",
                i + 1, npc.name, npc.gender, npc.race, npc.level), t::AMBER)).to_string());
        // Combat-essential derived stats — these are what the GM
        // actually rolls against at the table. Mirror Amar-Tools'
        // single-line stat block format (BP / DB / MD / Reaction /
        // Dodge) and follow it with per-weapon OFF / DEF / Dam.
        let reaction = npc.skill_total(crate::pc::Char::Mind, "Awareness", "Reaction Speed");
        let dodge    = npc.skill_total(crate::pc::Char::Body, "Athletics", "Dodge");
        out.push(format!(
            "    BP {} | DB {} | MD {} | SIZE {} | Reaction {} | Dodge {}",
            npc.bp_max(), npc.db(), npc.md(), fmt_size(npc.size),
            reaction, dodge));
        for w in npc.weapons.iter() {
            let melee = matches!(w.kind, crate::pc::WeaponKind::Melee);
            let attr = if melee { "Melee Combat" } else { "Missile Combat" };
            let weap_skill = npc.skill(attr, &w.skill_name);
            let base = npc.ch(crate::pc::Char::Body) + npc.attr(attr) + weap_skill;
            let off_total = base + w.off_mod;
            let dodge_bonus = dodge / 5;
            let def_total = base + w.def_mod + dodge_bonus;
            let init_total = w.init + reaction;
            if melee {
                out.push(format!(
                    "    melee:   {:<14} Init {:+}  OFF {}  DEF {}  Dam {:+}",
                    w.name, init_total, off_total, def_total, w.damage));
            } else {
                out.push(format!(
                    "    missile: {:<14} Init {:+}  OFF {}  Rng {}m  Dam {:+}",
                    w.name, init_total, off_total, w.range_m, w.damage));
            }
        }
        if let Some(loc) = npc.hit_locations.values().next() {
            out.push(format!("    armor:   {} (AP {})", loc.armor, loc.ap));
        }
        out.push(String::new());
    }
    out.push(style::fg(
        "  Press 'A' for AI flavour — backstory, purpose, scenery, opening line.",
        t::AMBER,
    ).to_string());
    out
}

/// Pretty-print a town built by `forge::town::build_town`. Shows
/// the size bracket, total residents, and a grouped breakdown of
/// buildings (count per type) so a 200-house city doesn't fill the
/// pane with 200 individual lines.
fn format_town(t: &crate::forge::town::Town) -> Vec<String> {
    use crust::style;
    let mut out = Vec::new();
    let title = format!("{} — {}  ({} buildings, {} residents)",
        t.name, t.size_class, t.buildings.len(), t.total_residents);
    out.push(style::bold(&style::fg(&title, t::ACCENT)).to_string());
    out.push(String::new());

    // Group buildings by base name (strip ": Open ..." / ": <god>"
    // suffix) so the breakdown is digestible.
    let mut counts: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
    let mut temple_gods: Vec<String> = Vec::new();
    for b in &t.buildings {
        if let Some(god) = b.name.strip_prefix("Temple: ") {
            temple_gods.push(god.to_string());
            *counts.entry("Temple".into()).or_insert(0) += 1;
        } else {
            let base = b.name.split(':').next().unwrap_or(&b.name).to_string();
            *counts.entry(base).or_insert(0) += 1;
        }
    }

    out.push(style::fg("Buildings", t::TAN).to_string());
    let mut entries: Vec<(String, u32)> = counts.into_iter().collect();
    entries.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    for (name, n) in &entries {
        out.push(format!("  {} × {}",
            style::fg(&format!("{:>3}", n), t::FG_MUTED),
            style::fg(name, t::FG)));
    }
    out.push(String::new());

    if !temple_gods.is_empty() {
        out.push(style::fg("Temples", t::TAN).to_string());
        for g in &temple_gods {
            out.push(format!("  • {}", style::fg(g, t::AMBER)));
        }
        out.push(String::new());
    }

    // Detail table — names, residents. Truncated if the town is large.
    out.push(style::fg("Buildings (detail)", t::TAN).to_string());
    let limit = 60usize;
    for (i, b) in t.buildings.iter().enumerate() {
        if i >= limit {
            out.push(style::fg(&format!("  … and {} more (open the campaign log to dump full list)",
                t.buildings.len() - limit), t::FG_DIM).to_string());
            break;
        }
        // First line: building name + resident count. Subsequent
        // lines: one per named inhabitant with sex / age /
        // personality so the GM can pick a "face" for the building
        // at a glance without diving into the relationship graph.
        // Cap at 3 named heads — beyond that the table swells and
        // the user can press 'r' for the full picture.
        out.push(format!("  {} ({} residents)",
            style::fg(&b.name, t::FG),
            style::fg(&b.residents.to_string(), t::FG_MUTED)));
        for p in b.people.iter().take(3) {
            out.push(format!("      {} {}",
                style::fg(&p.name, t::FG_MUTED),
                style::fg(
                    &format!("({}/{} · {})", p.sex, p.age, p.personality),
                    t::FG_DIM,
                ),
            ));
        }
        if b.people.len() > 3 {
            out.push(style::fg(
                &format!("      …and {} more", b.people.len() - 3), t::FG_DIM,
            ).to_string());
        }
    }
    out.push(String::new());
    out.push(style::fg(
        &format!("  Press 'r' to view the relationship graph ({} persons{}).",
            t.relations.persons.len(),
            if t.relations.truncated { ", truncated" } else { "" }),
        t::AMBER,
    ).to_string());
    out.push(style::fg(
        "  Press 'A' for AI flavour — overall feel, keep / inn / temple vignettes.",
        t::AMBER,
    ).to_string());
    out
}

fn attitude_color(a: &str) -> u8 {
    match a {
        "HOSTILE"      => 196,
        "ANTAGONISTIC" => 208,
        "NEUTRAL"      => 244,
        "POSITIVE"     => 76,
        "FRIENDLY"     => 46,
        _ => 252,
    }
}

fn truncate_or_pad(s: &str, n: usize) -> String {
    let cc = s.chars().count();
    if cc <= n { s.to_string() }
    else { format!("{}…", s.chars().take(n.saturating_sub(1)).collect::<String>()) }
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

/// One past the highest `id` already used in a Saved-vector, so each
/// new entry gets a stable monotonic id without re-using deleted ones.
fn next_id<T>(v: &[crate::store::Saved<T>]) -> u64 {
    v.iter().map(|s| s.id).max().map(|n| n + 1).unwrap_or(1)
}

/// Format a Unix timestamp (seconds since epoch, UTC) as
/// `YYYY-MM-DD HH:MM` for display in the Campaign tab footer. Uses
/// civil-from-days arithmetic; no `chrono` dep on the hot path.
fn fmt_unix(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let secs_of_day = secs % 86_400;
    let hh = secs_of_day / 3_600;
    let mm = (secs_of_day % 3_600) / 60;
    // Howard Hinnant's civil_from_days algorithm.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{:04}-{:02}-{:02} {:02}:{:02}", y, m, d, hh, mm)
}

/// Pull the AI-flavour prose out of `forge_output` so it can be
/// saved alongside the artefact. The `ai_enrich_*` family writes a
/// known marker line (`  AI flavour` styled with the ACCENT colour)
/// followed by a blank line and then the wrapped prose. We strip
/// the marker plus the leading-2-space indent and return one
/// joined string. Returns `None` when no flavour section is present.
fn extract_ai_flavour(lines: &[String]) -> Option<String> {
    let start = lines.iter().position(|l| {
        // ANSI-styled marker — match on the stripped form.
        let plain = crust::strip_ansi(l);
        plain.trim() == "AI flavour"
    })?;
    // Skip the marker line plus the blank that follows.
    let mut out = String::new();
    let mut started = false;
    for line in &lines[start + 1..] {
        let plain = crust::strip_ansi(line);
        if !started && plain.trim().is_empty() { continue; }
        started = true;
        // The renderer indented each line with two spaces; strip
        // that to keep the saved blob clean.
        let body = plain.strip_prefix("  ").unwrap_or(&plain);
        out.push_str(body);
        out.push('\n');
    }
    let trimmed = out.trim();
    if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
}

/// Pipe `input` to `claude -p <prompt>`, return stdout. The Inspire
/// tab hands the terminal off to an interactive Claude session;
/// **this** is the opposite mode: a one-shot capture call used by
/// the AI-flavour shortcuts (`A` on Forge results). Inherits the
/// user's PATH so the same `claude` binary they invoke from the
/// shell answers. 60-second wall clock is plenty for short prose;
/// callers display a "asking claude…" status while it runs.
fn claude_pipe(prompt: &str, input: &str) -> Result<String, String> {
    use std::io::Write as _;
    use std::process::{Command, Stdio};
    let mut child = Command::new("claude")
        .args(["-p", prompt])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => "binary not on PATH".to_string(),
            _ => format!("spawn: {}", e),
        })?;
    if !input.is_empty() {
        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(input.as_bytes())
                .map_err(|e| format!("stdin: {}", e))?;
        }
    }
    drop(child.stdin.take());
    let output = child.wait_with_output()
        .map_err(|e| format!("wait: {}", e))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let snippet = err.lines().next().unwrap_or("(no message)");
        return Err(snippet.chars().take(80).collect());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
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
