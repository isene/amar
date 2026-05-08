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
/// pane.
///
/// Order of operations matters:
///
/// 1. Apply inline styles (`**bold**`, `*italic*`, `` `code` ``,
///    `[link](url)`) FIRST. The result has ANSI escapes embedded.
/// 2. Run `crust::text::format_markdown_tables` on that. The table
///    renderer's `display_width_cell` strips ANSI when computing column
///    widths, so cells with bold content land at the right visible
///    width and rows align with the `─/┼` header rule.
/// 3. Per-line pass for block-level styling (headings, list bullets).
///    No more inline-style work — those were already applied in step 1
///    and would only get mangled if we tried to re-process them now
///    (the inline pass would mistake `\x1b[1m…` for a `[link]`).
pub fn render_markdown(body: &str, max_width: usize) -> Vec<String> {
    use crust::style;
    let styled = apply_inline_styles(body);
    let cooked = crust::text::format_markdown_tables(&styled, max_width);
    let mut out: Vec<String> = Vec::new();
    for line in cooked.lines() {
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
            out.push(format!("  • {}", rest));
        } else {
            out.push(line.to_string());
        }
    }
    out
}

/// Apply inline markdown styling — `**bold**`, `*italic*`,
/// `` `code` ``, `[text](url)` — across the full body, including
/// inside table cells. Unclosed markers (e.g. a single `*` with no
/// pair) are emitted verbatim so list-marker lines like `* Item` and
/// stray asterisks don't get gobbled.
fn apply_inline_styles(s: &str) -> String {
    use crust::style;
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '*' && chars.peek() == Some(&'*') {
            // **bold** — needs a closing **.
            chars.next();
            let mut buf = String::new();
            let mut closed = false;
            while let Some(&c2) = chars.peek() {
                if c2 == '*' {
                    let mut peeker = chars.clone();
                    peeker.next();
                    if peeker.peek() == Some(&'*') {
                        chars.next();
                        chars.next();
                        closed = true;
                        break;
                    }
                }
                buf.push(c2);
                chars.next();
            }
            if closed { out.push_str(&style::bold(&buf)); }
            else { out.push_str("**"); out.push_str(&buf); }
        } else if c == '*' {
            // *italic* — single asterisk, needs closing *.
            let mut buf = String::new();
            let mut closed = false;
            while let Some(&c2) = chars.peek() {
                if c2 == '*' { chars.next(); closed = true; break; }
                if c2 == '\n' { break; } // never cross a newline
                buf.push(c2);
                chars.next();
            }
            if closed { out.push_str(&style::italic(&buf)); }
            else { out.push('*'); out.push_str(&buf); }
        } else if c == '`' {
            let mut buf = String::new();
            let mut closed = false;
            while let Some(&c2) = chars.peek() {
                if c2 == '`' { chars.next(); closed = true; break; }
                if c2 == '\n' { break; }
                buf.push(c2);
                chars.next();
            }
            if closed { out.push_str(&style::fg(&buf, 220)); }
            else { out.push('`'); out.push_str(&buf); }
        } else if c == '[' {
            let mut text = String::new();
            let mut closed = false;
            while let Some(&c2) = chars.peek() {
                if c2 == ']' { chars.next(); closed = true; break; }
                if c2 == '\n' { break; }
                text.push(c2);
                chars.next();
            }
            if closed && chars.peek() == Some(&'(') {
                chars.next();
                while let Some(&c2) = chars.peek() {
                    chars.next();
                    if c2 == ')' { break; }
                    if c2 == '\n' { break; }
                }
                out.push_str(&style::fg(&text, 117));
            } else if closed {
                out.push('[');
                out.push_str(&text);
                out.push(']');
            } else {
                out.push('[');
                out.push_str(&text);
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kingdom_table_columns_align_with_separator() {
        let body = include_str!("../data/lore/kingdom.md");
        let out = render_markdown(body, 113);
        // Find the lines belonging to the Six Districts table and
        // verify they all have the same visible width (rule, header,
        // every body row). Pre-fix, rows that contained `**X**` came
        // out 4 cols shorter than the rule because inline() stripped
        // the markers AFTER format_table had padded around them.
        let mut widths: Vec<usize> = Vec::new();
        let mut in_zone = false;
        for line in &out {
            let stripped = crust::strip_ansi(line);
            if stripped.contains("Six Districts") { in_zone = true; continue; }
            if in_zone && stripped.contains("Magick in the Kingdom") { break; }
            if in_zone && stripped.contains("│") {
                widths.push(crust::display_width(&stripped));
            }
            // Catch the rule too — it has ─/┼ but not │.
            if in_zone && stripped.contains("┼") {
                widths.push(crust::display_width(&stripped));
            }
        }
        assert!(widths.len() >= 6, "found {} table lines", widths.len());
        let max = *widths.iter().max().unwrap();
        let min = *widths.iter().min().unwrap();
        assert!(max - min <= 1,
            "table rows misaligned: widths={:?} (max-min={})", widths, max - min);
    }

    /// Visual preview the table at 113 cols. `cargo test --
    /// --nocapture _dump_kingdom_table_at_113_preview` to print the
    /// table alongside per-line widths during development. Underscore
    /// prefix keeps the test out of normal output and `cargo test`
    /// summaries.
    #[test]
    fn _dump_kingdom_table_at_113_preview() {
        let body = include_str!("../data/lore/kingdom.md");
        let out = render_markdown(body, 113);
        let mut in_zone = false;
        for line in &out {
            let stripped = crust::strip_ansi(line);
            if stripped.contains("Six Districts") { in_zone = true; }
            if in_zone {
                let dw = crust::display_width(&stripped);
                eprintln!("[{:3}] {}", dw, stripped);
            }
            if in_zone && stripped.contains("Magick in the Kingdom") { break; }
        }
    }
}
