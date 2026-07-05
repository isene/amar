//! Adventure — a published / GM-authored adventure sitting inside a
//! campaign. References an on-disk directory (no copying) and indexes
//! whatever's interesting there: the narrative markdown, parsed
//! section list, scene / floorplan / NPC-portrait images, and any
//! `.npc` stat-block text files. Plus a "current section" pointer so
//! the GM can pause + resume between sessions.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// One adventure inside a campaign. All asset paths are RELATIVE to
/// `root_dir` so the campaign.json stays portable if the user later
/// moves the directory.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Adventure {
    pub id: u64,
    pub name: String,
    /// Absolute filesystem path to the adventure's root directory.
    /// Everything else (markdown, images, .npc files) is recorded as
    /// a path *relative* to this root.
    pub root_dir: String,
    /// Path (relative to `root_dir`) of the main narrative markdown.
    /// Empty if no .md was found at import time.
    #[serde(default)]
    pub narrative_md: String,
    /// Parsed section list (h2 + h3 headings) for navigation. Indexes
    /// into the file lined up at import time; can be re-derived by
    /// running `parse_sections` against the current file contents.
    #[serde(default)]
    pub sections: Vec<AdventureSection>,
    /// Which section is the GM currently on. Index into `sections`.
    /// `None` until the GM cursors onto a section + presses ENTER.
    #[serde(default)]
    pub current_section: Option<usize>,
    #[serde(default)]
    pub scenes: Vec<AdventureAsset>,
    #[serde(default)]
    pub floorplans: Vec<AdventureAsset>,
    #[serde(default)]
    pub npc_portraits: Vec<AdventureAsset>,
    /// `.npc` text files — raw Amar-Tools stat blocks. Stored as
    /// opaque docs (the parser → Character work is a follow-up).
    #[serde(default)]
    pub npc_docs: Vec<AdventureAsset>,
    /// Free-form GM notes. Saved on the adventure so they live with
    /// the campaign rather than scattering in side files.
    #[serde(default)]
    pub notes: String,
}

/// One parsed heading from the narrative markdown. Level 2 (##) is
/// a top-level chapter, 3 (###) is a scene / location / NPC inside it.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdventureSection {
    pub heading: String,
    pub level: u8,
    /// 1-based line number of the heading itself.
    pub line_start: usize,
    /// 1-based line number of the last line that still belongs to
    /// this section (exclusive of the next heading at same or higher
    /// level).
    pub line_end: usize,
    /// Scene / floorplan image rel-paths attached to this section.
    /// Populated by `attach_scene_images` at import / rescan time.
    /// Empty if nothing matched.
    #[serde(default)]
    pub attached_images: Vec<String>,
    /// In-session GM notes appended to this section over time. Each
    /// entry carries a unix timestamp so the next session can scroll
    /// back through what happened. Saved with the campaign.json.
    #[serde(default)]
    pub notes: Vec<TimestampedNote>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TimestampedNote {
    /// Unix seconds.
    pub at: u64,
    pub text: String,
}

/// One filesystem asset (PNG / JPG / text doc) belonging to an
/// adventure. Display name is derived from the filename so the user
/// gets readable labels in the tree.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdventureAsset {
    pub name: String,
    /// Path relative to the parent adventure's `root_dir`.
    pub path: String,
}

impl Adventure {
    /// Resolve an asset's stored relative path against `root_dir` so
    /// the caller has an absolute path it can hand to glow / fs::read.
    pub fn absolute(&self, rel: &str) -> PathBuf {
        Path::new(&self.root_dir).join(rel)
    }

    /// Refresh `sections` from the current contents of `narrative_md`.
    /// Cheap (single linear scan). Call on import + whenever the GM
    /// has edited the markdown out-of-band and wants the tree to
    /// catch up.
    pub fn rescan_sections(&mut self) {
        let path = self.absolute(&self.narrative_md);
        let Ok(text) = std::fs::read_to_string(&path) else {
            self.sections = Vec::new();
            return;
        };
        self.sections = parse_sections(&text);
    }
}

/// Walk an on-disk directory and produce an `Adventure`. Doesn't
/// recurse into nested adventures — assumes one adventure per
/// directory tree.
pub fn import_from_dir(root: &Path, next_id: u64) -> Result<Adventure, String> {
    if !root.is_dir() {
        return Err(format!("not a directory: {}", root.display()));
    }
    let name = root.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Adventure".into());

    // Pick the primary narrative file. Preference order:
    //   1. <DirName>.md at the root
    //   2. The largest .md at the root
    //   3. ""  (no narrative yet — empty adventures are allowed)
    let preferred = root.join(format!("{}.md", name));
    let narrative_md: String = if preferred.is_file() {
        format!("{}.md", name)
    } else {
        find_largest_md(root).unwrap_or_default()
    };

    let mut adv = Adventure {
        id: next_id,
        name: name.clone(),
        root_dir: root.to_string_lossy().to_string(),
        narrative_md,
        sections: Vec::new(),
        current_section: None,
        scenes: scan_assets(root, &["Scenes", "scenes"], IMAGE_EXTS),
        floorplans: scan_assets(root, &["Floorplans", "floorplans", "Maps", "maps"], IMAGE_EXTS),
        npc_portraits: scan_assets(root, &["NPCs", "npcs", "Portraits", "portraits"], IMAGE_EXTS),
        npc_docs: scan_npc_docs(root),
        notes: String::new(),
    };
    adv.rescan_sections();
    attach_scene_images(&mut adv);
    Ok(adv)
}

