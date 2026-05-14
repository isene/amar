//! NPC generator — port of Amar-Tools' `class_npc_new.rb`.
//!
//! The algorithm:
//!   1. Pick a chartype (template) and level (1-6).
//!   2. Generate physical fields (age / height / weight) from level
//!      and template archetype keywords.
//!   3. Apply the template's characteristic / attribute / skill bases,
//!      scaled per `calculate_tier_level` (tier modifier 1.0 / 0.8 / 0.6
//!      for characteristics / attributes / skills).
//!   4. Add weapon skill rosters from the template plus a baseline
//!      Unarmed skill for every NPC.
//!   5. Pick a weapon row from `data::MELEE` based on the BODY
//!      characteristic, an armor row from `data::ARMOUR` likewise,
//!      and a missile from `data::MISSILE`.
//!   6. Emit a fully-populated `pc::Character` so the same render +
//!      save path that handles PCs handles NPCs too.

use crate::canon::Canon;
use crate::dice::{Rng, StdRng};
use crate::forge::data::{
    self, ArmourRow, Chartype, MeleeRow, MissileRow,
};
use crate::pc::{Character, Char, OpenSkill, Spell, Weapon, WeaponKind, HitLocation,
                ATTRIBUTES, SKILLS, attribute_parent, size_from_weight_kg, HIT_LOCATIONS};
use std::collections::BTreeMap;

/// Top-level entry: build an NPC from a chartype name + level + sex.
/// Pass empty / 0 to randomise. Returns a `Character` with every
/// tier populated so the existing PC sheet renderer can show it.
pub fn build_npc(chartype_name: &str, level: u8, sex: &str) -> Character {
    let mut rng = StdRng::from_time();
    build_npc_seeded(chartype_name, level, sex, &mut rng)
}

/// Build an NPC with the bundled spell canon available. Used by
/// callers (the Forge tab) that want magic-user NPCs to come
/// pre-loaded with full canon spell details (DR/cost/distance/…).
pub fn build_npc_with_canon(
    chartype_name: &str, level: u8, sex: &str, canon: &Canon,
) -> Character {
    let mut rng = StdRng::from_time();
    let mut c = build_npc_seeded(chartype_name, level, sex, &mut rng);
    populate_spells_from_canon(&mut c, chartype_name, canon, &mut rng);
    c
}

/// Same as `build_npc` but uses a caller-provided RNG. Used by the
/// encounter generator to share a single RNG stream across all NPCs
/// in the encounter, so a single ENTER produces a deterministic-ish
/// trace per session.
pub fn build_npc_seeded(
    chartype_name: &str, level: u8, sex: &str, rng: &mut impl Rng,
) -> Character {
    let template = data::chartype(chartype_name)
        .or_else(|| data::chartype("Commoner"))
        .expect("Commoner template must exist");
    let level = if level == 0 { (oroll(rng).rem_euclid(6) + 1) as u8 } else { level };

    let mut c = Character::new_blank("");
    c.is_pc = false;
    c.race = race_from_chartype(template.name);
    c.gender = pick_sex(rng, sex, template.name);
    c.level = level;
    c.age = generate_age(rng, level, &c.race);
    c.height_cm = generate_height(rng, &c.gender, c.age, template.name);
    c.weight_kg = generate_weight(rng, c.height_cm, template.name);
    c.size = size_from_weight_kg(c.weight_kg);

    // Fresh maps — new_blank prefilled the canonical attribute zeros.
    c.characteristics.clear();
    c.attributes.clear();
    c.skills.clear();

    // Lay down every canonical char + attribute + skill at zero
    // first, so the eventual JSON saved file is fully populated and
    // every cell on the sheet has somewhere to land.
    for ch in Char::all() {
        c.characteristics.insert(ch.name().to_string(), 0);
    }
    for (_, attr) in ATTRIBUTES {
        c.attributes.insert((*attr).to_string(), 0);
    }

    // Apply the template — characteristics first, then attributes,
    // then skills. The `calculate_tier_level` math here mirrors
    // Amar-Tools so the statistical distribution stays the same.
    apply_characteristics(&mut c, template, rng);
    apply_attributes(&mut c, template, level, rng);
    apply_skills(&mut c, template, level, rng);
    add_experience_skills(&mut c, template, level, rng);
    add_weapon_skills(&mut c, template, level, rng);

    // Hit locations: defaults at unarmored.
    for loc in HIT_LOCATIONS {
        c.hit_locations.insert((*loc).to_string(), HitLocation::default());
    }

    // Wiki essential rule: every character starts with at least
    // Spoken Language 2 (native tongue), regardless of template.
    ensure_essential_skills(&mut c);

    // Role-flavored experience skills: Magic / Combat / Scholar /
    // Rogue archetypes each pick from a tailored skill pool so a
    // veteran wizard reads differently from a veteran fighter.
    add_role_skills(&mut c, template, level, rng);

    // Gods: assign one (or several, for priests) to the Worship
    // attribute so the open Worship column lights up on the sheet.
    assign_gods(&mut c, template, rng);

    // Equipment.
    select_armour(&mut c, template, rng);
    select_weapons(&mut c, template, rng);

    // Magic users: minimal placeholder spell list (full canon
    // pre-fill happens via build_npc_with_canon at the call site).
    if has_template_magic(template) {
        seed_spells(&mut c, template);
    }

    // Flavor: birthplace, personality (→ notes), auto-description,
    // money by social class. These don't affect game stats but the
    // sheet looks empty without them.
    seed_flavor(&mut c, template, rng);

    // Name (pulled from the bundled name lists by race + sex).
    let names = crate::forge::generate_names(name_category_for(&c.race, &c.gender), 1);
    c.name = names.into_iter().next().unwrap_or_else(|| "Unnamed".into());

    // Refresh BP/MF current pools from the now-populated derived stats.
    c.bp_current = c.bp_max();
    c.mf_current = c.mf_max();
    c
}

// ---------------------------------------------------------------- physical

fn generate_age(rng: &mut impl Rng, level: u8, race: &str) -> u32 {
    let base = 15;
    let years = match level {
        1 => 1 + (rng.d6() % 3) as u32,
        2 => 3 + (rng.d6() % 5) as u32,
        3 => 7 + (rng.d6() % 6) as u32,
        4 => 12 + (rng.d6() % 9) as u32,
        5 => 20 + (rng.d6() % 11) as u32,
        6 => 30 + (rng.d6() % 16) as u32,
        _ => 45 + (rng.d6() % 16) as u32,
    };
    let human_age = base + years;
    // Per Amar-Tools class_town.rb: elves and faeries age 3× slower,
    // half-elves 1.5×, dwarves 1.5×. Other races track human years.
    let scaled = match race {
        "Elf" | "Faerie"      => human_age * 3,
        "Half-elf" | "Dwarf"  => (human_age * 3) / 2,
        _                     => human_age,
    };
    scaled
}

