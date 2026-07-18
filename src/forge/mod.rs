//! Forge generators ported from Amar-Tools.
//!
//! Phase 1 (current):
//!   - Open-ended d6 roller (one-shot wrapper around `dice::o6`)
//!   - Weather generator (single day or full month, calendar-aware)
//!   - Name generator (16 categories from the original Amar-Tools)
//!
//! Phase 2 (this revision):
//!   - NPC generator (`npc::build_npc`)  — full 3-tier NPC produced
//!     faithfully against the Amar-Tools rule set: chartype + race
//!     templates, tier-scaled bases, weapon + armor selection.
//!   - Encounter generator (`encounter::build_encounter`) — terrain
//!     and time-of-day driven rolls producing N populated NPCs.
//!
//! Phase 3 (queued): Town generator + relations map.

pub mod data;
pub mod npc;
pub mod monster;
pub mod encounter;
pub mod town;

use crate::calendar::{AmarDate, MONTHS, DAYS_PER_MONTH};
use crate::dice::{Rng, StdRng, o6};

// ---------------------------------------------------------------- O6 roll

pub struct RollResult {
    pub total: i32,
    pub sequence: Vec<u8>,
    pub outcome: &'static str,
}

pub fn roll_o6() -> RollResult {
    let mut rng = StdRng::from_time();
    let r = o6(&mut rng);
    let outcome = match r.outcome {
        crate::dice::Outcome::Critical => "CRITICAL",
        crate::dice::Outcome::Fumble   => "FUMBLE",
        crate::dice::Outcome::Normal   => "normal",
    };
    RollResult { total: r.total, sequence: r.sequence, outcome }
}

// ---------------------------------------------------------------- Weather

const WEATHER: [&str; 21] = [
    "Weird weather",
    "Clear skies",
    "Mainly clear",
    "Partly cloudy",
    "Mainly cloudy",
    "Partly cloudy, some fog",
    "Cloudy, but lucid",
    "Cloudy",
    "Cloudy and gray",
    "Fog",
    "Misty and overcast",
    "Partly cloudy, a bit of rain",
    "Partly cloudy, fog and some rain",
    "Partly cloudy, possible lightning",
    "Partly cloudy, possible lightning, rain",
    "Cloudy with some rain",
    "Cloudy and rainy",
    "Cloudy with heavy rain",
    "Cloudy with possible lightning",
    "Cloudy and rainy with lightning",
    "Cloudy, heavy rain, thunderstorm",
];

const WIND_STR: [&str; 4] = ["No wind", "Soft wind", "Windy", "Very windy"];
const WIND_DIR: [&str; 8] = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WeatherDay {
    pub date: AmarDate,
    pub weather: u8,
    pub wind_str: u8,
    pub wind_dir: u8,
    /// Feast / notable-day name. Owned String so serde can round-trip
    /// when a weather batch is saved into a campaign; comes from the
    /// static `special_for(month, day)` table at generation time.
    pub special: String,
}

impl WeatherDay {
    pub fn weather_text(&self) -> &'static str {
        WEATHER.get(self.weather as usize).copied().unwrap_or("?")
    }
    pub fn wind_text(&self) -> String {
        if self.wind_str == 0 { "No wind".to_string() }
        else { format!("{}, from the {}",
            WIND_STR[self.wind_str as usize],
            WIND_DIR[self.wind_dir as usize]) }
    }

    /// Pictograph for the day's sky. All entries are non-BMP emoji
    /// (or U+26C5 which is in crust's wide-BMP list) so cell_width
    /// reliably returns 2 — keeping table columns aligned.
    pub fn weather_emoji(&self) -> &'static str {
        weather_emoji(self.weather)
    }

    /// xterm-256 colour for the sky tone. Tuned to read at a glance:
    /// sunny = gold, cloudy = slate / grey, rain = blue, lightning =
    /// violet, freak weather = magenta.
    pub fn weather_color(&self) -> u8 {
        weather_color(self.weather)
    }

    /// Direction the air is *blowing toward*. The textual `wind_text`
    /// names the direction it comes FROM ("from the N"); a glyph that
    /// points the same way would mislead, so we invert: wind FROM N
    /// renders as ↓ (the air's heading south).
    pub fn wind_arrow(&self) -> &'static str {
        if self.wind_str == 0 { " " } else { WIND_ARROW[self.wind_dir as usize] }
    }

    /// Colour for the wind cell. Calm → muted, gusts → progressively
    /// warmer so a Very Windy day visually pops in the monthly grid.
    pub fn wind_color(&self) -> u8 {
        match self.wind_str {
            0 => 244,
            1 => 108,
            2 => 67,
            _ => 174,
        }
    }
}

