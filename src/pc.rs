//! Player-character data model. 3-tier (Characteristic → Attribute → Skill)
//! per d6gaming.org/The_Character.
//!
//! All NPCs use the same structure; PCs are NPCs with `is_pc = true` and
//! a player name attached. The full attribute / skill list is built at
//! load time from the wiki canonical breakdown so we never miss a skill
//! and so adding a new skill in the future is a one-line change.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Char {
    Body,
    Mind,
    Spirit,
}

impl Char {
    pub fn name(self) -> &'static str {
        match self {
            Char::Body => "BODY",
            Char::Mind => "MIND",
            Char::Spirit => "SPIRIT",
        }
    }
    pub fn all() -> [Char; 3] { [Char::Body, Char::Mind, Char::Spirit] }
}

/// A skill is identified by its parent characteristic, parent attribute
/// name, and its own name. We keep the canonical structure in static
/// tables so the editor can render the tree without parsing names.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SkillId {
    pub parent_char: Char,
    pub attribute: String,
    pub skill: String,
}

pub const ATTRIBUTES: &[(Char, &str)] = &[
    // BODY
    (Char::Body, "Strength"),
    (Char::Body, "Endurance"),
    (Char::Body, "Athletics"),
    (Char::Body, "Melee Combat"),
    (Char::Body, "Missile Combat"),
    (Char::Body, "Sleight"),
    // MIND — Intelligence dropped; its sub-skills now live in the
    // free-form 4-slot section per char column.
    (Char::Mind, "Nature Knowledge"),
    (Char::Mind, "Social Knowledge"),
    (Char::Mind, "Practical Knowledge"),
    (Char::Mind, "Awareness"),
    (Char::Mind, "Willpower"),
    // SPIRIT — Innate dropped; rare per-race ability, not a default.
    (Char::Spirit, "Casting"),
    (Char::Spirit, "Attunement"),
    (Char::Spirit, "Worship"),
];

/// Skills under each attribute. Verbatim from d6gaming.org/The_Character
/// minus a few defaults the user prefers in the free-form slots:
/// Plant Lore, Animal Lore, Courage, Hold Breath. The slots show 4
/// open rows per char column where the user picks an attribute, names
/// the skill and sets a rank, so any of these can be added back
/// per-character without cluttering the canonical list.
pub const SKILLS: &[(&str, &[&str])] = &[
    // Body
    ("Strength",       &["Carrying", "Weight Lifting", "Wield Weapon"]),
    ("Endurance",      &["Fortitude", "Combat Tenacity", "Running", "Poison Resistance"]),
    ("Athletics",      &["Hide", "Move Quietly", "Climb", "Swim", "Ride", "Jump", "Balance", "Tumble", "Dodge"]),
    ("Melee Combat",   &[]),  // weapon skills added per-PC as they're acquired
    ("Missile Combat", &[]),
    ("Sleight",        &["Pick Pockets", "Disarm Traps"]),
    // Mind
    ("Nature Knowledge", &["Medical Lore", "Animal Handling", "Magick Rituals", "Alchemy"]),
    ("Social Knowledge", &["Social Lore", "Spoken Language", "Literacy", "Mythology", "Legend Lore"]),
    ("Practical Knowledge", &["Survival Lore", "Set Traps", "Ambush"]),
    // Awareness: defaults trimmed to the four most-used skills.
    // The five wiki-canonical extras (Sense Emotions, Sense Ambush,
    // Sense of Direction, Sense Magick, Listening) cluttered the
    // sheet for most PCs; users can add them back via the open slots
    // if their character actually trains them.
    ("Awareness",        &["Reaction Speed", "Alertness", "Tracking", "Detect Traps"]),
    ("Willpower",        &["Pain Tolerance", "Mental Fortitude"]),
    // Spirit
    ("Casting",    &["Range", "Duration", "Area of Effect", "Weight", "Number of Targets"]),
    // Attunement skills (Self, Fire, Water, Air, Earth, Life, Death,
    // Mind, Body) live in the generic open-slot section under SPIRIT
    // — most PCs only train a couple, so the canonical list cluttered
    // the column for everyone else.
    ("Attunement", &[]),
    ("Worship",    &[]),  // god names added per-PC
];

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum WeaponKind {
    #[default]
    Melee,
    Missile,
}

/// One weapon slot on a PC sheet. Mirrors the Mellee/Missile blocks on
/// CharacterSheet-new.xml: H (one/two-handed), Init, ±O, ±D (melee
/// only) or shots-per-round (missile only), OFF, DEF, Dam, HP, plus
/// xp marks.
///
/// `skill_name` is the per-weapon skill (e.g. "Sword", "Bow") used to
/// derive OFF / DEF totals. The skill rank is read from
/// `pc.skills["Melee Combat"][skill_name]` (or "Missile Combat") so
/// weapon-skill bookkeeping flows through the normal skill system.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)] // sparse weapon JSON (name + damage, say) loads fine
pub struct Weapon {
    pub name: String,
    pub kind: WeaponKind,
    pub skill_name: String,
    pub two_handed: bool,
    pub init: i32,
    pub off_mod: i32,
    pub def_mod: i32,
    pub shots_per_round: u8,
    pub damage: i32,
    pub hp: i32,
    pub range_m: u32,
    pub xp: i32,
}