fn generate_height(rng: &mut impl Rng, sex: &str, age: u32, ctype: &str) -> u32 {
    let base = 160i32;
    let var = oroll(rng) * 2 + oroll(rng) + (rng.d6() as i32 % 10);
    let mut h = base + var;
    if sex == "F" { h -= 5; }
    if age < 17 { h -= 3 * (16 - age as i32); }
    // Type-archetype tweaks lifted from Amar-Tools.
    if ctype.contains("Warrior") || ctype.contains("Guard")
        || ctype.contains("Soldier") || ctype.contains("Body guard")
        || ctype.contains("Barbarian") {
        h += rng.d6() as i32 % 11;
    }
    if ctype.contains("Thief") || ctype.contains("Assassin")
        || ctype.contains("Rogue") {
        h -= rng.d6() as i32 % 6;
    }
    h.max(120) as u32
}

fn generate_weight(rng: &mut impl Rng, height_cm: u32, ctype: &str) -> u32 {
    let base = (height_cm as i32) - 120;
    let bm = if ctype.contains("Warrior") || ctype.contains("Guard")
        || ctype.contains("Soldier") || ctype.contains("Body guard")
        || ctype.contains("Barbarian") || ctype.contains("Worker")
        || ctype.contains("Farmer") {
        avg(rng) * 5 + (rng.d6() as i32 % 16)
    } else if ctype.contains("Noble") || ctype.contains("Merchant")
        || ctype.contains("Scholar") || ctype.contains("Sage")
        || ctype.contains("Priest") || ctype.contains("Mage")
        || ctype.contains("Wizard") {
        avg(rng) * 3 + (rng.d6() as i32 % 11)
    } else if ctype.contains("Thief") || ctype.contains("Assassin")
        || ctype.contains("Rogue") || ctype.contains("Scout") {
        avg(rng) * 2 + (rng.d6() as i32 % 11)
    } else {
        avg(rng) * 4 + (rng.d6() as i32 % 11)
    };
    (base + bm).max(40) as u32
}

fn pick_sex(rng: &mut impl Rng, sex_in: &str, ctype: &str) -> String {
    if !sex_in.is_empty() { return sex_in.to_string(); }
    let mut s = if rng.d6() % 2 == 0 { "M" } else { "F" };
    if ctype.contains("officer") && rng.d6() != 1 { s = "M"; }
    if ctype.contains("Prostitute") && rng.d6() != 1 { s = "F"; }
    if ctype.contains("Nanny") && rng.d6() != 1 { s = "F"; }
    if ctype.contains("wife") { s = "F"; }
    s.to_string()
}

fn race_from_chartype(name: &str) -> String {
    if let Some(prefix) = name.split_once(':').map(|(p, _)| p.trim()) {
        // "Elf: Warrior" → "Elf"; "Dwarf: Smith" → "Dwarf".
        prefix.to_string()
    } else {
        "Human".to_string()
    }
}

fn name_category_for(race: &str, sex: &str) -> usize {
    // Map race + sex onto the index in `NAME_CATEGORIES` from
    // `forge::names`. 0 = Human male, 1 = Human female, 2/3 = dwarf,
    // 4/5 = elf, 6 = Lizardfolk, 7 = Troll, 8 = Araxi, 9/10 = generic.
    match (race, sex) {
        ("Human", "F")    => 1,
        ("Human", _)      => 0,
        ("Dwarf", "F")    => 3,
        ("Dwarf", _)      => 2,
        ("Elf", "F")      => 5,
        ("Elf", _)        => 4,
        ("Lizard Man", _) | ("Lizard", _) => 6,
        ("Troll", _)      => 7,
        ("Arax", _) | ("Araxi", _) => 8,
        (_, "F")          => 10,
        _                 => 9,
    }
}

// ---------------------------------------------------------------- tiers

fn apply_characteristics(c: &mut Character, t: &Chartype, _rng: &mut impl Rng) {
    c.characteristics.insert("BODY".into(),   t.body);
    c.characteristics.insert("MIND".into(),   t.mind);
    c.characteristics.insert("SPIRIT".into(), t.spirit);
}

fn apply_attributes(c: &mut Character, t: &Chartype, level: u8, rng: &mut impl Rng) {
    let lookup: BTreeMap<&str, i32> = t.attributes.iter().copied().collect();
    for (_, attr_name) in ATTRIBUTES {
        let key = format!("{}/{}", attribute_char(attr_name), attr_name);
        let base = lookup.get(key.as_str()).copied().unwrap_or(0);
        let val = calculate_tier_level(base, level, TierKind::Attribute, rng);
        c.attributes.insert((*attr_name).to_string(), val);
    }
}

fn apply_skills(c: &mut Character, t: &Chartype, level: u8, rng: &mut impl Rng) {
    let template_skills: BTreeMap<&str, i32> = t.skills.iter().copied().collect();
    // Fill the canonical roster first so every cell has a (possibly 0) value.
    for (attr, skill_list) in SKILLS {
        let mut attr_map: BTreeMap<String, i32> = BTreeMap::new();
        for skill in *skill_list {
            let key = format!("{}/{}/{}", attribute_char(attr), attr, skill);
            let base = template_skills.get(key.as_str()).copied().unwrap_or(0);
            let val = calculate_tier_level(base, level, TierKind::Skill, rng);
            attr_map.insert((*skill).to_string(), val);
        }
        if !attr_map.is_empty() {
            c.skills.insert((*attr).to_string(), attr_map);
        }
    }
    // Then drop any extra template-only skills (e.g. "Smithing" under
    // Practical Knowledge) into the same map.
    for (key, base) in t.skills {
        let parts: Vec<&str> = key.split('/').collect();
        if parts.len() != 3 { continue; }
        let (_ch, attr, skill) = (parts[0], parts[1], parts[2]);
        let val = calculate_tier_level(*base, level, TierKind::Skill, rng);
        let entry = c.skills.entry(attr.to_string()).or_default();
        let cur = entry.get(skill).copied().unwrap_or(0);
        entry.insert(skill.to_string(), cur.max(val));
    }
}

fn add_experience_skills(c: &mut Character, _t: &Chartype, level: u8, rng: &mut impl Rng) {
    if level < 4 { return; }
    // Sprinkle a bonus +1 onto a handful of useful skills for vets.
    let bonus = (level - 3) as i32;
    let candidates: &[(&str, &str, &str)] = &[
        ("BODY", "Athletics", "Hide"),
        ("BODY", "Athletics", "Move Quietly"),
        ("BODY", "Athletics", "Climb"),
        ("BODY", "Endurance", "Running"),
        ("BODY", "Endurance", "Combat Tenacity"),
        ("MIND", "Awareness", "Tracking"),
        ("MIND", "Awareness", "Alertness"),
        ("MIND", "Practical Knowledge", "Survival Lore"),
        ("MIND", "Practical Knowledge", "Ambush"),
        ("MIND", "Willpower", "Mental Fortitude"),
    ];
    let count = (level as usize / 2 + 2).min(candidates.len());
    let mut idx = rng.d6() as usize;
    for _ in 0..count {
        idx = (idx + rng.d6() as usize + 1) % candidates.len();
        let (_ch, attr, skill) = candidates[idx];
        let entry = c.skills.entry(attr.to_string()).or_default();
        let cur = entry.get(skill).copied().unwrap_or(0);
        entry.insert(skill.to_string(), cur + bonus);
    }
}