pub fn weather_emoji(idx: u8) -> &'static str {
    match idx {
        0       => "🌀",  // weird weather (cyclone, U+1F300)
        1       => "🌞",  // clear skies (sun with face, U+1F31E)
        2       => "🌤\u{FE0F}", // mainly clear (sun behind small cloud, U+1F324)
        3       => "⛅",  // partly cloudy (U+26C5 — in crust's wide-BMP list)
        4       => "🌥\u{FE0F}", // mainly cloudy (U+1F325)
        5       => "🌫\u{FE0F}", // partly cloudy + some fog
        6 | 7 | 8 => "🌥\u{FE0F}", // cloudy variants
        9 | 10  => "🌫\u{FE0F}", // fog / mist / overcast
        11 | 12 => "🌦\u{FE0F}", // partly cloudy + rain
        13 | 14 => "🌩\u{FE0F}", // partly cloudy + lightning (± rain)
        15 | 16 | 17 => "🌧\u{FE0F}", // cloudy + rain (light → heavy)
        18 | 19 | 20 => "🌩\u{FE0F}", // cloudy + lightning / thunderstorm
        _ => "  ",
    }
}

pub fn weather_color(idx: u8) -> u8 {
    match idx {
        0       => 213,                       // freak weather: magenta
        1 | 2   => 220,                       // sunny: warm gold
        3       => 117,                       // partly cloudy: sky blue
        4 | 6 | 7 | 8 => 109,                 // cloudy: slate
        5 | 9 | 10  => 245,                   // fog / mist: muted grey
        11 | 12 => 67,                        // light rain: steel
        13 | 14 => 141,                       // partly + lightning: violet
        15 | 16 | 17 => 39,                   // rain: deep blue
        18 | 19 | 20 => 141,                  // storm: violet
        _ => 252,
    }
}

/// Arrows ordered to match WIND_DIR (N, NE, E, SE, S, SW, W, NW).
/// Each one points the direction the wind is *blowing toward* — i.e.
/// opposite the meteorological "from" name, so "from the N" → ↓.
pub const WIND_ARROW: [&str; 8] = ["↓", "↙", "←", "↖", "↑", "↗", "→", "↘"];

/// Step weather + wind one day forward following the same cascade
/// rules Amar-Tools used (oD6 deltas with month modifiers).
fn weather_step(
    rng: &mut impl Rng,
    mut weather: i32, mut wind_dir: i32, mut wind_str: i32,
    month: u32, day_of_month: u32,
) -> (i32, i32, i32, &'static str) {
    let od6 = || -> i32 { o6(&mut StdRng::from_time()).total };
    // We need fresh rng each step; o6() advances rng state — simulate
    // here by generating a couple of inline rolls using `rng`.
    let oroll = |rng: &mut dyn Rng| -> i32 {
        let mut wrapper = RngWrap { inner: rng };
        o6(&mut wrapper).total
    };
    let _ = od6;
    if month == 6 { weather += oroll(rng) - 4; }              // Juba: warmer/clearer
    if rng.d6() <= 3 {                                          // 50% jitter
        weather = ((weather + oroll(rng) + oroll(rng) - 7).abs()) % 41;
        if month == 1  && rng.d6() == 1 { weather += 4; }      // Walmaer
        if month == 7                    { weather -= 1; }     // Taroc
        if month == 8  && (rng.d6() % 3) == 0 { weather -= 4; } // Man Peggon
        if month == 13 && rng.d6() <= 3 { weather += 4; }      // Mestronorpha
        if weather > 20 { weather = 40 - weather; }
    }
    // (Elemental holy-day weather is applied at the end, driven by the
    // calendar's holy-day table so the two can never drift apart.)
    if month == 6 { wind_dir = ((wind_dir + (oroll(rng) + oroll(rng) - 7) / 3) % 8 + 8) % 8; }
    if rng.d6() <= 3 {
        wind_dir = ((wind_dir + (oroll(rng) + oroll(rng) - 7) / 3) % 8 + 8) % 8;
    }
    if month == 6 { wind_str = (wind_str + (oroll(rng) + oroll(rng) - 5) / 6).abs(); }
    if rng.d6() <= 3 {
        wind_str = (wind_str + (oroll(rng) + oroll(rng) - 7) / 6).abs();
        if month == 1  && (rng.d6() % 3) == 0 { wind_str += 1; }    // Walmaer
        if month == 10 && rng.d6() <= 3 { wind_str += 1; }          // Fal Munir
    }
    if wind_str < 0 { wind_str = 0; }
    if wind_str > 3 { wind_str = 3; }
    // Elemental holy days bend the sky to the god's element. Dates come
    // straight from the calendar's holy-day table (special_day), so weather
    // and the calendar always agree — no more day-21-vs-22 drift.
    if let Some(sp) = crate::calendar::special_day(month, day_of_month) {
        match sp.god {
            "Walmaer"  => weather = 15,                     // Water → steady rain
            "Ikalio"   => weather = 1,                      // Fire → clear & sunny
            "Shalissa" => { weather = 2; wind_str = 3; }    // Wind → clear but very windy
            "Alesia"   => weather = 8,                      // Earth → heavy, overcast
            "Ielina"   => { weather = 2; wind_str = 0; }    // Moon → clear & calm
            _ => {}
        }
    }
    if weather < 1 { weather = 1; }
    let special = special_for(month, day_of_month);
    (weather, wind_dir, wind_str, special)
}

