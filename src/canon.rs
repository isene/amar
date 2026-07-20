//! Loader for `data/canon.toml` — the d6gaming.org-derived rule data.
//!
//! Bundled into the binary via `include_str!` so amar runs with no
//! disk reads after startup. One-shot parse cost is ~5 ms on a laptop.

use std::collections::BTreeMap;

const CANON_TOML: &str = include_str!("../data/canon.toml");

#[derive(Debug, serde::Deserialize)]
pub struct Canon {
    pub domain_index: BTreeMap<String, Vec<String>>,
    pub entries: BTreeMap<String, Entry>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Entry {
    pub name: String,
    pub url: String,
    pub details: String,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default)]
    pub fields: BTreeMap<String, String>,
}

impl Canon {
    pub fn load() -> Self {
        toml::from_str(CANON_TOML).expect("canon.toml is malformed - regenerate via scrape_canon")
    }

    pub fn lookup(&self, name: &str) -> Option<&Entry> {
        self.entries.get(name)
    }

    pub fn category(&self, cat: &str) -> &[String] {
        self.domain_index.get(cat).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn spell_count(&self) -> usize { self.category("Spells").len() }
    pub fn ritual_count(&self) -> usize { self.category("Rituals").len() }
    pub fn potion_count(&self) -> usize { self.category("Potions").len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canon_loads() {
        let c = Canon::load();
        assert!(c.entries.len() > 150, "entries: {}", c.entries.len());
        assert_eq!(c.spell_count(), 101);
        assert_eq!(c.ritual_count(), 23);
        assert_eq!(c.potion_count(), 21);
    }

    #[test]
    fn fireball_has_full_fields() {
        let c = Canon::load();
        let fb = c.lookup("Fireball").expect("Fireball missing");
        assert_eq!(fb.fields.get("dr").map(String::as_str), Some("11"));
        assert_eq!(fb.fields.get("encumbrance").map(String::as_str), Some("5"));
        assert_eq!(fb.fields.get("domain").map(String::as_str), Some("Fire"));
    }
}
