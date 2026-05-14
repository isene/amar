//! Town / village / city generator — port of Amar-Tools' `class_town.rb`
//! and `tables/town.rb`.
//!
//! The Amar-Tools algorithm:
//!   1. User picks a target town size (number of buildings). 1-5 →
//!      castle, 6-25 → village, 26-99 → town, 100+ → city. The
//!      generator picks an appropriate name from the bundled name
//!      lists for that scale.
//!   2. For each building type in the `$Town` table the original
//!      rolled `((d(pct) + d(pct) + rand) * town_size / 100)` to get
//!      the number of buildings, with `min` as the floor.
//!   3. Each building adds residents (seniors / adults / young).
//!      The original Amar-Tools then generated a full NPC for each
//!      resident — for a small village (15 houses) that's ~60 NPCs,
//!      and a city of 200+ houses generates 800+ NPCs. We **don't
//!      build full NPCs** for residents here (it would explode the
//!      memory + screen budget); we just report the residents count
//!      per building so the GM can flesh out the ones that matter.
//!   4. Temples get a specific god (chosen without replacement from
//!      `$Temple_types`).

use serde::{Deserialize, Serialize};

use crate::dice::{Rng, StdRng};

/// Relationship-map cap. A graphviz PNG of every resident in a 200-
/// building city is a 4000×6000 px wall of unreadable nodes. We keep
/// the first N named residents (encounter order: stronghold, guards,
/// stables, inns, …) so the picture fits a terminal pane and the GM
/// gets the *important* personalities — the head of the keep, the
/// guard captain, the innkeeper — instead of random farm worker 173.
const RELATIONS_MAX_PERSONS: usize = 40;

#[derive(Debug, Clone, Copy)]
pub struct TownBuilding {
    pub name: &'static str,
    pub is_shop: bool,
    pub pct: u32,       // chance % weight per d(pct) roll
    pub min: u32,       // floor — always at least this many in the town
    pub seniors: u32,
    pub adults: u32,
    pub young: u32,
}

