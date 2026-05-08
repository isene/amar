//! Lore module — bundles the markdown reference files into the binary
//! and exposes the tree structure the Lore tab navigates.
//!
//! Six setting / system docs sourced from d6gaming.org + the skill refs
//! get include_str!()-baked. The three canon categories (Spells,
//! Rituals, Potions) are drilled into via the live `Canon` index, so
//! the tree stays in sync with whatever the scraper produced.

use crate::canon::{Canon, Entry};

pub const SETTING_DOCS: &[(&str, &str)] = &[
    ("Mythology",        include_str!("../data/lore/mythology.md")),
    ("Kingdom of Amar",  include_str!("../data/lore/kingdom.md")),
    ("World",            include_str!("../data/lore/world.md")),
    ("Calendar",         include_str!("../data/lore/calendar.md")),
    ("Combat (quickref)", include_str!("../data/lore/combat.md")),
    ("Magic (quickref)",  include_str!("../data/lore/magic.md")),
];

/// One node in the Lore tree. Doc nodes carry their markdown directly;
/// CanonCategory nodes are containers we expand into a list of entries
/// (which are themselves Entry leaves, drawn from the Canon at render
/// time).
#[derive(Debug, Clone)]
pub enum Node {
    Doc { title: String, body: String },
    CanonCategory { title: String, category: String, count: usize },
    CanonEntry { name: String },
}

impl Node {
    pub fn title(&self) -> &str {
        match self {
            Node::Doc { title, .. } => title,
            Node::CanonCategory { title, .. } => title,
            Node::CanonEntry { name } => name,
        }
    }
}

/// The flat tree the Lore tab navigates. We keep it flat (with depth
/// stored alongside each node) so cursor math is just an index, not a
/// recursive descent.
pub struct Tree {
    pub items: Vec<TreeItem>,
}

#[derive(Debug, Clone)]
pub struct TreeItem {
    pub node: Node,
    pub depth: u8,
    /// Set on CanonCategory items so we know to render the +/- glyph.
    pub expandable: bool,
    pub expanded: bool,
}

impl Tree {
    pub fn build(canon: &Canon, expanded: &[String]) -> Self {
        let mut items: Vec<TreeItem> = Vec::new();

        // Setting / system docs come first.
        for (title, body) in SETTING_DOCS {
            items.push(TreeItem {
                node: Node::Doc { title: (*title).to_string(), body: (*body).to_string() },
                depth: 0,
                expandable: false,
                expanded: false,
            });
        }

        // Then the three canon categories — drillable.
        for cat in ["Spells", "Rituals", "Potions"] {
            let count = canon.category(cat).len();
            let title = format!("{} ({})", cat, count);
            let is_expanded = expanded.iter().any(|e| e == cat);
            items.push(TreeItem {
                node: Node::CanonCategory {
                    title,
                    category: cat.to_string(),
                    count,
                },
                depth: 0,
                expandable: true,
                expanded: is_expanded,
            });
            if is_expanded {
                for name in canon.category(cat) {
                    items.push(TreeItem {
                        node: Node::CanonEntry { name: name.clone() },
                        depth: 1,
                        expandable: false,
                        expanded: false,
                    });
                }
            }
        }

        Tree { items }
    }

    pub fn len(&self) -> usize { self.items.len() }
    pub fn get(&self, i: usize) -> Option<&TreeItem> { self.items.get(i) }
}

/// Render a markdown body into a vec of styled lines for a content
/// pane. We support the basics: # / ## / ### headings, **bold**,
/// *italic*, - / * lists, and link text. Tables get pass-through (the
/// pane renders the pipe characters as-is — fine for monospace).
pub fn render_markdown(body: &str) -> Vec<String> {
    use crust::style;
    let mut out: Vec<String> = Vec::new();
    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("# ") {
            out.push(String::new());
            out.push(style::bold(&style::fg(rest, 226)));
            out.push(style::fg(&"-".repeat(rest.chars().count()), 244));
        } else if let Some(rest) = line.strip_prefix("## ") {
            out.push(String::new());
            out.push(style::bold(&style::fg(rest, 117)));
        } else if let Some(rest) = line.strip_prefix("### ") {
            out.push(style::bold(&style::fg(rest, 250)));
        } else if let Some(rest) = line.strip_prefix("- ").or_else(|| line.strip_prefix("* ")) {
            out.push(format!("  • {}", inline(rest)));
        } else if line.starts_with("|") {
            out.push(style::fg(line, 250));
        } else {
            out.push(inline(line));
        }
    }
    out
}

