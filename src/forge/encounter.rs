//! Encounter generator — port of Amar-Tools' `class_enc_new.rb`.
//!
//! Flow (matches Amar-Tools):
//!   1. Caller supplies a terrain (City/Rural/Road/Plains/Hills/Mountain/
//!      Woods/Wilderness) and a day/night flag → `terraintype` index.
//!   2. Pick an encounter category from `ENC_TYPE` weights for that
//!      terrain (NO ENCOUNTER, smallanimal, largeanimal, human, dwarf,
//!      elf, araxi, monster, event).
//!   3. Pick a specific entry from `ENC_SPECIFIC[category]` (e.g.
//!      "Human: Warrior", "Monster: Troll (large)").
//!   4. Roll the encounter count (1, d3, d6, 2d6, 3d6 by oD6 cascade).
//!   5. Roll attitude (Hostile / Antagonistic / Neutral / Positive
//!      / Friendly).
//!   6. For each spot, build an NPC via `npc::build_npc` against a
//!      chartype derived from the encounter spec.

use serde::{Deserialize, Serialize};

use crate::dice::{Rng, StdRng, o6};
use crate::forge::data::{ENC_TYPE, ENC_SPECIFIC, TERRAIN_NAMES};
use crate::forge::{monster, npc};
use crate::pc::Character;

/// Aggregated result of a single encounter roll. Serializable so a
/// rolled encounter can be saved into the campaign's `saved_encounters`
/// vector and round-tripped through JSON without losing the attitude
/// or the NPC stat blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Encounter {
    pub terrain_idx: usize,
    pub day: bool,
    pub category: String,    // "human", "monster", "NO ENCOUNTER", …
    pub spec: String,        // "Human: Warrior", "Monster: Troll (large)", …
    pub count: u32,
    /// Attitude is stored as String so it survives serde — the random
    /// table only ever produces one of `HOSTILE / ANTAGONISTIC /
    /// NEUTRAL / POSITIVE / FRIENDLY / —`, but the field is open in
    /// case future tables grow more.
    pub attitude: String,
    /// Generated NPCs. Empty when `category == "NO ENCOUNTER"` or when
    /// the spec was an Event (no NPCs are summoned for events).
    pub npcs: Vec<Character>,
}

impl Encounter {
    pub fn terrain_name(&self) -> &'static str {
        TERRAIN_NAMES.get(self.terrain_idx).copied().unwrap_or("?")
    }
    pub fn time_of_day(&self) -> &'static str {
        if self.day { "day" } else { "night" }
    }
    pub fn is_no_encounter(&self) -> bool {
        self.category == "NO ENCOUNTER"
    }
    pub fn is_event(&self) -> bool {
        self.spec.starts_with("Event:")
    }
}

/// Build an encounter for the given terrain (0..=7) and day flag
/// (`true` = day, `false` = night). `level_mod` is added to every
/// generated NPC's level (use 0 for "natural").
pub fn build_encounter(terrain_idx: usize, day: bool, level_mod: i32) -> Encounter {
    let mut rng = StdRng::from_time();
    build_encounter_seeded(terrain_idx, day, level_mod, &mut rng)
}

