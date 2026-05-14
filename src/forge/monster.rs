//! Monster builder — port of Amar-Tools' `class_monster_new.rb`
//! that uses the encounter stat block (`EncStats`) directly instead
//! of running through the chartype template path.
//!
//! Monsters and animals don't have full chartypes; their canonical
//! representation in Amar-Tools is the 11-int row in `$Encounters`
//! (size, strength, endurance, awareness, dodge, melee, missile,
//! ap, magic, magic_lore). We build a `pc::Character` from those
//! numbers, scaling skill values by the rolled level.

use crate::dice::Rng;
use crate::forge::data::{self, EncStats};
use crate::pc::{Character, Char, Spell, Weapon, WeaponKind, HitLocation,
                ATTRIBUTES, attribute_parent, size_from_weight_kg, HIT_LOCATIONS};
use std::collections::BTreeMap;

/// Build a monster / animal NPC from the encounter spec name. Falls
/// back to a plain `Commoner` chartype if the spec isn't in the
/// encounter table — the caller (`encounter::build_encounter`) only
/// dispatches here when the spec starts with "Monster: ", "Small
/// animal:" or "Large animal:".
pub fn build_monster(spec: &str, level: u8, rng: &mut impl Rng) -> Character {
    let stats = match data::enc_stats(spec) {
        Some(s) => s.clone(),
        None => return crate::forge::npc::build_npc_seeded("Commoner", level, "", rng),
    };
    build_from_stats(spec, &stats, level, rng)
}