/// `Rng`-trait impl over a `&mut dyn Rng` so the closures above can
/// thread the RNG without taking it by value.
struct RngWrap<'a> { inner: &'a mut dyn Rng }
impl<'a> Rng for RngWrap<'a> {
    fn d6(&mut self) -> u8 { self.inner.d6() }
}

fn special_for(month: u32, day_of_month: u32) -> &'static str {
    match (month, day_of_month) {
        (1, 1)  => "Walmaer (king's day)",
        (1, 9)  => "Cal Amae",
        (2, 2)  => "Elesi",
        (3, 4)  => "Anashina",
        (3, 15) => "Ish Nakil",
        (3, 18) => "Fenimaal",
        (3, 21) => "Fionella",
        (4, 8)  => "Alesia",
        (4, 12) => "Gwendyll",
        (5, 13) => "MacGillan",
        (6, 10) => "Juba (Ielina day)",
        (7, 11) => "Taroc",
        (7, 15) => "Ikalio",
        (8, 4)  => "Man Peggon",
        (9, 1)  => "Maleko",
        (10, 7) => "Fal Munir",
        (10, 22)=> "Shalissa",
        (11, 3) => "Moltan",
        (12, 8) => "Kraagh",
        (13, 28)=> "Mestronorpha (Ielina day)",
        _ => "",
    }
}

/// Generate weather for `n_days` consecutive days starting at `start`.
/// First-day seed values are rolled cleanly (no carry).
pub fn generate_weather(start: AmarDate, n_days: u32) -> Vec<WeatherDay> {
    let mut rng = StdRng::from_time();
    let mut date = start;
    let mut weather = (rng.d6() as i32 + rng.d6() as i32 + rng.d6() as i32) / 3;
    let mut wind_dir = (rng.d6() as i32 - 1) % 8;
    let mut wind_str = (rng.d6() as i32 - 1) / 2;
    let mut out = Vec::with_capacity(n_days as usize);
    for _ in 0..n_days {
        let (m, d) = (date.month(), date.day_of_month());
        let (w, wd, ws, sp) = weather_step(&mut rng, weather, wind_dir, wind_str, m, d);
        weather = w; wind_dir = wd; wind_str = ws;
        out.push(WeatherDay {
            date,
            weather: weather.clamp(0, 20) as u8,
            wind_str: wind_str.clamp(0, 3) as u8,
            wind_dir: wind_dir.rem_euclid(8) as u8,
            special: sp.to_string(),
        });
        date = date.advance(1);
    }
    out
}

pub fn month_name(month: u32) -> &'static str {
    let i = month.saturating_sub(1) as usize;
    MONTHS.get(i).copied().unwrap_or("?")
}

pub fn days_in_month() -> u32 { DAYS_PER_MONTH }

// ---------------------------------------------------------------- Names

/// Ordered list of name categories. The data file is bundled at
/// compile time so the binary is self-contained (no runtime file I/O).
pub const NAME_CATEGORIES: &[(&str, &[&str], &[&str])] = &[
    ("Human male",      &[FIRST_HUMAN_MALE],   &[LAST_HUMAN]),
    ("Human female",    &[FIRST_HUMAN_FEMALE], &[LAST_HUMAN]),
    ("Dwarven male",    &[DWARVEN_MALE],   &[]),
    ("Dwarven female",  &[DWARVEN_FEMALE], &[]),
    ("Elven male",      &[ELVEN_MALE],   &[]),
    ("Elven female",    &[ELVEN_FEMALE], &[]),
    ("Lizardfolk",      &[LIZARDFOLK],   &[]),
    ("Troll",           &[TROLL],        &[]),
    ("Araxi",           &[ARAXI],        &[]),
    ("Generic male",    &[FANTASY_MALE],   &[]),
    ("Generic female",  &[FANTASY_FEMALE], &[]),
    ("Castle",          &[CASTLE],   &[]),
    ("Town/Village",    &[TOWN],     &[]),
    ("City",            &[CITY],     &[]),
    ("Weapon",          &[WEAPON],   &[]),
];

