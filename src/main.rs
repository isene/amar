//! amar — Amar RPG companion (Fe2O3 suite).

#![allow(dead_code)]  // PC sheet edit / Forge generators / Inspire prompts populate dead code in v0.2+.

mod app;
mod calendar;
mod canon;
mod dice;
mod pc;
mod store;

fn main() {
    crust::Crust::set_app_identity("Amar");
    let mut app = app::App::new();
    app.run();
}