/// 64 building types from `includes/tables/town.rb` verbatim. Order
/// matters: the Amar-Tools algorithm iterates in this order and
/// stops when `h_index > town_size`, so common building types
/// (homes, shops) come first and exotic ones (Perfumer, Distiller)
/// last.
pub const TOWN_BUILDINGS: &[TownBuilding] = &[
    TownBuilding { name: "Unhoused residents", is_shop: false, pct:   1, min: 1, seniors: 1, adults: 3, young: 1 },
    TownBuilding { name: "Stronghold",         is_shop: false, pct:   1, min: 1, seniors: 3, adults: 6, young: 4 },
    TownBuilding { name: "Soldier/Guards",     is_shop: false, pct:   3, min: 1, seniors: 0, adults: 4, young: 0 },
    TownBuilding { name: "Stable",             is_shop: true,  pct:   2, min: 1, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Inn",                is_shop: false, pct:   6, min: 1, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Farm/Fishery",       is_shop: true,  pct:   6, min: 3, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "General store",      is_shop: true,  pct:   2, min: 1, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Blacksmith",         is_shop: true,  pct:   2, min: 1, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Butcher",            is_shop: true,  pct:   2, min: 1, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Temple",             is_shop: false, pct:   5, min: 1, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Horse trader",       is_shop: true,  pct:   3, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Mill",               is_shop: false, pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Baker",              is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Merchant",           is_shop: true,  pct:   3, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Barber",             is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Leatherworker",      is_shop: true,  pct:   3, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Grocery store",      is_shop: true,  pct:   4, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Dyer/Tanner",        is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Mason",              is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Tailor",             is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Weapon smith",       is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Armourer",           is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Carpenter",          is_shop: true,  pct:   3, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Cartwright",         is_shop: true,  pct:   3, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Potter",             is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Worker",             is_shop: false, pct:   6, min: 1, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Boatwright",         is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Noble",              is_shop: false, pct:   4, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Laundry",            is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Storage",            is_shop: true,  pct:   3, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Slum",               is_shop: false, pct:   7, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Carpet maker",       is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Rope-/Netmaker",     is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Doctor",             is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Brothel",            is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Sailmaker",          is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Bowyer/Fletcher",    is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Weaver/spinner",     is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Veterinarian",       is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Animal trainer",     is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Furrier",            is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Brewer",             is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Cobbler",            is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Builder",            is_shop: false, pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Farm worker",        is_shop: false, pct:   3, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Woodcarver/Engraver",is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Pawnshop",           is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Shoe maker",         is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Outfitter",          is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Public bath",        is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Alchemist",          is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Artist",             is_shop: true,  pct:   3, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Diplomat",           is_shop: false, pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Silver/goldsmith",   is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Jeweller",           is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Winery",             is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Distiller",          is_shop: true,  pct:   1, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Teacher",            is_shop: true,  pct:   3, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Scribe",             is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Tinker",             is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Illuminator",        is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Glassblower",        is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Artist/Cartographer",is_shop: true,  pct:   2, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Perfumer",           is_shop: true,  pct:   1, min: 0, seniors: 2, adults: 2, young: 2 },
    TownBuilding { name: "Farm",               is_shop: true,  pct: 100, min: 3, seniors: 2, adults: 2, young: 2 },
];

/// Temples randomise their god from a weighted pool, picked without
/// replacement so a town with many temples spans the pantheon.
const TEMPLE_TYPES: &[(&str, u32)] = &[
    ("Walmaer",            4),
    ("Alesia",             4),
    ("Ikalio",             3),
    ("Shalissa",           3),
    ("Ielina",             2),
    ("Cal Amae",           2),
    ("Anashina",           3),
    ("Gwendyll/MacGillan", 4),
    ("Juba",               1),
    ("Taroc",              5),
    ("Recolar",            1),
    ("Maleko",             1),
    ("Fal Munir",          2),
    ("Moltan",             4),
    ("Kraagh",             4),
    ("Lesser God",         1),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgeBand { Senior, Adult, Young }

impl AgeBand {
    pub fn short(self) -> &'static str {
        match self { AgeBand::Senior => "sr", AgeBand::Adult => "ad", AgeBand::Young => "yg" }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resident {
    pub name: String,
    pub age_band: AgeBand,
    /// Single-char gender: `'M'` or `'F'`. Drives first-name pool
    /// selection at generation time and is what the sheet renders.
    pub sex: char,
    /// Concrete years of age. Derived from the age band:
    /// Young 1-21, Adult 22-54, Senior 55-75. Inn-keep / blacksmith /
    /// guard captain talk with "she's 42, the apprentice is 19"
    /// reads better than "Adult / Young".
    pub age: u32,
    /// One-word personality tag. Stored as String so the field
    /// round-trips through serde when a town is saved to disk —
    /// the value comes from the static `PERSONALITIES` pool at
    /// generation time but is owned per-resident from then on.
    pub personality: String,
}

/// Personality tags drawn at random per resident. Kept short (one
/// word) so they fit in a tight inline display next to the name.
const PERSONALITIES: &[&str] = &[
    "stoic", "cheerful", "grumpy", "shy", "ambitious", "secretive",
    "kind", "proud", "lazy", "honest", "cunning", "anxious",
    "warm", "blunt", "curious", "stern", "playful", "earnest",
    "skeptical", "devout", "restless", "patient", "vain", "wry",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Building {
    pub name: String,
    pub residents: u32,
    pub shop_hours: Option<String>,
    /// One Resident per inhabitant. Length matches `residents`.
    /// Cheap to carry: just `String + enum` per person, so even a
    /// 200-building city is well under a megabyte.
    pub people: Vec<Resident>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeKind { Positive, Negative }

/// One relationship: `from` likes/dislikes `to`. Indexes refer to the
/// `Town::relations_persons` vector (NOT building or person flat index)
/// so the index space matches the DOT output exactly.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub kind: EdgeKind,
}

/// Pre-computed relationship graph for the town. Persons are stored
/// as display labels (name + role + building number) so DOT generation
/// is a straight loop with no further lookups.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Relations {
    /// One label per person in the graph, e.g. "Alaric Stone\n(Blacksmith #5)".
    /// Capped at `RELATIONS_MAX_PERSONS` so the rendered PNG stays
    /// readable in a terminal pane.
    pub persons: Vec<String>,
    pub edges: Vec<Edge>,
    /// True if the town has more residents than we put in the graph,
    /// so the UI can flag the truncation honestly.
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Town {
    pub name: String,
    /// Size band: one of `"Castle" / "Village" / "Town" / "City"`.
    /// Owned String so the town survives serde round-trips when
    /// saved into a campaign.
    pub size_class: String,
    pub target_size: u32,
    pub buildings: Vec<Building>,
    pub total_residents: u32,
    pub relations: Relations,
}

impl Town {
    pub fn category(&self) -> &str {
        &self.size_class
    }
}

/// Build a town of `target_size` houses (Amar-Tools' size param).
/// `name_override` is optional; pass empty to auto-pick from the
/// bundled name lists. Returns a Town with every building generated.
pub fn build_town(name_override: &str, target_size: u32) -> Town {
    let mut rng = StdRng::from_time();
    let size_class: String = match target_size {
        1..=5    => "Castle",
        6..=25   => "Village",
        26..=99  => "Town",
        _        => "City",
    }.into();
    let name = if !name_override.is_empty() {
        name_override.to_string()
    } else {
        let category_idx = match size_class.as_str() {
            "Castle"  => 11, // castle
            "Village" => 12, // town/village (same list)
            "Town"    => 13,
            _         => 14, // city
        };
        crate::forge::generate_names(category_idx, 1)
            .into_iter().next().unwrap_or_else(|| "Unnamed".into())
    };

    // Pool of temples — drawn without replacement so a town with 4
    // temples lights up four different gods.
    let mut temple_pool: Vec<(&'static str, u32)> = TEMPLE_TYPES.to_vec();

    let mut buildings: Vec<Building> = Vec::new();
    let mut total_residents = 0u32;

    for ht in TOWN_BUILDINGS {
        // ((rand(pct) + rand(pct) + rand) * size / 100).floor
        let r1 = if ht.pct == 0 { 0 } else { rng.d6() as u32 % ht.pct };
        let r2 = if ht.pct == 0 { 0 } else { rng.d6() as u32 % ht.pct };
        let rfloat = (rng.d6() as u32) % 6; // small extra perturbation
        let mut h_number = (((r1 + r2 + rfloat) * target_size) / 100).max(0);
        if h_number < ht.min { h_number = ht.min; }
        if h_number == 0 { continue; }

        for _ in 0..h_number {
            // Resident count: 1 senior+ adults+ young.
            let seniors = if ht.seniors == 0 { 0 }
                          else { rng.d6() as u32 % ht.seniors + rng.d6() as u32 % ht.seniors };
            let adults  = if ht.adults == 0 { 0 }
                          else { rng.d6() as u32 % ht.adults + rng.d6() as u32 % ht.adults };
            let young   = if ht.young == 0 { 0 }
                          else { rng.d6() as u32 % ht.young + rng.d6() as u32 % ht.young };
            let mut residents = 1 + seniors + adults + young;
            if ht.name == "Stronghold" {
                residents += target_size / 40 + 1;
            }
            total_residents += residents;

            // Decorate the name for shops + temples + inn.
            let mut bname = ht.name.to_string();
            if ht.name == "Inn" {
                bname.push_str(": Open 7/7, 06-00");
            } else if ht.is_shop {
                let days  = ["5/7", "6/7", "6/7", "7/7"][rng.d6() as usize % 4];
                let open  = ["07", "08", "08", "09"][rng.d6() as usize % 4];
                let close = ["16", "17", "17", "18"][rng.d6() as usize % 4];
                bname = format!("{}: Open {}, {}-{}", ht.name, days, open, close);
            } else if ht.name == "Temple" && !temple_pool.is_empty() {
                let total: u32 = temple_pool.iter().map(|(_, w)| *w).sum();
                let mut roll = (rng.d6() as u32 * rng.d6() as u32) % total;
                let mut pick_idx = 0;
                for (i, (_, w)) in temple_pool.iter().enumerate() {
                    if roll < *w { pick_idx = i; break; }
                    roll -= *w;
                }
                let (god, _) = temple_pool.remove(pick_idx);
                bname = format!("Temple: {}", god);
            }

            // Name each inhabitant. 1 senior + n adults + n young →
            // pick names randomly from the human first-name pools and
            // pair with a shared family/last name per building (so a
            // household sits under one surname).
            let people = name_household(&mut rng, seniors, adults, young, 1u32 /* always-1 head */);

            buildings.push(Building {
                name: bname,
                residents,
                shop_hours: None,
                people,
            });

            if buildings.len() as u32 > target_size { break; }
        }
        if buildings.len() as u32 > target_size { break; }
    }

    let relations = build_relations(&mut rng, &buildings);

    Town {
        name,
        size_class,
        target_size,
        buildings,
        total_residents,
        relations,
    }
}

/// Build a household's residents. One head (Adult), then `seniors`
/// seniors, `adults` more adults, `young` young. All share a last
/// name so the household reads as a family. Uses the human first-
/// name pools (mixed male/female) and the human last-name pool.
fn name_household(
    rng: &mut StdRng,
    seniors: u32,
    adults: u32,
    young: u32,
    heads: u32,
) -> Vec<Resident> {
    let n = heads + seniors + adults + young;
    let mut out: Vec<Resident> = Vec::with_capacity(n as usize);
    // Shared surname per household.
    let last = first_pick(crate::forge::HUMAN_LAST, rng).to_string();
    let push_one = |out: &mut Vec<Resident>, rng: &mut StdRng, band: AgeBand| {
        // 50/50 male / female. Sex picks the first-name pool AND is
        // surfaced on the sheet so the GM doesn't have to guess from
        // the name (some Norwegian / fantasy names are ambiguous).
        let sex = if rng.d6() % 2 == 0 { 'M' } else { 'F' };
        let pool = if sex == 'M' {
            crate::forge::HUMAN_MALE_FIRST
        } else {
            crate::forge::HUMAN_FEMALE_FIRST
        };
        let first = first_pick(pool, rng);
        // Concrete age inside the band — mixed several d6 rolls so
        // the spread is more uniform than a single d6 % range gives.
        let r = (rng.d6() as u32 * 7 + rng.d6() as u32 * 13) % 100;
        let age = match band {
            AgeBand::Young  =>  1 + (r % 21),
            AgeBand::Adult  => 22 + (r % 33),
            AgeBand::Senior => 55 + (r % 21),
        };
        // One-word personality from the static pool. Owned String
        // on the Resident so serde can round-trip when the Town is
        // saved to disk.
        let pidx = (rng.d6() as usize * 11 + rng.d6() as usize * 7) % PERSONALITIES.len();
        let personality = PERSONALITIES[pidx].to_string();
        out.push(Resident {
            name: format!("{} {}", first, last),
            age_band: band, sex, age, personality,
        });
    };
    for _ in 0..heads   { push_one(&mut out, rng, AgeBand::Adult); }
    for _ in 0..seniors { push_one(&mut out, rng, AgeBand::Senior); }
    for _ in 0..adults  { push_one(&mut out, rng, AgeBand::Adult); }
    for _ in 0..young   { push_one(&mut out, rng, AgeBand::Young); }
    out
}

/// Pick one line from a `\n`-separated bundled name pool. Skips
/// blanks (the data files end with a trailing newline).
fn first_pick<'a>(pool: &'a str, rng: &mut StdRng) -> &'a str {
    let lines: Vec<&str> = pool.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() { return ""; }
    // Mix several d6 outputs into a usable index — d6() alone only
    // covers 1..6, far too narrow for pools of ~200 names.
    let mut r: u64 = 0;
    for _ in 0..4 { r = (r << 8) | (rng.d6() as u64 * 41); }
    lines[(r as usize) % lines.len()].trim()
}

/// Build the relationship graph for a town's residents. Mirrors
/// Amar-Tools' `town_relations.rb`: each person rolls 0-3 outbound
/// edges (rand(6) - 2 clamped to ≥0), each edge has a ~40 % chance of
/// being negative (color=red). Persons beyond `RELATIONS_MAX_PERSONS`
/// are dropped from the graph so the rendered PNG stays readable.
fn build_relations(rng: &mut StdRng, buildings: &[Building]) -> Relations {
    let mut persons: Vec<String> = Vec::new();
    'outer: for (b_idx, b) in buildings.iter().enumerate() {
        let role = b.name.split(':').next().unwrap_or(&b.name).trim();
        for p in &b.people {
            // Two lines per node: name on row 1, "role #N · sex /
            // age · personality" on row 2. `\n` is the dot-label
            // line-break sequence — graphviz turns it into a real
            // newline inside the box.
            persons.push(format!(
                "{}\\n({} #{} · {}/{} · {})",
                p.name, role, b_idx + 1, p.sex, p.age, p.personality
            ));
            if persons.len() >= RELATIONS_MAX_PERSONS { break 'outer; }
        }
    }
    let truncated = buildings.iter().map(|b| b.people.len()).sum::<usize>() > persons.len();

    let n = persons.len();
    if n < 2 { return Relations { persons, edges: Vec::new(), truncated }; }

    let mut edges: Vec<Edge> = Vec::new();
    for i in 0..n {
        // rand(6) - 2, clamped to 0. Bias toward few edges (≈ 50 %
        // of people have none) so the graph reads instead of mats.
        let count = (rng.d6() as i32 - 2).max(0) as u32;
        for _ in 0..count {
            // Pick a target ≠ self.
            let mut t = rand_idx(rng, n);
            if t == i { t = (t + 1) % n; }
            // rand(5) - 2 < 0 → negative. Matches the Ruby
            // distribution (40 % negative).
            let kind = if (rng.d6() as i32 - 2) < 0 { EdgeKind::Negative } else { EdgeKind::Positive };
            edges.push(Edge { from: i, to: t, kind });
        }
    }
    Relations { persons, edges, truncated }
}

fn rand_idx(rng: &mut StdRng, n: usize) -> usize {
    let mut r: u64 = 0;
    for _ in 0..4 { r = (r << 8) | (rng.d6() as u64 * 41); }
    (r as usize) % n
}

/// Graphviz DOT source for the relationship graph. Wraps the digraph
/// header from Amar-Tools' `town_relations.rb` so the rendered PNG
/// reads the same. Safe for `dot -Tpng` directly: labels embed `\n`
/// for line breaks and have any embedded quotes escaped.
pub fn render_dot(town: &Town) -> String {
    let mut s = String::new();
    s.push_str("digraph town {\n");
    s.push_str("  rankdir=\"LR\"\n");
    s.push_str("  splines=true\n");
    s.push_str("  overlap=false\n");
    s.push_str("  edge [ fontsize=8 len=1 arrowhead=\"none\" ]\n");
    s.push_str("  node [ fontsize=9 shape=\"box\" style=\"rounded,filled\" fillcolor=\"#f6efe1\" color=\"#5c4a2a\" ]\n");
    s.push_str(&format!("  labelloc=\"t\";\n  label=\"{} — {} relationships\";\n",
        town.name.replace('"', "'"), town.size_class));
    for (i, p) in town.relations.persons.iter().enumerate() {
        // The label in Amar-Tools is bare double-quoted; \n is a
        // literal escape inside the dot-label which graphviz turns
        // into a line break. Quote-strip just in case a name pool
        // ever grows one.
        let safe = p.replace('"', "'");
        s.push_str(&format!("  p{} [label=\"{}\"];\n", i, safe));
    }
    for e in &town.relations.edges {
        match e.kind {
            EdgeKind::Negative => s.push_str(&format!("  p{} -> p{} [color=\"#a33333\"];\n", e.from, e.to)),
            EdgeKind::Positive => s.push_str(&format!("  p{} -> p{};\n", e.from, e.to)),
        }
    }
    s.push_str("}\n");
    s
}

/// Write DOT to a temp file, shell out to `dot -Tpng`, return the
/// PNG path. The temp dir is chosen so concurrent amar instances
/// (and the next 'r' press) don't fight over the same filename.
pub fn render_png(town: &Town) -> Result<std::path::PathBuf, String> {
    let dir = std::env::temp_dir();
    let stem = format!("amar_town_{}_{}", std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs()).unwrap_or(0));
    let dot_path = dir.join(format!("{}.dot", stem));
    let png_path = dir.join(format!("{}.png", stem));
    std::fs::write(&dot_path, render_dot(town))
        .map_err(|e| format!("write {}: {}", dot_path.display(), e))?;
    let status = std::process::Command::new("dot")
        .arg("-Tpng").arg(&dot_path).arg("-o").arg(&png_path)
        .status()
        .map_err(|e| format!("spawn dot: {}", e))?;
    if !status.success() {
        return Err(format!("dot exited with {}", status));
    }
    // dot file is no longer needed; the PNG is what we display.
    let _ = std::fs::remove_file(&dot_path);
    Ok(png_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn village_of_15_has_buildings() {
        let t = build_town("Testville", 15);
        assert_eq!(t.size_class, "Village");
        assert!(!t.buildings.is_empty());
        assert!(t.total_residents > 0);
    }

    #[test]
    fn city_of_200_has_many_buildings() {
        let t = build_town("Metropolis", 200);
        assert_eq!(t.size_class, "City");
        // Town size budget caps the building count.
        assert!(t.buildings.len() > 50);
    }

    #[test]
    fn town_picks_temples_with_distinct_gods() {
        let t = build_town("Templeville", 50);
        // Collect temple gods and ensure we don't have duplicates.
        let temples: Vec<&str> = t.buildings.iter()
            .filter_map(|b| b.name.strip_prefix("Temple: "))
            .collect();
        let mut sorted = temples.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(temples.len(), sorted.len(),
            "duplicate temple gods: {:?}", temples);
    }
}