// Bundled name files — relative to crate root.
// Human pools also published as `HUMAN_*` so the town generator can
// give every resident a name without re-routing through generate_names
// (which is geared toward picking N at once with its own RNG seed).
const FIRST_HUMAN_MALE:   &str = include_str!("../../data/names/human_male_first.txt");
const FIRST_HUMAN_FEMALE: &str = include_str!("../../data/names/human_female_first.txt");
const LAST_HUMAN:         &str = include_str!("../../data/names/human_last.txt");
pub const HUMAN_MALE_FIRST:   &str = FIRST_HUMAN_MALE;
pub const HUMAN_FEMALE_FIRST: &str = FIRST_HUMAN_FEMALE;
pub const HUMAN_LAST:         &str = LAST_HUMAN;
const DWARVEN_MALE:       &str = include_str!("../../data/names/dwarven_male.txt");
const DWARVEN_FEMALE:     &str = include_str!("../../data/names/dwarven_female.txt");
const ELVEN_MALE:         &str = include_str!("../../data/names/elven_male.txt");
const ELVEN_FEMALE:       &str = include_str!("../../data/names/elven_female.txt");
const LIZARDFOLK:         &str = include_str!("../../data/names/lizardfolk.txt");
const TROLL:              &str = include_str!("../../data/names/troll.txt");
const ARAXI:              &str = include_str!("../../data/names/araxi.txt");
const FANTASY_MALE:       &str = include_str!("../../data/names/fantasy_male.txt");
const FANTASY_FEMALE:     &str = include_str!("../../data/names/fantasy_female.txt");
const CASTLE:             &str = include_str!("../../data/names/castle_names.txt");
const TOWN:               &str = include_str!("../../data/names/town_names.txt");
const CITY:               &str = include_str!("../../data/names/city_names.txt");
const WEAPON:             &str = include_str!("../../data/names/weapon_names.txt");

/// Pick `n` names from category `idx`. When the category has both a
/// first-name and last-name list, joins one of each per row (so the
/// human categories produce "First Last" pairs).
pub fn generate_names(idx: usize, n: usize) -> Vec<String> {
    let Some((_, firsts, lasts)) = NAME_CATEGORIES.get(idx) else { return Vec::new(); };
    let mut rng = StdRng::from_time();
    let mut out = Vec::with_capacity(n);
    let first_list = firsts.first().copied().unwrap_or("");
    let last_list  = lasts.first().copied().unwrap_or("");
    let firsts_v: Vec<&str> = first_list.lines().filter(|s| !s.trim().is_empty()).collect();
    let lasts_v:  Vec<&str> = last_list.lines().filter(|s| !s.trim().is_empty()).collect();
    for _ in 0..n {
        if firsts_v.is_empty() { continue; }
        // mod the rng output into the index range
        let mut idx_pick = || -> usize {
            let mut r: u64 = 0;
            for _ in 0..4 { r = (r << 8) | (rng.d6() as u64 * 41); }
            r as usize
        };
        let f = firsts_v[idx_pick() % firsts_v.len()].trim();
        if lasts_v.is_empty() {
            out.push(f.to_string());
        } else {
            let l = lasts_v[idx_pick() % lasts_v.len()].trim();
            out.push(format!("{} {}", f, l));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_for_month_advances_28_days() {
        let start = AmarDate::from_ymd(354, 1, 1);
        let days = generate_weather(start, 28);
        assert_eq!(days.len(), 28);
        // First and last day's calendar should differ correctly.
        assert_eq!(days[0].date.day_of_month(), 1);
        assert_eq!(days[27].date.day_of_month(), 28);
    }

    #[test]
    fn elemental_holy_days_bend_the_weather() {
        // Walmaer day (1 Cal Amae) — Water god → steady rain.
        let w = &generate_weather(AmarDate::from_ymd(354, 1, 1), 1)[0];
        assert_eq!(w.weather, 15, "Walmaer day should rain");
        assert_eq!(w.special, "Walmaer (king's day)");

        // Ikalio day (15 Taroc) — Fire god → clear & sunny.
        let f = &generate_weather(AmarDate::from_ymd(354, 7, 15), 1)[0];
        assert_eq!(f.weather, 1, "Ikalio day should be sunny");

        // Shalissa day (22 Fal Munir) — Wind god → very windy. Its date
        // now comes from the calendar table (day 22), not the old day 21.
        let s = &generate_weather(AmarDate::from_ymd(354, 10, 22), 1)[0];
        assert_eq!(s.wind_str, 3, "Shalissa day should be very windy");
        assert_eq!(s.special, "Shalissa");
    }

    #[test]
    fn names_human_male_produces_first_and_last() {
        let v = generate_names(0, 5);
        assert_eq!(v.len(), 5);
        for n in &v {
            assert!(n.contains(' '), "human name should be 'First Last': {}", n);
        }
    }

    #[test]
    fn names_castle_is_single_token_or_phrase() {
        let v = generate_names(11, 3);
        assert_eq!(v.len(), 3);
        // Castle names may contain spaces ("Castle of …") but always
        // pull from a single list, so they're not first+last joined.
    }
}