fn add_weapon_skills(c: &mut Character, t: &Chartype, level: u8, rng: &mut impl Rng) {
    let unarmed = (level as i32 / 2 + 1).max(1);
    let mc = c.skills.entry("Melee Combat".into()).or_default();
    mc.entry("Unarmed".into()).or_insert(unarmed);
    let primary = t.melee_weapons.iter().max_by_key(|(_, v)| *v).map(|(n, _)| *n);
    for (weapon, base) in t.melee_weapons {
        let mut v = calculate_tier_level(*base, level, TierKind::Skill, rng);
        if Some(*weapon) == primary && *base >= 4 { v += 1 + (rng.d6() as i32 % 2); }
        let mc = c.skills.entry("Melee Combat".into()).or_default();
        mc.insert((*weapon).to_string(), v);
    }
    let primary_m = t.missile_weapons.iter().max_by_key(|(_, v)| *v).map(|(n, _)| *n);
    for (weapon, base) in t.missile_weapons {
        let mut v = calculate_tier_level(*base, level, TierKind::Skill, rng);
        if Some(*weapon) == primary_m && *base >= 3 { v += 1 + (rng.d6() as i32 % 2); }
        let ms = c.skills.entry("Missile Combat".into()).or_default();
        ms.insert((*weapon).to_string(), v);
    }
}

// ---------------------------------------------------------------- gods

/// Pick a deity (or two) for the NPC and write Worship/&lt;god&gt; skills
/// into the SPIRIT column. Priests get 2-3 gods (a small pantheon);
/// other types get one. Falls back to "no religion" when the chartype
/// map says "None" for the rolled slot.
fn assign_gods(c: &mut Character, t: &Chartype, rng: &mut impl Rng) {
    let count = if t.name == "Priest" || t.name == "Clergyman" { 2 + rng.d6() as usize % 2 }
                else { 1 };
    let pool: Vec<&'static str> = data::CHARTYPE_RELIGIONS.iter()
        .find(|(k, _)| *k == t.name)
        .map(|(_, v)| v.to_vec())
        .unwrap_or_else(|| vec!["any"]);
    let mut already = std::collections::BTreeSet::<String>::new();
    for _ in 0..count {
        let raw = pool[(rng.d6() as usize * rng.d6() as usize) % pool.len()];
        let god = match raw {
            "nobility" => if c.gender == "F" { "Gwendyll" } else { "MacGillan" },
            "any"  => pick_weighted_god(rng),
            "None" => {
                // Atheism is rare in Amar — only ~5% of NPCs in
                // even "irreligious" chartypes (Thief, Highwayman,
                // Assassin) end up with no god. Otherwise re-roll
                // against the pool's non-"None" entries.
                if is_atheist(rng) { continue; }
                let alts: Vec<&str> = pool.iter().copied()
                    .filter(|n| *n != "None").collect();
                if alts.is_empty() { continue; }
                alts[(rng.d6() as usize * rng.d6() as usize) % alts.len()]
            }
            other => other,
        };
        if already.contains(god) { continue; }
        already.insert(god.into());
        // Worship rank scales with level: novice priests have 2,
        // master priests 5+.
        let rank = match c.level {
            1..=2 => 1 + (rng.d6() as i32 % 2),
            3..=4 => 2 + (rng.d6() as i32 % 2),
            5..=6 => 3 + (rng.d6() as i32 % 2),
            _     => 4 + (rng.d6() as i32 % 2),
        };
        let entry = c.skills.entry("Worship".into()).or_default();
        entry.insert(god.to_string(), rank);
    }
}

/// Return true ~5% of the time. Used to gate the rare "no god"
/// outcome — Amar is a deeply religious setting, so even nominally
/// godless professions (thieves, assassins) usually keep a patron
/// saint or two. 1/6 × 2/6 = 1/18 ≈ 5.6%.
fn is_atheist(rng: &mut impl Rng) -> bool {
    rng.d6() == 1 && rng.d6() <= 2
}

fn pick_weighted_god(rng: &mut impl Rng) -> &'static str {
    let total: u32 = data::RANDOM_DEITY_WEIGHTS.iter().map(|(_, w)| *w).sum();
    // Build a roughly-uniform roll into [0, total).
    let mut roll: u32 = 0;
    let mut span = 1u32;
    while span < total {
        roll = (roll << 4) ^ (rng.d6() as u32);
        span <<= 4;
    }
    roll %= total;
    let mut acc = 0;
    for (name, w) in data::RANDOM_DEITY_WEIGHTS {
        acc += *w;
        if roll < acc { return *name; }
    }
    "None"
}

// ---------------------------------------------------------------- role skills

/// Type-specific experience-skill pools. Mirrors Amar-Tools'
/// `add_experience_skills` branching: magic users get Sense Magick /
/// Magick Rituals, combat types get Fortitude / Combat Tenacity,
/// scholars get Innovation / Literacy / Alchemy, rogues get Hide /
/// Move Quietly / Pick Pockets / Disarm Traps. Each pool member adds
/// `+rand bonus` to the existing skill rank so the role flavor lands
/// on top of the canonical base.
fn add_role_skills(c: &mut Character, t: &Chartype, level: u8, rng: &mut impl Rng) {
    if level < 2 { return; }
    let n = t.name;
    let bonus = (level as i32 / 2).max(1);

    // Build a pool of (attr, skill, weight) entries based on the
    // chartype keyword. A given chartype can match multiple pools
    // (e.g. a Witch is both Magic and Scholar-ish).
    let mut pool: Vec<(&str, &str)> = Vec::new();

    let is_magic = n.contains("Wizard") || n.contains("Witch") || n == "Mage"
        || n == "Sorcerer" || n == "Summoner" || n == "Seer";
    let is_priest = n == "Priest" || n == "Clergyman" || n == "Monk";
    let is_combat = n == "Warrior" || n == "Guard" || n == "Soldier"
        || n == "Gladiator" || n == "Body guard" || n == "Ranger"
        || n == "Hunter" || n == "Barbarian" || n == "Berserker"
        || n.contains(": Warrior") || n.contains(": Guard");
    let is_scholar = n == "Scholar" || n == "Sage" || n == "Scribe"
        || n == "Bard" || n.contains("Wizard") || n == "Mage";
    let is_rogue = n == "Thief" || n == "Assassin" || n == "Highwayman"
        || n == "Scout" || n.contains(": Thief");

    if is_magic {
        pool.extend([
            ("Awareness", "Sense Magick"),
            ("Nature Knowledge", "Magick Rituals"),
            ("Nature Knowledge", "Alchemy"),
            ("Social Knowledge", "Mythology"),
            ("Social Knowledge", "Legend Lore"),
            ("Casting", "Range"),
            ("Casting", "Duration"),
            ("Casting", "Area of Effect"),
            ("Willpower", "Mental Fortitude"),
        ]);
    }
    if is_priest {
        pool.extend([
            ("Social Knowledge", "Mythology"),
            ("Social Knowledge", "Legend Lore"),
            ("Nature Knowledge", "Medical Lore"),
            ("Willpower", "Mental Fortitude"),
        ]);
    }
    if is_combat {
        pool.extend([
            ("Endurance", "Fortitude"),
            ("Endurance", "Combat Tenacity"),
            ("Endurance", "Running"),
            ("Practical Knowledge", "Ambush"),
            ("Awareness", "Sense Ambush"),
            ("Willpower", "Pain Tolerance"),
            ("Athletics", "Climb"),
        ]);
    }
    if is_scholar {
        pool.extend([
            ("Intelligence", "Innovation"),
            ("Intelligence", "Problem Solving"),
            ("Social Knowledge", "Literacy"),
            ("Social Knowledge", "Spoken Language"),
            ("Nature Knowledge", "Alchemy"),
        ]);
    }
    if is_rogue {
        pool.extend([
            ("Athletics", "Hide"),
            ("Athletics", "Move Quietly"),
            ("Athletics", "Climb"),
            ("Athletics", "Balance"),
            ("Sleight", "Pick Pockets"),
            ("Sleight", "Disarm Traps"),
            ("Awareness", "Detect Traps"),
            ("Awareness", "Alertness"),
            ("Practical Knowledge", "Ambush"),
        ]);
    }
    if pool.is_empty() { return; }

    // How many bumps? Scale with level.
    let bumps = match level {
        1..=2 => 2,
        3..=4 => 4,
        5..=6 => 6,
        _     => 8,
    }.min(pool.len());

    let mut idx = (rng.d6() as usize) % pool.len();
    for _ in 0..bumps {
        let (attr, skill) = pool[idx];
        let entry = c.skills.entry(attr.into()).or_default();
        let cur = entry.get(skill).copied().unwrap_or(0);
        entry.insert(skill.into(), cur + bonus);
        idx = (idx + rng.d6() as usize + 1) % pool.len();
    }
}