/// Inline markdown: **bold**, *italic*, [text](url) -> just text.
/// Cheap state-machine pass — we don't need a full parser.
fn inline(s: &str) -> String {
    use crust::style;
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '*' && chars.peek() == Some(&'*') {
            chars.next();
            let mut buf = String::new();
            while let Some(&c2) = chars.peek() {
                if c2 == '*' {
                    chars.next();
                    if chars.peek() == Some(&'*') { chars.next(); break; }
                    buf.push('*');
                } else {
                    buf.push(c2);
                    chars.next();
                }
            }
            out.push_str(&style::bold(&buf));
        } else if c == '*' {
            let mut buf = String::new();
            while let Some(&c2) = chars.peek() {
                if c2 == '*' { chars.next(); break; }
                buf.push(c2);
                chars.next();
            }
            out.push_str(&style::italic(&buf));
        } else if c == '[' {
            let mut text = String::new();
            while let Some(&c2) = chars.peek() {
                chars.next();
                if c2 == ']' { break; }
                text.push(c2);
            }
            // Skip "(...)" if present.
            if chars.peek() == Some(&'(') {
                chars.next();
                while let Some(&c2) = chars.peek() {
                    chars.next();
                    if c2 == ')' { break; }
                }
            }
            out.push_str(&style::fg(&text, 117));
        } else if c == '`' {
            let mut buf = String::new();
            while let Some(&c2) = chars.peek() {
                chars.next();
                if c2 == '`' { break; }
                buf.push(c2);
            }
            out.push_str(&style::fg(&buf, 220));
        } else {
            out.push(c);
        }
    }
    out
}

/// Render a single canon Entry (spell / ritual / potion) into a vec of
/// content lines. Header + bullet-list of fields + description blurb +
/// wiki URL footer.
pub fn render_canon_entry(entry: &Entry) -> Vec<String> {
    use crust::style;
    let mut out: Vec<String> = Vec::new();
    out.push(String::new());
    out.push(style::bold(&style::fg(&entry.name, 226)));
    out.push(style::fg(&"-".repeat(entry.name.chars().count()), 244));
    if !entry.domains.is_empty() {
        out.push(style::fg(&format!("Domains: {}", entry.domains.join(", ")), 117));
    }
    out.push(String::new());

    // Stat fields. We render a known order first, then any extras.
    let preferred_order = [
        "domain", "encumbrance", "casting_time", "cooldown", "active_passive",
        "restrictions", "dr", "cost", "distance", "range", "duration",
        "area_of_effect", "effects", "receiving", "giving", "path(s)", "resist?",
    ];
    let mut shown = std::collections::BTreeSet::new();
    for key in preferred_order {
        if let Some(v) = entry.fields.get(key) {
            out.push(format!("  {:<18} {}",
                style::fg(&humanize(key), 245),
                v));
            shown.insert(key.to_string());
        }
    }
    for (k, v) in &entry.fields {
        if !shown.contains(k) && k != "description" && k != "display_name" && k != "back" && k != "category" {
            out.push(format!("  {:<18} {}",
                style::fg(&humanize(k), 245),
                v));
        }
    }

    if let Some(desc) = entry.fields.get("description") {
        out.push(String::new());
        out.push(style::fg("Description", 245).to_string());
        out.push(desc.clone());
    }

    out.push(String::new());
    out.push(style::fg(&format!("Source: {}", entry.url), 244));
    out
}

fn humanize(key: &str) -> String {
    let mut s = key.replace('_', " ");
    if let Some(c) = s.get_mut(0..1) {
        c.make_ascii_uppercase();
    }
    s
}
