# amar — Amar RPG Companion

[![Rust](https://img.shields.io/badge/language-Rust-f74c00)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-Unlicense-green)](https://unlicense.org/)

Terminal companion for the **[Amar RPG](https://d6gaming.org)** — a 3-tier d6 system (O6) by Geir Isene. Five-tab TUI: live session tools, a Forge for NPC / encounter / town / weather generation, a persistent Campaign tracker, browsable Lore, and AI-assisted Inspire prompts. Built on [crust](https://github.com/isene/crust). Part of the [Fe₂O₃ Rust terminal suite](https://github.com/isene/fe2o3).

> **Status:** v0.1.0 in progress. The d6gaming.org canon is locked (187 entries). The TUI is being built next.

## Source-of-truth pipeline

All Amar mechanics — spells, rituals, potions, weapons, armor, formulas — come from **[d6gaming.org](https://d6gaming.org)**. The wiki is the source of truth; amar never invents canonical rules.

`scripts/` holds the canon scraper:

```bash
cargo run --release --features scraper --bin scrape_canon
```

Reads from d6gaming.org via `?action=raw` (the wiki's wikitext export), parses the `{{infobox}}` template of every spell / ritual / potion page, and writes `data/canon.toml`. Each entry carries its source URL so any value can be verified against the wiki by hand.

When the wiki is updated, re-run the scraper. The TOML is committed to the repo so amar runs without internet access.

## Three concentric circles of canon

1. **Wiki canon** — `data/canon.toml`, scraped from d6gaming.org. Verified, source-linked. The TUI never modifies this file.
2. **Author canon** — `data/death_spells.toml`, `data/magic_items.toml`, `data/monsters.toml` and similar. Hand-authored entries that fill gaps the wiki has not yet covered. Marked `source = "amar-author"` so they are visually distinct in the Lore tab.
3. **Campaign canon** — `~/.amar/campaigns/<name>/`. Per-user, per-campaign data: PCs, NPCs, locations, adventures, session logs, calendar.

## License

Public domain ([Unlicense](https://unlicense.org/)).
