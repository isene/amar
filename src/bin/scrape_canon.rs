//! d6gaming.org → data/canon.toml.
//!
//! Run once (or whenever the wiki has been updated) with:
//!
//!     cargo run --release --features scraper --bin scrape_canon
//!
//! Walks the relevant Category: pages, fetches each entry's wikitext via
//! ?action=raw, parses out the structured fields we care about, and emits
//! a single TOML file consumed by amar at startup.
//!
//! Source-of-truth invariant: every entry in canon.toml MUST be traceable
//! back to a wiki URL. Each scraped record carries the URL it came from so
//! we can verify by hand and so the user can click through to the wiki.
//!
//! Sparse pages (a name in a Category list with no detail page) are
//! emitted with `details = "missing"` so amar can render them as
//! placeholders rather than inventing content.

use std::collections::BTreeMap;
use std::path::Path;

const WIKI: &str = "https://d6gaming.org";

fn main() {
    let out_path = std::env::args().nth(1).unwrap_or_else(|| "data/canon.toml".to_string());
    let mut canon = Canon::default();

    eprintln!("amar canon scraper — d6gaming.org → {}", out_path);

    // The category list drives everything we pull. Adding a new category
    // here is the only change needed when the wiki adds a new section.
    let categories: &[(&str, &str)] = &[
        ("Spells",            "spells"),
        ("Rituals",           "rituals"),
        ("Potions",           "potions"),
        ("Fire_Magick",       "fire"),
        ("Water_Magick",      "water"),
        ("Earth_Magick",      "earth"),
        ("Air_Magick",        "air"),
        ("Life_Magick",       "life"),
        ("Black_Magick",      "black"),
        ("Ice_Magick",        "ice"),
        ("Lava_Magick",       "lava"),
        ("Magic_Magick",      "magic"),
        ("Perception_Magick", "perception"),
        ("Protection_Magick", "protection"),
        ("Summoning_Magick",  "summoning"),
    ];

    let mut domain_index: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut all_pages: BTreeMap<String, String> = BTreeMap::new();

    for (cat, _tag) in categories {
        eprintln!("\n[category] {}", cat);
        let pages = fetch_category(cat);
        eprintln!("  {} pages", pages.len());
        for p in &pages {
            domain_index.entry(cat.to_string()).or_default().push(p.clone());
            all_pages.entry(p.clone()).or_insert_with(|| cat.to_string());
        }
    }

    eprintln!("\n[total unique pages] {}", all_pages.len());

    // Pull each page's wikitext and parse. Polite single-threaded loop —
    // d6gaming.org is a small wiki, no point hammering it.
    for (page, first_cat) in &all_pages {
        eprint!("  [{}] {}…", first_cat, page);
        match fetch_raw(page) {
            Ok(wikitext) => {
                let entry = parse_entry(page, &wikitext);
                eprintln!(" ok");
                canon.entries.insert(page.clone(), entry);
            }
            Err(e) => {
                eprintln!(" FAIL: {}", e);
                canon.entries.insert(page.clone(), Entry {
                    name: page.clone(),
                    url: page_url(page),
                    details: "missing".to_string(),
                    domains: Vec::new(),
                    fields: BTreeMap::new(),
                });
            }
        }
    }

    canon.domain_index = domain_index;

    let s = toml::to_string_pretty(&canon).expect("serialize canon");
    let out_dir = Path::new(&out_path).parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(out_dir).expect("create canon dir");
    std::fs::write(&out_path, s).expect("write canon.toml");
    eprintln!("\nwrote {} ({} entries)", out_path, canon.entries.len());
}

fn page_url(page: &str) -> String {
    format!("{}/index.php/{}", WIKI, pct_encode_page(page))
}

fn raw_url(page: &str) -> String {
    format!("{}/index.php?title={}&action=raw", WIKI, pct_encode_page(page))
}

fn category_url(category: &str) -> String {
    format!("{}/index.php/Category:{}", WIKI, category.replace(' ', "_"))
}