fn build_from_stats(spec: &str, s: &EncStats, level: u8, rng: &mut impl Rng) -> Character {
    let mut c = Character::new_blank("");
    c.is_pc = false;
    c.race = race_from_spec(spec);
    c.gender = if rng.d6() % 2 == 0 { "M".into() } else { "F".into() };
    c.level = level;

    // SIZE → weight: invert the half-size table by picking the bottom
    // of the bucket. So SIZE 5 → 150 kg, SIZE 7 → 300 kg, etc.
    c.weight_kg = weight_for_size(s.size as f32);
    c.size = size_from_weight_kg(c.weight_kg);
    c.height_cm = (c.size as u32 * 100).max(50); // rough, cosmetic only
    c.age = 5 + (rng.d6() as u32 % 20);

    // Lay down all canonical char/attr keys at zero, then fill what
    // the encounter stats imply.
    c.characteristics.clear();
    c.attributes.clear();
    c.skills.clear();
    for ch in Char::all() {
        c.characteristics.insert(ch.name().to_string(), 0);
    }
    for (_, attr) in ATTRIBUTES {
        c.attributes.insert((*attr).to_string(), 0);
    }

    // Distribute the stat block into the tier system. The encounter
    // numbers are already large (SIZE 12, Strength 12 for a Dragon),
    // so we map them onto BODY characteristic + Strength attribute
    // by splitting at 5 (anything above flows into Strength).
    let body_char = (s.strength.min(5)).max(0);
    let body_str  = (s.strength - body_char).max(0);
    c.characteristics.insert("BODY".into(), body_char);

    // Magic split similarly: magic ≤ 3 fills SPIRIT, the remainder
    // flows into Casting attribute.
    let sp_char = (s.magic.min(3)).max(0);
    let sp_cast = (s.magic - sp_char).max(0);
    c.characteristics.insert("SPIRIT".into(), sp_char);

    c.attributes.insert("Strength".into(),     body_str);
    c.attributes.insert("Endurance".into(),    s.endurance.max(0));
    c.attributes.insert("Athletics".into(),    s.dodge.max(0));
    c.attributes.insert("Melee Combat".into(), s.melee_skill.max(0));
    c.attributes.insert("Missile Combat".into(), s.miss_skill.max(0));
    c.attributes.insert("Awareness".into(),    s.awareness.max(0));
    if s.magic > 0 {
        c.attributes.insert("Casting".into(),   sp_cast);
        c.attributes.insert("Attunement".into(), s.magic_lore.max(0));
    }

    // Weapon-skill ranks for Monster: Dragon's natural attacks etc.
    // Stored under "Melee Combat" for unarmed-style natural weapons.
    let mut mc = BTreeMap::new();
    mc.insert("Natural".into(), s.melee_skill.max(0));
    mc.insert("Unarmed".into(), s.melee_skill.max(0));
    c.skills.insert("Melee Combat".into(), mc);
    if s.miss_skill > 0 {
        let mut ms = BTreeMap::new();
        ms.insert("Natural".into(), s.miss_skill);
        c.skills.insert("Missile Combat".into(), ms);
    }
    // Endurance/Fortitude → BP. Use the encounter's endurance number.
    let mut end = BTreeMap::new();
    end.insert("Fortitude".into(), s.endurance.max(0));
    c.skills.insert("Endurance".into(), end);
    // Awareness → Reaction Speed for initiative.
    let mut aware = BTreeMap::new();
    aware.insert("Reaction Speed".into(), s.awareness.max(0));
    aware.insert("Alertness".into(),       s.awareness.max(0));
    c.skills.insert("Awareness".into(), aware);
    // Dodge → Athletics/Balance (closest existing skill).
    if s.dodge > 0 {
        let mut ath = BTreeMap::new();
        ath.insert("Balance".into(), s.dodge);
        c.skills.insert("Athletics".into(), ath);
    }

    // Hit locations + AP from the stat block. Negative AP means
    // "naturally tough hide" (Troll +3 etc — the original encounter
    // numbers use negative AP to mean *bonus* to wound resistance).
    let armor_ap = s.ap.abs();
    let armor_name = armor_label_for(spec);
    for loc in HIT_LOCATIONS {
        c.hit_locations.insert((*loc).to_string(), HitLocation {
            armor: armor_name.into(),
            ap: armor_ap,
        });
    }

    // Natural weapon — use Unarmed row from the melee table as the
    // fallback shape, then override the damage to track the
    // monster's level + size.
    let nat_dam = s.melee_skill.max(0) + (level as i32 / 2);
    c.weapons.push(Weapon {
        name: natural_weapon_for(spec).into(),
        kind: WeaponKind::Melee,
        skill_name: "Natural".into(),
        two_handed: false,
        init: 4,
        off_mod: s.melee_skill,
        def_mod: s.dodge,
        shots_per_round: 0,
        damage: nat_dam,
        hp: s.size * 2,
        range_m: 0,
        xp: 0,
    });

    // Spellcaster monsters (Dragons, Vampires, Faerie, Drake/Basilisk
    // at lower levels) get a hint spell so the sheet shows them as
    // magic-capable.
    if s.magic > 0 {
        c.spells.push(Spell { name: "Innate magic".into(), ..Default::default() });
    }

    // Name. Use the spec as a display name with a numeric suffix
    // when the encounter generator stamps multiple of the same spec.
    c.name = display_name(spec);
    c.bp_current = c.bp_max();
    c.mf_current = c.mf_max();
    let _ = attribute_parent("Strength"); // keep import alive
    c
}

fn race_from_spec(spec: &str) -> String {
    if let Some(rest) = spec.strip_prefix("Monster: ") {
        rest.split(['(', ' ']).next().unwrap_or(rest).into()
    } else if spec.starts_with("Small animal") {
        "Small animal".into()
    } else if spec.starts_with("Large animal") {
        "Large animal".into()
    } else {
        spec.into()
    }
}

fn display_name(spec: &str) -> String {
    if let Some(rest) = spec.strip_prefix("Monster: ") {
        rest.into()
    } else {
        spec.into()
    }
}