// ---------------------------------------------------------------- flavor

/// Birthplace, personality (→ notes), description, money. None of
/// this affects game stats, but the sheet looks anemic without it.
fn seed_flavor(c: &mut Character, t: &Chartype, rng: &mut impl Rng) {
    // Birthplace: 6 Amar districts. Race overrides for non-humans
    // (elves from Aleresir's deep forest, dwarves from the mountains).
    let birthplaces_human: &[&str] = &[
        "Amaronir", "Merisir", "Calaronir", "Feronir", "Rauinir", "Aleresir",
    ];
    c.birthplace = match c.race.as_str() {
        "Dwarf"  => ["Mountainholm", "Stonehold", "Ironforge"][rng.d6() as usize % 3].into(),
        "Elf"    => ["Aleresir Forest", "Greenwood", "Silverleaf"][rng.d6() as usize % 3].into(),
        "Troll"  => ["The Crags", "Trollmoor"][rng.d6() as usize % 2].into(),
        "Faerie" => ["Faerieglade", "Moonpool"][rng.d6() as usize % 2].into(),
        "Arax" | "Araxi" => ["Black Tents", "Salt Wastes"][rng.d6() as usize % 2].into(),
        _ => birthplaces_human[rng.d6() as usize % birthplaces_human.len()].into(),
    };

    // Personality from the weighted table (already ported in data.rs).
    let total: u32 = data::PERSONALITY.iter().map(|(_, w)| *w).sum();
    let mut roll = (rng.d6() as u32 * rng.d6() as u32) % total;
    let mut trait_str = "Indifferent, unstructured";
    for (name, w) in data::PERSONALITY {
        if roll < *w { trait_str = name; break; }
        roll -= *w;
    }

    // Auto-description: clothing isn't seeded here (kept simple)
    // — just a one-line physical sketch.
    let desc = format!("{}, {} cm, {} kg. {}.",
        if c.gender == "F" { "Female" } else { "Male" },
        c.height_cm, c.weight_kg, trait_str);
    c.description = desc;

    // Notes: stash the personality + chartype + social-class hint so
    // GMs have something to riff on without picking up another table.
    let social = ["Slave (S)", "Lower Class (LC)", "Lower Middle (LMC)",
                  "Middle Class (MC)", "Upper Class (UC)", "Noble (N)"];
    let sclass_idx = match t.name {
        "Noble" | "High class" => 5,
        "Merchant" | "Bard" | "Scholar" | "Sage" | "Mage" | "Priest" => 4,
        "Soldier" | "Guard" | "Smith" | "Hunter" | "Ranger" | "Sailor" => 3,
        "Farmer" | "Commoner" | "Worker" => 1,
        _ => 2,
    };
    let sclass = social[sclass_idx];
    c.notes = format!("Personality: {}\nSocial class: {}\nChartype: {}",
        trait_str, sclass, t.name);

    // Money by social class. Slave → 0, Noble → 3d6 × 1000.
    c.money_sp = match sclass_idx {
        0 => 0,
        1 => rng.d6() as i32,
        2 => 2 * rng.d6() as i32,
        3 => 3 * rng.d6() as i32 * 10,
        4 => 3 * rng.d6() as i32 * 100,
        _ => 3 * rng.d6() as i32 * 1000,
    };

    // Clothing one-liner so the Equipment block shows something.
    c.clothing = match sclass_idx {
        0     => "Rags".into(),
        1     => "Plain wool tunic, leather belt".into(),
        2     => "Working clothes, sturdy boots".into(),
        3     => "Travel cloak, layered tunic, good boots".into(),
        4     => "Embroidered tunic, fine linen, silk sash".into(),
        _     => "Silk-and-velvet attire, jewelry, signet ring".into(),
    };
}

// ---------------------------------------------------------------- essentials

/// Wiki essentials. Every character — PC or NPC — has these skills
/// at the minimums called out in the rules. We apply them as floors
/// so chartype templates can override upward but never downward.
///
///   • Spoken Language (Social Knowledge) ≥ 2 — native tongue rule.
///   • Reaction Speed (Awareness) ≥ 0  — always present so the sheet
///     can show initiative even at zero rank.
///   • Alertness (Awareness) ≥ 0       — likewise for combat awareness.
fn ensure_essential_skills(c: &mut Character) {
    let bumps: &[(&str, &str, i32)] = &[
        ("Social Knowledge", "Spoken Language", 2),
        ("Awareness",        "Reaction Speed",  0),
        ("Awareness",        "Alertness",       0),
    ];
    for (attr, skill, floor) in bumps {
        let entry = c.skills.entry((*attr).into()).or_default();
        let cur = entry.get(*skill).copied().unwrap_or(0);
        entry.insert((*skill).into(), cur.max(*floor));
    }
}

// ---------------------------------------------------------------- equipment

/// Uniform-ish index sampler in `0..span`. A bare `rng.d6() % span`
/// caps at 5 (d6 returns 1-6) regardless of span — that's why
/// armor / weapon picks from wide windows kept rolling the low
/// slots. Combine three d6 throws into a base-6 number (0..215)
/// and mod by span for a properly-spread roll.
fn pick_idx(rng: &mut impl Rng, span: usize) -> usize {
    if span <= 1 { return 0; }
    let a = (rng.d6() - 1) as usize;
    let b = (rng.d6() - 1) as usize;
    let c = (rng.d6() - 1) as usize;
    (a * 36 + b * 6 + c) % span
}