pub fn build_encounter_seeded(
    terrain_idx: usize, day: bool, level_mod: i32, rng: &mut impl Rng,
) -> Encounter {
    let terrain = terrain_idx.min(7);
    let col = terrain + if day { 8 } else { 0 };

    let category: String = pick_weighted(rng, ENC_TYPE, col, "NO ENCOUNTER");
    if category == "NO ENCOUNTER" {
        return Encounter {
            terrain_idx: terrain,
            day,
            category: "NO ENCOUNTER".into(),
            spec: "—".into(),
            count: 0,
            attitude: "—".into(),
            npcs: Vec::new(),
        };
    }

    let spec = ENC_SPECIFIC.iter()
        .find(|(c, _)| *c == category)
        .map(|(_, rows)| pick_weighted(rng, *rows, col, &category))
        .unwrap_or_else(|| category.clone());

    // Roll the count.
    let r = oroll(rng);
    let mut count: u32 = match r {
        i32::MIN..=3 => 1,
        4 => (rng.d6() % 3 + 1) as u32,
        5 => (rng.d6() + 1) as u32,
        6..=7 => 2 * (rng.d6() + 1) as u32,
        _ => 3 * (rng.d6() + 1) as u32,
    };
    if count > 5 && spec.contains("onster") { count = 5; }

    let attitude: String = match rng.d6() {
        1 => "HOSTILE",
        2 => "ANTAGONISTIC",
        3 | 4 => "NEUTRAL",
        5 => "POSITIVE",
        _ => "FRIENDLY",
    }.into();

    // Events don't produce NPCs.
    if spec.starts_with("Event:") {
        return Encounter {
            terrain_idx: terrain, day,
            category: category.into(), spec, count,
            attitude, npcs: Vec::new(),
        };
    }

    // Build NPCs. Monsters and animals route to the EncStats-driven
    // `monster::build_monster` (which uses the encounter stat block
    // directly — wiki-faithful for non-humanoids); humanoids route to
    // the chartype-template-driven `npc::build_npc_seeded`.
    let is_monster = spec.starts_with("Monster:")
        || spec.starts_with("Small animal")
        || spec.starts_with("Large animal");
    let chartype = chartype_for_spec(&spec);
    // Per Amar-Tools class_enc.rb: Elves and Faeries are older / more
    // experienced than the d6-rolled "natural" level, so the wiki
    // bumps their encounter level by 2. Same race detection used by
    // chartype_for_spec — the prefix on the spec string.
    let race_level_bonus: i32 = if spec.starts_with("Elf:")
        || spec.starts_with("Faerie:") { 2 } else { 0 };
    let mut npcs = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let level = (rng.d6() as i32 % 3 + 1 + level_mod + race_level_bonus).max(1) as u8;
        let npc = if is_monster {
            monster::build_monster(&spec, level, rng)
        } else {
            let sex = if rng.d6() % 2 == 0 { "M" } else { "F" };
            npc::build_npc_seeded(chartype, level, sex, rng)
        };
        npcs.push(npc);
    }

    Encounter {
        terrain_idx: terrain, day,
        category: category.into(),
        spec, count,
        attitude, npcs,
    }
}

// ---------------------------------------------------------------- helpers

fn oroll(rng: &mut impl Rng) -> i32 {
    o6(rng).total
}