/// One spell on a PC sheet. Captures the printed-character-sheet
/// fields so a GM can record house-ruled spells alongside the
/// canonical wiki ones. Mirrors the canon entry shape so the lookup
/// in `add_spell` can pre-fill every field for known spells.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Spell {
    pub name: String,
    #[serde(default)] pub domain: String,        // Fire / Water / Air / …
    #[serde(default)] pub active_passive: String, // "Active" or "Passive"
    #[serde(default)] pub dr: i32,
    #[serde(default)] pub cost: i32,
    #[serde(default)] pub casting_time: String,
    #[serde(default)] pub distance: String,
    #[serde(default)] pub duration: String,
    #[serde(default)] pub area: String,
    #[serde(default)] pub cooldown: String,
    #[serde(default)] pub effects: String,
}

/// One open / free-form skill slot. Lives in the generic slot area
/// under the SPIRIT column on the PC sheet — but the slot itself can
/// target any characteristic + attribute (so e.g. a "Singing"
/// Athletics-under-BODY skill, or a "Folklore" Social Knowledge-under-
/// MIND skill, both fit). Total = char + attr + rank.
///
/// `#[serde(default)]` applies to every field so old per-char-map
/// saves (which had no `parent_char`) still load — the flatten step
/// in `deserialize_open_skills` stamps `parent_char` from the map key.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OpenSkill {
    /// "BODY" / "MIND" / "SPIRIT" (also accepts "B" / "M" / "S" on input).
    pub parent_char: String,
    pub attribute: String,
    pub name: String,
    pub rank: i32,
}

/// Per-location armor + AP, per the hit-location table on the
/// character sheet. d6 hit-location roll: 6 head, 5 R-arm, 4 L-arm,
/// 3 body, 2 R-leg, 1 L-leg.
///
/// Per-location BP is NOT stored — it derives from the character's
/// total BP via the wiki rule "50% of BP in head + arms, 80% in body
/// + legs", computed at render time by `bp_for_location`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HitLocation {
    pub armor: String,
    pub ap: i32,
}

/// Per-hit-location BP from the character's total BP. Wiki rule
/// (Combat → Hit Locations): "A human has 50 % of his body points
/// in the head and arms and 80 % in the body and legs." Rounds up
/// so a 7 BP human gets Head/Arms = 4, Body/Legs = 6.
pub fn bp_for_location(total_bp: i32, loc: &str) -> i32 {
    let factor = if loc == "Head" || loc.contains("Arm") { 0.5 } else { 0.8 };
    ((total_bp as f32 * factor).ceil()) as i32
}

// Container-level serde(default): a sparse "short form" JSON — name, level,
// a few skills, weapons — deserializes cleanly, with every omitted field
// defaulted. Campaign::load then runs `normalize()` on each character so
// the sparse import becomes a fully-working sheet. This is the companion
// Claude Code session's injection format (see CLAUDE.md, data contract).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Character {
    // Identity
    pub name: String,
    pub player: String,
    pub is_pc: bool,
    /// Inactive PCs sit out of the campaign: dimmed on the roster and
    /// skipped when a combat pulls in "all PCs". Toggled with `a` on
    /// the PC row. Defaults to true so existing sheets stay active.
    #[serde(default = "default_active")]
    pub active: bool,
    pub race: String,
    pub gender: String,
    pub age: u32,
    pub height_cm: u32,
    pub weight_kg: u32,
    pub birthplace: String,
    /// Kingdom region/district the character is tied to (e.g.
    /// "Amaronir"). Empty = not tied to a region; the World tab groups
    /// those under "Other".
    pub region: String,
    /// World location (city/village name) the character belongs to;
    /// the World tree nests the NPC under that location row. Empty =
    /// listed directly under the region fold.
    pub location: String,
    pub description: String,
    pub clothing: String,

    // Derived foundation
    pub size: f32,
    pub level: u8,

    // 3-tier abilities
    pub characteristics: BTreeMap<String, i32>,
    pub attributes: BTreeMap<String, i32>,
    pub skills: BTreeMap<String, BTreeMap<String, i32>>,

    // Live state
    pub bp_current: i32,
    pub mf_current: i32,
    pub conditions: Vec<String>,
    /// Situational adjustments (label, value) — the "Other Modifiers"
    /// block on the character sheet. Eight slots are normal but we
    /// store any number.
    pub modifiers: Vec<(String, i32)>,

    /// Absolute path to the PC's portrait image. Set when the user
    /// generates one via the P key (clipboard → ChatGPT path import,
    /// or DALL-E / Gemini direct API). Empty string means no
    /// portrait — the placeholder frame is shown instead.
    #[serde(default)]
    pub portrait_path: String,

    // Equipment
    pub hit_locations: BTreeMap<String, HitLocation>,
    pub weapons: Vec<Weapon>,
    /// Spells the PC knows. Backwards-compatible deserializer accepts
    /// the legacy `Vec<String>` shape and upgrades to the full Spell
    /// struct (other fields default to 0 / empty).
    #[serde(default, deserialize_with = "deserialize_spells")]
    pub spells: Vec<Spell>,
    pub equipment: Vec<String>,
    pub money_sp: i32,

    /// Generic open / free-form skill slots. All slots live in one
    /// flat list rendered under the SPIRIT column on the PC sheet
    /// (which has plenty of vertical space since Attunement no longer
    /// has canonical skills). Each slot independently picks its
    /// parent characteristic + attribute, so a "Singing" slot can
    /// roll under BODY/Athletics while a "Folklore" slot rolls under
    /// MIND/Social Knowledge.
    ///
    /// Backwards-compat deserializer accepts the original map shape
    /// (`{ "BODY": [...], "MIND": [...], "SPIRIT": [...] }`) and
    /// flattens it into the flat list, stamping `parent_char` from
    /// the map key.
    #[serde(default, deserialize_with = "deserialize_open_skills")]
    pub open_skills: Vec<OpenSkill>,

    pub notes: String,
}