fn select_armour(c: &mut Character, t: &Chartype, rng: &mut impl Rng) {
    // Original Amar-Tools rule: index range determined by BODY
    // characteristic (1-2 → 1, 3 → 2, …, 7+ → 8). Combat
    // chartypes bump that ceiling by `level-1` so a level-5
    // Gladiator stops rolling Heavy Cloth and starts seeing
    // Cuir-boullie / Chain mail. Slot 0 (None) is skipped for
    // combat chartypes — a fighter without armour is the
    // exception, not 50% of the roll.
    let body = c.ch(Char::Body);
    let mut arm_level = match body {
        0..=2 => 1,
        3 => 2,
        4 => 3,
        5 => 5,
        6 => 6,
        7 => 7,
        _ => 8,
    };
    let combat = is_combat_chartype(t.name);
    if combat {
        // Per level bump (not level-1) so a level-1 combatant
        // already widens past the BODY-only baseline. Level 5
        // BODY 3: 2 + 5 = 7 → window reaches Chain mail.
        arm_level += c.level as usize;
    }
    arm_level = arm_level.min(data::ARMOUR.len());
    let start = if combat { 1 } else { 0 };
    let span = arm_level.saturating_sub(start).max(1);
    // Combat chartypes: roll the index twice, take the higher.
    // Skews picks toward the heavy end so a veteran fighter
    // doesn't keep landing on Heavy Cloth.
    let mut roll = pick_idx(rng, span);
    if combat {
        roll = roll.max(pick_idx(rng, span));
    }
    let idx = (start + roll).min(data::ARMOUR.len() - 1);
    let row: &ArmourRow = &data::ARMOUR[idx];
    // Spread the AP across all hit locations for now (the AP table
    // is a single number; per-location can be tweaked manually).
    for loc in HIT_LOCATIONS {
        let entry = c.hit_locations.entry((*loc).to_string()).or_default();
        entry.armor = row.name.into();
        entry.ap = row.ap;
    }
}

fn select_weapons(c: &mut Character, t: &Chartype, rng: &mut impl Rng) {
    // 0. Unarmed: every character is always armed with their fists.
    //    The skill is already inserted by `add_weapon_skills`; here
    //    we add the weapon row so the sheet has a "punch" line the GM
    //    can resolve without manual entry.
    c.weapons.push(Weapon {
        name: "Unarmed".into(),
        kind: WeaponKind::Melee,
        skill_name: "Unarmed".into(),
        two_handed: false,
        init: 1, off_mod: -2, def_mod: -4,
        shots_per_round: 0, damage: -4, hp: 0,
        range_m: 0, xp: 0,
    });

    // Same indexing rule as Amar-Tools: BODY drives the melee
    // weapon-table window the NPC can sample from. Skip slot 0
    // (Unarmed) since the explicit row above already covers it —
    // otherwise BODY 1 picks Unarmed 50% of the time and the sheet
    // shows the same row twice.
    //
    // Combat chartypes (Warrior / Gladiator / Body guard / …) at
    // higher levels pull from a wider range so they actually see
    // longswords + 2H + combo rows. Their pick is also biased
    // toward the HEAVY end (d6 rolled twice, take higher) so a
    // level-5 Gladiator stops landing on a Knife.
    let body = c.ch(Char::Body);
    let mut wpn_level = match body {
        1 => 2,
        2 => 4,
        3 => 11,
        4 => 18,
        5 => 22,
        7..=8 => 28,
        _ => 30,
    };
    let combat = is_combat_chartype(t.name);
    if combat {
        // Wide level bump so a level-5 Gladiator pulls combo /
        // 2H weapons (idx 16-25 in the MELEE table) rather than
        // capping at the 1H-handed cluster around idx 10. Level
        // 5 + BODY 3 → 11 + 15 = 26 → reaches all combo rows
        // plus Battle axe / Kite shield / Great sword combos.
        wpn_level += (c.level as usize) * 3;
    }
    let wpn_level = wpn_level.min(data::MELEE.len());
    let span = (wpn_level.saturating_sub(1)).max(1);
    // pick_idx samples across the FULL span (d6 % span was
    // capped at 5). Combat chartypes roll thrice and take the
    // highest, hard-skewing toward the heavy end so a veteran
    // fighter doesn't keep landing on Knife / Short sword.
    let mut roll = pick_idx(rng, span);
    if combat {
        roll = roll.max(pick_idx(rng, span));
        roll = roll.max(pick_idx(rng, span));
    }
    let idx = (1 + roll).min(data::MELEE.len() - 1);
    let row: &MeleeRow = &data::MELEE[idx];
    // Show the combo as a single row with its combined stats — that's
    // the form the Amar rules table balances: the off/def numbers on
    // `Longsword/Buc` already encode the buckler's contribution, the
    // damage on `Knife*2` already accounts for dual-wielding, etc.
    // The shorthand from the MELEE table (`B. axe/Ksh`, `Lt. mace/Buc`,
    // `Knife*2`) is expanded to the full long-form for the sheet
    // (`Battle axe + Kite shield`, `Light mace + Buckler`, `Knife + Knife`).
    let display_name = format_combo_name(row.name);
    // Use the raw shorthand for skill lookup so the combo-name's
    // " + Kite shield" suffix doesn't fool the resolver into picking
    // the `Shield` skill for what is actually an Axe / Sword / Mace.
    let weapon_skill = pick_weapon_skill_name(row.name);
    let primary_two_handed = row.kind.starts_with("2H") || row.kind == "Polearm";
    // If the combo includes a shield, also seed a "Shield" skill so
    // the GM has something to roll against when the shield itself is
    // targeted (called shot, sundering, etc.) without manually
    // adding the skill afterwards.
    if row.kind == "1H/Shield" {
        let rank = (c.level as i32 / 2 + 1).max(1);
        c.skills.entry("Melee Combat".into()).or_default()
            .entry("Shield".into()).or_insert(rank);
    }
    c.weapons.push(Weapon {
        name: display_name.clone(),
        kind: WeaponKind::Melee,
        skill_name: weapon_skill.into(),
        two_handed: primary_two_handed,
        init: row.init,
        off_mod: row.off,
        def_mod: row.def,
        shots_per_round: 0,
        damage: row.dam,
        hp: row.hp,
        range_m: 0,
        xp: 0,
    });

    // Tack on a sidearm for combat-trained chartypes ONLY when the
    // combo didn't already provide one. A Warrior with `Longsword/Buc`
    // already has the buckler as the off-hand; no need to clutter the
    // sheet with a third item. A Warrior with a bare `Great sword`
    // still gets the backup knife / short sword.
    let is_combo = row.name.contains('/') || row.name.contains('*');
    let is_already_small = row.name == "Knife";
    if is_combat_chartype(t.name) && !is_combo && !is_already_small {
        // Cap the table window to the small-weapon end (idx 1..=5).
        // Slot 0 is Unarmed — same dedup reason as the primary pick.
        let cap = 6.min(data::MELEE.len());
        let span = (cap.saturating_sub(1)).max(1);
        let bidx = (1 + (rng.d6() as usize % span)).min(cap - 1);
        let brow: &MeleeRow = &data::MELEE[bidx];
        // Roll again if the sidearm pick also happens to be a combo —
        // a clean single-weapon backup reads better as "Dagger" than
        // as "Rapier + Knife".
        if !brow.name.contains('/') && !brow.name.contains('*') && brow.name != row.name {
            let bname = expand_weapon_short(brow.name);
            let bskill = pick_weapon_skill_name(&bname);
            c.weapons.push(Weapon {
                name: bname,
                kind: WeaponKind::Melee,
                skill_name: bskill.into(),
                two_handed: false,
                init: brow.init,
                off_mod: brow.off,
                def_mod: brow.def,
                shots_per_round: 0,
                damage: brow.dam,
                hp: brow.hp,
                range_m: 0,
                xp: 0,
            });
        }
    }

    // Missile weapon — bow strength based on Wield Weapon total.
    let wield_total = c.skill_total(Char::Body, "Strength", "Wield Weapon");
    if wield_total >= 2 || c.skills.get("Missile Combat").is_some() {
        let mrow: &MissileRow = pick_missile(rng, wield_total);
        let mskill = pick_missile_skill_name(mrow.name);
        c.weapons.push(Weapon {
            name: mrow.name.into(),
            kind: WeaponKind::Missile,
            skill_name: mskill.into(),
            two_handed: false,
            init: mrow.init,
            off_mod: mrow.off,
            def_mod: 0,
            shots_per_round: 1,
            damage: mrow.dam,
            hp: 0,
            range_m: mrow.rng,
            xp: 0,
        });
    }

    // Safety net: collapse exact duplicates that slipped past the
    // index-skip above (e.g. if a future code path adds a second
    // Unarmed row, or the sidearm pick lands on the same shorthand
    // the primary just used).
    let mut seen = std::collections::BTreeSet::<(String, String)>::new();
    c.weapons.retain(|w| seen.insert((w.name.clone(), w.skill_name.clone())));
}