/// Build the per-section attachment of scene images. Matches by:
///   * Numeric prefix on the image name (e.g. "1.png", "1_Bandits.png")
///     against any heading containing a matching standalone number
///     ("Scene 1:", "1.", "Map 1").
///   * Keyword fragments inside the image's pretty-name against the
///     heading text (case-insensitive substring), so
///     "antechamber.png" finds the "Antechamber" section.
///
/// An image attaches to AT MOST ONE section (the first match in
/// document order). Sections can collect multiple images. Unmatched
/// images stay in `adv.scenes` / `adv.floorplans` as unattached so
/// they're still browsable.
fn attach_scene_images(adv: &mut Adventure) {
    // Clear any previous attachments before re-running so a
    // post-`R`escan refresh re-binds cleanly.
    for sec in adv.sections.iter_mut() {
        sec.attached_images.clear();
    }
    // Track which image paths have been attached so we don't double-
    // attach the same file to multiple sections.
    let mut taken: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Walk scenes + floorplans in stable order so the same input
    // always produces the same attachments.
    let candidates: Vec<(String, String)> = adv.scenes.iter()
        .chain(adv.floorplans.iter())
        .map(|a| (a.name.clone(), a.path.clone()))
        .collect();
    for (img_name, img_path) in &candidates {
        if let Some(sec_idx) = best_section_for(adv, img_name) {
            adv.sections[sec_idx].attached_images.push(img_path.clone());
            taken.insert(img_path.clone());
        }
    }
}

/// Return the index of the most plausible section for the given
/// image-pretty-name. None if no rule matches.
///
/// Match priority (high → low):
///   1. Keyword in the image name appears in the heading.
///      Picks up `antechamber.png` → "Antechamber" before any
///      coincidental number-only collisions.
///   2. Pure-numeric image (1.png, 17.png) preferentially binds
///      to a `Scene N:` / `Map N` heading — those are the
///      common-case sources of scene illustrations.
///   3. Same pure-numeric image then falls back to a numbered
///      location / complication heading (`N. Temple of Ielina`).
fn best_section_for(adv: &Adventure, img_name: &str) -> Option<usize> {
    let img_lower = img_name.to_lowercase();
    // Numeric-only names (1.png, 17.png) are intentional scene
    // illustrations; try them FIRST so they outrank coincidental
    // keyword hits.
    if img_lower.chars().all(|c| c.is_ascii_digit()) && !img_lower.is_empty() {
        let n: u32 = img_lower.parse().ok()?;
        // Prefer Scene N over Map N — "scene images" is the
        // primary intent the user expressed for these numeric
        // filenames; maps generally have explicit names.
        for (i, sec) in adv.sections.iter().enumerate() {
            let h = sec.heading.to_lowercase();
            if h.starts_with(&format!("scene {}:", n))
                || h.starts_with(&format!("scene {} ", n))
            {
                return Some(i);
            }
        }
        for (i, sec) in adv.sections.iter().enumerate() {
            let h = sec.heading.to_lowercase();
            if h.starts_with(&format!("map {}:", n))
                || h.starts_with(&format!("map {} ", n))
            {
                return Some(i);
            }
        }
        for (i, sec) in adv.sections.iter().enumerate() {
            let h = sec.heading.to_lowercase();
            if h.starts_with(&format!("{}. ", n))
                || h.starts_with(&format!("{}: ", n))
            {
                return Some(i);
            }
        }
        return None;
    }
    // Keyword match. Skip NPC stat-block headings — those contain
    // a "(M," / "(F," demographic mark or end with "[Level N]" and
    // their bodies often mention an environment word that would
    // hijack an unrelated image (e.g. "sewer Torik" matching
    // "Torik the Rat-Catcher ... Sewer Worker").
    //
    // Try the full pretty-name as a substring first — that's how
    // multi-word names like "lower catacombs" land on the right
    // section. Then fall back to the longest single token, min
    // length 3 (lets "lab" / "lab.png" find "Laboratory").
    let full = img_lower.replace(|c: char| !c.is_alphanumeric() && c != ' ', " ");
    let full = full.trim();
    if !full.is_empty() && full.len() >= 3 {
        for (i, sec) in adv.sections.iter().enumerate() {
            if is_stat_block_heading(&sec.heading) { continue; }
            if sec.heading.to_lowercase().contains(full) {
                return Some(i);
            }
        }
    }
    // Common words are useless as match keys: they appear in unrelated
    // headings and hijack an image onto the wrong section. e.g. the scene
    // "The Hourglass" has no heading containing "hourglass", so without
    // this it falls through to "the" and lands on the first "The …"
    // section ("The Hook"). Drop stopwords so such images stay unattached
    // (browsable as plain scene assets) instead of mis-attaching.
    const STOP: &[&str] = &[
        "the", "and", "for", "with", "from", "into", "onto", "over",
        "that", "this", "your", "you", "are", "was", "not", "all",
    ];
    let mut tokens: Vec<&str> = img_lower.split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty() && s.len() >= 3 && !STOP.contains(s))
        .collect();
    tokens.sort_by_key(|s| std::cmp::Reverse(s.len()));
    for key in tokens {
        for (i, sec) in adv.sections.iter().enumerate() {
            if is_stat_block_heading(&sec.heading) { continue; }
            if sec.heading.to_lowercase().contains(key) {
                return Some(i);
            }
        }
    }
    None
}

