//! amar — Amar RPG companion (Fe2O3 suite).
//!
//! v0.1.0 in progress. This file currently holds only a placeholder so the
//! crate builds; the TUI, tabs, and Forge / Session / Campaign / Lore /
//! Inspire modes will land in subsequent commits once the canon (data
//! pulled from d6gaming.org via src/bin/scrape_canon.rs) is locked in.

fn main() {
    eprintln!("amar v{} — Amar RPG companion (TUI not yet wired up).", env!("CARGO_PKG_VERSION"));
    eprintln!("Run `cargo run --features scraper --bin scrape_canon` to (re)generate data/canon.toml.");
}