/// Backwards-compat deserializer: accepts either `["Fireball", ...]`
/// (old format) or `[{ "name": "Fireball", "dr": 4, ... }, ...]`
/// (new format). Bare strings become Spells with their stat fields
/// zeroed / empty so existing saves load cleanly.
fn deserialize_spells<'de, D>(d: D) -> Result<Vec<Spell>, D::Error>
where D: serde::Deserializer<'de> {
    use serde::Deserialize;
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum SpellRepr { Name(String), Full(Spell) }
    let raw: Vec<SpellRepr> = Vec::deserialize(d)?;
    Ok(raw.into_iter().map(|r| match r {
        SpellRepr::Name(n) => Spell { name: n, ..Default::default() },
        SpellRepr::Full(s) => s,
    }).collect())
}

/// Backwards-compat deserializer for `open_skills`. Old saves had a
/// per-char map (`{ "BODY": [...], ... }`); new saves are a flat
/// `Vec<OpenSkill>`. We accept either shape, flattening the map
/// into the vec and stamping `parent_char` from the map key.
fn deserialize_open_skills<'de, D>(d: D) -> Result<Vec<OpenSkill>, D::Error>
where D: serde::Deserializer<'de> {
    use serde::Deserialize;
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OpenSkillsRepr {
        Flat(Vec<OpenSkill>),
        ByChar(BTreeMap<String, Vec<OpenSkill>>),
    }
    Ok(match OpenSkillsRepr::deserialize(d)? {
        OpenSkillsRepr::Flat(v) => v,
        OpenSkillsRepr::ByChar(m) => m.into_iter()
            .flat_map(|(ch, mut v)| {
                for s in v.iter_mut() {
                    if s.parent_char.is_empty() { s.parent_char = ch.clone(); }
                }
                v.into_iter()
            })
            .collect(),
    })
}

/// Hit-location names in d6-roll order: 6 Head, 5 R. Arm, 4 L. Arm,
/// 3 Body, 2 R. Leg, 1 L. Leg.
pub const HIT_LOCATIONS: &[&str] = &[
    "Head", "R. Arm", "L. Arm", "Body", "R. Leg", "L. Leg",
];

fn default_active() -> bool { true }

fn parse_u(s: &str, label: &str) -> Result<u32, String> {
    s.parse::<u32>().map_err(|e| format!("bad {}: {}", label, e))
}

fn parse_i(s: &str, label: &str) -> Result<i32, String> {
    s.parse::<i32>().map_err(|e| format!("bad {}: {}", label, e))
}

/// SIZE from weight — wiki **Half-Size Points** table, used throughout
/// amar (NPCs and PCs alike). The half-size table is the wiki's
/// canonical granular weight table; SIZE 3.5 covers 75-99 kg, SIZE
/// 4.5 covers 125-149 kg, and so on. Above 499 kg the half-size table
/// ends and the whole-size table picks up: 500-599 → 9, 600-724 → 10,
/// up to 16 < 1600. Beyond 1600 kg, +1 per +200 kg.
pub fn size_from_weight_kg(kg: u32) -> f32 {
    match kg {
        // Half-Size Points table (wiki, optional rule used by amar):
        0..=9     => 0.5,
        10..=14   => 1.0,
        15..=19   => 1.5,
        20..=34   => 2.0,
        35..=49   => 2.5,
        50..=74   => 3.0,
        75..=99   => 3.5,
        100..=124 => 4.0,
        125..=149 => 4.5,
        150..=187 => 5.0,
        188..=224 => 5.5,
        225..=262 => 6.0,
        263..=299 => 6.5,
        300..=349 => 7.0,
        350..=399 => 7.5,
        400..=449 => 8.0,
        450..=499 => 8.5,
        // Above the half-size table — whole-size canonical table:
        500..=599   => 9.0,
        600..=724   => 10.0,
        725..=849   => 11.0,
        850..=999   => 12.0,
        1000..=1149 => 13.0,
        1150..=1299 => 14.0,
        1300..=1449 => 15.0,
        1450..=1599 => 16.0,
        // Beyond 1600 kg — linear "+1 per +200 kg" extension.
        _ => 16.0 + ((kg - 1600) / 200) as f32 + 1.0,
    }
}