/// Which chartypes get a sidearm in addition to their primary
/// melee + (optional) missile. Anyone who lives by the sword should
/// have a backup; commoners / sages / merchants don't.
fn is_combat_chartype(name: &str) -> bool {
    matches!(name,
        "Warrior" | "Guard" | "Soldier" | "Bandit" | "Highwayman" |
        "Assassin" | "Hunter" | "Tracker" | "Scout" | "Barbarian" |
        "Berserker" | "Gladiator" | "Body guard" | "Ranger" |
        "Thief" | "Sailor" | "Monk"
    )
}

fn pick_missile(rng: &mut impl Rng, wield_total: i32) -> &'static MissileRow {
    let cap = match wield_total {
        i32::MIN..=1 => 3,
        2..=3 => 5,
        4..=5 => 8,
        6..=7 => 10,
        8..=9 => 11,
        _ => 12,
    }.min(data::MISSILE.len());
    let idx = pick_idx(rng, cap).min(data::MISSILE.len() - 1);
    &data::MISSILE[idx]
}

fn pick_weapon_skill_name(name: &str) -> &'static str {
    let n = name.to_ascii_lowercase();
    if n.contains("shield") || n.contains("buckler") { "Shield" }
    else if n.contains("sword")  { "Sword" }
    else if n.contains("axe") || n.contains("hatchet") { "Axe" }
    else if n.contains("mace") || n.contains("hammer") || n.contains("club") { "Club" }
    else if n.contains("spear") || n.contains("polearm") || n.contains("halberd") { "Spear" }
    else if n.contains("staff")  { "Staff" }
    else if n.contains("knife") || n.contains("dagger") { "Dagger" }
    else if n.contains("rapier") { "Rapier" }
    else if n.contains("unarmed") { "Unarmed" }
    else { "Sword" }
}

/// Expand the MELEE table's shorthand into the long-form combo name
/// shown on the sheet. The table itself stays compact (`B. axe/Ksh`,
/// `Knife*2`); the display reads as `Battle axe + Kite shield` /
/// `Knife + Knife` so the GM doesn't have to translate at the table.
/// Stats stay on the single row — they are the COMBO's combined
/// numbers from the Amar rules table.
fn format_combo_name(name: &str) -> String {
    if let Some((base, count_str)) = name.split_once('*') {
        if count_str.trim() == "2" {
            let p = expand_weapon_short(base);
            return format!("{} + {}", p, p);
        }
    }
    if let Some((primary, secondary)) = name.split_once('/') {
        return format!(
            "{} + {}",
            expand_weapon_short(primary),
            expand_secondary_short(secondary),
        );
    }
    expand_weapon_short(name)
}

/// Map Amar-Tools shorthand weapon names ("Lt. mace", "B. sword",
/// "Br. axe", "H. Mace") to full names so the sheet reads cleanly.
/// Unknown names pass through.
fn expand_weapon_short(s: &str) -> String {
    match s.trim() {
        "Lt. mace"            => "Light mace".to_string(),
        "B. sword"            => "Broad sword".to_string(),
        "Br. axe"             => "Bronze axe".to_string(),
        "B. axe"              => "Battle axe".to_string(),
        "H. Mace" | "H. mace" => "Heavy mace".to_string(),
        other                 => other.to_string(),
    }
}

/// Map the shield / off-hand shorthand to a full name. The MELEE
/// table uses `Buc` / `RSh` / `Rsh` / `KSh` / `Ksh` consistently.
fn expand_secondary_short(s: &str) -> String {
    match s.trim() {
        "Buc"            => "Buckler".to_string(),
        "RSh" | "Rsh"    => "Round shield".to_string(),
        "KSh" | "Ksh"    => "Kite shield".to_string(),
        other            => other.to_string(),
    }
}

fn pick_missile_skill_name(name: &str) -> &'static str {
    let n = name.to_ascii_lowercase();
    if n.contains("bow") && !n.contains("x-bow") && !n.contains("crossbow") { "Bow" }
    else if n.contains("crossbow") || n.contains("x-bow") { "Crossbow" }
    else if n.contains("sling")    { "Sling" }
    else if n.contains("javelin")  { "Javelin" }
    else if n.contains("rock") || n.contains("knife") || n.contains("th ") { "Throwing" }
    else { "Bow" }
}

// ---------------------------------------------------------------- spells

fn has_template_magic(t: &Chartype) -> bool {
    t.spirit > 0 && t.attributes.iter().any(|(k, v)| *k == "SPIRIT/Casting" && *v > 0)
}

