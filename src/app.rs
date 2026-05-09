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
        }
    }
    fn all() -> [CampSection; 6] {
        [CampSection::Pcs, CampSection::Adventures, CampSection::Npcs,
         CampSection::Locations, CampSection::Calendar, CampSection::Factions]
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
    /// Placeholder shown under an expanded section that has no items.
    Placeholder { section: CampSection, msg: String },
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
            .and_then(|n| Campaign::load(n).ok());
        let (cols, rows) = Crust::terminal_size();
        let mut header = Pane::new(1, 1, cols, 1, 255, 236);
        header.wrap = false;
        header.scroll = false;

        let body_h = rows.saturating_sub(2);
        let mut body = Pane::new(1, 2, cols, body_h, 252, 0);
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

        let mut left_marker = Pane::new(1, 2, 2, body_h, 240, 0);
        left_marker.wrap = false;
        left_marker.scroll = false;
        let mut left_pane = Pane::new(3, 2, left_pane_w, body_h, 252, 0);
        left_pane.wrap = false;

        let mut right_marker = Pane::new(left_total + 1, 2, 2, body_h, 240, 0);
        right_marker.wrap = false;
        right_marker.scroll = false;
        let mut right_pane = Pane::new(left_total + 3, 2, right_pane_w, body_h, 252, 0);
        right_pane.wrap = true;

        let mut footer = Pane::new(1, rows, cols, 1, 245, 236);
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
                "1" => { self.set_tab(Tab::Session); }
                "2" => { self.set_tab(Tab::Forge); }
                "3" => { self.set_tab(Tab::Campaign); }
                "4" => { self.set_tab(Tab::Lore); }
                "5" => { self.set_tab(Tab::Inspire); }
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
                "r" => self.render_all(),
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

    fn cycle_width(&mut self, reverse: bool) {
        self.pane_width = if reverse {
            if self.pane_width <= 1 { 6 } else { self.pane_width - 1 }
        } else {
            if self.pane_width >= 6 { 1 } else { self.pane_width + 1 }
        };
        self.config.pane_width = self.pane_width;
        let _ = self.config.save();
        self.rebuild_panes();
        self.status_msg(&format!("Pane width: {} / 6", self.pane_width), 117);
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
        self.tab = t;
        // Tabs that have only one pane don't make sense with Right focus.
        if !self.tab_has_two_panes() {
            self.focus = Focus::Left;
        }
        self.render_all();
    }

    fn tab_has_two_panes(&self) -> bool {
        matches!(self.tab, Tab::Lore | Tab::Campaign)
    }

    fn handle_tab_key(&mut self, key: &str) {
        match self.tab {
            Tab::Lore     => self.handle_lore_key(key),
            Tab::Campaign => self.handle_campaign_key(key),
            _ => {}
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
        match key {
            "n" => { self.pc_new(); return; }
            "D" => { self.try_delete_under_cursor(); return; }
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
            "g" => { self.camp_idx = 0; self.right_pane.ix = 0; }
            "G" => { self.camp_idx = n.saturating_sub(1); self.right_pane.ix = 0; }
            "ENTER" | "l" | "RIGHT" => {
                if let Some(item) = tree.get(self.camp_idx) {
                    if let CampNode::Section(sec) = &item.node {
                        let id = sec.id().to_string();
                        if !self.camp_expanded.iter().any(|e| e == &id) {
                            self.camp_expanded.push(id);
                        }
                    }
                }
            }
            "h" | "LEFT" => {
                if let Some(item) = tree.get(self.camp_idx) {
                    let parent_section = match &item.node {
                        CampNode::Section(sec) => Some(*sec),
                        CampNode::Pc(_)        => Some(CampSection::Pcs),
                        CampNode::Adventure(_) => Some(CampSection::Adventures),
                        CampNode::Npc(_)       => Some(CampSection::Npcs),
                        CampNode::Location(_)  => Some(CampSection::Locations),
                        CampNode::Placeholder { section, .. } => Some(*section),
                    };
                    if let Some(sec) = parent_section {
                        let id = sec.id().to_string();
                        // Collapse + jump cursor to the section header.
                        self.camp_expanded.retain(|e| e != &id);
                        if let Some(pos) = build_camp_tree(camp, &self.camp_expanded)
                            .iter().position(|it| matches!(&it.node,
                                CampNode::Section(s) if *s == sec))
                        {
                            self.camp_idx = pos;
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
            // Weapons + spells — context-free shortcuts so the user
            // doesn't have to navigate to a specific section.
            "M" => self.add_weapon(crate::pc::WeaponKind::Melee),
            "I" => self.add_weapon(crate::pc::WeaponKind::Missile),
            "S" => self.add_spell(),
            _ => {}
        }
    }

    /// Add a melee or missile weapon to the focused PC. Prompts in
    /// sequence: name, hands (1H/2H, melee only), Init, ±O, ±D (melee
    /// only) or shots/round (missile only), Damage, Range (missile
    /// only), HP. Defaults are sensible for a knife / short sword.
    fn add_weapon(&mut self, kind: crate::pc::WeaponKind) {
        let pc_idx = match self.current_pc_idx() {
            Some(i) => i,
            None => return,
        };
        let kind_name = match kind {
            crate::pc::WeaponKind::Melee   => "melee",
            crate::pc::WeaponKind::Missile => "missile",
        };
        let name = self.footer.ask(&format!(" New {} weapon name: ", kind_name), "");
        let name = name.trim().to_string();
        if name.is_empty() {
            self.status_msg("Cancelled.", 208);
            return;
        }
        let two_handed = if matches!(kind, crate::pc::WeaponKind::Melee) {
            self.footer.ask(" Two-handed? (y/N): ", "").trim().eq_ignore_ascii_case("y")
        } else { false };
        let init: i32  = self.footer.ask(" Init: ", "0").trim().parse().unwrap_or(0);
        let off: i32   = self.footer.ask(" ±O (offence mod): ", "0").trim().parse().unwrap_or(0);
        let def: i32 = if matches!(kind, crate::pc::WeaponKind::Melee) {
            self.footer.ask(" ±D (defence mod): ", "0").trim().parse().unwrap_or(0)
        } else { 0 };
        let shots: u8 = if matches!(kind, crate::pc::WeaponKind::Missile) {
            self.footer.ask(" shots/round: ", "1").trim().parse().unwrap_or(1)
        } else { 0 };
        let dam: i32  = self.footer.ask(" Damage: ", "0").trim().parse().unwrap_or(0);
        let range: u32 = if matches!(kind, crate::pc::WeaponKind::Missile) {
            self.footer.ask(" Range (m): ", "30").trim().parse().unwrap_or(30)
        } else { 0 };
        let hp: i32   = self.footer.ask(" HP: ", "8").trim().parse().unwrap_or(8);

        if let Some(c) = self.campaign.as_mut() {
            if let Some(pc) = c.pcs.get_mut(pc_idx) {
                pc.weapons.push(crate::pc::Weapon {
                    name: name.clone(),
                    kind: kind.clone(),
                    two_handed,
                    init,
                    off_mod: off,
                    def_mod: def,
                    shots_per_round: shots,
                    damage: dam,
                    hp,
                    range_m: range,
                    xp: 0,
                });
            }
            let _ = c.save();
        }
        self.status_msg(&format!("Added {} weapon '{}'.", kind_name, name), 46);
    }

    /// Add a spell to the focused PC. The spell name is matched
    /// against the wiki canon — known spells display their full stat
    /// block on the sheet. Unknown names are accepted but flagged
    /// "(not in canon)" until the canon is regenerated.
    fn add_spell(&mut self) {
        let pc_idx = match self.current_pc_idx() {
            Some(i) => i,
            None => return,
        };
        let name = self.footer.ask(" Spell name (canon entry): ", "");
        let name = name.trim().to_string();
        if name.is_empty() {
            self.status_msg("Cancelled.", 208);
            return;
        }
        let in_canon = self.canon.lookup(&name).is_some();
        if let Some(c) = self.campaign.as_mut() {
            if let Some(pc) = c.pcs.get_mut(pc_idx) {
                pc.spells.push(name.clone());
            }
            let _ = c.save();
        }
        let suffix = if in_canon { "" } else { " (not in canon — will show as such)" };
        self.status_msg(&format!("Added spell '{}'.{}", name, suffix), 46);
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
            self.status_msg("Move the cursor onto an attribute or skill row first.", 208);
            return;
        };
        let name = self.footer.ask(&format!(" New skill under {} (name): ", attr), "");
        let name = name.trim().to_string();
        if name.is_empty() {
            self.status_msg("Cancelled.", 208);
            return;
        }
        let rank_str = self.footer.ask(" Initial rank [0]: ", "0");
        let rank: i32 = rank_str.trim().parse().unwrap_or(0);

        // Find the active PC and add the skill.
        let tree = match self.campaign.as_ref() {
            Some(camp) => build_camp_tree(camp, &self.camp_expanded),
            None => return,
        };
        let pc_idx = match tree.get(self.camp_idx).map(|t| t.node.clone()) {
            Some(CampNode::Pc(i)) => i,
            _ => return,
        };
        if let Some(c) = self.campaign.as_mut() {
            if let Some(pc) = c.pcs.get_mut(pc_idx) {
                pc.skills.entry(attr.clone())
                    .or_default()
                    .insert(name.clone(), rank);
            }
            let _ = c.save();
        }
        self.status_msg(&format!("Added skill '{}' under {}.", name, attr), 46);
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
    /// the campaign on success.
    fn edit_focused_field(&mut self) {
        let Some(field) = self.edits.get(self.sheet_idx).cloned() else { return; };
        let prompt = format!("{}: ", field.label);
        let value = self.footer.ask(&prompt, &field.current);
        // Look up the PC currently selected in the tree.
        let tree = match self.campaign.as_ref() {
            Some(camp) => build_camp_tree(camp, &self.camp_expanded),
            None => return,
        };
        let pc_idx = match tree.get(self.camp_idx).map(|t| t.node.clone()) {
            Some(CampNode::Pc(i)) => i,
            _ => return,
        };
        let result = if let Some(camp) = self.campaign.as_mut() {
            if let Some(pc) = camp.pcs.get_mut(pc_idx) {
                pc.set_field(&field.field_id, &value)
            } else { Err("PC not found".into()) }
        } else { Err("No campaign loaded".into()) };
        match result {
            Ok(_) => {
                if let Some(c) = self.campaign.as_ref() { let _ = c.save(); }
                self.status_msg(&format!("Updated {}.", field.label.trim()), 46);
            }
            Err(e) => self.status_msg(&format!("Edit failed: {}", e), 196),
        }
    }

    /// New PC — prompts only for the name; everything else gets a
    /// sensible default (Human, 70 kg → SIZE 3) and the user edits
    /// the rest inline by pressing ENTER on individual fields.
    fn pc_new(&mut self) {
        if self.campaign.is_none() {
            self.status_msg("No campaign loaded — press C to create one first.", 208);
            return;
        }
        let name = self.footer.ask(" PC name: ", "");
        let name = name.trim().to_string();
        if name.is_empty() {
            self.status_msg("Cancelled.", 208);
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
        self.status_msg(&format!("Added '{}'.", name), 46);
    }

    /// Delete whatever the cursor is currently on (PC for now).
    fn try_delete_under_cursor(&mut self) {
        let Some(camp) = self.campaign.as_ref() else { return; };
        let tree = build_camp_tree(camp, &self.camp_expanded);
        let Some(item) = tree.get(self.camp_idx) else { return; };
        let CampNode::Pc(idx) = item.node.clone() else {
            self.status_msg("Move cursor onto a PC to delete it (D).", 208);
            return;
        };
        let pc_name = camp.pcs.get(idx).map(|p| p.name.clone()).unwrap_or_default();
        let answer = self.footer.ask(&format!(" Delete '{}'? (y/N): ", pc_name), "");
        if answer.trim() != "y" && answer.trim() != "Y" { return; }
        if let Some(c) = self.campaign.as_mut() {
            c.pcs.remove(idx);
            let _ = c.save();
        }
        // Re-anchor cursor near the previous position.
        if let Some(camp) = self.campaign.as_ref() {
            let tree = build_camp_tree(camp, &self.camp_expanded);
            if self.camp_idx >= tree.len() {
                self.camp_idx = tree.len().saturating_sub(1);
            }
        }
        self.status_msg(&format!("Deleted '{}'.", pc_name), 46);
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
            "g" => { self.lore_idx = 0; self.right_pane.ix = 0; }
            "G" => {
                self.lore_idx = tree.len().saturating_sub(1);
                self.right_pane.ix = 0;
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
                _ => {}
            }
            return;
        }
        let lines = match self.tab {
            Tab::Session  => self.render_session(),
            Tab::Forge    => self.render_forge(),
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
        self.left_marker.fg = if left_active { 226 } else { 240 };
        self.left_marker.set_text(&stripe);
        self.left_marker.full_refresh();
        self.right_marker.fg = if right_active { 226 } else { 240 };
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
        self.right_pane.set_text(&content.join("\n"));
        self.right_pane.full_refresh();
    }

    fn render_footer(&mut self) {
        if let Some((ref msg, color)) = self.status {
            let pad = self.cols.saturating_sub(crust::display_width(msg) as u16 + 12 + 1) as usize;
            let right = format!("amar v{}", VERSION);
            let line = format!("{}{}{}", style::fg(msg, color), " ".repeat(pad), style::fg(&right, 244));
            self.footer.set_text(&line);
            self.footer.full_refresh();
            return;
        }
        let hint = match self.tab {
            Tab::Session  => " 1-5:tabs  C-LEFT/RIGHT:tabs  C:new-campaign  L:load  ?:help  q:quit",
            Tab::Forge    => " 1-5:tabs  C-LEFT/RIGHT:tabs  C:new-campaign  L:load  ?:help  q:quit",
            Tab::Campaign => match self.focus {
                Focus::Left  => " TAB:focus-sheet  j/k:tree  l/h:expand/collapse  n:add-PC  D:delete  C:new-camp  L:load",
                Focus::Right => " l/h:±1  j/k:±10  ENTER:edit  +:skill  M:melee  I:missile  S:spell  TAB:list",
            },
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
        self.footer.set_text(&line);
        self.footer.full_refresh();
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

    fn render_campaign_panes(&mut self) {
        // No campaign loaded → spell out the load/create flow on the
        // right pane, leave the left blank. Both markers stay dim.
        let Some(camp) = self.campaign.as_ref() else {
            self.left_pane.set_text("");
            self.left_pane.full_refresh();
            let mut hint = vec![
                String::new(),
                style::bold(&style::fg("  No campaign loaded", 226)).to_string(),
                String::new(),
                "  C — create a new campaign".into(),
                "  L — load an existing campaign".into(),
            ];
            let existing = list_campaigns();
            if !existing.is_empty() {
                hint.push(String::new());
                hint.push(style::fg("  Existing campaigns:", 245).to_string());
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
        left_lines.push(style::fg(&format!(" {}", camp.date.fmt_header()), 245));
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
                    style::bold(&style::fg(&row, 226))
                } else {
                    style::fg(&row, 244)
                }
            } else {
                match &item.node {
                    CampNode::Section(_) => style::fg(&row, 117),
                    CampNode::Placeholder { .. } => style::fg(&row, 245),
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
                let _ = idx;
                vec![style::fg("Adventure detail coming in v0.6+.", 245).to_string()]
            }
            Some(CampNode::Npc(_)) | Some(CampNode::Location(_)) => {
                vec![style::fg("(Coming in a later version.)", 245).to_string()]
            }
            Some(CampNode::Placeholder { msg, .. }) => {
                vec![String::new(), style::fg(&format!("  {}", msg), 245).to_string()]
            }
            None => vec![],
        };
        self.right_pane.set_text(&content.join("\n"));
        self.right_pane.full_refresh();
    }

    fn render_camp_section(&self, camp: &Campaign, sec: CampSection) -> Vec<String> {
        const LBL: u8 = 245;
        let mut out = vec![String::new()];
        match sec {
            CampSection::Pcs => {
                out.push(style::bold(&style::fg("Player characters", 226)));
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
                out.push(style::bold(&style::fg("Adventures", 226)));
                out.push(String::new());
                out.push("  0 adventures stored. Adventure authoring lands in v0.6.0".into());
                out.push("  via the Inspire tab (Adventure mode) or a deterministic".into());
                out.push("  table-driven skeleton when claude is not on PATH.".into());
            }
            CampSection::Npcs => {
                out.push(style::bold(&style::fg("NPCs", 226)));
                out.push(String::new());
                out.push("  Persistent NPC roster. Land in v0.4.0 once the".into());
                out.push("  Forge → NPC generator can save into here.".into());
            }
            CampSection::Locations => {
                out.push(style::bold(&style::fg("Locations", 226)));
                out.push(String::new());
                out.push("  Towns + landmarks visited or known to the party.".into());
                out.push("  Land in v0.4.0 alongside the Forge → Town generator.".into());
            }
            CampSection::Calendar => {
                out.push(style::bold(&style::fg("Calendar", 226)));
                out.push(String::new());
                out.push(field_row(LBL, "Today", &camp.date.fmt_header()));
                out.push(field_row(LBL, "Bortle", &camp.bortle.to_string()));
                out.push(String::new());
                out.push(style::fg("  Calendar advance + weather hookup land in v0.5.0.", LBL).to_string());
            }
            CampSection::Factions => {
                out.push(style::bold(&style::fg("Factions", 226)));
                out.push(String::new());
                out.push("  Faction reputation tracks (King's court, the Calah,".into());
                out.push("  the Cloaks, Dark Dagger, Magick Circle, the gods…)".into());
                out.push("  land in v0.5+.".into());
            }
        }
        out
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
        // portrait placeholder box in the top-right corner. Per-row
        // content stays inside `top_left_w` so it never overlaps the
        // portrait area.
        let port_w: usize = if pane_w >= 90 { 28 } else { 0 };
        let top_left_w = pane_w.saturating_sub(port_w + 2);
        let top_start = out.len();

        // Title
        let name_disp = if pc.name.is_empty() { "(unnamed)".to_string() } else { pc.name.clone() };
        let bp_max = pc.bp_max().max(1);
        let (state_text, state_color, status_penalty) =
            if pc.bp_current <= 0          { ("Helpless", STATUS_X,  None) }
            else if pc.bp_current <= bp_max / 4 { ("Heavily Wounded", STATUS_HW, Some(-4)) }
            else if pc.bp_current <= bp_max / 2 { ("Wounded",         STATUS_W,  Some(-2)) }
            else                                { ("Healthy",         STATUS_OK, Some(0))  };
        let title = if pc.player.is_empty() {
            style::bold(&style::fg(&name_disp, TITLE))
        } else {
            format!("{}  {}",
                style::bold(&style::fg(&name_disp, TITLE)),
                style::fg(&format!("({})", pc.player), PLAYER))
        };
        out.push(format!(" {}", title));

        // Identity rows — 3 cells × ~16 cols each. Tight enough that
        // the right portion stays free for the portrait area.
        let id_cell_w = (top_left_w / 3).max(14);
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
            out.push(cells.iter().map(|c| pad_visible(c, id_cell_w)).collect::<String>());
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

        // Description (multiline)
        let desc_active = active_id == Some("description");
        edits.push(EditableField { line: out.len(),
            field_id: "description".into(),
            label: " Description".into(),
            current: pc.description.clone() });
        if pc.description.is_empty() {
            out.push(format!(" {} {}",
                style::fg("Description:", LBL_ID),
                value_cell("", 8, desc_active)));
        } else {
            let mut lines = pc.description.lines();
            let first = lines.next().unwrap_or("");
            out.push(format!(" {} {}",
                style::fg("Description:", LBL_ID),
                if desc_active { value_cell(first, first.chars().count().max(1), true) }
                else { first.to_string() }));
            for cont in lines {
                out.push(format!("  {}", cont));
            }
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
        let _ = top_left_w;
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
                style::fg(&loc_bp.to_string(), 252));
            out.push(combined);
        }

        // Post-process the top section: overlay a portrait placeholder
        // box whose right edge aligns with the right edge of the
        // SPIRIT column in the 3-tier section below. Frame extends
        // through every row of the top section so it lines up
        // visually with the Stats column.
        let top_end = out.len();
        let three_col_right = if pane_w >= 96 {
            (pane_w / 3).max(30) * 3
        } else {
            pane_w
        };
        let port_left_col = three_col_right.saturating_sub(port_w);
        let port_h = top_end - top_start;
        if port_w >= 16 && port_h >= 4 {
            for i in 0..port_h {
                let row_idx = top_start + i;
                let original = out[row_idx].clone();
                let right = portrait_row(i, port_w, port_h);
                // Pad the original content to start the portrait at
                // `port_left_col` (right-edge aligned with SPIRIT).
                out[row_idx] = format!("{}{}",
                    pad_visible(&original, port_left_col),
                    right);
            }
        }
        out.push(String::new());

        // --- 3-tier Attributes & Skills (no header — obvious from
        // the BODY/MIND/SPIRIT column titles) ---
        let three_col = pane_w >= 96;
        if three_col {
            let body_col   = render_char_column(pc, Char::Body,   ATTRIBUTES, SKILLS, active_id);
            let mind_col   = render_char_column(pc, Char::Mind,   ATTRIBUTES, SKILLS, active_id);
            let spirit_col = render_char_column(pc, Char::Spirit, ATTRIBUTES, SKILLS, active_id);
            let col_w = (pane_w / 3).max(30);
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
        }
        out.push(String::new());

        // Melee weapons (always shown)
        out.push(style::bold(&style::fg("Melee weapons", 209)));
        out.push(format!("  {:<22} {:<3} {:>4} {:>4} {:>4} {:>4} {:>3}",
            style::fg("Name", LBL), style::fg("H", LBL), style::fg("Init", LBL),
            style::fg("±O", LBL), style::fg("±D", LBL), style::fg("Dam", LBL), style::fg("HP", LBL)));
        let mut any_melee = false;
        for w in pc.weapons.iter().filter(|w| matches!(w.kind, crate::pc::WeaponKind::Melee)) {
            any_melee = true;
            let h = if w.two_handed { "2H" } else { "1H" };
            out.push(format!("  {:<22} {:<3} {:>+4} {:>+4} {:>+4} {:>+4} {:>3}",
                w.name, h, w.init, w.off_mod, w.def_mod, w.damage, w.hp));
        }
        if !any_melee { out.push(style::fg("  (none)", LBL).to_string()); }
        out.push(String::new());

        // Missile weapons (always shown)
        out.push(style::bold(&style::fg("Missile weapons", 130)));
        out.push(format!("  {:<22} {:>4} {:>4} {:>4} {:>4} {:>4} {:>3}",
            style::fg("Name", LBL), style::fg("Init", LBL), style::fg("±O", LBL),
            style::fg("s/r", LBL), style::fg("Dam", LBL), style::fg("Rng", LBL), style::fg("HP", LBL)));
        let mut any_missile = false;
        for w in pc.weapons.iter().filter(|w| matches!(w.kind, crate::pc::WeaponKind::Missile)) {
            any_missile = true;
            out.push(format!("  {:<22} {:>+4} {:>+4} {:>4} {:>+4} {:>4} {:>3}",
                w.name, w.init, w.off_mod, w.shots_per_round, w.damage, w.range_m, w.hp));
        }
        if !any_missile { out.push(style::fg("  (none)", LBL).to_string()); }
        out.push(String::new());

        // Spells
        if !pc.spells.is_empty() {
            out.push(style::bold(&style::fg("Spells", 141)));
            for spell_name in &pc.spells {
                if let Some(entry) = self.canon.lookup(spell_name) {
                    let dr   = entry.fields.get("dr").map(|s| s.as_str()).unwrap_or("?");
                    let cost = entry.fields.get("cost").map(|s| s.as_str()).unwrap_or("?");
                    let dist = entry.fields.get("distance").map(|s| s.as_str()).unwrap_or("?");
                    out.push(format!("  {}  DR {}  cost {}  dist {}",
                        style::bold(spell_name), dr, cost, dist));
                } else {
                    out.push(format!("  {} {}", spell_name,
                        style::fg("(not in canon)", 208)));
                }
            }
            out.push(String::new());
        }

        // Equipment + money
        out.push(style::bold(&style::fg("Equipment", 180)));
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
        out.push(style::bold(&style::fg("Notes", 248)));
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
        value_cell(&format!("({})", pc.ch(ch)), 4, char_active)));
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
            value_cell(&format!("{:>2}", av), 3, attr_active)));

        // Skills — even lighter shade of the same hue.
        let canonical: &[&str] = skills.iter()
            .find(|(a, _)| *a == attr)
            .map(|(_, s)| *s)
            .unwrap_or(&[]);
        let mut shown = std::collections::BTreeSet::new();
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
            col.lines.push(format!("     {} {}  {:>3}",
                pad_visible(&skill_styled, 19),
                value_cell(&format!("{:>2}", rank), 3, skill_active),
                total));
            shown.insert((*skill).to_string());
        }
        if let Some(extras) = pc.skills.get(attr) {
            for (skill, rank) in extras {
                if shown.contains(skill) { continue; }
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
                col.lines.push(format!("     {} {}  {:>3}",
                    pad_visible(&skill_styled, 19),
                    value_cell(&format!("{:>2}", rank), 3, skill_active),
                    total));
            }
        }
    }
    col
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

/// Build one row of the portrait placeholder frame in the PC sheet's
/// top-right corner. The frame is dim — when the image renderer lands
/// (kitty/sixel via glow), the actual portrait will be drawn over the
/// inside cells. Until then, the frame and "(no portrait)" label make
/// it visible that the area is reserved.
fn portrait_row(row: usize, w: usize, total: usize) -> String {
    use crust::style;
    if w < 8 || total < 4 { return String::new(); }
    let dim: u8 = 240;
    let inner = w.saturating_sub(2);
    let frame = if row == 0 {
        format!("┌{}┐", "─".repeat(inner))
    } else if row == total - 1 {
        format!("└{}┘", "─".repeat(inner))
    } else if row == total / 3 {
        format!("│{:^iw$}│", "(no portrait)", iw = inner)
    } else if row == 2 * total / 3 {
        format!("│{:^iw$}│", "press P later", iw = inner)
    } else {
        format!("│{}│", " ".repeat(inner))
    };
    format!(" {}", style::fg(&frame, dim))
}

/// Pad a string with trailing spaces to reach the given visible width.
/// `crust::display_width` is ANSI-aware so embedded escape sequences
/// don't throw off the alignment.
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
            | CampSection::Npcs | CampSection::Locations);
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
                out.push(CampTreeItem {
                    node: CampNode::Placeholder { section: sec,
                        msg: "(adventure authoring lands in v0.6.0)".into() },
                    depth: 1,
                    expandable: false, expanded: false,
                });
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
            CampSection::Adventures => "Adventures (0)".to_string(),
            CampSection::Npcs       => format!("NPCs ({})", camp.npcs.len()),
            CampSection::Locations  => "Locations (0)".to_string(),
            CampSection::Calendar   => "Calendar".to_string(),
            CampSection::Factions   => "Factions".to_string(),
        },
        CampNode::Pc(idx) => {
            camp.pcs.get(*idx)
                .map(|p| format!("{}  L{}", p.name, p.level))
                .unwrap_or_else(|| "(missing PC)".to_string())
        }
        CampNode::Adventure(idx) => format!("Adventure #{}", idx + 1),
        CampNode::Npc(idx) => {
            camp.npcs.get(*idx).map(|n| n.name.clone())
                .unwrap_or_else(|| "(missing NPC)".to_string())
        }
        CampNode::Location(idx) => format!("Location #{}", idx + 1),
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