fn armor_label_for(spec: &str) -> &'static str {
    let s = spec.to_ascii_lowercase();
    if s.contains("dragon") || s.contains("drake") || s.contains("basilisk")
        || s.contains("wyvern") { "Scales" }
    else if s.contains("troll") || s.contains("giant") || s.contains("ogre") { "Tough hide" }
    else if s.contains("zombie") || s.contains("skeleton") { "Decay-toughened" }
    else if s.contains("animal") { "Hide" }
    else { "Hide" }
}

fn natural_weapon_for(spec: &str) -> &'static str {
    let s = spec.to_ascii_lowercase();
    if s.contains("dragon") { "Bite + claws + breath" }
    else if s.contains("drake") || s.contains("wyvern") { "Bite + claws" }
    else if s.contains("hydra") { "Bites" }
    else if s.contains("basilisk") { "Bite (gaze)" }
    else if s.contains("troll") || s.contains("giant") { "Massive blow" }
    else if s.contains("werewolf") { "Bite + claws" }
    else if s.contains("vampire") { "Bite + claws" }
    else if s.contains("zombie") || s.contains("skeleton") { "Claws" }
    else if s.contains("predator") { "Claws + bite" }
    else if s.contains("animal") { "Bite + horn" }
    else if s.contains("faerie") { "Tiny dagger" }
    else { "Natural weapon" }
}

/// Pick a representative weight for a SIZE value (bottom of bucket).
/// Mirrors the inverse of `pc::size_from_weight_kg`.
fn weight_for_size(size: f32) -> u32 {
    match size {
        s if s <= 0.5 => 5,
        s if s <= 1.0 => 10,
        s if s <= 1.5 => 15,
        s if s <= 2.0 => 20,
        s if s <= 2.5 => 35,
        s if s <= 3.0 => 50,
        s if s <= 3.5 => 75,
        s if s <= 4.0 => 100,
        s if s <= 4.5 => 125,
        s if s <= 5.0 => 150,
        s if s <= 5.5 => 188,
        s if s <= 6.0 => 225,
        s if s <= 6.5 => 263,
        s if s <= 7.0 => 300,
        s if s <= 7.5 => 350,
        s if s <= 8.0 => 400,
        s if s <= 8.5 => 450,
        s if s <= 9.0 => 500,
        s if s <= 10.0 => 600,
        s if s <= 11.0 => 725,
        s if s <= 12.0 => 850,
        s if s <= 13.0 => 1000,
        s if s <= 14.0 => 1150,
        s if s <= 15.0 => 1300,
        s if s <= 16.0 => 1450,
        // Beyond 16 → +200 kg per +1.
        s => 1600 + (((s - 16.0).max(0.0)) as u32 * 200),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dice::StdRng;

    #[test]
    fn troll_large_has_high_strength_and_size() {
        let mut rng = StdRng::from_time();
        let m = build_monster("Monster: Troll (large)", 4, &mut rng);
        assert_eq!(m.race, "Troll");
        // Troll (large) stats: SIZE 7, Strength 7, Endurance 7.
        assert!(m.size >= 6.0);
        // Total Strength = BODY + Strength attribute should be
        // close to the encounter's strength value.
        let total = m.ch(Char::Body) + m.attr("Strength");
        assert!(total >= 6, "got Body+Str = {}", total);
    }

    #[test]
    fn dragon_has_natural_weapon() {
        let mut rng = StdRng::from_time();
        let m = build_monster("Monster: Dragon", 6, &mut rng);
        // Every Character::new_blank() seeds an "Unarmed" weapon, so
        // a monster's natural attack lands later in the vector now.
        // Just verify some entry mentions breath or claws.
        assert!(m.weapons.iter()
            .any(|w| w.name.contains("breath") || w.name.contains("claws")),
            "weapons = {:?}", m.weapons.iter().map(|w| &w.name).collect::<Vec<_>>());
    }

    #[test]
    fn animal_predator_falls_back_gracefully() {
        let mut rng = StdRng::from_time();
        let m = build_monster("Small animal: Predator", 1, &mut rng);
        assert_eq!(m.race, "Small animal");
    }
}