/// Fully populate the spell list for a magic-user NPC from the
/// bundled wiki canon. Picks N spells from the type's preferred
/// domains, pre-filling every field (DR, cost, distance, duration,
/// area, cooldown, casting time, active/passive, effects) so the
/// spell table on the sheet is a real reference, not a placeholder.
///
/// Spell count scales with level + casting (Amar-Tools formula):
///   L1-2 → 1-2, L3-4 → 2-4, L5 → 4-6, L6 → 6-10, L7+ → 8-12.
///
/// Wiki exclusion: only run for NPCs with SPIRIT ≥ 2 AND Casting
/// total ≥ 5. Templates set most of this, but the floor catches the
/// "anyone can be a wizard" edge case.
pub fn populate_spells_from_canon(
    c: &mut Character, chartype_name: &str, canon: &Canon, rng: &mut impl Rng,
) {
    let spirit = c.ch(Char::Spirit);
    let casting_total = spirit + c.attr("Casting");
    if spirit < 2 || casting_total < 5 { return; }

    // Throw away the placeholder spells `seed_spells` may have
    // dropped in earlier.
    c.spells.clear();

    // Domain pool per chartype, mirroring Amar-Tools
    // generate_spell_cards.
    let domains: Vec<&str> = if let Some(rest) = chartype_name
        .strip_prefix("Wizard (").and_then(|s| s.strip_suffix(")"))
    {
        // Wizard (fire/water/air/earth/prot.) → focus that domain.
        let d = match rest {
            "prot." | "protection" => "Self",
            other => match other {
                "fire"  => "Fire",
                "water" => "Water",
                "air"   => "Air",
                "earth" => "Earth",
                x => Box::leak(x.to_string().into_boxed_str()),
            },
        };
        vec![d]
    } else {
        match chartype_name {
            "Mage"          => vec!["Fire", "Air", "Mind", "Self"],
            "Priest"        => vec!["Life", "Body", "Mind"],
            "Witch (white)" => vec!["Life", "Body", "Self"],
            "Witch (black)" => vec!["Death", "Mind", "Body"],
            "Sorcerer"      => vec!["Mind", "Death"],
            "Summoner"      => vec!["Mind", "Death", "Self"],
            "Seer"          => vec!["Mind", "Self"],
            "Sage"          => vec!["Mind", "Self"],
            _ => vec!["Fire", "Water", "Air", "Earth"],
        }
    };

    // Build the canon spell pool — entries in the Spells domain that
    // match one of the NPC's preferred domains.
    let mut pool: Vec<&str> = canon.category("Spells").iter()
        .filter(|name| {
            let e = match canon.lookup(name) { Some(e) => e, None => return false };
            let dom = e.fields.get("domain").map(|s| s.as_str()).unwrap_or("");
            domains.iter().any(|d| dom.eq_ignore_ascii_case(d))
        })
        .map(|s| s.as_str())
        .collect();
    if pool.is_empty() { return; }

    // Spell count per Amar-Tools scaling.
    let base = match c.level {
        1..=2 => 1 + (rng.d6() as usize % 2),
        3..=4 => 2 + (rng.d6() as usize % 3),
        5     => 4 + (rng.d6() as usize % 3),
        6     => 6 + (rng.d6() as usize % 5),
        _     => 8 + (rng.d6() as usize % 5),
    };
    let bonus = (casting_total / 2) as usize;
    let count = (base + bonus).clamp(1, 15);

    let parse_lead_int = |s: &str| -> i32 {
        s.split_whitespace().next()
            .and_then(|t| t.parse::<i32>().ok())
            .unwrap_or(0)
    };

    let mut already = std::collections::BTreeSet::<String>::new();
    let mut attempts = 0usize;
    while c.spells.len() < count && attempts < 50 && !pool.is_empty() {
        attempts += 1;
        let idx = (rng.d6() as usize * rng.d6() as usize) % pool.len();
        let name = pool[idx].to_string();
        if already.contains(&name) { continue; }
        already.insert(name.clone());
        let entry = match canon.lookup(&name) { Some(e) => e, None => continue };
        c.spells.push(Spell {
            name: name.clone(),
            domain:         entry.fields.get("domain").cloned().unwrap_or_default(),
            active_passive: entry.fields.get("active_passive").cloned().unwrap_or_default(),
            dr:             entry.fields.get("dr").map(|s| parse_lead_int(s)).unwrap_or(0),
            cost:           entry.fields.get("cost").map(|s| parse_lead_int(s)).unwrap_or(0),
            casting_time:   entry.fields.get("casting_time").cloned().unwrap_or_default(),
            distance:       entry.fields.get("distance").cloned().unwrap_or_default(),
            duration:       entry.fields.get("duration").cloned().unwrap_or_default(),
            area:           entry.fields.get("area_of_effect")
                                  .or_else(|| entry.fields.get("area"))
                                  .cloned().unwrap_or_default(),
            cooldown:       entry.fields.get("cooldown").cloned().unwrap_or_default(),
            effects:        entry.fields.get("effects").cloned().unwrap_or_default(),
        });
        // Avoid re-picking the same name on the next iteration.
        pool.retain(|n| *n != name);
    }
}

fn seed_spells(c: &mut Character, t: &Chartype) {
    // Minimal seed: store the casting domains the template hinted at
    // as `Spell` records, so the sheet shows them. Full canon
    // pre-fill for individual spell names lives in `add_spell` on
    // the App; the forge just makes sure spell-using NPCs leave
    // generation with a non-empty casting block.
    for (k, _) in t.attributes {
        if let Some(rest) = k.strip_prefix("SPIRIT/Attunement/") {
            c.spells.push(Spell { name: format!("Attunement: {}", rest), ..Default::default() });
        }
    }
    // Drop a single spell line keyed off the template name as a hint.
    let hint = if t.name.starts_with("Wizard") {
        "Bolt"
    } else if t.name.starts_with("Witch (white)") {
        "Heal"
    } else if t.name.starts_with("Witch (black)") {
        "Curse"
    } else if t.name == "Mage" {
        "Magic Missile"
    } else { "" };
    if !hint.is_empty() {
        c.spells.push(Spell { name: hint.into(), ..Default::default() });
    }
}

// ---------------------------------------------------------------- math

#[derive(Debug, Clone, Copy)]
enum TierKind { Attribute, Skill }

/// Same shape as Amar-Tools' `calculate_tier_level`. The `tier_kind`
/// switches between the two non-characteristic curves; we don't
/// drive the chosen characteristic through this fn at all because
/// the Rust port keeps characteristics as the simple template
/// number (matches the wiki rules — characteristics rarely change
/// at generation time anyway).
fn calculate_tier_level(base: i32, level: u8, kind: TierKind, rng: &mut impl Rng) -> i32 {
    if base <= 0 { return 0; }
    let (cap_normal, cap_exp, cap_master, cap_hero, growth) = match kind {
        TierKind::Attribute => (3, 5, 6, 7, 0.6_f32),
        TierKind::Skill     => (5, 7, 9, 11, 0.8_f32),
    };
    let cap = match level {
        1..=2 => cap_normal,
        3..=4 => cap_exp,
        5..=6 => cap_master,
        _     => cap_hero,
    };
    let lvl_mult = match level {
        1 => 0.7_f32,
        2 => 0.95,
        3 => 1.2,
        4 => 1.45,
        5 => 1.7,
        6 => 1.95,
        _ => 2.2,
    };
    let mut out = (base as f32 * lvl_mult * growth).floor() as i32;
    out += (rng.d6() as i32 % 3) - 1; // -1, 0, +1
    // Soft floors so trained NPCs aren't pushed below "competent".
    let min = match kind {
        TierKind::Attribute => match level { 1..=2 => 1, 3..=4 => 2, _ => 3 },
        TierKind::Skill     => match level { 1..=2 => 2, 3..=4 => 3, _ => 4 },
    };
    if out < min { out = min; }
    if out > cap { out = cap; }
    out
}