/// True if a heading looks like an NPC / creature stat-block
/// (carries a demographic in parens or a `[Level N]` suffix).
/// Used by the image-attachment matcher to skip these headings
/// so an unrelated image keyword doesn't bind to an NPC entry.
fn is_stat_block_heading(h: &str) -> bool {
    let hl = h.to_lowercase();
    if hl.contains("[level ") { return true; }
    if hl.contains("(m,") || hl.contains("(f,")
        || hl.contains("(m ") || hl.contains("(f ")
    { return true; }
    false
}

const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif"];

/// Recursively walk one of the canonical subdirectories of an
/// adventure root and collect every matching file. Returns an empty
/// Vec if none of the candidate names resolves.
fn scan_assets(root: &Path, candidate_dirs: &[&str], exts: &[&str]) -> Vec<AdventureAsset> {
    let mut out: Vec<AdventureAsset> = Vec::new();
    for cand in candidate_dirs {
        let dir = root.join(cand);
        if !dir.is_dir() { continue; }
        walk_dir(&dir, &mut |p| {
            if !is_image_or_ext(p, exts) { return; }
            let rel = p.strip_prefix(root).ok()
                .map(|r| r.to_string_lossy().to_string())
                .unwrap_or_else(|| p.to_string_lossy().to_string());
            out.push(AdventureAsset {
                name: pretty_name(p),
                path: rel,
            });
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Scan for `.npc` files. Looks at the root + a handful of common
/// subdirectory names (most ThePortal-style layouts keep them in a
/// dedicated NPCs / GeneralNPCs folder, but some sprinkle them at
/// the root). De-duplicates by relative path.
fn scan_npc_docs(root: &Path) -> Vec<AdventureAsset> {
    let mut out: Vec<AdventureAsset> = Vec::new();
    let mut visit = |p: &Path| {
        if p.extension().and_then(|e| e.to_str()) != Some("npc") { return; }
        let rel = p.strip_prefix(root).ok()
            .map(|r| r.to_string_lossy().to_string())
            .unwrap_or_else(|| p.to_string_lossy().to_string());
        if out.iter().any(|a| a.path == rel) { return; }
        out.push(AdventureAsset { name: pretty_name(p), path: rel });
    };
    // Root level
    if let Ok(entries) = std::fs::read_dir(root) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_file() { visit(&p); }
        }
    }
    // Common subdirs
    for sub in ["GeneralNPCs", "NPCs", "npcs", "Trial", "Encounters"] {
        let dir = root.join(sub);
        if !dir.is_dir() { continue; }
        walk_dir(&dir, &mut |p| visit(p));
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Recursive directory walk; calls `f` for every regular file. Skips
/// hidden directories (`.git`, `.claude` etc.).
fn walk_dir(dir: &Path, f: &mut dyn FnMut(&Path)) {
    let Ok(entries) = std::fs::read_dir(dir) else { return; };
    for e in entries.flatten() {
        let p = e.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') { continue; }
        if p.is_dir() {
            walk_dir(&p, f);
        } else if p.is_file() {
            f(&p);
        }
    }
}

fn is_image_or_ext(p: &Path, exts: &[&str]) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            let e = e.to_ascii_lowercase();
            exts.iter().any(|x| *x == e)
        })
        .unwrap_or(false)
}

/// Turn a file path into a readable display name. Splits CamelCase /
/// `snake_case` / `kebab-case`, drops the extension, and trims any
/// numeric prefix the user used for sort-ordering (e.g. `1_Bandits`
/// becomes `Bandits`).
fn pretty_name(p: &Path) -> String {
    let stem = p.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    // Drop a leading "N_" or "N-" numeric prefix.
    let stem = stem.split_once(|c: char| c == '_' || c == '-')
        .and_then(|(prefix, rest)| {
            if prefix.chars().all(|c| c.is_ascii_digit()) && !prefix.is_empty()
                { Some(rest) } else { None }
        })
        .unwrap_or(stem);
    // CamelCase → "Camel Case"
    let mut out = String::new();
    let mut prev_lower = false;
    for ch in stem.chars() {
        if ch == '_' || ch == '-' {
            out.push(' ');
            prev_lower = false;
        } else if ch.is_uppercase() && prev_lower {
            out.push(' ');
            out.push(ch);
            prev_lower = false;
        } else {
            out.push(ch);
            prev_lower = ch.is_lowercase();
        }
    }
    out.trim().to_string()
}

/// Look at every `.md` file at the root and return the largest one.
fn find_largest_md(root: &Path) -> Option<String> {
    let entries = std::fs::read_dir(root).ok()?;
    let mut best: Option<(u64, String)> = None;
    for e in entries.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) != Some("md") { continue; }
        let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
        let name = p.file_name().and_then(|n| n.to_str()).map(|s| s.to_string())?;
        if best.as_ref().map(|(s, _)| size > *s).unwrap_or(true) {
            best = Some((size, name));
        }
    }
    best.map(|(_, n)| n)
}