impl Character {
    /// Fill in everything a sparse (companion-injected) character omits,
    /// so a short-form JSON — name, level, a few skills, weapons — loads
    /// into a fully-working sheet. Idempotent: a complete character passes
    /// through untouched. `size == 0` marks a fresh sparse import (no real
    /// character has SIZE 0); only then are weight/SIZE/BP/MF/active
    /// seeded, so a saved character's wound state is never "healed" by a
    /// reload. Called on every character by `Campaign::load`.
    pub fn normalize(&mut self) {
        let fresh = self.size <= 0.0;
        if self.race.trim().is_empty() { self.race = "Human".into(); }
        if self.level == 0 { self.level = 1; }
        // Universal baselines — native tongue, fists, hit locations.
        self.skills.entry("Social Knowledge".into()).or_default()
            .entry("Spoken Language".into()).or_insert(2);
        self.skills.entry("Melee Combat".into()).or_default()
            .entry("Unarmed".into()).or_insert(1);
        if !self.weapons.iter().any(|w| w.name == "Unarmed") {
            let mut fists = Weapon::default();
            fists.name = "Unarmed".into();
            fists.skill_name = "Unarmed".into();
            self.weapons.insert(0, fists);
        }
        // A weapon without a governing skill uses its own name (the same
        // rule the sheet editor applies).
        for w in self.weapons.iter_mut() {
            if w.skill_name.trim().is_empty() { w.skill_name = w.name.clone(); }
        }
        for loc in HIT_LOCATIONS {
            self.hit_locations.entry((*loc).to_string()).or_default();
        }
        if fresh {
            if self.weight_kg == 0 { self.weight_kg = 70; }
            self.size = size_from_weight_kg(self.weight_kg);
            self.active = true;
            self.bp_current = self.bp_max();
            self.mf_current = self.mf_max().max(1);
        }
    }

    pub fn new_blank(name: &str) -> Self {
        let mut c = Character::default();
        c.active = true; // derived Default is false; every new sheet starts active
        c.name = name.to_string();
        c.race = "Human".into();
        // 70 kg → SIZE 3.0 in the half-size table — lean adult human,
        // matches the SIZE used by the example adventure NPCs.
        c.weight_kg = 70;
        c.size = size_from_weight_kg(70);
        c.level = 1;
        for ch in Char::all() { c.characteristics.insert(ch.name().to_string(), 0); }
        for (_, attr) in ATTRIBUTES { c.attributes.insert((*attr).to_string(), 0); }
        // Wiki rule: every character starts with 2 in their native
        // tongue (Social Knowledge → Spoken Language) regardless of
        // build. Stored under the canonical attribute key so the
        // sheet renderer picks it up.
        c.skills.insert("Social Knowledge".into(), {
            let mut m = BTreeMap::new();
            m.insert("Spoken Language".into(), 2);
            m
        });
        // Default hit locations: every body location starts unarmored.
        // Per-location BP shown on the sheet is informative — it
        // mirrors the wiki rule "50% of BP in head + arms, 80% in body
        // + legs" — but the canonical BP pool is `bp_max()`.
        for loc in HIT_LOCATIONS {
            c.hit_locations.insert((*loc).to_string(), HitLocation::default());
        }
        // Every character is always armed with their fists. Unarmed
        // gets a free slot on the weapons table (and a 1-rank skill
        // under Melee Combat) so the GM can resolve a punch without
        // having to add the weapon manually for each new sheet.
        c.skills.entry("Melee Combat".into()).or_default()
            .entry("Unarmed".into()).or_insert(1);
        c.weapons.push(Weapon {
            name: "Unarmed".into(),
            kind: WeaponKind::Melee,
            skill_name: "Unarmed".into(),
            two_handed: false,
            init: 0,
            off_mod: 0,
            def_mod: 0,
            shots_per_round: 0,
            damage: 0,
            hp: 0,
            range_m: 0,
            xp: 0,
        });
        c.bp_current = c.bp_max();
        c.mf_current = c.mf_max();
        c
    }

