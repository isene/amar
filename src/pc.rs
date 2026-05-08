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
    // MIND
    (Char::Mind, "Intelligence"),
    (Char::Mind, "Nature Knowledge"),
    (Char::Mind, "Social Knowledge"),
    (Char::Mind, "Practical Knowledge"),
    (Char::Mind, "Awareness"),
    (Char::Mind, "Willpower"),
    // SPIRIT
    (Char::Spirit, "Casting"),
    (Char::Spirit, "Attunement"),
    (Char::Spirit, "Innate"),
    (Char::Spirit, "Worship"),
];

/// Skills under each attribute. Verbatim from d6gaming.org/The_Character.
/// Worship sub-skills are god names per Mythology page; we list the
/// commonly-worshipped ones plus a free-form slot users can fill in.
pub const SKILLS: &[(&str, &[&str])] = &[
    // Body
    ("Strength",       &["Carrying", "Weight Lifting", "Wield Weapon"]),
    ("Endurance",      &["Fortitude", "Combat Tenacity", "Running", "Poison Resistance"]),
    ("Athletics",      &["Hide", "Move Quietly", "Climb", "Swim", "Ride", "Jump", "Balance", "Tumble"]),
    ("Melee Combat",   &[]),  // weapon skills added per-PC as they're acquired
    ("Missile Combat", &[]),
    ("Sleight",        &["Pick Pockets", "Stage Magic", "Disarm Traps"]),
    // Mind
    ("Intelligence",     &["Innovation", "Problem Solving"]),
    ("Nature Knowledge", &["Medical Lore", "Plant Lore", "Animal Lore", "Animal Handling", "Magick Rituals", "Alchemy"]),
    ("Social Knowledge", &["Social Lore", "Spoken Language", "Literacy", "Mythology", "Legend Lore"]),
    ("Practical Knowledge", &["Survival Lore", "Set Traps", "Ambush"]),
    ("Awareness",        &["Reaction Speed", "Alertness", "Tracking", "Detect Traps", "Sense Emotions",
                           "Sense Ambush", "Sense of Direction", "Sense Magick", "Listening"]),
    ("Willpower",        &["Pain Tolerance", "Courage", "Hold Breath", "Mental Fortitude"]),
    // Spirit
    ("Casting",    &["Range", "Duration", "Area of Effect", "Weight", "Number of Targets"]),
    ("Attunement", &["Self", "Fire", "Water", "Air", "Earth", "Life", "Death", "Mind", "Body"]),
    ("Innate",     &["Flying", "Camouflage", "Shape Shifting"]),
    ("Worship",    &[]),  // god names added per-PC
];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Character {
    pub name: String,
    pub player: String,
    pub is_pc: bool,
    pub race: String,
    pub gender: String,
    pub age: u32,
    pub size: u8,
    pub level: u8,
    pub characteristics: BTreeMap<String, i32>,
    pub attributes: BTreeMap<String, i32>,
    pub skills: BTreeMap<String, BTreeMap<String, i32>>,
    pub bp_current: i32,
    pub mf_current: i32,
    pub conditions: Vec<String>,
    pub equipment: Vec<String>,
    pub money_sp: i32,
    pub notes: String,
}

impl Character {
    pub fn new_blank(name: &str) -> Self {
        let mut c = Character::default();
        c.name = name.to_string();
        c.race = "Human".into();
        c.size = 3;
        c.level = 1;
        for ch in Char::all() { c.characteristics.insert(ch.name().to_string(), 0); }
        for (_, attr) in ATTRIBUTES { c.attributes.insert((*attr).to_string(), 0); }
        c.skills.insert("Spoken Language".into(), {
            let mut m = BTreeMap::new();
            m.insert("Native".into(), 2);
            m
        });
        c.bp_current = c.bp_max();
        c.mf_current = c.mf_max();
        c
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

    /// Total Skill Value = characteristic + attribute + skill rank.
    pub fn skill_total(&self, parent_char: Char, attr: &str, skill: &str) -> i32 {
        self.ch(parent_char) + self.attr(attr) + self.skill(attr, skill)
    }

    /// BP = SIZE × 2 + Fortitude / 3.
    pub fn bp_max(&self) -> i32 {
        let fort = self.skill("Endurance", "Fortitude");
        (self.size as i32) * 2 + fort / 3
    }

    /// DB = (SIZE + Wield Weapon) / 3.
    pub fn db(&self) -> i32 {
        let ww = self.skill("Strength", "Wield Weapon");
        ((self.size as i32) + ww) / 3
    }

    /// MD = (Mental Fortitude + Attunement Self) / 3.
    pub fn md(&self) -> i32 {
        let mf = self.skill("Willpower", "Mental Fortitude");
        let aself = self.skill("Attunement", "Self");
        (mf + aself) / 3
    }

    /// Reaction = Awareness + Reaction Speed (rolled with O6 in play).
    pub fn reaction(&self) -> i32 {
        self.skill_total(Char::Mind, "Awareness", "Reaction Speed")
    }

    /// Mental Fortitude pool (the "active spell capacity" budget).
    /// 1 point per hour rest; full recovery on 8h sleep — caller manages
    /// recovery, this just gives the cap.
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
        assert_eq!(c.size, 3);
        assert_eq!(c.bp_max(), 6); // 3*2 + 0/3
        assert_eq!(c.skill("Spoken Language", "Native"), 2);
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
    fn derived_stats_match_wiki_formulas() {
        let mut c = Character::new_blank("Test");
        c.size = 3;
        c.skills.entry("Endurance".into()).or_default().insert("Fortitude".into(), 6);
        c.skills.entry("Strength".into()).or_default().insert("Wield Weapon".into(), 3);
        c.skills.entry("Willpower".into()).or_default().insert("Mental Fortitude".into(), 6);
        c.skills.entry("Attunement".into()).or_default().insert("Self".into(), 3);
        assert_eq!(c.bp_max(), 8);  // 3*2 + 6/3 = 6 + 2
        assert_eq!(c.db(), 2);      // (3 + 3) / 3
        assert_eq!(c.md(), 3);      // (6 + 3) / 3
    }
}