fn attribute_char(attr_name: &str) -> &'static str {
    match attribute_parent(attr_name) {
        Some(Char::Body)   => "BODY",
        Some(Char::Mind)   => "MIND",
        Some(Char::Spirit) => "SPIRIT",
        None => "BODY",
    }
}

// ---------------------------------------------------------------- helpers

fn oroll(rng: &mut impl Rng) -> i32 {
    crate::dice::o6(rng).total
}

fn avg(rng: &mut impl Rng) -> i32 {
    // Average d6 — ~3.5
    (rng.d6() as i32 + crate::dice::o6(rng).total) / 2
}

// `OpenSkill` import keeps the module self-contained even though
// no slot writes are issued from generated NPCs (yet).
#[allow(dead_code)]
fn _force_use(_: OpenSkill) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warrior_has_combat_skills() {
        let c = build_npc("Warrior", 3, "M");
        assert_eq!(c.race, "Human");
        // Warrior template: Wield Weapon base 3 → at level 3 should be > 0.
        assert!(c.skill("Strength", "Wield Weapon") > 0);
        // Should have at least one melee weapon equipped.
        assert!(c.weapons.iter().any(|w|
            matches!(w.kind, WeaponKind::Melee) && !w.name.is_empty()));
    }

    #[test]
    fn npc_age_grows_with_level() {
        let young = build_npc("Commoner", 1, "M");
        let veteran = build_npc("Commoner", 6, "M");
        assert!(young.age < veteran.age);
    }

    #[test]
    fn elf_warrior_inherits_race() {
        let c = build_npc("Elf: Warrior", 3, "F");
        assert_eq!(c.race, "Elf");
    }

    // -------- Wiki conformance: rules from data/lore/character.md.

    /// Wiki: every character starts with Spoken Language 2 in their
    /// native tongue, regardless of build. This must hold for any
    /// chartype + level combination.
    #[test]
    fn wiki_spoken_language_floor_is_two() {
        for ct in &["Warrior", "Mage", "Commoner", "Farmer", "Sage",
                    "Dwarf: Warrior", "Elf: Wizard", "Goblin: Thief",
                    "Troll: Warrior", "Faerie: Mage"] {
            for lvl in 1..=6 {
                let c = build_npc(ct, lvl, "M");
                let v = c.skill("Social Knowledge", "Spoken Language");
                assert!(v >= 2, "{} level {}: Spoken Language = {}, expected >= 2",
                    ct, lvl, v);
            }
        }
    }

    /// Wiki: BP = SIZE × 2 + Fortitude_total / 3. Verify the formula
    /// holds for a generated NPC.
    #[test]
    fn wiki_bp_formula_holds() {
        for ct in &["Warrior", "Dwarf: Warrior", "Mage"] {
            let c = build_npc(ct, 4, "M");
            let fort = c.skill_total(crate::pc::Char::Body, "Endurance", "Fortitude");
            let expected = (c.size * 2.0).floor() as i32 + fort / 3;
            assert_eq!(c.bp_max(), expected,
                "{}: BP {}, expected {} (SIZE {} × 2 + Fortitude {} / 3)",
                ct, c.bp_max(), expected, c.size, fort);
        }
    }

    /// Wiki: DB = (SIZE + Wield Weapon_total) / 3.
    #[test]
    fn wiki_db_formula_holds() {
        let c = build_npc("Warrior", 4, "M");
        let ww = c.skill_total(crate::pc::Char::Body, "Strength", "Wield Weapon");
        let expected = ((c.size + ww as f32) / 3.0).floor() as i32;
        assert_eq!(c.db(), expected);
    }

    /// Wiki: MD = (Mental Fortitude_total + Attunement Self_total) / 3.
    #[test]
    fn wiki_md_formula_holds() {
        let c = build_npc("Mage", 4, "M");
        let mf = c.skill_total(crate::pc::Char::Mind, "Willpower", "Mental Fortitude");
        let att = c.skill_total(crate::pc::Char::Spirit, "Attunement", "Self");
        let expected = (mf + att) / 3;
        assert_eq!(c.md(), expected);
    }

    /// Wiki race bonuses (baked into chartype templates):
    /// Dwarf +1 BODY characteristic, Elf +1 SPIRIT characteristic.
    /// Verify the templated chartypes reflect this against their
    /// human counterparts.
    #[test]
    fn wiki_race_bonuses_baked_into_templates() {
        let human  = crate::forge::data::chartype("Warrior").unwrap();
        let dwarf  = crate::forge::data::chartype("Dwarf: Warrior").unwrap();
        let elf    = crate::forge::data::chartype("Elf: Warrior").unwrap();
        // Dwarf +1 BODY vs human warrior baseline.
        assert!(dwarf.body >= human.body + 1,
            "Dwarf BODY ({}) should be >= Human BODY ({}) + 1", dwarf.body, human.body);
        // Elf +1 SPIRIT vs human warrior baseline (which is 0).
        assert!(elf.spirit >= human.spirit + 1,
            "Elf SPIRIT ({}) should be >= Human SPIRIT ({}) + 1",
            elf.spirit, human.spirit);
    }

    /// Wiki: skill total = char + attr + skill rank (every roll uses
    /// this). Verify a generated NPC's totals match the formula.
    #[test]
    fn wiki_skill_total_is_sum_of_three_tiers() {
        let c = build_npc("Warrior", 4, "M");
        let body = c.ch(crate::pc::Char::Body);
        let str_attr = c.attr("Strength");
        let ww_skill = c.skill("Strength", "Wield Weapon");
        let total = c.skill_total(crate::pc::Char::Body, "Strength", "Wield Weapon");
        assert_eq!(total, body + str_attr + ww_skill);
    }

    /// Wiki: Characteristic 0-3 typical (max ~5 for legendary). Even
    /// at level 6 our chartype templates shouldn't exceed 5. (Race
    /// templates can hit 5: Troll BODY 5, Faerie SPIRIT 4.)
    #[test]
    fn wiki_characteristics_within_canon_range() {
        for ct in crate::forge::data::CHARTYPES {
            let max = ct.body.max(ct.mind.max(ct.spirit));
            assert!(max <= 5, "{}: characteristic max {} exceeds wiki cap of 5",
                ct.name, max);
        }
    }

    /// Wiki attribute range 0-5. Bigger templates (e.g. Berserker
    /// Strength 5, Faerie Athletics 5) sit at the upper edge.
    #[test]
    fn wiki_attribute_bases_within_canon_range() {
        for ct in crate::forge::data::CHARTYPES {
            for (path, base) in ct.attributes {
                assert!(*base >= 0 && *base <= 6,
                    "{}: attribute {} base {} outside wiki 0-5 range (allow up to 6 for monsters)",
                    ct.name, path, base);
            }
        }
    }

    /// Wiki: Wound thresholds — 1/2 BP wounded, 1/4 BP heavily.
    /// Verify we can still compute BP for a generated NPC and it
    /// stays positive.
    #[test]
    fn wiki_wound_thresholds_reachable() {
        let c = build_npc("Warrior", 3, "M");
        let bp = c.bp_max();
        assert!(bp > 0);
        assert!(bp / 2 < bp);  // Wounded threshold makes sense.
        assert!(bp / 4 < bp / 2); // HW threshold lower than W.
    }
}