    /// Edit a field by string id. Used by the Campaign tab's inline
    /// editor to dispatch ENTER → set the value the user typed. Returns
    /// Ok(()) on success, Err(msg) on parse failure.
    ///
    /// Supported ids:
    ///   "name" "player" "race" "sex" "birthplace" "description"
    ///   "clothing" "notes"        — string fields
    ///   "age" "height" "money"     — integer fields
    ///   "weight"                   — integer kg, also re-derives SIZE
    ///   "level"                    — integer level
    ///   "char/<NAME>"              — characteristic rank (e.g. "char/BODY")
    ///   "attr/<Attr>"              — attribute rank (e.g. "attr/Strength")
    ///   "skill/<Attr>/<Skill>"     — skill rank (e.g. "skill/Strength/Carrying")
    ///   "bp_current" "mf_current"  — running pool values
    ///   "hit/<Loc>/armor"          — armor name for a hit location
    ///   "hit/<Loc>/ap" "hit/<Loc>/bp" — AP / BP per location
    pub fn set_field(&mut self, id: &str, value: &str) -> Result<(), String> {
        let trim = value.trim();
        match id {
            "name"        => self.name = trim.to_string(),
            "player"      => self.player = trim.to_string(),
            "race"        => self.race = trim.to_string(),
            "sex"         => self.gender = trim.to_string(),
            "birthplace"  => self.birthplace = trim.to_string(),
            "description" => {
                // Edit the first logical line only — preserve any
                // continuation lines that already exist.
                let rest: String = self.description.split_once('\n')
                    .map(|(_, after)| after.to_string())
                    .unwrap_or_default();
                self.description = if rest.is_empty() {
                    trim.to_string()
                } else {
                    format!("{}\n{}", trim, rest)
                };
            }
            "description2" => {
                // Edit the second logical line. Keep line 1 intact,
                // and any line 3+ that may already exist.
                let mut lines: Vec<String> = self.description.lines().map(|s| s.to_string()).collect();
                while lines.len() < 2 { lines.push(String::new()); }
                lines[1] = trim.to_string();
                // Trim trailing empty lines so a freshly-blanked line
                // 2 doesn't leave a dangling "\n" in the saved value.
                while lines.last().map_or(false, |s| s.is_empty()) {
                    lines.pop();
                }
                self.description = lines.join("\n");
            }
            "clothing"    => self.clothing = trim.to_string(),
            "notes"       => self.notes = trim.to_string(),
            "age"     => self.age = parse_u(trim, "age")?,
            "height"  => self.height_cm = parse_u(trim, "height")?,
            "money"   => self.money_sp = parse_i(trim, "money")?,
            "level"   => self.level = parse_u(trim, "level")? as u8,
            "weight" => {
                let kg = parse_u(trim, "weight")?;
                self.weight_kg = kg;
                self.size = size_from_weight_kg(kg);
            }
            "bp_current" => self.bp_current = parse_i(trim, "bp_current")?,
            "mf_current" => self.mf_current = parse_i(trim, "mf_current")?,
            other => {
                if let Some(name) = other.strip_prefix("char/") {
                    let v = parse_i(trim, "characteristic")?;
                    self.characteristics.insert(name.to_string(), v);
                } else if let Some(name) = other.strip_prefix("attr/") {
                    let v = parse_i(trim, "attribute")?;
                    self.attributes.insert(name.to_string(), v);
                } else if let Some(rest) = other.strip_prefix("skill/") {
                    let mut parts = rest.splitn(2, '/');
                    let attr = parts.next().ok_or("skill id missing attribute")?;
                    let skill = parts.next().ok_or("skill id missing skill name")?;
                    let v = parse_i(trim, "skill")?;
                    self.skills.entry(attr.to_string())
                        .or_default()
                        .insert(skill.to_string(), v);
                } else if let Some(rest) = other.strip_prefix("hit/") {
                    let mut parts = rest.splitn(2, '/');
                    let loc = parts.next().ok_or("hit id missing location")?.to_string();
                    let kind = parts.next().ok_or("hit id missing field")?;
                    let entry = self.hit_locations.entry(loc).or_default();
                    match kind {
                        "armor" => entry.armor = trim.to_string(),
                        "ap"    => entry.ap = parse_i(trim, "AP")?,
                        _ => return Err(format!("unknown hit field: {}", kind)),
                    }
                } else if let Some(rest) = other.strip_prefix("weapon/") {
                    // weapon/<idx>/<field>
                    let mut parts = rest.splitn(2, '/');
                    let idx: usize = parts.next().ok_or("weapon id missing idx")?
                        .parse().map_err(|_| "weapon idx not a number")?;
                    let field = parts.next().ok_or("weapon id missing field")?;
                    // "skillrank" sets the CHARACTER's skill rank for this
                    // weapon's governing skill (a number folded into OFF/DEF),
                    // not a field on the weapon itself — so it writes into
                    // self.skills. Handled first to avoid holding a mutable
                    // borrow of self.weapons while touching self.skills.
                    if field == "skillrank" {
                        let rank = parse_i(trim, "skill rank")?;
                        let (attr, skill_name) = {
                            let w = self.weapons.get(idx)
                                .ok_or_else(|| format!("weapon idx {} out of range", idx))?;
                            let attr = match w.kind {
                                WeaponKind::Melee   => "Melee Combat",
                                WeaponKind::Missile => "Missile Combat",
                            };
                            // The governing skill defaults to the weapon's own
                            // name when unset (a "Trident" uses "Trident").
                            let sn = if w.skill_name.trim().is_empty() {
                                w.name.clone()
                            } else {
                                w.skill_name.clone()
                            };
                            (attr, sn)
                        };
                        if skill_name.trim().is_empty() {
                            return Err("weapon has no name to hang a skill on".into());
                        }
                        // Keep the weapon linked to that skill name.
                        if let Some(w) = self.weapons.get_mut(idx) {
                            if w.skill_name.trim().is_empty() { w.skill_name = skill_name.clone(); }
                        }
                        self.skills.entry(attr.to_string()).or_default()
                            .insert(skill_name, rank);
                    } else {
                        let w = self.weapons.get_mut(idx)
                            .ok_or_else(|| format!("weapon idx {} out of range", idx))?;
                        match field {
                            "name"        => w.name = trim.to_string(),
                            "skill"       => w.skill_name = trim.to_string(),
                            "two_handed"  => {
                                w.two_handed = matches!(trim.to_ascii_lowercase().as_str(),
                                    "y"|"yes"|"2h"|"true"|"t"|"1");
                            }
                            "init"        => w.init = parse_i(trim, "init")?,
                            "off_mod"     => w.off_mod = parse_i(trim, "off_mod")?,
                            "def_mod"     => w.def_mod = parse_i(trim, "def_mod")?,
                            "shots"       => w.shots_per_round = trim.parse().map_err(|e| format!("bad shots: {}", e))?,
                            "damage"      => w.damage = parse_i(trim, "damage")?,
                            "range"       => w.range_m = parse_u(trim, "range")?,
                            "hp"          => w.hp = parse_i(trim, "hp")?,
                            "xp"          => w.xp = parse_i(trim, "xp")?,
                            _ => return Err(format!("unknown weapon field: {}", field)),
                        }
                    }
                } else if let Some(rest) = other.strip_prefix("spell/") {
                    // spell/<idx>/<field>
                    let mut parts = rest.splitn(2, '/');
                    let idx: usize = parts.next().ok_or("spell id missing idx")?
                        .parse().map_err(|_| "spell idx not a number")?;
                    let field = parts.next().ok_or("spell id missing field")?;
                    let sp = self.spells.get_mut(idx)
                        .ok_or_else(|| format!("spell idx {} out of range", idx))?;
                    match field {
                        "name"           => sp.name = trim.to_string(),
                        "domain"         => sp.domain = trim.to_string(),
                        "active_passive" => sp.active_passive = trim.to_string(),
                        "dr"             => sp.dr = parse_i(trim, "DR")?,
                        "cost"           => sp.cost = parse_i(trim, "cost")?,
                        "casting_time"   => sp.casting_time = trim.to_string(),
                        "distance"       => sp.distance = trim.to_string(),
                        "duration"       => sp.duration = trim.to_string(),
                        "area"           => sp.area = trim.to_string(),
                        "cooldown"       => sp.cooldown = trim.to_string(),
                        "effects"        => sp.effects = trim.to_string(),
                        _ => return Err(format!("unknown spell field: {}", field)),
                    }
                } else if let Some(rest) = other.strip_prefix("slot/") {
                    // slot/<idx>/<field>
                    let mut parts = rest.splitn(2, '/');
                    let idx: usize = parts.next().ok_or("slot id missing idx")?
                        .parse().map_err(|_| "slot idx not a number")?;
                    let field = parts.next().ok_or("slot id missing field")?;
                    while self.open_skills.len() <= idx { self.open_skills.push(OpenSkill::default()); }
                    let s = &mut self.open_skills[idx];
                    match field {
                        "char" => {
                            // Accept B/M/S shortcuts as well as full names.
                            let v = trim.to_ascii_uppercase();
                            s.parent_char = match v.as_str() {
                                "B" | "BODY"   => "BODY".into(),
                                "M" | "MIND"   => "MIND".into(),
                                "S" | "SPIRIT" => "SPIRIT".into(),
                                ""             => String::new(),
                                _ => return Err(format!("unknown char: {} (expected BODY/MIND/SPIRIT)", trim)),
                            };
                        }
                        "attribute" => s.attribute = trim.to_string(),
                        "name"      => s.name = trim.to_string(),
                        "rank"      => s.rank = parse_i(trim, "rank")?,
                        _ => return Err(format!("unknown slot field: {}", field)),
                    }
                } else {
                    return Err(format!("unknown field id: {}", id));
                }
            }
        }
        // BP/MF current can never exceed their respective maxes after
        // a field change — keep them in range so the wound-state
        // computation stays sane.
        let bp_cap = self.bp_max();
        if self.bp_current > bp_cap { self.bp_current = bp_cap; }
        let mf_cap = self.mf_max();
        if self.mf_current > mf_cap { self.mf_current = mf_cap; }
        Ok(())
    }