/// Percent-encode a page name for use in a URL. Spaces become `_`
/// (MediaWiki convention); everything else outside the unreserved set
/// (alphanumerics, `_`, `-`, `.`, `~`) gets `%XX`-escaped per RFC 3986.
/// Multi-byte UTF-8 is encoded byte-by-byte (so `'` → %E2%80%99).
fn pct_encode_page(page: &str) -> String {
    let mut out = String::with_capacity(page.len());
    for &b in page.as_bytes() {
        if b == b' ' {
            out.push('_');
        } else if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.' || b == b'~' {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

/// Fetch the wikitext of a single article. Returns an Err for HTTP errors
/// or empty bodies.
fn fetch_raw(page: &str) -> Result<String, String> {
    let url = raw_url(page);
    let body = ureq::get(&url)
        .set("User-Agent", "amar-canon-scraper/0.1 (Geir Isene)")
        .timeout(std::time::Duration::from_secs(15))
        .call()
        .map_err(|e| e.to_string())?
        .into_string()
        .map_err(|e| e.to_string())?;
    if body.trim().is_empty() {
        return Err("empty body".to_string());
    }
    Ok(body)
}

/// Walk a Category page and pull out every link to a member article.
/// MediaWiki renders Category pages with a flat list of `<a href="/index.php/PageName">…</a>`
/// inside `<div class="mw-category-group">`. We grep the HTML for those
/// links — simpler than parsing wikitext for category contents.
fn fetch_category(category: &str) -> Vec<String> {
    let url = category_url(category);
    let body = match ureq::get(&url)
        .set("User-Agent", "amar-canon-scraper/0.1 (Geir Isene)")
        .timeout(std::time::Duration::from_secs(15))
        .call()
        .and_then(|r| Ok(r.into_string()))
    {
        Ok(Ok(s)) => s,
        _ => return Vec::new(),
    };

    // Extract /index.php/PageName URLs that are NOT category-meta links.
    let re = regex::Regex::new(r#"href="/index\.php/([^":#?]+)""#).unwrap();
    let mut seen = std::collections::BTreeSet::new();
    for cap in re.captures_iter(&body) {
        let raw = &cap[1];
        // Skip MediaWiki's own structural links and any sub-category descents.
        if raw.starts_with("Category:") || raw.starts_with("Special:") || raw.starts_with("File:")
            || raw.starts_with("Help:")  || raw.starts_with("Talk:")
        {
            continue;
        }
        if raw == "Main_Page" {
            continue;
        }
        let name = url_decode(&raw.replace('_', " "));
        // Drop meta-pages that cross-reference into magic categories
        // (system docs, not spells). And drop the 181 "Legacy " duplicates
        // — those pages are pre-rewrite snapshots, kept on the wiki for
        // history but not part of current canon.
        if META_PAGES.contains(&name.as_str()) {
            continue;
        }
        if name.starts_with("Legacy ") {
            continue;
        }
        seen.insert(name);
    }
    seen.into_iter().collect()
}

/// MediaWiki keeps category-meta pages on the wiki (rules docs that
/// reference spells via See-Also lists). They show up in our HTML link
/// scrape but aren't entries — exclude them by name.
const META_PAGES: &[&str] = &[
    "Amar Lite",
    "Campaign tracker",
    "Combat",
    "Encounters",
    "Equipment",
    "Experimental Systems",
    "GM's Screen",
    "Incantation Magic",
    "Magick",
    "Movement and Weather",
    "Mythology",
    "The Character",
    "The Kingdom of Amar",
    "The World",
    "Advantages and Disadvantages",
    "Main Page",
    "Playable Races",
    "List of Magick Items",
    "Crystal of Spell Storing",
];

/// Tiny URL-decoder for the handful of percent-encoded chars that appear
/// in d6gaming.org page names (apostrophes, smart quotes). Not a full
/// urlencoding implementation — we only see %xx for ASCII chars and the
/// 3-byte UTF-8 right-single-quote (%E2%80%99).
fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i+1..i+3]).unwrap_or("");
            if let Ok(b) = u8::from_str_radix(hex, 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Parse a wikitext body into a structured Entry. d6gaming.org puts the
/// stat block of every spell / ritual / potion / monster inside a
/// `{{ infobox | label1 = X | data1 = Y | label2 = ... | data2 = ... }}`
/// template, so we extract that template and pair `labelN` with `dataN`
/// for the canonical field set.
fn parse_entry(page: &str, wikitext: &str) -> Entry {
    use regex::Regex;
    let mut fields: BTreeMap<String, String> = BTreeMap::new();
    let mut domains: Vec<String> = Vec::new();

    if let Some(infobox) = extract_infobox(wikitext) {
        // Within the infobox body, each line is `| key = value`. Grab them
        // all into a flat map keyed by the raw `labelN` / `dataN` names,
        // then pair them up.
        let re_kv = Regex::new(r"(?m)^\s*\|\s*([A-Za-z][A-Za-z0-9 _/-]*?)\s*=\s*(.*?)\s*$").unwrap();
        let mut raw: BTreeMap<String, String> = BTreeMap::new();
        for cap in re_kv.captures_iter(&infobox) {
            let k = cap[1].trim().to_lowercase().replace(' ', "_");
            let v = strip_wiki_markup(&cap[2]).trim().to_string();
            raw.insert(k, v);
        }
        // Pair labelN with dataN. N runs 1..=20 in practice on this wiki.
        for n in 1..=20 {
            let lk = format!("label{}", n);
            let dk = format!("data{}", n);
            if let (Some(label), Some(value)) = (raw.get(&lk), raw.get(&dk)) {
                if !label.is_empty() && !value.is_empty() {
                    let key = label.to_lowercase()
                        .replace(' ', "_")
                        .replace('/', "_")
                        .replace('-', "_");
                    fields.insert(key, value.clone());
                }
            }
        }
        // The `above = Name` field carries the canonical display name.
        if let Some(above) = raw.get("above") {
            if !above.is_empty() {
                fields.insert("display_name".to_string(), above.clone());
            }
        }
    }

    // Domain detection: pages list `[[Category:Fire_Magick]]` etc. Pull
    // them all so a multi-domain spell carries every tag.
    let re_cat = Regex::new(r"\[\[Category:([^\]]+)\]\]").unwrap();
    for cap in re_cat.captures_iter(wikitext) {
        let tag = cap[1].trim().to_string();
        if tag.ends_with("_Magick") || tag == "Spells" || tag == "Rituals" || tag == "Potions" {
            if !domains.contains(&tag) {
                domains.push(tag);
            }
        }
    }

    // Pull a 1-2 sentence description out of the body (anything between
    // the closing `}}` of the infobox and the first `==` heading).
    let body_blurb = body_after_infobox(wikitext);
    if !body_blurb.is_empty() {
        fields.insert("description".to_string(), body_blurb);
    }

    // Ingredients section (rituals + potions): bullet list under
    // `=== Ingredients ===`. Stored newline-joined; the Lore renderer
    // re-bullets them.
    let re_ing = Regex::new(r"(?s)===\s*Ingredients\s*===\s*\n(.*?)(?:\n\n|\nBack:|\n\[\[Category)").unwrap();
    if let Some(cap) = re_ing.captures(wikitext) {
        let items: Vec<String> = cap[1].lines()
            .filter(|l| l.starts_with("* "))
            .map(|l| l[2..].trim().to_string())
            .collect();
        if !items.is_empty() {
            fields.insert("ingredients".to_string(), items.join("\n"));
        }
    }

    let details = if fields.is_empty() && wikitext.trim().len() < 64 {
        "missing".to_string()
    } else {
        "full".to_string()
    };

    Entry {
        name: page.to_string(),
        url: page_url(page),
        details,
        domains,
        fields,
    }
}

/// Pull the contents of the first `{{ infobox … }}` template, balanced
/// against nested `{{ … }}` so a `{{Note|…}}` inside the infobox doesn't
/// terminate the outer template prematurely.
fn extract_infobox(wikitext: &str) -> Option<String> {
    let lower = wikitext.to_lowercase();
    let start = lower.find("{{ infobox").or_else(|| lower.find("{{infobox"))?;
    // Walk character-by-character from the start, tracking `{{ }}` depth.
    let bytes = wikitext.as_bytes();
    let mut depth = 0i32;
    let mut i = start;
    let mut content_start = 0usize;
    let mut content_end = 0usize;
    while i + 1 < bytes.len() {
        if bytes[i] == b'{' && bytes[i+1] == b'{' {
            depth += 1;
            if depth == 1 {
                content_start = i + 2;
            }
            i += 2;
            continue;
        }
        if bytes[i] == b'}' && bytes[i+1] == b'}' {
            depth -= 1;
            if depth == 0 {
                content_end = i;
                break;
            }
            i += 2;
            continue;
        }
        i += 1;
    }
    if content_end > content_start {
        Some(wikitext[content_start..content_end].to_string())
    } else {
        None
    }
}

/// Take the prose between the infobox and the first `==` heading. This
/// is usually the spell's flavour description ("A tiny ball of flame
/// thrown from the hand …") — useful for Lore display and for AI
/// context.
fn body_after_infobox(wikitext: &str) -> String {
    let after = match wikitext.find("}}") {
        Some(idx) => &wikitext[idx+2..],
        None => wikitext,
    };
    // Truncate at first `==` heading.
    let body = match after.find("\n==") {
        Some(idx) => &after[..idx],
        None => after,
    };
    let cleaned = strip_wiki_markup(body).trim().to_string();
    // Cap at ~300 chars so a chatty wiki page doesn't bloat the canon.
    if cleaned.chars().count() > 300 {
        let mut s: String = cleaned.chars().take(300).collect();
        s.push('…');
        s
    } else {
        cleaned
    }
}

/// Strip the most common wikitext markup so a "Field: '''value'''" line
/// parses cleanly. Conservative — we leave unfamiliar markup alone.
fn strip_wiki_markup(line: &str) -> String {
    let mut s = line.to_string();
    // Bold / italic
    s = s.replace("'''", "").replace("''", "");
    // Internal links: [[X|Y]] → Y, [[X]] → X
    let re_link = regex::Regex::new(r"\[\[(?:[^\]|]+\|)?([^\]]+)\]\]").unwrap();
    s = re_link.replace_all(&s, "$1").to_string();
    // External links: [http://… text] → text
    let re_ext = regex::Regex::new(r"\[https?://[^\s\]]+\s+([^\]]+)\]").unwrap();
    s = re_ext.replace_all(&s, "$1").to_string();
    // Templates: {{X|Y}} → "" (drop entirely; we capture domain via category)
    let re_tmpl = regex::Regex::new(r"\{\{[^}]+\}\}").unwrap();
    s = re_tmpl.replace_all(&s, "").to_string();
    s
}

#[derive(Default, serde::Serialize)]
struct Canon {
    /// Map from category name (Spells, Rituals, …) → list of page names.
    domain_index: BTreeMap<String, Vec<String>>,
    /// Map from page name → parsed entry.
    entries: BTreeMap<String, Entry>,
}

#[derive(serde::Serialize)]
struct Entry {
    name: String,
    url: String,
    /// "full" if we got a body with fields, "missing" if just a name in a category list.
    details: String,
    domains: Vec<String>,
    fields: BTreeMap<String, String>,
}