/// Weighted pick from a `[(name, [weights; 16])]` table, sampling
/// column `col`. Returns the matched name as `String`.
fn pick_weighted(
    rng: &mut impl Rng,
    rows: &[(&'static str, [u32; 16])],
    col: usize,
    fallback: &str,
) -> String {
    let total: u32 = rows.iter().map(|(_, w)| w[col]).sum();
    if total == 0 { return fallback.to_string(); }
    // Build a 0..total roll using a couple of d6's (we don't need
    // cryptographic distribution, just reasonable spread).
    let mut roll: u32 = 0;
    let mut span = 1;
    while span < total { roll = (roll << 4) ^ rng.d6() as u32; span <<= 4; }
    roll %= total;
    let mut acc = 0;
    for (name, w) in rows {
        acc += w[col];
        if roll < acc { return (*name).into(); }
    }
    rows.last().map(|(n, _)| (*n).into()).unwrap_or_else(|| fallback.into())
}

/// Map an encounter spec ("Human: Warrior", "Monster: Troll (large)",
/// "Elf: Archer", …) to a chartype name we can hand to
/// `npc::build_npc`. Falls back to "Commoner" for plain humans, or
/// "Warrior" if the profession looks combat-oriented.
fn chartype_for_spec(spec: &str) -> &'static str {
    // Race-prefixed encounters: "Race: Profession" → either an exact
    // template like "Elf: Warrior" or a profession-only fallback.
    if let Some((race, prof)) = spec.split_once(": ") {
        // Exact match first.
        if crate::forge::data::chartype(spec).is_some() {
            // Static lookup — return the literal name in the table so
            // we hand `npc::build_npc` the &'static str it expects.
            return crate::forge::data::CHARTYPES.iter()
                .find(|c| c.name == spec)
                .map(|c| c.name).unwrap_or("Commoner");
        }
        // Race-aware fallback: "Elf: Sailor" → "Elf: Warrior" if there
        // isn't a Sailor template, otherwise the bare profession.
        let race_warrior = format!("{}: Warrior", race);
        if crate::forge::data::chartype(&race_warrior).is_some() {
            // This relies on the static table containing the warrior
            // form for that race (Elf/Dwarf/Araxi all do).
            return crate::forge::data::CHARTYPES.iter()
                .find(|c| c.name == race_warrior)
                .map(|c| c.name).unwrap_or("Commoner");
        }
        // Plain profession lookup.
        return profession_to_chartype(prof);
    }
    profession_to_chartype(spec)
}

fn profession_to_chartype(prof: &str) -> &'static str {
    let p = prof.to_ascii_lowercase();
    // Order matters: more specific compound matches first
    // (e.g. "body guard" before "guard", "fine smith" before "smith").
    if p.contains("body guard") { "Body guard" }
    else if p.contains("army officer") { "Army officer" }
    else if p.contains("animal trainer") { "Animal trainer" }
    else if p.contains("armour smith") || p.contains("armor smith") { "Armour smith" }
    else if p.contains("fine smith") { "Fine smith" }
    else if p.contains("fine artist") { "Fine artist" }
    else if p.contains("crafts (fine)") { "Crafts (fine)" }
    else if p.contains("crafts (heavy)") { "Crafts (heavy)" }
    else if p.contains("high class") { "High class" }
    else if p.contains("house wife") { "House wife" }
    else if p.contains("sports contender") { "Sports contender" }
    else if p.contains("warrior") || p.contains("soldier") || p.contains("fighter")
        { "Warrior" }
    else if p.contains("archer") { "Archer" }
    else if p.contains("ranger") || p.contains("scout") { "Ranger" }
    else if p.contains("hunter") { "Hunter" }
    else if p.contains("tracker") { "Tracker" }
    else if p.contains("summoner") { "Summoner" }
    else if p.contains("seer") { "Seer" }
    else if p.contains("mage") || p.contains("wizard")  { "Wizard (fire)" }
    else if p.contains("sorcerer") || p.contains("witch black") { "Witch (black)" }
    else if p.contains("witch white") { "Witch (white)" }
    else if p.contains("thief") || p.contains("rogue") || p.contains("bandit")
        { "Thief" }
    else if p.contains("guard") || p.contains("watchman") { "Guard" }
    else if p.contains("clergyman") { "Clergyman" }
    else if p.contains("priest") || p.contains("cleric") { "Priest" }
    else if p.contains("monk") { "Monk" }
    else if p.contains("merchant") || p.contains("trader") { "Merchant" }
    else if p.contains("noble") || p.contains("lord") || p.contains("lady")
        { "Noble" }
    else if p.contains("bard") { "Bard" }
    else if p.contains("entertainer") { "Entertainer" }
    else if p.contains("prostitute") { "Prostitute" }
    else if p.contains("assassin") { "Assassin" }
    else if p.contains("executioner") { "Executioner" }
    else if p.contains("sage") { "Sage" }
    else if p.contains("scholar") { "Scholar" }
    else if p.contains("scribe") { "Scribe" }
    else if p.contains("bureaucrat") { "Bureaucrat" }
    else if p.contains("commoner") || p.contains("peasant") { "Commoner" }
    else if p.contains("farmer") { "Farmer" }
    else if p.contains("baker") || p.contains("cook") { "Baker/Cook" }
    else if p.contains("fisherman") { "Fisherman" }
    else if p.contains("sailor") { "Sailor" }
    else if p.contains("navigator") { "Navigator" }
    else if p.contains("boatbuilder") { "Boatbuilder" }
    else if p.contains("messenger") { "Messenger" }
    else if p.contains("mapmaker") { "Mapmaker" }
    else if p.contains("mason") { "Mason" }
    else if p.contains("carpenter") { "Carpenter" }
    else if p.contains("builder") { "Builder" }
    else if p.contains("jeweller") || p.contains("jeweler") { "Jeweller" }
    else if p.contains("tailor") { "Tailor" }
    else if p.contains("tanner") { "Tanner" }
    else if p.contains("nanny") { "Nanny" }
    else if p.contains("smith") { "Smith" }
    else if p.contains("highwayman") { "Highwayman" }
    else if p.contains("gladiator") { "Gladiator" }
    else if p.contains("barbarian") { "Barbarian" }
    else if p.contains("berserker") { "Berserker" }
    else if p.contains("monster") { "Warrior" }   // monsters use enc-stat path; fallback
    else if p.contains("animal")  { "Commoner" }  // ditto
    else { "Commoner" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn road_at_day_produces_some_encounter_or_none() {
        let e = build_encounter(2, true, 0); // Road, day
        // Either no encounter, an event (which has no NPCs), or
        // there are NPCs equal to count.
        if !e.is_no_encounter() && !e.is_event() {
            assert_eq!(e.npcs.len(), e.count as usize);
        }
    }

    #[test]
    fn no_encounter_has_zero_count_and_no_npcs() {
        // Force NO ENCOUNTER by using a terrain where it has the
        // largest weight (Wilderness day): index 7, day=true → col 15.
        let e = build_encounter(7, true, 0);
        if e.is_no_encounter() {
            assert_eq!(e.count, 0);
            assert!(e.npcs.is_empty());
        }
    }
}