    pub fn ch(&self, c: Char) -> i32 {
        *self.characteristics.get(c.name()).unwrap_or(&0)
    }
    pub fn attr(&self, name: &str) -> i32 {
        *self.attributes.get(name).unwrap_or(&0)
    }
    pub fn skill(&self, attr: &str, skill: &str) -> i32 {
        self.skills.get(attr).and_then(|m| m.get(skill)).copied().unwrap_or(0)
    }

    /// Total Skill Value = Characteristic + Attribute + Skill rank.
    /// Every roll in the system uses this total — never just the
    /// Characteristic, never Char+Attr — even when the skill rank is
    /// 0. Skills not yet tracked on the character resolve to 0 here,
    /// so the caller still gets the Char+Attr+0 baseline.
    pub fn skill_total(&self, parent_char: Char, attr: &str, skill: &str) -> i32 {
        self.ch(parent_char) + self.attr(attr) + self.skill(attr, skill)
    }

    /// BP = SIZE × 2 + (Fortitude_total / 3).
    /// Fortitude_total = BODY + Endurance + Fortitude (skill rank).
    /// Half-size SIZE values (e.g. 3.5) flow through the formula as
    /// floats; the final result is floored to i32 the same way the
    /// wiki's "/3" implies integer division.
    pub fn bp_max(&self) -> i32 {
        let fort_total = self.skill_total(Char::Body, "Endurance", "Fortitude");
        (self.size * 2.0).floor() as i32 + fort_total / 3
    }

    /// DB = (SIZE + Wield Weapon_total) / 3.
    /// Wield Weapon_total = BODY + Strength + Wield Weapon (skill rank).
    pub fn db(&self) -> i32 {
        let ww_total = self.skill_total(Char::Body, "Strength", "Wield Weapon");
        ((self.size + ww_total as f32) / 3.0).floor() as i32
    }

    /// MD = (Mental Fortitude_total + Attunement Self_total) / 3.
    /// Each side is a full Char+Attr+Skill total per the system rule.
    pub fn md(&self) -> i32 {
        let mf_total = self.skill_total(Char::Mind, "Willpower", "Mental Fortitude");
        let aself_total = self.skill_total(Char::Spirit, "Attunement", "Self");
        (mf_total + aself_total) / 3
    }

    /// Reaction Speed total = MIND + Awareness + Reaction Speed.
    /// (Rolled with an O6 added at table.)
    pub fn reaction(&self) -> i32 {
        self.skill_total(Char::Mind, "Awareness", "Reaction Speed")
    }