/// Parse `## ` and `### ` headings out of a markdown blob.
/// `line_start` is 1-based and points at the heading itself;
/// `line_end` is 1-based and inclusive of the last line that still
/// belongs to that section.
pub fn parse_sections(text: &str) -> Vec<AdventureSection> {
    let lines: Vec<&str> = text.lines().collect();
    let mut headings: Vec<AdventureSection> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let (level, body) = if let Some(rest) = line.strip_prefix("### ") {
            (3u8, rest)
        } else if let Some(rest) = line.strip_prefix("## ") {
            (2u8, rest)
        } else {
            continue;
        };
        headings.push(AdventureSection {
            heading: body.trim().to_string(),
            level,
            line_start: i + 1,
            line_end: lines.len(),  // patched below
            attached_images: Vec::new(),
            notes: Vec::new(),
        });
    }
    // Patch `line_end` so each section ends just before the next
    // heading at SAME or HIGHER level (lower numeric = higher level).
    for i in 0..headings.len() {
        let me = headings[i].clone();
        let end = headings.iter().skip(i + 1)
            .find(|h| h.level <= me.level)
            .map(|h| h.line_start - 1)
            .unwrap_or(lines.len());
        headings[i].line_end = end;
    }
    headings
}

/// Pull the body lines for a section out of the markdown. Returns
/// the lines (without trailing newlines), ready to push into a
/// right-pane render. Bounds-checked against the live file.
pub fn section_body(adv: &Adventure, sec_idx: usize) -> Vec<String> {
    let Some(sec) = adv.sections.get(sec_idx) else { return Vec::new(); };
    let path = adv.absolute(&adv.narrative_md);
    let Ok(text) = std::fs::read_to_string(&path) else { return Vec::new(); };
    let lines: Vec<&str> = text.lines().collect();
    // Start AFTER the heading line itself — the renderer paints the
    // heading separately, so including it here would double it up.
    let start = sec.line_start;  // line_start is 1-based; the slice
                                 // is 0-based, so this skips line N.
    let end = sec.line_end.min(lines.len());
    if start >= end { return Vec::new(); }
    lines[start..end].iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_name_strips_numeric_prefix_and_splits_camel() {
        assert_eq!(pretty_name(Path::new("1_Bandits.png")), "Bandits");
        assert_eq!(pretty_name(Path::new("BrotherAlricDovennan.png")),
            "Brother Alric Dovennan");
        assert_eq!(pretty_name(Path::new("Old-Palthar-Keep.npc")),
            "Old Palthar Keep");
    }

    #[test]
    fn parse_sections_handles_nested_levels() {
        let text = "# Title\n## Overview\nBody\n### Hook\nMore\n## Next\nDone\n";
        let secs = parse_sections(text);
        assert_eq!(secs.len(), 3);
        assert_eq!(secs[0].heading, "Overview");
        assert_eq!(secs[0].level, 2);
        // Overview should end before "## Next" (line 6), so line_end = 5.
        assert_eq!(secs[0].line_end, 5);
        assert_eq!(secs[1].heading, "Hook");
        // Hook ends before "## Next" (line 6), so line_end = 5.
        assert_eq!(secs[1].line_end, 5);
        assert_eq!(secs[2].heading, "Next");
    }
}