    /// Mental Fortitude total = MIND + Willpower + Mental Fortitude.
    /// This is the "active spell capacity" cap. Recovery is 1 point
    /// per hour rest; full recovery on 8 h sleep. The caller manages
    /// the running pool against this cap.
    pub fn mf_max(&self) -> i32 {
        self.skill_total(Char::Mind, "Willpower", "Mental Fortitude")
    }
}

/// Find an attribute's parent characteristic. None for unknown names.
pub fn attribute_parent(name: &str) -> Option<Char> {
    ATTRIBUTES.iter().find(|(_, n)| *n == name).map(|(c, _)| *c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_pc_has_canonical_structure() {
        let c = Character::new_blank("Test");
        assert_eq!(c.size, 3.0);   // default 75 kg → SIZE 3
        assert_eq!(c.bp_max(), 6); // SIZE 3 * 2 + 0/3
        // Wiki rule: native tongue starts at 2.
        assert_eq!(c.skill("Social Knowledge", "Spoken Language"), 2);
    }

    #[test]
    fn skill_total_combines_three_tiers() {
        let mut c = Character::new_blank("Test");
        c.characteristics.insert("BODY".into(), 2);
        c.attributes.insert("Strength".into(), 3);
        c.skills.entry("Strength".into()).or_default().insert("Carrying".into(), 4);
        // Wiki example: BODY 2 + Strength 3 + Carrying 4 = 9.
        assert_eq!(c.skill_total(Char::Body, "Strength", "Carrying"), 9);
    }

    #[test]
    fn weapon_skillrank_field_sets_character_skill_rank() {
        let mut c = Character::new_blank("Test");
        // A trident: governing skill defaults to the weapon name.
        c.weapons.push(Weapon {
            name: "Trident".into(), kind: WeaponKind::Melee,
            skill_name: "Trident".into(), two_handed: false,
            init: 0, off_mod: 0, def_mod: 0, shots_per_round: 0,
            damage: 0, hp: 7, range_m: 0, xp: 0,
        });
        let idx = c.weapons.len() - 1; // after the free Unarmed slot
        // The Skill column edits the RANK (a number), not the name.
        c.set_field(&format!("weapon/{}/skillrank", idx), "5").unwrap();
        assert_eq!(c.skill("Melee Combat", "Trident"), 5);
        // A missing skill_name falls back to the weapon name.
        c.weapons.push(Weapon {
            name: "Short Bow".into(), kind: WeaponKind::Missile,
            skill_name: String::new(), two_handed: true,
            init: 0, off_mod: 0, def_mod: 0, shots_per_round: 1,
            damage: 0, hp: 0, range_m: 30, xp: 0,
        });
        let bow = c.weapons.len() - 1;
        c.set_field(&format!("weapon/{}/skillrank", bow), "3").unwrap();
        assert_eq!(c.skill("Missile Combat", "Short Bow"), 3);
        assert_eq!(c.weapons[bow].skill_name, "Short Bow"); // linked back
    }

    #[test]
    fn skill_total_uses_char_attr_baseline_when_skill_untracked() {
        let mut c = Character::new_blank("Test");
        c.characteristics.insert("BODY".into(), 1);
        c.attributes.insert("Athletics".into(), 2);
        // No "Climb" rank set on the character.
        // Wiki rule: every roll uses Char+Attr+Skill, even with skill = 0.
        assert_eq!(c.skill_total(Char::Body, "Athletics", "Climb"), 3);
    }

    #[test]
    fn derived_stats_use_full_skill_totals() {
        // Each derived stat must consult the Char+Attr+Skill total,
        // not the bare skill rank. We populate all three tiers and
        // check the result reflects the sum.
        let mut c = Character::new_blank("Test");
        c.size = 3.0;
        c.characteristics.insert("BODY".into(), 1);
        c.characteristics.insert("MIND".into(), 1);
        c.characteristics.insert("SPIRIT".into(), 1);
        c.attributes.insert("Endurance".into(), 2);
        c.attributes.insert("Strength".into(), 2);
        c.attributes.insert("Willpower".into(), 2);
        c.attributes.insert("Attunement".into(), 2);
        c.skills.entry("Endurance".into()).or_default().insert("Fortitude".into(), 3);
        c.skills.entry("Strength".into()).or_default().insert("Wield Weapon".into(), 1);
        c.skills.entry("Willpower".into()).or_default().insert("Mental Fortitude".into(), 3);
        c.skills.entry("Attunement".into()).or_default().insert("Self".into(), 2);

        // Fortitude total = 1+2+3 = 6 → BP = 3*2 + 6/3 = 8
        assert_eq!(c.bp_max(), 8);
        // Wield Weapon total = 1+2+1 = 4 → DB = (3 + 4) / 3 = 2
        assert_eq!(c.db(), 2);
        // MF total = 1+2+3 = 6, Attunement Self total = 1+2+2 = 5
        // → MD = (6 + 5) / 3 = 11/3 = 3
        assert_eq!(c.md(), 3);
    }

    #[test]
    fn half_size_flows_through_formulas() {
        let mut c = Character::new_blank("Halfling");
        c.size = 1.5;     // Optional Half-Size Points rule, NPC use
        c.characteristics.insert("BODY".into(), 1);
        c.attributes.insert("Endurance".into(), 1);
        c.skills.entry("Endurance".into()).or_default().insert("Fortitude".into(), 1);
        // Fortitude total = 1+1+1 = 3 → BP = floor(1.5 * 2) + 3/3 = 3 + 1 = 4
        assert_eq!(c.bp_max(), 4);

        c.attributes.insert("Strength".into(), 2);
        c.skills.entry("Strength".into()).or_default().insert("Wield Weapon".into(), 1);
        // Wield Weapon total = 1+2+1 = 4 → DB = floor((1.5 + 4) / 3) = floor(1.83) = 1
        assert_eq!(c.db(), 1);
    }

    #[test]
    fn size_from_weight_uses_half_size_table() {
        // Half-size table — granular up to 499 kg.
        assert_eq!(size_from_weight_kg(5),    0.5);
        assert_eq!(size_from_weight_kg(12),   1.0);
        assert_eq!(size_from_weight_kg(17),   1.5);
        assert_eq!(size_from_weight_kg(25),   2.0);
        assert_eq!(size_from_weight_kg(40),   2.5);
        assert_eq!(size_from_weight_kg(60),   3.0);   // lean adult human
        assert_eq!(size_from_weight_kg(75),   3.5);   // average adult human
        assert_eq!(size_from_weight_kg(99),   3.5);   // top of 3.5 bucket
        assert_eq!(size_from_weight_kg(110),  4.0);
        assert_eq!(size_from_weight_kg(140),  4.5);
        assert_eq!(size_from_weight_kg(170),  5.0);
        assert_eq!(size_from_weight_kg(200),  5.5);
        assert_eq!(size_from_weight_kg(250),  6.0);
        assert_eq!(size_from_weight_kg(280),  6.5);
        assert_eq!(size_from_weight_kg(320),  7.0);
        assert_eq!(size_from_weight_kg(380),  7.5);
        assert_eq!(size_from_weight_kg(420),  8.0);
        assert_eq!(size_from_weight_kg(480),  8.5);
        // Above 499 kg — whole-size table continues.
        assert_eq!(size_from_weight_kg(550),  9.0);
        assert_eq!(size_from_weight_kg(700),  10.0);
        assert_eq!(size_from_weight_kg(1599), 16.0);
        assert_eq!(size_from_weight_kg(1600), 17.0);  // first +1/200 step
        assert_eq!(size_from_weight_kg(1799), 17.0);
        assert_eq!(size_from_weight_kg(1800), 18.0);
    }

    #[test]
    fn old_save_with_map_open_skills_loads() {
        // Earlier saves wrote `open_skills` as a per-char map. Make
        // sure the deserializer accepts that shape and flattens to
        // the new flat-vec form.
        let json = r#"{
            "name":"Test","player":"","is_pc":false,"race":"Human",
            "gender":"","age":0,"height_cm":0,"weight_kg":75,
            "birthplace":"","description":"","clothing":"",
            "size":3.5,"level":1,
            "characteristics":{},"attributes":{},"skills":{},
            "bp_current":7,"mf_current":0,"conditions":[],"modifiers":[],
            "hit_locations":{},"weapons":[],"spells":[],"equipment":[],
            "money_sp":0,"open_skills":{},"notes":""
        }"#;
        let c: Character = serde_json::from_str(json)
            .expect("old-format save should still deserialize");
        assert_eq!(c.open_skills.len(), 0);
    }

    #[test]
    fn old_map_open_skills_flatten_to_vec() {
        let json = r#"{
            "name":"T","player":"","is_pc":false,"race":"Human",
            "gender":"","age":0,"height_cm":0,"weight_kg":75,
            "birthplace":"","description":"","clothing":"",
            "size":3.5,"level":1,
            "characteristics":{},"attributes":{},"skills":{},
            "bp_current":7,"mf_current":0,"conditions":[],"modifiers":[],
            "hit_locations":{},"weapons":[],"spells":[],"equipment":[],
            "money_sp":0,
            "open_skills":{
                "BODY":[{"attribute":"Athletics","name":"Singing","rank":3}],
                "MIND":[{"attribute":"Awareness","name":"Folklore","rank":2}]
            },
            "notes":""
        }"#;
        let c: Character = serde_json::from_str(json).unwrap();
        assert_eq!(c.open_skills.len(), 2);
        // parent_char should be stamped from the map key during flatten.
        assert!(c.open_skills.iter().any(|s| s.parent_char == "BODY" && s.name == "Singing"));
        assert!(c.open_skills.iter().any(|s| s.parent_char == "MIND" && s.name == "Folklore"));
    }

    #[test]
    fn old_save_with_string_spells_loads() {
        let json = r#"{
            "name":"T","player":"","is_pc":false,"race":"Human",
            "gender":"","age":0,"height_cm":0,"weight_kg":75,
            "birthplace":"","description":"","clothing":"",
            "size":3.5,"level":1,
            "characteristics":{},"attributes":{},"skills":{},
            "bp_current":7,"mf_current":0,"conditions":[],"modifiers":[],
            "hit_locations":{},"weapons":[],"spells":["Fireball","Heal"],"equipment":[],
            "money_sp":0,"open_skills":[],"notes":""
        }"#;
        let c: Character = serde_json::from_str(json).unwrap();
        assert_eq!(c.spells.len(), 2);
        assert_eq!(c.spells[0].name, "Fireball");
        assert_eq!(c.spells[1].name, "Heal");
    }
}
