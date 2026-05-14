//! Static data tables ported verbatim from Amar-Tools.
//!
//! Sources (Ruby `$Globals` → Rust constants):
//!   - `includes/tables/melee.rb`        → `MELEE`
//!   - `includes/tables/missile.rb`      → `MISSILE`
//!   - `includes/tables/armour.rb`       → `ARMOUR`
//!   - `includes/tables/personality.rb`  → `PERSONALITY`
//!   - `includes/tables/enc_type.rb`     → `ENC_TYPE`
//!   - `includes/tables/enc_specific.rb` → `ENC_SPECIFIC`
//!   - `includes/tables/encounters.rb`   → `ENCOUNTERS`
//!   - `includes/tables/chartype_new_full.rb` (subset) → `CHARTYPES`
//!
//! Numbers are kept identical so the Rust port produces the same
//! statistical distribution Amar-Tools did.

// ---------------------------------------------------------------- Melee

/// Melee weapon row. Mirrors Amar-Tools' `[name, type, str, dam,
/// init, off, def, hp, wt]` 9-tuple.
#[derive(Debug, Clone, Copy)]
pub struct MeleeRow {
    pub name: &'static str,
    pub kind: &'static str,
    pub str_req: i32,
    pub dam: i32,
    pub init: i32,
    pub off: i32,
    pub def: i32,
    pub hp: i32,
    pub wt: f32,
}

/// 30 melee weapons in increasing strength-requirement order. Index
/// 1..=30 (index 0 reserved for "header" semantics in the Ruby
/// table — we omit the header and use 0-based indexing in Rust).
pub const MELEE: &[MeleeRow] = &[
    MeleeRow { name: "Unarmed",        kind: "Unarmed",   str_req: 0,  dam: -4, init: 1, off: -2, def: -4, hp: 0,  wt: 0.0 },
    MeleeRow { name: "Knife",          kind: "Knife",     str_req: 1,  dam: -2, init: 2, off: -2, def: -3, hp: 8,  wt: 0.4 },
    MeleeRow { name: "Short sword",    kind: "1H",        str_req: 2,  dam: -2, init: 3, off: -1, def: -1, hp: 12, wt: 0.6 },
    MeleeRow { name: "Rapier",         kind: "1H",        str_req: 2,  dam: -2, init: 4, off: 0,  def: -1, hp: 7,  wt: 0.7 },
    MeleeRow { name: "Knife*2",        kind: "Knife",     str_req: 3,  dam: -2, init: 2, off: -1, def: -2, hp: 8,  wt: 0.8 },
    MeleeRow { name: "Rapier/Knife",   kind: "1H/Knife",  str_req: 3,  dam: -2, init: 4, off: 1,  def: 0,  hp: 7,  wt: 1.1 },
    MeleeRow { name: "Staff",          kind: "Polearm",   str_req: 3,  dam: -2, init: 6, off: 0,  def: 2,  hp: 7,  wt: 1.5 },
    MeleeRow { name: "Light mace",     kind: "1H",        str_req: 3,  dam: -2, init: 3, off: -1, def: -2, hp: 8,  wt: 1.0 },
    MeleeRow { name: "Hatchet/Knife",  kind: "1H",        str_req: 3,  dam: -1, init: 3, off: -1, def: -2, hp: 8,  wt: 0.8 },
    MeleeRow { name: "Lt. mace/Buc",   kind: "1H/Shield", str_req: 3,  dam: -2, init: 3, off: 0,  def: 1,  hp: 8,  wt: 1.8 },
    MeleeRow { name: "Hatchet/Buc",    kind: "1H/Shield", str_req: 3,  dam: -1, init: 3, off: -1, def: 1,  hp: 8,  wt: 1.6 },
    MeleeRow { name: "Longsword",      kind: "1H",        str_req: 4,  dam: -1, init: 5, off: 0,  def: 0,  hp: 12, wt: 1.2 },
    MeleeRow { name: "Spear 2H",       kind: "Polearm",   str_req: 4,  dam: -1, init: 7, off: 0,  def: 2,  hp: 7,  wt: 2.0 },
    MeleeRow { name: "Club",           kind: "1H",        str_req: 4,  dam: -2, init: 4, off: -1, def: -2, hp: 8,  wt: 1.6 },
    MeleeRow { name: "H. mace 2H",     kind: "2H",        str_req: 4,  dam: 0,  init: 4, off: 0,  def: 0,  hp: 8,  wt: 1.8 },
    MeleeRow { name: "B. sword 2H",    kind: "2H",        str_req: 4,  dam: 0,  init: 6, off: 0,  def: 1,  hp: 12, wt: 2.0 },
    MeleeRow { name: "Longsword/Buc",  kind: "1H/Shield", str_req: 4,  dam: -1, init: 5, off: 1,  def: 1,  hp: 12, wt: 2.0 },
    MeleeRow { name: "Spear/Buc",      kind: "1H/Shield", str_req: 4,  dam: -2, init: 6, off: 1,  def: 1,  hp: 7,  wt: 2.0 },
    MeleeRow { name: "B. sword/Buc",   kind: "1H/Shield", str_req: 5,  dam: -1, init: 6, off: 1,  def: 1,  hp: 12, wt: 2.8 },
    MeleeRow { name: "Br. axe/Buc",    kind: "1H/Shield", str_req: 5,  dam: 0,  init: 4, off: 0,  def: 1,  hp: 8,  wt: 3.0 },
    MeleeRow { name: "Longsword/RSh",  kind: "1H/Shield", str_req: 5,  dam: -1, init: 5, off: 1,  def: 3,  hp: 12, wt: 3.2 },
    MeleeRow { name: "B. axe 2H",      kind: "2H",        str_req: 5,  dam: 2,  init: 5, off: -1, def: 0,  hp: 8,  wt: 2.5 },
    MeleeRow { name: "H. Mace/Rsh",    kind: "1H/Shield", str_req: 6,  dam: -1, init: 4, off: 1,  def: 3,  hp: 8,  wt: 3.8 },
    MeleeRow { name: "B. sword/KSh",   kind: "1H/Shield", str_req: 6,  dam: -1, init: 6, off: 1,  def: 4,  hp: 12, wt: 6.0 },
    MeleeRow { name: "H. Mace/KSh",    kind: "1H/Shield", str_req: 6,  dam: -1, init: 4, off: 1,  def: 4,  hp: 12, wt: 5.8 },
    MeleeRow { name: "Great sword",    kind: "2H",        str_req: 6,  dam: 1,  init: 7, off: 0,  def: 1,  hp: 13, wt: 4.0 },
    MeleeRow { name: "Hercules club",  kind: "2H",        str_req: 6,  dam: 2,  init: 7, off: 0,  def: 1,  hp: 10, wt: 5.0 },
    MeleeRow { name: "Halberd",        kind: "Polearm",   str_req: 7,  dam: 2,  init: 7, off: 0,  def: 2,  hp: 7,  wt: 6.0 },
    MeleeRow { name: "B. axe/Rsh",     kind: "1H/Shield", str_req: 9,  dam: 1,  init: 5, off: 1,  def: 3,  hp: 8,  wt: 4.5 },
    MeleeRow { name: "B. axe/Ksh",     kind: "1H/Shield", str_req: 9,  dam: 1,  init: 5, off: 1,  def: 4,  hp: 8,  wt: 6.5 },
];

// ---------------------------------------------------------------- Missile

#[derive(Debug, Clone, Copy)]
pub struct MissileRow {
    pub name: &'static str,
    pub kind: &'static str,
    pub str_req: i32,
    pub dam: i32,
    pub off: i32,
    pub rng: u32,
    pub max_rng: u32,
    pub init: i32,
    pub wt: f32,
}

pub const MISSILE: &[MissileRow] = &[
    MissileRow { name: "Rock [2]",     kind: "Rock",     str_req: 0,  dam: -3, off: -2, rng: 15, max_rng: 40,  init: 5, wt: 0.2 },
    MissileRow { name: "Th Knife [2]", kind: "Knife",    str_req: 1,  dam: -2, off: -1, rng: 15, max_rng: 25,  init: 5, wt: 0.2 },
    MissileRow { name: "Sling [1]",    kind: "Sling",    str_req: 2,  dam: -1, off: -3, rng: 40, max_rng: 120, init: 0, wt: 0.3 },
    MissileRow { name: "X-bow(L) [½]", kind: "Crossbow", str_req: 2,  dam: 2,  off: 2,  rng: 20, max_rng: 100, init: 0, wt: 1.5 },
    MissileRow { name: "Bow(L) [1]",   kind: "Bow",      str_req: 2,  dam: 1,  off: 0,  rng: 30, max_rng: 130, init: 0, wt: 1.5 },
    MissileRow { name: "X-bow(M) [⅓]", kind: "Crossbow", str_req: 3,  dam: 3,  off: 2,  rng: 25, max_rng: 175, init: 0, wt: 2.0 },
    MissileRow { name: "Javelin [1]",  kind: "Javelin",  str_req: 3,  dam: 0,  off: -2, rng: 20, max_rng: 40,  init: 0, wt: 2.0 },
    MissileRow { name: "Bow(M) [1]",   kind: "Bow",      str_req: 4,  dam: 2,  off: 0,  rng: 35, max_rng: 160, init: 0, wt: 2.0 },
    MissileRow { name: "X-bow(H) [¼]", kind: "Crossbow", str_req: 4,  dam: 4,  off: 2,  rng: 30, max_rng: 250, init: 0, wt: 3.0 },
    MissileRow { name: "Bow(H) [1]",   kind: "Bow",      str_req: 6,  dam: 3,  off: 0,  rng: 40, max_rng: 190, init: 0, wt: 2.5 },
    MissileRow { name: "Bow(H2) [1]",  kind: "Bow",      str_req: 8,  dam: 4,  off: 0,  rng: 45, max_rng: 215, init: 0, wt: 3.0 },
    MissileRow { name: "Bow(H3) [1]",  kind: "Bow",      str_req: 10, dam: 5,  off: 0,  rng: 50, max_rng: 240, init: 0, wt: 3.5 },
];

// ---------------------------------------------------------------- Armour

#[derive(Debug, Clone, Copy)]
pub struct ArmourRow {
    pub name: &'static str,
    pub ap: i32,
    pub m_mod: i32,  // movement modifier
    pub h_mod: i32,  // hold-breath / encumbrance modifier
    pub wt: f32,
}

pub const ARMOUR: &[ArmourRow] = &[
    ArmourRow { name: "None",            ap: 0, m_mod: 0,  h_mod: 0,  wt: 2.0 },
    ArmourRow { name: "Heavy Cloth",     ap: 1, m_mod: -2, h_mod: 0,  wt: 5.0 },
    ArmourRow { name: "Leather armour",  ap: 1, m_mod: -1, h_mod: 0,  wt: 7.0 },
    ArmourRow { name: "Leather scale",   ap: 2, m_mod: -3, h_mod: 0,  wt: 9.0 },
    ArmourRow { name: "Ringed mail",     ap: 2, m_mod: -1, h_mod: -1, wt: 9.0 },
    ArmourRow { name: "Cuir-boullie",    ap: 3, m_mod: -2, h_mod: 0,  wt: 15.0 },
    ArmourRow { name: "Chain mail",      ap: 4, m_mod: -4, h_mod: -2, wt: 20.0 },
    ArmourRow { name: "Metal scale",     ap: 5, m_mod: -6, h_mod: -3, wt: 25.0 },
];

// ---------------------------------------------------------------- Personality

/// Personality traits, weighted (Amar-Tools `$Personality` table).
pub const PERSONALITY: &[(&str, u32)] = &[
    ("Generous, empathetic",      7),
    ("Friendly, service minded", 10),
    ("Conservative, structured", 10),
    ("Indifferent, unstructured", 7),
    ("Greedy, self-centered",     7),
    ("Antagonistic, combattive",  4),
    ("Cautious, fearful",         3),
    ("Sad, gloomy",               2),
];

// ---------------------------------------------------------------- Encounter
// stat blocks

/// Encounter stat row — the 11-int tuple `[L, Z, S, E, A, d, ml, ms,
/// ap, ma, ml]` from Amar-Tools' `$Encounters` table.
#[derive(Debug, Clone, Copy)]
pub struct EncStats {
    pub max_lvl:     i32,  // L: max level
    pub size:        i32,  // Z: SIZE
    pub strength:    i32,  // S: BODY+Strength
    pub endurance:   i32,  // E: Endurance
    pub awareness:   i32,  // A: Awareness
    pub dodge:       i32,  // d: Dodge
    pub melee_skill: i32,  // ml: Melee skill
    pub miss_skill:  i32,  // ms: Missile skill
    pub ap:          i32,  // ap: Armor Points
    pub magic:       i32,  // ma: Magic
    pub magic_lore:  i32,  // ml: Magic Lore
}

/// 60+ encounter stat blocks. Used both for humanoid encounters
/// (when the chartype template is missing detailed numbers) and for
/// monster encounters (where this is the only stat source).
pub const ENCOUNTERS: &[(&str, EncStats)] = &[
    ("Animal trainer",          EncStats { max_lvl: 5, size: 3, strength: 2, endurance: 3, awareness: 3, dodge: 3, melee_skill: 3, miss_skill: 3, ap: 4, magic: 0, magic_lore: 0 }),
    ("Archer",                  EncStats { max_lvl: 5, size: 3, strength: 3, endurance: 3, awareness: 3, dodge: 4, melee_skill: 4, miss_skill: 6, ap: 5, magic: 0, magic_lore: 0 }),
    ("Armour smith",            EncStats { max_lvl: 5, size: 3, strength: 5, endurance: 4, awareness: 1, dodge: 3, melee_skill: 3, miss_skill: 3, ap: 6, magic: 0, magic_lore: 0 }),
    ("Army officer",            EncStats { max_lvl: 5, size: 3, strength: 4, endurance: 3, awareness: 2, dodge: 5, melee_skill: 5, miss_skill: 5, ap: 6, magic: 0, magic_lore: 0 }),
    ("Assassin",                EncStats { max_lvl: 5, size: 3, strength: 3, endurance: 2, awareness: 3, dodge: 4, melee_skill: 5, miss_skill: 5, ap: 4, magic: 0, magic_lore: 0 }),
    ("Baker/Cook",              EncStats { max_lvl: 5, size: 3, strength: 2, endurance: 1, awareness: 1, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 2, magic: 0, magic_lore: 0 }),
    ("Bard",                    EncStats { max_lvl: 5, size: 3, strength: 1, endurance: 2, awareness: 2, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 3, magic: 1, magic_lore: 0 }),
    ("Boatbuilder",             EncStats { max_lvl: 5, size: 3, strength: 4, endurance: 3, awareness: 1, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 3, magic: 0, magic_lore: 0 }),
    ("Body guard",              EncStats { max_lvl: 5, size: 4, strength: 4, endurance: 4, awareness: 3, dodge: 5, melee_skill: 5, miss_skill: 5, ap: 6, magic: 0, magic_lore: 0 }),
    ("Builder",                 EncStats { max_lvl: 5, size: 3, strength: 4, endurance: 3, awareness: 1, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 2, magic: 0, magic_lore: 0 }),
    ("Bureaucrat",              EncStats { max_lvl: 5, size: 3, strength: 0, endurance: 1, awareness: 2, dodge: 1, melee_skill: 1, miss_skill: 1, ap: 2, magic: 0, magic_lore: 0 }),
    ("Carpenter",               EncStats { max_lvl: 5, size: 3, strength: 3, endurance: 2, awareness: 1, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 2, magic: 0, magic_lore: 0 }),
    ("Clergyman",               EncStats { max_lvl: 5, size: 3, strength: 2, endurance: 2, awareness: 2, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 4, magic: 2, magic_lore: 1 }),
    ("Crafts (fine)",           EncStats { max_lvl: 5, size: 3, strength: 2, endurance: 1, awareness: 2, dodge: 1, melee_skill: 1, miss_skill: 1, ap: 2, magic: 0, magic_lore: 0 }),
    ("Crafts (heavy)",          EncStats { max_lvl: 5, size: 3, strength: 3, endurance: 3, awareness: 1, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 2, magic: 0, magic_lore: 0 }),
    ("Entertainer",             EncStats { max_lvl: 5, size: 3, strength: 3, endurance: 3, awareness: 2, dodge: 5, melee_skill: 4, miss_skill: 4, ap: 2, magic: 0, magic_lore: 0 }),
    ("Executioner",             EncStats { max_lvl: 5, size: 4, strength: 5, endurance: 2, awareness: 1, dodge: 1, melee_skill: 4, miss_skill: 1, ap: 4, magic: 0, magic_lore: 0 }),
    ("Farmer",                  EncStats { max_lvl: 5, size: 3, strength: 3, endurance: 2, awareness: 2, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 2, magic: 0, magic_lore: 0 }),
    ("Fine artist",             EncStats { max_lvl: 5, size: 3, strength: 1, endurance: 1, awareness: 2, dodge: 1, melee_skill: 1, miss_skill: 1, ap: 0, magic: 0, magic_lore: 0 }),
    ("Fine smith",              EncStats { max_lvl: 5, size: 3, strength: 2, endurance: 1, awareness: 2, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 2, magic: 0, magic_lore: 0 }),
    ("Fisherman",               EncStats { max_lvl: 5, size: 3, strength: 4, endurance: 3, awareness: 2, dodge: 3, melee_skill: 3, miss_skill: 3, ap: 3, magic: 0, magic_lore: 0 }),
    ("Gladiator",               EncStats { max_lvl: 5, size: 4, strength: 4, endurance: 4, awareness: 2, dodge: 6, melee_skill: 6, miss_skill: 4, ap: 6, magic: 0, magic_lore: 0 }),
    ("Noble",                   EncStats { max_lvl: 5, size: 3, strength: 2, endurance: 2, awareness: 2, dodge: 3, melee_skill: 3, miss_skill: 3, ap: 6, magic: 0, magic_lore: 0 }),
    ("Highwayman",              EncStats { max_lvl: 5, size: 3, strength: 2, endurance: 3, awareness: 2, dodge: 4, melee_skill: 4, miss_skill: 4, ap: 5, magic: 0, magic_lore: 0 }),
    ("House wife",              EncStats { max_lvl: 5, size: 3, strength: 2, endurance: 1, awareness: 1, dodge: 1, melee_skill: 1, miss_skill: 1, ap: 0, magic: 0, magic_lore: 0 }),
    ("Hunter",                  EncStats { max_lvl: 5, size: 3, strength: 3, endurance: 4, awareness: 4, dodge: 4, melee_skill: 3, miss_skill: 4, ap: 4, magic: 0, magic_lore: 0 }),
    ("Jeweller",                EncStats { max_lvl: 5, size: 3, strength: 1, endurance: 1, awareness: 3, dodge: 1, melee_skill: 1, miss_skill: 1, ap: 0, magic: 0, magic_lore: 0 }),
    ("High class",              EncStats { max_lvl: 5, size: 3, strength: 2, endurance: 3, awareness: 2, dodge: 3, melee_skill: 3, miss_skill: 3, ap: 6, magic: 0, magic_lore: 0 }),
    ("Mapmaker",                EncStats { max_lvl: 5, size: 3, strength: 1, endurance: 1, awareness: 2, dodge: 1, melee_skill: 1, miss_skill: 1, ap: 2, magic: 0, magic_lore: 0 }),
    ("Mason",                   EncStats { max_lvl: 5, size: 3, strength: 4, endurance: 3, awareness: 1, dodge: 1, melee_skill: 1, miss_skill: 1, ap: 2, magic: 0, magic_lore: 0 }),
    ("Merchant",                EncStats { max_lvl: 5, size: 3, strength: 2, endurance: 2, awareness: 2, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 2, magic: 0, magic_lore: 0 }),
    ("Messenger",               EncStats { max_lvl: 5, size: 3, strength: 2, endurance: 4, awareness: 3, dodge: 4, melee_skill: 3, miss_skill: 3, ap: 3, magic: 0, magic_lore: 0 }),
    ("Monk",                    EncStats { max_lvl: 5, size: 3, strength: 3, endurance: 4, awareness: 3, dodge: 5, melee_skill: 6, miss_skill: 4, ap: 3, magic: 2, magic_lore: 0 }),
    ("Nanny",                   EncStats { max_lvl: 5, size: 3, strength: 1, endurance: 2, awareness: 2, dodge: 1, melee_skill: 1, miss_skill: 1, ap: 0, magic: 0, magic_lore: 0 }),
    ("Navigator",               EncStats { max_lvl: 5, size: 3, strength: 1, endurance: 2, awareness: 3, dodge: 1, melee_skill: 1, miss_skill: 1, ap: 2, magic: 0, magic_lore: 0 }),
    ("Prostitute",              EncStats { max_lvl: 5, size: 3, strength: 2, endurance: 3, awareness: 2, dodge: 3, melee_skill: 3, miss_skill: 3, ap: 2, magic: 0, magic_lore: 0 }),
    ("Ranger",                  EncStats { max_lvl: 5, size: 3, strength: 2, endurance: 4, awareness: 3, dodge: 4, melee_skill: 4, miss_skill: 5, ap: 5, magic: 0, magic_lore: 0 }),
    ("Sage",                    EncStats { max_lvl: 5, size: 3, strength: 1, endurance: 1, awareness: 2, dodge: 1, melee_skill: 1, miss_skill: 1, ap: 2, magic: 1, magic_lore: 1 }),
    ("Sailor",                  EncStats { max_lvl: 5, size: 3, strength: 4, endurance: 4, awareness: 2, dodge: 3, melee_skill: 3, miss_skill: 3, ap: 3, magic: 0, magic_lore: 0 }),
    ("Scribe",                  EncStats { max_lvl: 5, size: 3, strength: 1, endurance: 1, awareness: 2, dodge: 1, melee_skill: 1, miss_skill: 1, ap: 2, magic: 0, magic_lore: 0 }),
    ("Seer",                    EncStats { max_lvl: 5, size: 3, strength: 1, endurance: 1, awareness: 3, dodge: 1, melee_skill: 1, miss_skill: 1, ap: 2, magic: 4, magic_lore: 4 }),
    ("Smith",                   EncStats { max_lvl: 5, size: 3, strength: 5, endurance: 3, awareness: 1, dodge: 1, melee_skill: 1, miss_skill: 1, ap: 5, magic: 0, magic_lore: 0 }),
    ("Soldier",                 EncStats { max_lvl: 5, size: 4, strength: 3, endurance: 4, awareness: 2, dodge: 5, melee_skill: 5, miss_skill: 5, ap: 6, magic: 0, magic_lore: 0 }),
    ("Sorcerer",                EncStats { max_lvl: 5, size: 3, strength: 1, endurance: 1, awareness: 2, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 3, magic: 4, magic_lore: 4 }),
    ("Sports contender",        EncStats { max_lvl: 5, size: 3, strength: 4, endurance: 4, awareness: 2, dodge: 5, melee_skill: 5, miss_skill: 5, ap: 4, magic: 0, magic_lore: 0 }),
    ("Summoner",                EncStats { max_lvl: 5, size: 3, strength: 1, endurance: 1, awareness: 2, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 3, magic: 4, magic_lore: 4 }),
    ("Tailor",                  EncStats { max_lvl: 5, size: 3, strength: 2, endurance: 1, awareness: 2, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 2, magic: 0, magic_lore: 0 }),
    ("Tanner",                  EncStats { max_lvl: 5, size: 3, strength: 2, endurance: 1, awareness: 2, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 2, magic: 0, magic_lore: 0 }),
    ("Thief",                   EncStats { max_lvl: 5, size: 3, strength: 2, endurance: 3, awareness: 3, dodge: 5, melee_skill: 5, miss_skill: 4, ap: 4, magic: 0, magic_lore: 0 }),
    ("Tracker",                 EncStats { max_lvl: 5, size: 3, strength: 2, endurance: 4, awareness: 5, dodge: 4, melee_skill: 4, miss_skill: 4, ap: 4, magic: 0, magic_lore: 0 }),
    ("Warrior",                 EncStats { max_lvl: 5, size: 4, strength: 4, endurance: 4, awareness: 2, dodge: 5, melee_skill: 5, miss_skill: 4, ap: 6, magic: 0, magic_lore: 0 }),
    ("Witch (black)",           EncStats { max_lvl: 5, size: 3, strength: 1, endurance: 1, awareness: 2, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 3, magic: 4, magic_lore: 4 }),
    ("Witch (white)",           EncStats { max_lvl: 5, size: 3, strength: 1, endurance: 1, awareness: 2, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 3, magic: 4, magic_lore: 4 }),
    ("Wizard (air)",            EncStats { max_lvl: 5, size: 3, strength: 1, endurance: 1, awareness: 2, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 3, magic: 4, magic_lore: 4 }),
    ("Wizard (earth)",          EncStats { max_lvl: 5, size: 3, strength: 1, endurance: 1, awareness: 2, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 3, magic: 4, magic_lore: 4 }),
    ("Wizard (fire)",           EncStats { max_lvl: 5, size: 3, strength: 1, endurance: 1, awareness: 2, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 3, magic: 4, magic_lore: 4 }),
    ("Wizard (water)",          EncStats { max_lvl: 5, size: 3, strength: 1, endurance: 1, awareness: 2, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 3, magic: 4, magic_lore: 4 }),
    ("Wizard (prot.)",          EncStats { max_lvl: 5, size: 3, strength: 1, endurance: 1, awareness: 2, dodge: 2, melee_skill: 2, miss_skill: 2, ap: 3, magic: 4, magic_lore: 4 }),
    ("Worker",                  EncStats { max_lvl: 5, size: 3, strength: 3, endurance: 3, awareness: 2, dodge: 2, melee_skill: 3, miss_skill: 2, ap: 3, magic: 0, magic_lore: 0 }),
    ("Guard",                   EncStats { max_lvl: 5, size: 3, strength: 4, endurance: 4, awareness: 3, dodge: 4, melee_skill: 5, miss_skill: 4, ap: 6, magic: 0, magic_lore: 0 }),
    ("Scout",                   EncStats { max_lvl: 5, size: 3, strength: 3, endurance: 4, awareness: 4, dodge: 5, melee_skill: 4, miss_skill: 5, ap: 4, magic: 0, magic_lore: 0 }),
    ("Barbarian",               EncStats { max_lvl: 5, size: 4, strength: 5, endurance: 4, awareness: 2, dodge: 4, melee_skill: 5, miss_skill: 3, ap: 4, magic: 0, magic_lore: 0 }),
    ("Shaman",                  EncStats { max_lvl: 5, size: 3, strength: 2, endurance: 3, awareness: 3, dodge: 3, melee_skill: 3, miss_skill: 3, ap: 3, magic: 3, magic_lore: 2 }),
    ("Berserker",               EncStats { max_lvl: 5, size: 4, strength: 5, endurance: 5, awareness: 2, dodge: 3, melee_skill: 6, miss_skill: 3, ap: 3, magic: 0, magic_lore: 0 }),
    ("Battle Mage",             EncStats { max_lvl: 5, size: 3, strength: 2, endurance: 3, awareness: 3, dodge: 4, melee_skill: 4, miss_skill: 4, ap: 4, magic: 4, magic_lore: 3 }),
    ("Monster: Troll (small)",  EncStats { max_lvl: 3, size: 3,  strength: 3,  endurance: 3, awareness: 2, dodge: 3, melee_skill: 3,  miss_skill: 3, ap: 5,  magic: 0, magic_lore: 0 }),
    ("Monster: Troll (large)",  EncStats { max_lvl: 6, size: 7,  strength: 7,  endurance: 7, awareness: 1, dodge: 2, melee_skill: 6,  miss_skill: 3, ap: -3, magic: 0, magic_lore: 0 }),
    ("Monster: Faerie",         EncStats { max_lvl: 6, size: 3,  strength: 2,  endurance: 3, awareness: 6, dodge: 6, melee_skill: 5,  miss_skill: 4, ap: 2,  magic: 5, magic_lore: 2 }),
    ("Monster: Lizardman",      EncStats { max_lvl: 5, size: 4,  strength: 5,  endurance: 5, awareness: 2, dodge: 2, melee_skill: 5,  miss_skill: 2, ap: 6,  magic: 0, magic_lore: 0 }),
    ("Monster: Giant",          EncStats { max_lvl: 6, size: 12, strength: 12, endurance: 7, awareness: 2, dodge: 0, melee_skill: 6,  miss_skill: 3, ap: -4, magic: 0, magic_lore: 0 }),
    ("Monster: Wyvern",         EncStats { max_lvl: 5, size: 10, strength: 10, endurance: 7, awareness: 5, dodge: 2, melee_skill: -5, miss_skill: 0, ap: -3, magic: 0, magic_lore: 0 }),
    ("Monster: Werewolf",       EncStats { max_lvl: 6, size: 3,  strength: 4,  endurance: 5, awareness: 5, dodge: 5, melee_skill: 5,  miss_skill: 3, ap: 3,  magic: 0, magic_lore: 0 }),
    ("Monster: Zombie",         EncStats { max_lvl: 4, size: 3,  strength: 4,  endurance: 7, awareness: 0, dodge: 0, melee_skill: 3,  miss_skill: 0, ap: 4,  magic: 0, magic_lore: 0 }),
    ("Monster: Skeleton",       EncStats { max_lvl: 3, size: 3,  strength: 2,  endurance: 5, awareness: 0, dodge: 0, melee_skill: 2,  miss_skill: 0, ap: 4,  magic: 0, magic_lore: 0 }),
    ("Monster: Vampire",        EncStats { max_lvl: 7, size: 3,  strength: 5,  endurance: 10,awareness: 3, dodge: 6, melee_skill: 5,  miss_skill: 3, ap: 3,  magic: 3, magic_lore: 3 }),
    ("Monster: Dragon",         EncStats { max_lvl: 7, size: 12, strength: 12, endurance: 7, awareness: 4, dodge: 1, melee_skill: -7, miss_skill: 0, ap: -6, magic: 5, magic_lore: 2 }),
    ("Monster: Red Dragon",     EncStats { max_lvl: 8, size: 14, strength: 14, endurance: 8, awareness: 4, dodge: 1, melee_skill: -8, miss_skill: 0, ap: -7, magic: 6, magic_lore: 3 }),
    ("Monster: Black Dragon",   EncStats { max_lvl: 8, size: 13, strength: 13, endurance: 9, awareness: 5, dodge: 2, melee_skill: -7, miss_skill: 0, ap: -6, magic: 6, magic_lore: 3 }),
    ("Monster: Green Dragon",   EncStats { max_lvl: 7, size: 12, strength: 13, endurance: 8, awareness: 4, dodge: 2, melee_skill: -7, miss_skill: 0, ap: -6, magic: 5, magic_lore: 2 }),
    ("Monster: Blue Dragon",    EncStats { max_lvl: 8, size: 13, strength: 12, endurance: 7, awareness: 5, dodge: 1, melee_skill: -7, miss_skill: 0, ap: -6, magic: 6, magic_lore: 3 }),
    ("Monster: Gold Dragon",    EncStats { max_lvl: 9, size: 15, strength: 15, endurance: 9, awareness: 6, dodge: 2, melee_skill: -8, miss_skill: 0, ap: -7, magic: 7, magic_lore: 4 }),
    ("Monster: Ancient Dragon", EncStats { max_lvl: 10,size: 16, strength: 16, endurance: 10,awareness: 6, dodge: 3, melee_skill: -9, miss_skill: 0, ap: -8, magic: 8, magic_lore: 5 }),
    ("Monster: Drake",          EncStats { max_lvl: 6, size: 8,  strength: 8,  endurance: 6, awareness: 4, dodge: 3, melee_skill: -5, miss_skill: 0, ap: -4, magic: 3, magic_lore: 1 }),
    ("Monster: Hydra",          EncStats { max_lvl: 7, size: 11, strength: 10, endurance: 9, awareness: 3, dodge: 1, melee_skill: -6, miss_skill: 0, ap: -5, magic: 0, magic_lore: 0 }),
    ("Monster: Basilisk",       EncStats { max_lvl: 6, size: 9,  strength: 9,  endurance: 7, awareness: 4, dodge: 2, melee_skill: -5, miss_skill: 0, ap: -4, magic: 3, magic_lore: 1 }),
    ("Monster: Special",        EncStats { max_lvl: 9, size: 7,  strength: 7,  endurance: 7, awareness: 5, dodge: 5, melee_skill: 5,  miss_skill: 5, ap: 10, magic: 0, magic_lore: 0 }),
    ("Small animal: Prey",      EncStats { max_lvl: 3, size: 2, strength: 1, endurance: 3, awareness: 5, dodge: 5, melee_skill: -1, miss_skill: 0, ap: 0, magic: 0, magic_lore: 0 }),
    ("Small animal: Predator",  EncStats { max_lvl: 3, size: 2, strength: 1, endurance: 4, awareness: 5, dodge: 4, melee_skill: -4, miss_skill: 0, ap: 0, magic: 0, magic_lore: 0 }),
    ("Large animal: Prey",      EncStats { max_lvl: 3, size: 5, strength: 5, endurance: 4, awareness: 5, dodge: 5, melee_skill: -2, miss_skill: 0, ap: -1, magic: 0, magic_lore: 0 }),
    ("Large animal: Predator",  EncStats { max_lvl: 3, size: 5, strength: 5, endurance: 5, awareness: 5, dodge: 4, melee_skill: -5, miss_skill: 0, ap: -1, magic: 0, magic_lore: 0 }),
];

pub fn enc_stats(name: &str) -> Option<&'static EncStats> {
    ENCOUNTERS.iter().find(|(n, _)| *n == name).map(|(_, s)| s)
}

// ---------------------------------------------------------------- Encounter
// terrain weight tables

/// Terrain index: 0 City · 1 Rural · 2 Road · 3 Plains · 4 Hills
/// · 5 Mountain · 6 Woods · 7 Wilderness. `terraintype = terrain
/// + 8*day` (day=0 night, day=1 day) — same convention as the
/// Ruby version. Indices into the 16-element weight rows below.
pub const TERRAIN_NAMES: &[&str] = &[
    "City", "Rural", "Road", "Plains", "Hills", "Mountain", "Woods", "Wilderness",
];

/// `$Enc_type` — base encounter category weights, 16 cols
/// (night × terrain | day × terrain).
pub const ENC_TYPE: &[(&str, [u32; 16])] = &[
    // Night                                     | Day
    // C   R   Ro  Pl  Hi  Mo  Wo  Wi              C   R   Ro  Pl  Hi  Mo  Wo  Wi
    ("NO ENCOUNTER",  [ 8,  9, 11, 13, 13, 15, 17, 15,  5,  7,  9, 11, 11, 13, 15, 13]),
    ("smallanimal",   [ 3,  4,  4, 10, 10,  8, 15, 10,  3,  4,  5, 10, 10,  8, 15, 10]),
    ("largeanimal",   [ 2,  5,  5,  8,  7,  6, 12,  7,  4,  5,  5,  8,  7,  6, 12,  7]),
    ("human",         [10,  9,  8,  7,  6,  5,  8,  4, 15, 14, 13, 10,  9,  7, 10,  6]),
    ("dwarf",         [ 2,  2,  2,  3,  4,  6,  1,  3,  3,  3,  3,  3,  6,  8,  1,  3]),
    ("elf",           [ 1,  1,  1,  2,  1,  1,  5,  3,  3,  3,  3,  3,  2,  1,  8,  3]),
    ("araxi",         [ 1,  1,  2,  3,  3,  3,  3,  3,  1,  1,  1,  1,  2,  2,  2,  3]),
    ("monster",       [ 1,  1,  1,  3,  3,  4,  3,  4,  1,  1,  1,  1,  2,  3,  2,  4]),
    ("event",         [ 4,  4,  3,  3,  3,  3,  4,  4,  5,  4,  3,  3,  3,  3,  4,  5]),
];

/// `$Enc_specific[<category>]` — sub-table per encounter category.
/// The category names match the keys in `ENC_TYPE` above.
/// Cells encode the same 16-column terrain/day weights.
pub const ENC_SPECIFIC: &[(&str, &[(&str, [u32; 16])])] = &[
    ("smallanimal", &[
        ("Small animal: Prey",     [ 7, 7, 6, 6, 6, 6, 6, 5, 8, 7, 6, 6, 6, 6, 6, 5]),
        ("Small animal: Predator", [ 3, 3, 4, 4, 4, 4, 4, 5, 2, 3, 4, 4, 4, 4, 4, 5]),
    ]),
    ("largeanimal", &[
        ("Large animal: Prey",     [ 8, 8, 7, 6, 6, 6, 6, 5, 9, 8, 7, 6, 6, 6, 6, 5]),
        ("Large animal: Predator", [ 2, 2, 3, 4, 4, 4, 4, 5, 1, 2, 3, 4, 4, 4, 4, 5]),
    ]),
    ("human", &[
        ("Human: Animal trainer",   [ 2, 5, 4, 5, 5, 5, 6, 6, 2, 5, 4, 5, 5, 5, 6, 6]),
        ("Human: Archer",           [ 3, 4, 4, 5, 5, 5, 5, 5, 3, 4, 4, 5, 5, 5, 5, 5]),
        ("Human: Armour smith",     [ 4, 2, 2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2]),
        ("Human: Army officer",     [ 5, 5, 4, 3, 3, 3, 3, 3, 5, 5, 4, 3, 3, 3, 3, 3]),
        ("Human: Assassin",         [ 3, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1]),
        ("Human: Baker/Cook",       [ 4, 4, 3, 3, 3, 3, 3, 3, 4, 4, 3, 3, 3, 3, 3, 3]),
        ("Human: Bard",             [ 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4]),
        ("Human: Body guard",       [ 4, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3]),
        ("Human: Builder",          [ 5, 5, 3, 4, 4, 4, 4, 4, 5, 5, 3, 4, 4, 4, 4, 4]),
        ("Human: Carpenter",        [ 4, 4, 3, 4, 4, 4, 6, 4, 4, 4, 3, 4, 4, 4, 6, 4]),
        ("Human: Clergyman",        [ 4, 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3]),
        ("Human: Crafts (fine)",    [ 4, 3, 2, 3, 3, 3, 3, 3, 4, 3, 2, 3, 3, 3, 3, 3]),
        ("Human: Crafts (heavy)",   [ 4, 4, 3, 4, 4, 4, 4, 4, 4, 4, 3, 4, 4, 4, 4, 4]),
        ("Human: Entertainer",      [ 5, 4, 4, 4, 4, 4, 4, 4, 5, 4, 4, 4, 4, 4, 4, 4]),
        ("Human: Farmer",           [ 4,10, 8, 7, 6, 5, 6, 5, 4,10, 8, 7, 6, 5, 6, 5]),
        ("Human: Fisherman",        [ 6, 8, 5, 4, 4, 4, 4, 4, 6, 8, 5, 4, 4, 4, 4, 4]),
        ("Human: Gladiator",        [ 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2]),
        ("Human: Highwayman",       [ 2, 3, 6, 6, 6, 6, 6, 6, 1, 3, 6, 6, 6, 6, 6, 6]),
        ("Human: Hunter",           [ 4, 8, 8, 9, 9, 9,10, 9, 4, 8, 8, 9, 9, 9,10, 9]),
        ("Human: Mason",            [ 6, 6, 5, 5, 5, 4, 4, 4, 6, 6, 5, 5, 5, 4, 4, 4]),
        ("Human: Merchant",         [ 7, 6, 6, 6, 5, 5, 5, 5, 7, 6, 6, 6, 5, 5, 5, 5]),
        ("Human: Monk",             [ 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4]),
        ("Human: Prostitute",       [ 6, 2, 2, 2, 2, 2, 2, 2, 4, 2, 2, 2, 2, 2, 2, 2]),
        ("Human: Ranger",           [ 3, 6, 6, 7, 7, 7, 9, 8, 3, 6, 6, 7, 7, 7, 9, 8]),
        ("Human: Sailor",           [ 7, 8, 3, 3, 3, 3, 3, 3, 7, 8, 3, 3, 3, 3, 3, 3]),
        ("Human: Smith",            [ 7, 6, 4, 4, 4, 4, 4, 4, 7, 6, 4, 4, 4, 4, 4, 4]),
        ("Human: Soldier",          [10, 8, 8, 8, 8, 8, 8, 8,  0, 8, 8, 8, 8, 8, 8, 8]),
        ("Human: Tailor",           [ 6, 6, 4, 4, 4, 4, 4, 4, 6, 6, 4, 4, 4, 4, 4, 4]),
        ("Human: Tanner",           [ 5, 5, 4, 4, 4, 4, 6, 5, 5, 5, 4, 4, 4, 4, 6, 5]),
        ("Human: Thief",            [ 5, 4, 5, 3, 3, 3, 3, 3, 3, 3, 5, 3, 3, 3, 3, 3]),
        ("Human: Tracker",          [ 3, 6, 7, 7, 7, 7, 8, 9, 3, 6, 7, 7, 7, 7, 8, 9]),
        ("Human: Warrior",          [ 7, 6, 7, 7, 7, 7, 7, 7, 7, 6, 7, 7, 7, 7, 7, 7]),
    ]),
    ("dwarf", &[
        ("Dwarf: Worker",  [ 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2]),
        ("Dwarf: Warrior", [ 2, 2, 2, 2, 2, 3, 2, 2, 2, 2, 2, 2, 3, 4, 2, 2]),
        ("Dwarf: Guard",   [ 4, 3, 3, 3, 3, 3, 2, 2, 3, 3, 4, 3, 3, 4, 2, 2]),
        ("Dwarf: Smith",   [ 4, 3, 2, 2, 2, 2, 2, 2, 6, 5, 4, 2, 2, 2, 2, 2]),
    ]),
    ("elf", &[
        ("Elf: Worker",  [ 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2]),
        ("Elf: Warrior", [ 2, 2, 2, 2, 2, 2, 3, 3, 2, 2, 2, 2, 2, 2, 3, 3]),
        ("Elf: Archer",  [ 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3]),
        ("Elf: Ranger",  [ 1, 1, 2, 3, 3, 3, 4, 4, 1, 1, 2, 3, 3, 3, 4, 4]),
        ("Elf: Wizard",  [ 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1]),
    ]),
    ("araxi", &[
        ("Araxi: Worker",  [ 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2]),
        ("Araxi: Warrior", [ 3, 3, 3, 3, 3, 3, 3, 4, 3, 3, 3, 3, 3, 3, 3, 4]),
        ("Araxi: Hunter",  [ 3, 3, 3, 4, 4, 4, 4, 4, 3, 3, 3, 4, 4, 4, 4, 4]),
    ]),
    ("monster", &[
        ("Monster: Troll (small)", [ 3, 3, 4, 5, 6, 6, 7, 7, 2, 2, 3, 4, 5, 5, 6, 6]),
        ("Monster: Troll (large)", [ 1, 1, 1, 2, 3, 3, 4, 4, 1, 1, 1, 2, 2, 2, 3, 3]),
        ("Monster: Faerie",        [ 4, 4, 4, 5, 4, 2,10, 9, 4, 4, 4, 6, 4, 2,10, 9]),
        ("Monster: Lizardman",     [ 5, 5, 5, 5, 4, 3, 5, 6, 5, 5, 5, 5, 5, 3, 5, 6]),
        ("Monster: Giant",         [ 2, 2, 2, 2, 3, 4, 2, 2, 2, 2, 2, 2, 3, 4, 2, 2]),
        ("Monster: Wyvern",        [ 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 3, 4, 5, 4, 5]),
        ("Monster: Werewolf",      [ 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4]),
        ("Monster: Zombie",        [ 4, 4, 4, 4, 4, 4, 4, 4, 1, 1, 1, 1, 2, 3, 1, 4]),
        ("Monster: Skeleton",      [ 4, 4, 4, 4, 4, 4, 4, 4, 1, 1, 1, 1, 2, 3, 1, 4]),
        ("Monster: Vampire",       [ 5, 5, 5, 5, 5, 5, 5, 5, 0, 0, 0, 0, 0, 0, 0, 0]),
        ("Monster: Dragon",        [ 1, 1, 1, 1, 1, 2, 2, 2, 1, 1, 1, 1, 1, 2, 2, 2]),
        ("Monster: Special",       [ 6, 6, 6, 6, 6, 6, 6, 6, 5, 5, 5, 5, 5, 5, 5, 6]),
    ]),
];

// ---------------------------------------------------------------- Chartypes

/// Character-type template — the "build" applied to a fresh NPC at
/// generation time. Mirrors Amar-Tools' `$ChartypeNew` rows: base
/// characteristics (BODY/MIND/SPIRIT), attribute bases keyed by
/// `"CHAR/Attribute"`, skill bases keyed by `"CHAR/Attribute/Skill"`,
/// plus melee/missile weapon skill rosters.
pub struct Chartype {
    pub name: &'static str,
    pub body: i32, pub mind: i32, pub spirit: i32,
    pub attributes: &'static [(&'static str, i32)],
    pub skills: &'static [(&'static str, i32)],
    pub melee_weapons: &'static [(&'static str, i32)],
    pub missile_weapons: &'static [(&'static str, i32)],
}

/// 25 most-used chartypes from Amar-Tools' 64-row table. The
/// remaining (Animal trainer, Carpenter, Boatbuilder, …) fall back
/// to the encounter-stat-block path when picked, so every encounter
/// type still produces a usable NPC.
pub const CHARTYPES: &[Chartype] = &[
    Chartype { name: "Warrior", body: 2, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Strength", 4), ("BODY/Endurance", 3), ("BODY/Athletics", 2),
            ("BODY/Melee Combat", 4), ("BODY/Missile Combat", 2), ("BODY/Sleight", 1),
            ("MIND/Awareness", 2), ("MIND/Willpower", 3),
        ],
        skills: &[
            ("BODY/Strength/Carrying", 2), ("BODY/Strength/Weight Lifting", 2),
            ("BODY/Strength/Wield Weapon", 3),
            ("BODY/Endurance/Fortitude", 3), ("BODY/Endurance/Combat Tenacity", 3),
            ("BODY/Endurance/Running", 2),
            ("BODY/Athletics/Climb", 1), ("BODY/Athletics/Balance", 2),
            ("BODY/Athletics/Dodge", 2),
            ("BODY/Melee Combat/Sword", 3),
            ("MIND/Awareness/Reaction Speed", 2),
            ("MIND/Willpower/Pain Tolerance", 3),
            ("MIND/Willpower/Courage", 3),
        ],
        melee_weapons: &[("Sword", 3), ("Shield", 3), ("Axe", 2), ("Spear", 2)],
        missile_weapons: &[("Bow", 2), ("Throwing", 2)],
    },
    Chartype { name: "Mage", body: 1, mind: 2, spirit: 2,
        attributes: &[
            ("BODY/Strength", 2), ("BODY/Endurance", 2), ("BODY/Athletics", 2),
            ("MIND/Nature Knowledge", 4), ("MIND/Social Knowledge", 3),
            ("MIND/Practical Knowledge", 2),
            ("MIND/Awareness", 3), ("MIND/Willpower", 4),
            ("SPIRIT/Casting", 4), ("SPIRIT/Attunement", 4),
        ],
        skills: &[
            ("BODY/Endurance/Fortitude", 2),
            ("MIND/Nature Knowledge/Magick Rituals", 3),
            ("MIND/Nature Knowledge/Alchemy", 3),
            ("MIND/Social Knowledge/Literacy", 3),
            ("MIND/Social Knowledge/Mythology", 3),
            ("MIND/Awareness/Sense Magick", 3),
            ("MIND/Willpower/Mental Fortitude", 3),
            ("SPIRIT/Casting/Range", 3), ("SPIRIT/Casting/Duration", 3),
            ("SPIRIT/Casting/Area of Effect", 2),
            ("SPIRIT/Attunement/Fire", 3), ("SPIRIT/Attunement/Mind", 3),
            ("SPIRIT/Attunement/Self", 3),
        ],
        melee_weapons: &[("Staff", 2), ("Dagger", 2)],
        missile_weapons: &[("Sling", 1)],
    },
    Chartype { name: "Thief", body: 2, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Strength", 2), ("BODY/Endurance", 3), ("BODY/Athletics", 4),
            ("BODY/Melee Combat", 2), ("BODY/Missile Combat", 3), ("BODY/Sleight", 4),
            ("MIND/Social Knowledge", 3), ("MIND/Practical Knowledge", 4),
            ("MIND/Awareness", 4), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Athletics/Hide", 4), ("BODY/Athletics/Move Quietly", 4),
            ("BODY/Athletics/Climb", 3), ("BODY/Athletics/Balance", 3),
            ("BODY/Athletics/Tumble", 2), ("BODY/Athletics/Dodge", 2),
            ("BODY/Sleight/Pick Pockets", 4), ("BODY/Sleight/Disarm Traps", 3),
            ("BODY/Sleight/Pick Locks", 4),
            ("MIND/Awareness/Detect Traps", 3), ("MIND/Awareness/Alertness", 3),
            ("MIND/Practical Knowledge/Ambush", 2),
            ("MIND/Social Knowledge/Social Lore", 2),
        ],
        melee_weapons: &[("Dagger", 3), ("Short Sword", 2), ("Club", 1)],
        missile_weapons: &[("Throwing", 2), ("Sling", 1)],
    },
    Chartype { name: "Ranger", body: 2, mind: 2, spirit: 1,
        attributes: &[
            ("BODY/Strength", 3), ("BODY/Endurance", 3), ("BODY/Athletics", 3),
            ("BODY/Melee Combat", 3), ("BODY/Missile Combat", 4),
            ("MIND/Nature Knowledge", 3), ("MIND/Practical Knowledge", 4),
            ("MIND/Awareness", 4),
            ("SPIRIT/Casting", 1), ("SPIRIT/Attunement", 2),
        ],
        skills: &[
            ("BODY/Athletics/Hide", 3), ("BODY/Athletics/Move Quietly", 3),
            ("BODY/Athletics/Climb", 2), ("BODY/Athletics/Swim", 2),
            ("BODY/Athletics/Ride", 3),
            ("BODY/Endurance/Running", 3),
            ("BODY/Missile Combat/Bow", 3),
            ("MIND/Awareness/Tracking", 3), ("MIND/Awareness/Alertness", 3),
            ("MIND/Awareness/Sense Ambush", 3),
            ("MIND/Practical Knowledge/Survival Lore", 4),
            ("MIND/Practical Knowledge/Set Traps", 3),
            ("MIND/Nature Knowledge/Animal Handling", 3),
            ("MIND/Nature Knowledge/Plant Lore", 3),
        ],
        melee_weapons: &[("Sword", 3), ("Axe", 2), ("Spear", 2), ("Dagger", 2)],
        missile_weapons: &[("Bow", 4), ("Throwing", 2)],
    },
    Chartype { name: "Guard", body: 2, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Strength", 3), ("BODY/Endurance", 3), ("BODY/Athletics", 2),
            ("BODY/Melee Combat", 3), ("BODY/Missile Combat", 2),
            ("MIND/Awareness", 3), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Strength/Wield Weapon", 2),
            ("BODY/Endurance/Fortitude", 2),
            ("BODY/Melee Combat/Spear", 3), ("BODY/Melee Combat/Sword", 2),
            ("BODY/Melee Combat/Shield", 2),
            ("BODY/Missile Combat/X-Bow", 2),
            ("MIND/Awareness/Alertness", 3), ("MIND/Awareness/Reaction Speed", 2),
            ("MIND/Awareness/Detect Traps", 2),
            ("MIND/Awareness/Sense Ambush", 2),
            ("MIND/Willpower/Pain Tolerance", 2),
            ("MIND/Willpower/Courage", 2),
        ],
        melee_weapons: &[("Spear", 3), ("Shield", 2), ("Sword", 2), ("Mace", 2)],
        missile_weapons: &[("X-Bow", 3), ("Throwing", 2)],
    },
    Chartype { name: "Soldier", body: 2, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Strength", 3), ("BODY/Endurance", 4), ("BODY/Athletics", 2),
            ("BODY/Melee Combat", 4), ("BODY/Missile Combat", 3),
            ("MIND/Practical Knowledge", 2),
            ("MIND/Awareness", 2), ("MIND/Willpower", 3),
        ],
        skills: &[
            ("BODY/Strength/Wield Weapon", 3),
            ("BODY/Endurance/Fortitude", 3), ("BODY/Endurance/Combat Tenacity", 3),
            ("BODY/Endurance/Running", 3),
            ("BODY/Athletics/March", 3),
            ("BODY/Melee Combat/Sword", 3), ("BODY/Melee Combat/Spear", 3),
            ("BODY/Melee Combat/Shield", 3),
            ("BODY/Missile Combat/Bow", 3),
            ("MIND/Practical Knowledge/Tactics", 3),
            ("MIND/Willpower/Pain Tolerance", 2),
            ("MIND/Willpower/Mental Fortitude", 3),
        ],
        melee_weapons: &[("Sword", 3), ("Shield", 3), ("Spear", 2)],
        missile_weapons: &[("Bow", 3), ("X-Bow", 2)],
    },
    Chartype { name: "Bandit", body: 2, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Strength", 2), ("BODY/Endurance", 3), ("BODY/Athletics", 3),
            ("BODY/Melee Combat", 3), ("BODY/Missile Combat", 2),
            ("BODY/Sleight", 2),
            ("MIND/Practical Knowledge", 2), ("MIND/Awareness", 3),
        ],
        skills: &[
            ("BODY/Athletics/Hide", 3), ("BODY/Athletics/Move Quietly", 2),
            ("BODY/Athletics/Ride", 2),
            ("BODY/Melee Combat/Sword", 2), ("BODY/Melee Combat/Axe", 2),
            ("BODY/Melee Combat/Dagger", 2),
            ("BODY/Missile Combat/Bow", 2),
            ("BODY/Sleight/Pick Pockets", 2),
            ("MIND/Awareness/Alertness", 2), ("MIND/Awareness/Tracking", 2),
            ("MIND/Practical Knowledge/Ambush", 3),
            ("MIND/Practical Knowledge/Survival Lore", 3),
        ],
        melee_weapons: &[("Sword", 2), ("Axe", 2), ("Dagger", 2)],
        missile_weapons: &[("Bow", 2), ("Throwing", 2)],
    },
    Chartype { name: "Highwayman", body: 2, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Strength", 2), ("BODY/Endurance", 3), ("BODY/Athletics", 3),
            ("BODY/Melee Combat", 4), ("BODY/Missile Combat", 4),
            ("BODY/Sleight", 2),
            ("MIND/Practical Knowledge", 2), ("MIND/Awareness", 3),
        ],
        skills: &[
            ("BODY/Athletics/Ride", 3), ("BODY/Athletics/Hide", 2),
            ("BODY/Athletics/Dodge", 2),
            ("BODY/Melee Combat/Sword", 3),
            ("BODY/Missile Combat/Bow", 3),
            ("BODY/Sleight/Pick Pockets", 2),
            ("MIND/Awareness/Spot Hidden", 3),
            ("MIND/Awareness/Reaction Speed", 2),
            ("MIND/Practical Knowledge/Ambush", 4),
        ],
        melee_weapons: &[("Sword", 3), ("Dagger", 2)],
        missile_weapons: &[("Bow", 3), ("X-Bow", 2)],
    },
    Chartype { name: "Assassin", body: 2, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Strength", 3), ("BODY/Endurance", 3), ("BODY/Athletics", 4),
            ("BODY/Melee Combat", 4), ("BODY/Missile Combat", 4), ("BODY/Sleight", 3),
            ("MIND/Nature Knowledge", 2),
            ("MIND/Practical Knowledge", 3), ("MIND/Awareness", 4),
        ],
        skills: &[
            ("BODY/Athletics/Hide", 4), ("BODY/Athletics/Move Quietly", 4),
            ("BODY/Athletics/Climb", 3), ("BODY/Athletics/Balance", 3),
            ("BODY/Athletics/Dodge", 3),
            ("BODY/Melee Combat/Dagger", 4),
            ("BODY/Missile Combat/Blowgun", 3), ("BODY/Missile Combat/Throwing", 3),
            ("BODY/Sleight/Pick Pockets", 2), ("BODY/Sleight/Pick Locks", 3),
            ("MIND/Practical Knowledge/Ambush", 4),
            ("MIND/Awareness/Detect Traps", 3),
            ("MIND/Nature Knowledge/Poisons", 3),
        ],
        melee_weapons: &[("Dagger", 4), ("Short Sword", 3)],
        missile_weapons: &[("Throwing", 3), ("Blowgun", 3), ("X-Bow", 2)],
    },
    Chartype { name: "Hunter", body: 2, mind: 2, spirit: 1,
        attributes: &[
            ("BODY/Strength", 3), ("BODY/Endurance", 4), ("BODY/Athletics", 3),
            ("BODY/Melee Combat", 3), ("BODY/Missile Combat", 4),
            ("MIND/Nature Knowledge", 3), ("MIND/Practical Knowledge", 3),
            ("MIND/Awareness", 4),
            ("SPIRIT/Attunement", 2), ("SPIRIT/Worship", 1),
        ],
        skills: &[
            ("BODY/Athletics/Hide", 3), ("BODY/Athletics/Move Quietly", 3),
            ("BODY/Missile Combat/Bow", 4),
            ("MIND/Awareness/Tracking", 4), ("MIND/Awareness/Spot Hidden", 3),
            ("MIND/Practical Knowledge/Survival Lore", 3),
            ("MIND/Practical Knowledge/Set Traps", 2),
            ("MIND/Nature Knowledge/Animal Handling", 2),
            ("MIND/Nature Knowledge/Plant Lore", 2),
        ],
        melee_weapons: &[("Spear", 2), ("Dagger", 3), ("Axe", 2)],
        missile_weapons: &[("Bow", 4)],
    },
    Chartype { name: "Tracker", body: 2, mind: 2, spirit: 1,
        attributes: &[
            ("BODY/Endurance", 4), ("BODY/Athletics", 3),
            ("BODY/Melee Combat", 3), ("BODY/Missile Combat", 3),
            ("MIND/Nature Knowledge", 3), ("MIND/Practical Knowledge", 4),
            ("MIND/Awareness", 5),
            ("SPIRIT/Attunement", 2),
        ],
        skills: &[
            ("BODY/Athletics/Hide", 3), ("BODY/Athletics/Move Quietly", 3),
            ("BODY/Missile Combat/Bow", 3),
            ("MIND/Awareness/Tracking", 5), ("MIND/Awareness/Alertness", 4),
            ("MIND/Awareness/Spot Hidden", 4),
            ("MIND/Practical Knowledge/Survival Lore", 4),
            ("MIND/Nature Knowledge/Animal Handling", 3),
        ],
        melee_weapons: &[("Sword", 2), ("Dagger", 2), ("Axe", 2)],
        missile_weapons: &[("Bow", 3)],
    },
    Chartype { name: "Scout", body: 2, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Endurance", 4), ("BODY/Athletics", 4),
            ("BODY/Melee Combat", 3), ("BODY/Missile Combat", 4),
            ("MIND/Practical Knowledge", 3), ("MIND/Awareness", 4),
        ],
        skills: &[
            ("BODY/Athletics/Hide", 4), ("BODY/Athletics/Move Quietly", 4),
            ("BODY/Endurance/Running", 3),
            ("MIND/Awareness/Tracking", 3), ("MIND/Awareness/Alertness", 4),
        ],
        melee_weapons: &[("Sword", 2), ("Knife", 2)],
        missile_weapons: &[("Bow", 4)],
    },
    Chartype { name: "Barbarian", body: 3, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Strength", 5), ("BODY/Endurance", 5), ("BODY/Athletics", 3),
            ("BODY/Melee Combat", 4), ("BODY/Missile Combat", 2),
            ("MIND/Awareness", 2), ("MIND/Willpower", 3),
        ],
        skills: &[
            ("BODY/Strength/Wield Weapon", 4), ("BODY/Strength/Carrying", 3),
            ("BODY/Endurance/Fortitude", 4), ("BODY/Endurance/Combat Tenacity", 4),
            ("MIND/Willpower/Pain Tolerance", 4),
        ],
        melee_weapons: &[("Axe", 3), ("Sword", 2), ("Club", 2)],
        missile_weapons: &[("Throwing", 2)],
    },
    Chartype { name: "Berserker", body: 3, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Strength", 5), ("BODY/Endurance", 5), ("BODY/Athletics", 2),
            ("BODY/Melee Combat", 5), ("BODY/Missile Combat", 1),
            ("MIND/Willpower", 4),
        ],
        skills: &[
            ("BODY/Strength/Wield Weapon", 5),
            ("BODY/Endurance/Fortitude", 4), ("BODY/Endurance/Combat Tenacity", 5),
            ("MIND/Willpower/Pain Tolerance", 5),
        ],
        melee_weapons: &[("Axe", 4), ("Sword", 3)],
        missile_weapons: &[],
    },
    Chartype { name: "Gladiator", body: 3, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Strength", 4), ("BODY/Endurance", 4), ("BODY/Athletics", 3),
            ("BODY/Melee Combat", 4), ("BODY/Missile Combat", 2),
            ("MIND/Social Knowledge", 2),
            ("MIND/Awareness", 3), ("MIND/Willpower", 3),
        ],
        skills: &[
            ("BODY/Strength/Wield Weapon", 4),
            ("BODY/Endurance/Fortitude", 3), ("BODY/Endurance/Combat Tenacity", 4),
            ("BODY/Athletics/Balance", 3), ("BODY/Athletics/Dodge", 3),
            ("BODY/Athletics/Tumble", 2),
            ("BODY/Melee Combat/Sword", 4), ("BODY/Melee Combat/Spear", 3),
            ("BODY/Melee Combat/Net", 3),
            ("MIND/Social Knowledge/Performance", 3),
            ("MIND/Willpower/Mental Fortitude", 3),
        ],
        melee_weapons: &[("Sword", 4), ("Spear", 3), ("Shield", 3), ("Net", 3)],
        missile_weapons: &[("Throwing", 2), ("Spear", 2)],
    },
    Chartype { name: "Body guard", body: 3, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Strength", 4), ("BODY/Endurance", 4), ("BODY/Athletics", 3),
            ("BODY/Melee Combat", 4), ("BODY/Sleight", 2),
            ("MIND/Practical Knowledge", 3), ("MIND/Awareness", 4),
            ("MIND/Willpower", 3),
        ],
        skills: &[
            ("BODY/Strength/Wield Weapon", 3),
            ("BODY/Endurance/Fortitude", 3),
            ("BODY/Athletics/Dodge", 3),
            ("BODY/Melee Combat/Sword", 3), ("BODY/Melee Combat/Shield", 4),
            ("MIND/Awareness/Alertness", 4),
            ("MIND/Awareness/Detect Traps", 3),
            ("MIND/Awareness/Spot Hidden", 3),
            ("MIND/Practical Knowledge/Ambush", 3),
        ],
        melee_weapons: &[("Sword", 3), ("Shield", 4), ("Mace", 2)],
        missile_weapons: &[("Sling", 2)],
    },
    Chartype { name: "Monk", body: 2, mind: 2, spirit: 2,
        attributes: &[
            ("BODY/Endurance", 4), ("BODY/Athletics", 4),
            ("BODY/Melee Combat", 4), ("BODY/Sleight", 2),
            ("MIND/Social Knowledge", 2), ("MIND/Awareness", 3),
            ("MIND/Willpower", 4),
            ("SPIRIT/Casting", 1), ("SPIRIT/Attunement", 3), ("SPIRIT/Worship", 3),
        ],
        skills: &[
            ("BODY/Athletics/Balance", 4), ("BODY/Athletics/Tumble", 3),
            ("BODY/Endurance/Combat Tenacity", 4),
            ("BODY/Melee Combat/Unarmed", 3),
            ("MIND/Social Knowledge/Theology", 3), ("MIND/Social Knowledge/Literacy", 2),
            ("MIND/Willpower/Pain Tolerance", 4),
            ("MIND/Willpower/Mental Fortitude", 4),
            ("MIND/Willpower/Meditation", 4),
            ("SPIRIT/Attunement/Self", 3),
            ("SPIRIT/Worship/Religious Rituals", 4),
        ],
        melee_weapons: &[("Staff", 3), ("Unarmed", 3)],
        missile_weapons: &[],
    },
    Chartype { name: "Priest", body: 1, mind: 2, spirit: 2,
        attributes: &[
            ("BODY/Endurance", 2),
            ("MIND/Nature Knowledge", 3), ("MIND/Social Knowledge", 4),
            ("MIND/Practical Knowledge", 2), ("MIND/Awareness", 2),
            ("MIND/Willpower", 3),
            ("SPIRIT/Casting", 3), ("SPIRIT/Attunement", 3), ("SPIRIT/Worship", 4),
        ],
        skills: &[
            ("MIND/Nature Knowledge/Medical Lore", 3),
            ("MIND/Nature Knowledge/Magick Rituals", 3),
            ("MIND/Social Knowledge/Mythology", 4),
            ("MIND/Social Knowledge/Literacy", 3),
            ("MIND/Willpower/Mental Fortitude", 3),
            ("SPIRIT/Casting/Duration", 3),
            ("SPIRIT/Attunement/Life", 3), ("SPIRIT/Attunement/Body", 3),
            ("SPIRIT/Worship/Ceremony", 4),
        ],
        melee_weapons: &[("Mace", 1), ("Staff", 2)],
        missile_weapons: &[("Sling", 1)],
    },
    Chartype { name: "Sage", body: 1, mind: 3, spirit: 1,
        attributes: &[
            ("BODY/Sleight", 3),
            ("MIND/Nature Knowledge", 4), ("MIND/Social Knowledge", 4),
            ("MIND/Practical Knowledge", 4), ("MIND/Awareness", 3),
            ("MIND/Willpower", 3),
            ("SPIRIT/Casting", 1), ("SPIRIT/Attunement", 2),
        ],
        skills: &[
            ("MIND/Nature Knowledge/Magick Rituals", 3),
            ("MIND/Nature Knowledge/Alchemy", 2),
            ("MIND/Social Knowledge/Literacy", 5),
            ("MIND/Social Knowledge/Mythology", 4),
            ("MIND/Social Knowledge/Legend Lore", 4),
            ("MIND/Social Knowledge/Spoken Language", 4),
            ("MIND/Practical Knowledge/Mathematics", 3),
            ("MIND/Willpower/Mental Fortitude", 3),
        ],
        melee_weapons: &[("Staff", 1)],
        missile_weapons: &[],
    },
    Chartype { name: "Scholar", body: 1, mind: 3, spirit: 0,
        attributes: &[
            ("BODY/Sleight", 2),
            ("MIND/Nature Knowledge", 3), ("MIND/Social Knowledge", 4),
            ("MIND/Practical Knowledge", 2), ("MIND/Awareness", 2),
            ("MIND/Willpower", 3),
        ],
        skills: &[
            ("MIND/Nature Knowledge/Medical Lore", 2),
            ("MIND/Nature Knowledge/Alchemy", 2),
            ("MIND/Social Knowledge/Literacy", 5),
            ("MIND/Social Knowledge/Legend Lore", 3),
            ("MIND/Social Knowledge/Mythology", 3),
            ("MIND/Social Knowledge/Spoken Language", 3),
            ("MIND/Practical Knowledge/Mathematics", 2),
            ("MIND/Willpower/Mental Fortitude", 2),
        ],
        melee_weapons: &[("Dagger", 1), ("Staff", 1)],
        missile_weapons: &[],
    },
    Chartype { name: "Merchant", body: 1, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Strength", 2), ("BODY/Sleight", 2),
            ("MIND/Social Knowledge", 4), ("MIND/Practical Knowledge", 3),
            ("MIND/Awareness", 3),
        ],
        skills: &[
            ("BODY/Strength/Carrying", 2),
            ("BODY/Sleight/Pick Pockets", 1),
            ("MIND/Social Knowledge/Social Lore", 4),
            ("MIND/Social Knowledge/Trading", 4),
            ("MIND/Social Knowledge/Literacy", 3),
            ("MIND/Social Knowledge/Spoken Language", 3),
            ("MIND/Practical Knowledge/Evaluate", 3),
            ("MIND/Awareness/Sense Emotions", 2),
        ],
        melee_weapons: &[("Dagger", 2), ("Club", 1)],
        missile_weapons: &[("Throwing", 1)],
    },
    Chartype { name: "Noble", body: 1, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 3), ("BODY/Melee Combat", 3),
            ("MIND/Social Knowledge", 4), ("MIND/Awareness", 3), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Athletics/Ride", 3),
            ("BODY/Melee Combat/Sword", 2),
            ("MIND/Social Knowledge/Social Lore", 4),
            ("MIND/Social Knowledge/Literacy", 4),
            ("MIND/Social Knowledge/Etiquette", 4),
            ("MIND/Social Knowledge/Leadership", 3),
            ("MIND/Social Knowledge/Legend Lore", 2),
            ("MIND/Willpower/Courage", 2),
        ],
        melee_weapons: &[("Sword", 2), ("Dagger", 1)],
        missile_weapons: &[("X-Bow", 1)],
    },
    Chartype { name: "Sailor", body: 2, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Strength", 3), ("BODY/Endurance", 4), ("BODY/Athletics", 3),
            ("BODY/Melee Combat", 2), ("BODY/Sleight", 3),
            ("MIND/Nature Knowledge", 3), ("MIND/Practical Knowledge", 2),
            ("MIND/Awareness", 2), ("MIND/Willpower", 2),
            ("SPIRIT/Worship", 1),
        ],
        skills: &[
            ("BODY/Strength/Carrying", 3),
            ("BODY/Athletics/Climb", 3), ("BODY/Athletics/Swim", 4),
            ("BODY/Athletics/Balance", 3),
            ("BODY/Sleight/Rope Use", 4),
            ("MIND/Nature Knowledge/Boating", 4),
            ("MIND/Nature Knowledge/Weather", 3),
            ("MIND/Practical Knowledge/Navigation", 2),
        ],
        melee_weapons: &[("Sword", 2), ("Club", 2)],
        missile_weapons: &[],
    },
    Chartype { name: "Smith", body: 3, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Strength", 4), ("BODY/Endurance", 4), ("BODY/Sleight", 3),
            ("BODY/Melee Combat", 2),
            ("MIND/Nature Knowledge", 3), ("MIND/Practical Knowledge", 3),
        ],
        skills: &[
            ("BODY/Strength/Weight Lifting", 4),
            ("BODY/Strength/Carrying", 3), ("BODY/Strength/Wield Weapon", 4),
            ("BODY/Endurance/Fortitude", 3),
            ("BODY/Sleight/Crafts", 3),
            ("MIND/Nature Knowledge/Metals", 4),
            ("MIND/Practical Knowledge/Smithing", 4),
            ("MIND/Practical Knowledge/Engineering", 2),
        ],
        melee_weapons: &[("Hammer", 2), ("Axe", 2), ("Club", 2)],
        missile_weapons: &[],
    },
    Chartype { name: "Commoner", body: 1, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Strength", 2), ("BODY/Endurance", 2), ("BODY/Athletics", 1),
            ("MIND/Nature Knowledge", 1), ("MIND/Social Knowledge", 1),
            ("MIND/Practical Knowledge", 2), ("MIND/Awareness", 1),
        ],
        skills: &[
            ("BODY/Strength/Carrying", 2),
            ("BODY/Endurance/Fortitude", 2), ("BODY/Endurance/Running", 1),
            ("MIND/Nature Knowledge/Medical Lore", 1),
            ("MIND/Social Knowledge/Social Lore", 1),
            ("MIND/Practical Knowledge/Survival Lore", 2),
        ],
        melee_weapons: &[("Dagger", 1), ("Club", 1)],
        missile_weapons: &[("Throwing", 1)],
    },
    Chartype { name: "Farmer", body: 2, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Strength", 3), ("BODY/Endurance", 3),
            ("MIND/Nature Knowledge", 3), ("MIND/Practical Knowledge", 2),
            ("SPIRIT/Attunement", 1), ("SPIRIT/Worship", 1),
        ],
        skills: &[
            ("BODY/Strength/Carrying", 3),
            ("BODY/Endurance/Fortitude", 3),
            ("MIND/Nature Knowledge/Plant Lore", 3),
            ("MIND/Nature Knowledge/Animal Handling", 3),
            ("MIND/Nature Knowledge/Agriculture", 4),
            ("MIND/Nature Knowledge/Weather", 3),
        ],
        melee_weapons: &[("Club", 2), ("Scythe", 2)],
        missile_weapons: &[("Sling", 2)],
    },
    Chartype { name: "Bard", body: 1, mind: 2, spirit: 1,
        attributes: &[
            ("BODY/Athletics", 2), ("BODY/Sleight", 3),
            ("MIND/Social Knowledge", 4), ("MIND/Awareness", 3),
            ("SPIRIT/Casting", 1), ("SPIRIT/Attunement", 2),
        ],
        skills: &[
            ("BODY/Athletics/Tumble", 2),
            ("BODY/Sleight/Crafts", 2),
            ("MIND/Social Knowledge/Social Lore", 3),
            ("MIND/Social Knowledge/Mythology", 3),
            ("MIND/Social Knowledge/Legend Lore", 4),
            ("MIND/Social Knowledge/Music", 4),
            ("MIND/Social Knowledge/Performance", 4),
            ("MIND/Awareness/Sense Emotions", 3),
        ],
        melee_weapons: &[("Sword", 2), ("Dagger", 2)],
        missile_weapons: &[("Throwing", 2)],
    },
    Chartype { name: "Witch (white)", body: 1, mind: 2, spirit: 3,
        attributes: &[
            ("BODY/Sleight", 3),
            ("MIND/Nature Knowledge", 4), ("MIND/Social Knowledge", 3),
            ("MIND/Practical Knowledge", 3), ("MIND/Awareness", 3),
            ("MIND/Willpower", 3),
            ("SPIRIT/Casting", 4), ("SPIRIT/Attunement", 4), ("SPIRIT/Worship", 2),
        ],
        skills: &[
            ("BODY/Sleight/Crafts", 3),
            ("MIND/Nature Knowledge/Alchemy", 4),
            ("MIND/Nature Knowledge/Magick Rituals", 3),
            ("MIND/Nature Knowledge/Medical Lore", 4),
            ("MIND/Social Knowledge/Mythology", 3),
            ("MIND/Awareness/Sense Magick", 3),
            ("MIND/Willpower/Mental Fortitude", 3),
            ("SPIRIT/Casting/Range", 3), ("SPIRIT/Casting/Duration", 3),
            ("SPIRIT/Attunement/Life", 4), ("SPIRIT/Attunement/Body", 3),
            ("SPIRIT/Attunement/Self", 4),
        ],
        melee_weapons: &[("Staff", 1), ("Dagger", 1)],
        missile_weapons: &[],
    },
    Chartype { name: "Witch (black)", body: 1, mind: 2, spirit: 3,
        attributes: &[
            ("BODY/Sleight", 3),
            ("MIND/Nature Knowledge", 4), ("MIND/Social Knowledge", 2),
            ("MIND/Practical Knowledge", 3), ("MIND/Awareness", 3),
            ("MIND/Willpower", 3),
            ("SPIRIT/Casting", 4), ("SPIRIT/Attunement", 4),
        ],
        skills: &[
            ("BODY/Sleight/Crafts", 3),
            ("MIND/Nature Knowledge/Alchemy", 4),
            ("MIND/Nature Knowledge/Magick Rituals", 3),
            ("MIND/Nature Knowledge/Poisons", 4),
            ("MIND/Awareness/Sense Magick", 3),
            ("MIND/Willpower/Mental Fortitude", 3),
            ("SPIRIT/Casting/Range", 3), ("SPIRIT/Casting/Duration", 3),
            ("SPIRIT/Attunement/Death", 4), ("SPIRIT/Attunement/Mind", 3),
            ("SPIRIT/Attunement/Spirits", 3),
        ],
        melee_weapons: &[("Dagger", 2), ("Staff", 1)],
        missile_weapons: &[],
    },
    Chartype { name: "Wizard (fire)", body: 1, mind: 3, spirit: 3,
        attributes: &[
            ("MIND/Nature Knowledge", 4), ("MIND/Social Knowledge", 3),
            ("MIND/Practical Knowledge", 3), ("MIND/Awareness", 3),
            ("MIND/Willpower", 4),
            ("SPIRIT/Casting", 4), ("SPIRIT/Attunement", 4),
        ],
        skills: &[
            ("MIND/Nature Knowledge/Magick Rituals", 4),
            ("MIND/Nature Knowledge/Alchemy", 3),
            ("MIND/Social Knowledge/Literacy", 3),
            ("MIND/Practical Knowledge/Mathematics", 3),
            ("MIND/Awareness/Sense Magick", 3),
            ("MIND/Willpower/Mental Fortitude", 3),
            ("SPIRIT/Casting/Range", 4), ("SPIRIT/Casting/Duration", 3),
            ("SPIRIT/Casting/Area of Effect", 3),
            ("SPIRIT/Attunement/Fire", 4), ("SPIRIT/Attunement/Self", 2),
        ],
        melee_weapons: &[("Staff", 2), ("Dagger", 1)],
        missile_weapons: &[],
    },
    Chartype { name: "Wizard (water)", body: 1, mind: 3, spirit: 3,
        attributes: &[
            ("MIND/Nature Knowledge", 4), ("MIND/Social Knowledge", 3),
            ("MIND/Practical Knowledge", 3), ("MIND/Awareness", 3),
            ("MIND/Willpower", 4),
            ("SPIRIT/Casting", 4), ("SPIRIT/Attunement", 4),
        ],
        skills: &[
            ("BODY/Athletics/Swim", 2),
            ("MIND/Nature Knowledge/Magick Rituals", 4),
            ("MIND/Nature Knowledge/Weather", 3),
            ("MIND/Social Knowledge/Literacy", 3),
            ("MIND/Awareness/Sense Magick", 3),
            ("MIND/Willpower/Mental Fortitude", 3),
            ("SPIRIT/Casting/Range", 3), ("SPIRIT/Casting/Duration", 4),
            ("SPIRIT/Attunement/Water", 4), ("SPIRIT/Attunement/Self", 2),
        ],
        melee_weapons: &[("Staff", 2), ("Dagger", 1)],
        missile_weapons: &[],
    },
    Chartype { name: "Wizard (air)", body: 1, mind: 3, spirit: 3,
        attributes: &[
            ("MIND/Nature Knowledge", 4), ("MIND/Social Knowledge", 3),
            ("MIND/Practical Knowledge", 3), ("MIND/Awareness", 3),
            ("MIND/Willpower", 4),
            ("SPIRIT/Casting", 4), ("SPIRIT/Attunement", 4),
        ],
        skills: &[
            ("MIND/Nature Knowledge/Magick Rituals", 4),
            ("MIND/Nature Knowledge/Weather", 3),
            ("MIND/Social Knowledge/Literacy", 3),
            ("MIND/Awareness/Sense Magick", 3),
            ("MIND/Willpower/Mental Fortitude", 3),
            ("SPIRIT/Casting/Range", 4), ("SPIRIT/Casting/Area of Effect", 3),
            ("SPIRIT/Attunement/Air", 4), ("SPIRIT/Attunement/Self", 2),
        ],
        melee_weapons: &[("Staff", 2), ("Dagger", 1)],
        missile_weapons: &[],
    },
    Chartype { name: "Wizard (earth)", body: 1, mind: 3, spirit: 3,
        attributes: &[
            ("MIND/Nature Knowledge", 4), ("MIND/Social Knowledge", 3),
            ("MIND/Practical Knowledge", 3), ("MIND/Awareness", 3),
            ("MIND/Willpower", 4),
            ("SPIRIT/Casting", 4), ("SPIRIT/Attunement", 4),
        ],
        skills: &[
            ("MIND/Nature Knowledge/Magick Rituals", 4),
            ("MIND/Nature Knowledge/Plant Lore", 3),
            ("MIND/Social Knowledge/Literacy", 3),
            ("MIND/Awareness/Sense Magick", 3),
            ("MIND/Willpower/Mental Fortitude", 3),
            ("SPIRIT/Casting/Duration", 4), ("SPIRIT/Casting/Area of Effect", 3),
            ("SPIRIT/Attunement/Earth", 4), ("SPIRIT/Attunement/Self", 2),
        ],
        melee_weapons: &[("Staff", 2), ("Dagger", 1)],
        missile_weapons: &[],
    },
    Chartype { name: "Sorcerer", body: 1, mind: 2, spirit: 3,
        attributes: &[
            ("MIND/Nature Knowledge", 3), ("MIND/Social Knowledge", 2),
            ("MIND/Practical Knowledge", 3), ("MIND/Awareness", 2),
            ("MIND/Willpower", 4),
            ("SPIRIT/Casting", 4), ("SPIRIT/Attunement", 3),
        ],
        skills: &[
            ("MIND/Nature Knowledge/Magick Rituals", 4),
            ("MIND/Social Knowledge/Mythology", 3),
            ("MIND/Awareness/Sense Magick", 3),
            ("MIND/Willpower/Mental Fortitude", 4),
            ("SPIRIT/Casting/Range", 3), ("SPIRIT/Casting/Duration", 3),
            ("SPIRIT/Attunement/Mind", 4), ("SPIRIT/Attunement/Spirits", 3),
        ],
        melee_weapons: &[("Staff", 1), ("Dagger", 1)],
        missile_weapons: &[],
    },
    // ---- Occupational chartypes ported from Amar-Tools' ----
    // chartype_new_full.rb so the encounter table's "Human: Animal
    // trainer / Archer / Armour smith / …" rolls land on richer
    // templates than the Commoner fallback.

    Chartype { name: "Animal trainer", body: 2, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 3), ("BODY/Endurance", 3), ("BODY/Melee Combat", 2),
            ("BODY/Missile Combat", 2), ("BODY/Sleight", 1), ("BODY/Strength", 2),
            ("MIND/Awareness", 3), ("MIND/Nature Knowledge", 4),
            ("MIND/Practical Knowledge", 3), ("MIND/Social Knowledge", 2), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Athletics/Balance", 2), ("BODY/Athletics/Climb", 1),
            ("BODY/Athletics/Dodge", 1), ("BODY/Athletics/Hide", 2),
            ("BODY/Athletics/Move Quietly", 2), ("BODY/Athletics/Ride", 3),
            ("BODY/Athletics/Swim", 1), ("BODY/Athletics/Tumble", 2),
            ("MIND/Awareness/Tracking", 3),
            ("MIND/Nature Knowledge/Animal Handling", 4),
            ("MIND/Nature Knowledge/Animal Lore", 3),
            ("MIND/Practical Knowledge/Survival Lore", 3),
        ],
        melee_weapons: &[("Staff", 2), ("Dagger", 1)],
        missile_weapons: &[("Sling", 2), ("Throwing", 1)],
    },

    Chartype { name: "Archer", body: 2, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 2), ("BODY/Endurance", 3), ("BODY/Melee Combat", 2),
            ("BODY/Missile Combat", 4), ("BODY/Sleight", 2), ("BODY/Strength", 3),
            ("MIND/Awareness", 4), ("MIND/Nature Knowledge", 2),
            ("MIND/Practical Knowledge", 2), ("MIND/Social Knowledge", 1), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Athletics/Hide", 3), ("BODY/Athletics/Move Quietly", 2),
            ("BODY/Missile Combat/Bow", 4), ("BODY/Missile Combat/X-Bow", 3),
            ("BODY/Strength/Wield Weapon", 3),
            ("MIND/Awareness/Detect Traps", 2), ("MIND/Awareness/Reaction Speed", 3),
            ("MIND/Awareness/Tracking", 2),
        ],
        melee_weapons: &[("Sword", 2), ("Dagger", 2)],
        missile_weapons: &[("Bow", 4), ("X-Bow", 3)],
    },

    Chartype { name: "Armour smith", body: 2, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 1), ("BODY/Endurance", 3), ("BODY/Melee Combat", 2),
            ("BODY/Missile Combat", 1), ("BODY/Sleight", 2), ("BODY/Strength", 4),
            ("MIND/Awareness", 2), ("MIND/Nature Knowledge", 2),
            ("MIND/Practical Knowledge", 3), ("MIND/Social Knowledge", 2), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Endurance/Fortitude", 2),
            ("BODY/Strength/Weight Lifting", 3),
            ("MIND/Nature Knowledge/Metals", 3),
            ("MIND/Practical Knowledge/Crafts", 4),
            ("MIND/Social Knowledge/Trading", 2),
        ],
        melee_weapons: &[("Hammer", 3), ("Dagger", 1)],
        missile_weapons: &[("Throwing", 1)],
    },

    Chartype { name: "Army officer", body: 2, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 2), ("BODY/Endurance", 3), ("BODY/Melee Combat", 3),
            ("BODY/Missile Combat", 2), ("BODY/Sleight", 1), ("BODY/Strength", 3),
            ("MIND/Awareness", 3), ("MIND/Nature Knowledge", 1),
            ("MIND/Practical Knowledge", 3), ("MIND/Social Knowledge", 3), ("MIND/Willpower", 3),
        ],
        skills: &[
            ("BODY/Athletics/Ride", 2),
            ("BODY/Melee Combat/Sword", 3),
            ("MIND/Practical Knowledge/Tactics", 3),
            ("MIND/Social Knowledge/Leadership", 3),
            ("MIND/Willpower/Courage", 3),
        ],
        melee_weapons: &[("Sword", 3), ("Spear", 2)],
        missile_weapons: &[("X-Bow", 2)],
    },

    Chartype { name: "Boatbuilder", body: 2, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 2), ("BODY/Endurance", 2), ("BODY/Melee Combat", 1),
            ("BODY/Missile Combat", 1), ("BODY/Sleight", 3), ("BODY/Strength", 3),
            ("MIND/Awareness", 2), ("MIND/Nature Knowledge", 2),
            ("MIND/Practical Knowledge", 3), ("MIND/Social Knowledge", 1), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Athletics/Swim", 3),
            ("BODY/Sleight/Crafts", 4), ("BODY/Sleight/Rope Use", 3),
            ("BODY/Strength/Carrying", 3), ("BODY/Strength/Wield Weapon", 2),
            ("MIND/Nature Knowledge/Boating", 4),
            ("MIND/Practical Knowledge/Engineering", 3),
            ("MIND/Practical Knowledge/Navigation", 2),
        ],
        melee_weapons: &[("Club", 2), ("Axe", 2)],
        missile_weapons: &[],
    },

    Chartype { name: "Builder", body: 2, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 2), ("BODY/Endurance", 3), ("BODY/Melee Combat", 1),
            ("BODY/Missile Combat", 1), ("BODY/Sleight", 3), ("BODY/Strength", 3),
            ("MIND/Awareness", 2), ("MIND/Nature Knowledge", 2),
            ("MIND/Practical Knowledge", 3), ("MIND/Social Knowledge", 1), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Athletics/Climb", 3),
            ("BODY/Endurance/Fortitude", 2),
            ("BODY/Sleight/Crafts", 3),
            ("BODY/Strength/Carrying", 3),
            ("MIND/Practical Knowledge/Engineering", 3),
            ("MIND/Practical Knowledge/Mathematics", 2),
        ],
        melee_weapons: &[("Club", 2)],
        missile_weapons: &[],
    },

    Chartype { name: "Bureaucrat", body: 1, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 1), ("BODY/Endurance", 1), ("BODY/Sleight", 2),
            ("BODY/Strength", 1),
            ("MIND/Awareness", 2), ("MIND/Nature Knowledge", 1),
            ("MIND/Practical Knowledge", 3), ("MIND/Social Knowledge", 3), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("MIND/Practical Knowledge/Administration", 4),
            ("MIND/Practical Knowledge/Mathematics", 3),
            ("MIND/Social Knowledge/Law", 4),
            ("MIND/Social Knowledge/Literacy", 4),
            ("MIND/Social Knowledge/Politics", 3),
        ],
        melee_weapons: &[],
        missile_weapons: &[],
    },

    Chartype { name: "Carpenter", body: 2, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 2), ("BODY/Endurance", 2), ("BODY/Melee Combat", 1),
            ("BODY/Missile Combat", 1), ("BODY/Sleight", 4), ("BODY/Strength", 2),
            ("MIND/Awareness", 2), ("MIND/Nature Knowledge", 2),
            ("MIND/Practical Knowledge", 3), ("MIND/Social Knowledge", 1), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Sleight/Crafts", 4), ("BODY/Sleight/Fine Crafts", 3),
            ("BODY/Strength/Wield Weapon", 2),
            ("MIND/Nature Knowledge/Wood", 3),
            ("MIND/Practical Knowledge/Engineering", 2),
            ("MIND/Practical Knowledge/Mathematics", 2),
        ],
        melee_weapons: &[("Club", 2), ("Axe", 2)],
        missile_weapons: &[],
    },

    Chartype { name: "Clergyman", body: 1, mind: 2, spirit: 2,
        attributes: &[
            ("BODY/Athletics", 1), ("BODY/Endurance", 1), ("BODY/Melee Combat", 1),
            ("BODY/Sleight", 2), ("BODY/Strength", 1),
            ("MIND/Awareness", 2), ("MIND/Nature Knowledge", 2),
            ("MIND/Practical Knowledge", 2), ("MIND/Social Knowledge", 3), ("MIND/Willpower", 3),
            ("SPIRIT/Attunement", 2), ("SPIRIT/Casting", 2), ("SPIRIT/Worship", 4),
        ],
        skills: &[
            ("MIND/Social Knowledge/Mythology", 4),
            ("MIND/Social Knowledge/Literacy", 3),
            ("MIND/Willpower/Mental Fortitude", 3),
            ("SPIRIT/Attunement/Life", 2),
        ],
        melee_weapons: &[("Staff", 2)],
        missile_weapons: &[],
    },

    Chartype { name: "Crafts (fine)", body: 1, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 1), ("BODY/Endurance", 1), ("BODY/Sleight", 4),
            ("BODY/Strength", 1),
            ("MIND/Awareness", 3), ("MIND/Nature Knowledge", 2),
            ("MIND/Practical Knowledge", 3), ("MIND/Social Knowledge", 2), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Sleight/Crafts", 4),
            ("MIND/Practical Knowledge/Mathematics", 3),
        ],
        melee_weapons: &[("Knife", 1)],
        missile_weapons: &[],
    },

    Chartype { name: "Crafts (heavy)", body: 2, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 1), ("BODY/Endurance", 3), ("BODY/Melee Combat", 1),
            ("BODY/Sleight", 4), ("BODY/Strength", 3),
            ("MIND/Awareness", 2), ("MIND/Nature Knowledge", 2),
            ("MIND/Practical Knowledge", 3), ("MIND/Social Knowledge", 1), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Endurance/Fortitude", 3),
            ("BODY/Sleight/Crafts", 4),
            ("BODY/Strength/Carrying", 3),
            ("MIND/Nature Knowledge/Metals", 3),
            ("MIND/Practical Knowledge/Engineering", 2),
        ],
        melee_weapons: &[("Club", 2), ("Hammer", 1)],
        missile_weapons: &[],
    },

    Chartype { name: "Entertainer", body: 1, mind: 1, spirit: 1,
        attributes: &[
            ("BODY/Athletics", 3), ("BODY/Endurance", 2), ("BODY/Melee Combat", 1),
            ("BODY/Missile Combat", 1), ("BODY/Sleight", 3), ("BODY/Strength", 1),
            ("MIND/Awareness", 3), ("MIND/Nature Knowledge", 1),
            ("MIND/Practical Knowledge", 2), ("MIND/Social Knowledge", 3), ("MIND/Willpower", 2),
            ("SPIRIT/Attunement", 2), ("SPIRIT/Worship", 1),
        ],
        skills: &[
            ("BODY/Athletics/Tumble", 3), ("BODY/Athletics/Balance", 3),
            ("MIND/Social Knowledge/Performance", 4), ("MIND/Social Knowledge/Social Lore", 3),
        ],
        melee_weapons: &[("Dagger", 2)],
        missile_weapons: &[("Throwing", 2)],
    },

    Chartype { name: "Executioner", body: 3, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 2), ("BODY/Endurance", 3), ("BODY/Melee Combat", 3),
            ("BODY/Missile Combat", 1), ("BODY/Sleight", 2), ("BODY/Strength", 4),
            ("MIND/Awareness", 2), ("MIND/Nature Knowledge", 2),
            ("MIND/Practical Knowledge", 2), ("MIND/Social Knowledge", 1), ("MIND/Willpower", 3),
        ],
        skills: &[
            ("BODY/Endurance/Fortitude", 3),
            ("BODY/Melee Combat/Axe", 4), ("BODY/Melee Combat/Sword", 3),
            ("BODY/Strength/Wield Weapon", 4),
            ("MIND/Willpower/Mental Fortitude", 3),
        ],
        melee_weapons: &[("Axe", 4), ("Sword", 3)],
        missile_weapons: &[],
    },

    Chartype { name: "Fine artist", body: 1, mind: 2, spirit: 2,
        attributes: &[
            ("BODY/Athletics", 1), ("BODY/Endurance", 1), ("BODY/Sleight", 4),
            ("BODY/Strength", 1),
            ("MIND/Awareness", 4), ("MIND/Nature Knowledge", 2),
            ("MIND/Practical Knowledge", 2), ("MIND/Social Knowledge", 3), ("MIND/Willpower", 2),
            ("SPIRIT/Attunement", 3), ("SPIRIT/Worship", 1),
        ],
        skills: &[
            ("BODY/Sleight/Crafts", 4),
            ("MIND/Social Knowledge/Literacy", 3),
        ],
        melee_weapons: &[],
        missile_weapons: &[],
    },

    Chartype { name: "Fine smith", body: 2, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 1), ("BODY/Endurance", 2), ("BODY/Melee Combat", 1),
            ("BODY/Sleight", 4), ("BODY/Strength", 2),
            ("MIND/Awareness", 3), ("MIND/Nature Knowledge", 3),
            ("MIND/Practical Knowledge", 3), ("MIND/Social Knowledge", 2), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Sleight/Crafts", 4),
            ("MIND/Nature Knowledge/Metals", 4),
            ("MIND/Practical Knowledge/Smithing", 4),
        ],
        melee_weapons: &[("Hammer", 2)],
        missile_weapons: &[],
    },

    Chartype { name: "Fisherman", body: 2, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 2), ("BODY/Endurance", 3), ("BODY/Melee Combat", 1),
            ("BODY/Missile Combat", 2), ("BODY/Sleight", 3), ("BODY/Strength", 2),
            ("MIND/Awareness", 3), ("MIND/Nature Knowledge", 3),
            ("MIND/Practical Knowledge", 2), ("MIND/Social Knowledge", 1), ("MIND/Willpower", 2),
            ("SPIRIT/Worship", 1),
        ],
        skills: &[
            ("BODY/Athletics/Swim", 3),
            ("BODY/Sleight/Rope Use", 3),
            ("MIND/Nature Knowledge/Boating", 3),
            ("MIND/Nature Knowledge/Weather", 3),
        ],
        melee_weapons: &[("Club", 2), ("Spear", 2)],
        missile_weapons: &[("Spear", 2)],
    },

    Chartype { name: "High class", body: 1, mind: 2, spirit: 1,
        attributes: &[
            ("BODY/Athletics", 2), ("BODY/Endurance", 1), ("BODY/Melee Combat", 2),
            ("BODY/Missile Combat", 1), ("BODY/Sleight", 2), ("BODY/Strength", 1),
            ("MIND/Awareness", 2), ("MIND/Nature Knowledge", 2),
            ("MIND/Practical Knowledge", 2), ("MIND/Social Knowledge", 4), ("MIND/Willpower", 2),
            ("SPIRIT/Attunement", 2), ("SPIRIT/Worship", 2),
        ],
        skills: &[
            ("BODY/Athletics/Ride", 3),
            ("BODY/Melee Combat/Sword", 2),
            ("MIND/Social Knowledge/Etiquette", 4),
            ("MIND/Social Knowledge/Literacy", 3),
            ("MIND/Social Knowledge/Politics", 3),
        ],
        melee_weapons: &[("Sword", 2), ("Dagger", 1)],
        missile_weapons: &[],
    },

    Chartype { name: "House wife", body: 1, mind: 1, spirit: 1,
        attributes: &[
            ("BODY/Athletics", 1), ("BODY/Endurance", 2), ("BODY/Melee Combat", 1),
            ("BODY/Missile Combat", 1), ("BODY/Sleight", 3), ("BODY/Strength", 2),
            ("MIND/Awareness", 2), ("MIND/Nature Knowledge", 2),
            ("MIND/Practical Knowledge", 3), ("MIND/Social Knowledge", 2), ("MIND/Willpower", 2),
            ("SPIRIT/Attunement", 2), ("SPIRIT/Worship", 2),
        ],
        skills: &[
            ("BODY/Endurance/Fortitude", 2),
            ("BODY/Sleight/Crafts", 3),
            ("MIND/Nature Knowledge/Medical Lore", 2),
            ("MIND/Practical Knowledge/Administration", 3),
        ],
        melee_weapons: &[("Club", 1), ("Knife", 2)],
        missile_weapons: &[],
    },

    Chartype { name: "Jeweller", body: 1, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 1), ("BODY/Endurance", 1), ("BODY/Sleight", 4),
            ("BODY/Strength", 1),
            ("MIND/Awareness", 4), ("MIND/Nature Knowledge", 3),
            ("MIND/Practical Knowledge", 3), ("MIND/Social Knowledge", 3), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Sleight/Crafts", 4),
            ("MIND/Nature Knowledge/Gems", 4),
            ("MIND/Nature Knowledge/Metals", 3),
            ("MIND/Practical Knowledge/Mathematics", 3),
            ("MIND/Social Knowledge/Trading", 3),
        ],
        melee_weapons: &[],
        missile_weapons: &[],
    },

    Chartype { name: "Mapmaker", body: 1, mind: 3, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 2), ("BODY/Endurance", 2), ("BODY/Sleight", 4),
            ("BODY/Strength", 1),
            ("MIND/Awareness", 4), ("MIND/Nature Knowledge", 3),
            ("MIND/Practical Knowledge", 4), ("MIND/Social Knowledge", 2), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Sleight/Crafts", 3),
            ("MIND/Nature Knowledge/Geography", 4),
            ("MIND/Practical Knowledge/Mathematics", 4),
            ("MIND/Practical Knowledge/Navigation", 4),
            ("MIND/Social Knowledge/Literacy", 4),
        ],
        melee_weapons: &[],
        missile_weapons: &[],
    },

    Chartype { name: "Mason", body: 2, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 2), ("BODY/Endurance", 3), ("BODY/Melee Combat", 1),
            ("BODY/Sleight", 3), ("BODY/Strength", 3),
            ("MIND/Awareness", 2), ("MIND/Nature Knowledge", 3),
            ("MIND/Practical Knowledge", 3), ("MIND/Social Knowledge", 1), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Endurance/Fortitude", 3),
            ("BODY/Sleight/Crafts", 3),
            ("BODY/Strength/Carrying", 3),
            ("MIND/Nature Knowledge/Stone", 4),
            ("MIND/Practical Knowledge/Engineering", 3),
            ("MIND/Practical Knowledge/Mathematics", 2),
        ],
        melee_weapons: &[("Club", 2), ("Hammer", 2)],
        missile_weapons: &[],
    },

    Chartype { name: "Messenger", body: 2, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 4), ("BODY/Endurance", 4), ("BODY/Melee Combat", 2),
            ("BODY/Missile Combat", 2), ("BODY/Sleight", 2), ("BODY/Strength", 2),
            ("MIND/Awareness", 3), ("MIND/Nature Knowledge", 2),
            ("MIND/Practical Knowledge", 3), ("MIND/Social Knowledge", 2), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Athletics/Dodge", 2), ("BODY/Athletics/Ride", 4),
            ("BODY/Endurance/Fortitude", 3), ("BODY/Endurance/Running", 4),
            ("MIND/Nature Knowledge/Geography", 3),
            ("MIND/Practical Knowledge/Navigation", 3),
        ],
        melee_weapons: &[("Sword", 2), ("Dagger", 2)],
        missile_weapons: &[("Bow", 2)],
    },

    Chartype { name: "Nanny", body: 1, mind: 1, spirit: 1,
        attributes: &[
            ("BODY/Athletics", 1), ("BODY/Endurance", 2), ("BODY/Sleight", 2),
            ("BODY/Strength", 1),
            ("MIND/Awareness", 3), ("MIND/Nature Knowledge", 2),
            ("MIND/Practical Knowledge", 3), ("MIND/Social Knowledge", 3), ("MIND/Willpower", 3),
            ("SPIRIT/Attunement", 3), ("SPIRIT/Worship", 2),
        ],
        skills: &[
            ("BODY/Sleight/Crafts", 2),
            ("MIND/Nature Knowledge/Medical Lore", 2),
            ("MIND/Social Knowledge/Etiquette", 2),
            ("MIND/Willpower/Mental Fortitude", 3),
        ],
        melee_weapons: &[],
        missile_weapons: &[],
    },

    Chartype { name: "Navigator", body: 1, mind: 3, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 2), ("BODY/Endurance", 2), ("BODY/Melee Combat", 1),
            ("BODY/Missile Combat", 1), ("BODY/Sleight", 2), ("BODY/Strength", 2),
            ("MIND/Awareness", 3), ("MIND/Nature Knowledge", 4),
            ("MIND/Practical Knowledge", 4), ("MIND/Social Knowledge", 2), ("MIND/Willpower", 2),
            ("SPIRIT/Attunement", 1),
        ],
        skills: &[
            ("BODY/Athletics/Swim", 2),
            ("MIND/Nature Knowledge/Boating", 3),
            ("MIND/Nature Knowledge/Geography", 4),
            ("MIND/Nature Knowledge/Weather", 3),
            ("MIND/Practical Knowledge/Mathematics", 4),
            ("MIND/Practical Knowledge/Navigation", 4),
        ],
        melee_weapons: &[("Sword", 1), ("Dagger", 1)],
        missile_weapons: &[],
    },

    Chartype { name: "Baker/Cook", body: 1, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 1), ("BODY/Endurance", 2), ("BODY/Melee Combat", 1),
            ("BODY/Missile Combat", 1), ("BODY/Sleight", 2), ("BODY/Strength", 2),
            ("MIND/Awareness", 2), ("MIND/Nature Knowledge", 2),
            ("MIND/Practical Knowledge", 3), ("MIND/Social Knowledge", 2), ("MIND/Willpower", 1),
        ],
        skills: &[
            ("BODY/Strength/Carrying", 2),
            ("MIND/Nature Knowledge/Plant Lore", 2),
            ("MIND/Practical Knowledge/Cooking", 4),
            ("MIND/Social Knowledge/Trading", 2),
        ],
        melee_weapons: &[("Knife", 2), ("Club", 1)],
        missile_weapons: &[("Throwing", 1)],
    },

    Chartype { name: "Prostitute", body: 1, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 2), ("BODY/Endurance", 2), ("BODY/Melee Combat", 1),
            ("BODY/Missile Combat", 1), ("BODY/Sleight", 2), ("BODY/Strength", 1),
            ("MIND/Awareness", 3), ("MIND/Nature Knowledge", 1),
            ("MIND/Practical Knowledge", 2), ("MIND/Social Knowledge", 4), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Athletics/Tumble", 2),
            ("BODY/Sleight/Pick Pockets", 2),
            ("MIND/Awareness/Reaction Speed", 2),
            ("MIND/Social Knowledge/Social Lore", 4),
        ],
        melee_weapons: &[("Dagger", 2), ("Club", 1)],
        missile_weapons: &[("Throwing", 1)],
    },

    Chartype { name: "Scribe", body: 1, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 1), ("BODY/Endurance", 1), ("BODY/Sleight", 4),
            ("BODY/Strength", 1),
            ("MIND/Awareness", 3), ("MIND/Nature Knowledge", 2),
            ("MIND/Practical Knowledge", 3), ("MIND/Social Knowledge", 3), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Sleight/Crafts", 3),
            ("MIND/Practical Knowledge/Mathematics", 2),
            ("MIND/Social Knowledge/Literacy", 5),
            ("MIND/Social Knowledge/Spoken Language", 3),
        ],
        melee_weapons: &[],
        missile_weapons: &[],
    },

    Chartype { name: "Seer", body: 1, mind: 2, spirit: 3,
        attributes: &[
            ("BODY/Athletics", 1), ("BODY/Endurance", 1), ("BODY/Sleight", 2),
            ("BODY/Strength", 1),
            ("MIND/Awareness", 4), ("MIND/Nature Knowledge", 3),
            ("MIND/Practical Knowledge", 2), ("MIND/Social Knowledge", 3), ("MIND/Willpower", 3),
            ("SPIRIT/Attunement", 4), ("SPIRIT/Casting", 2), ("SPIRIT/Worship", 2),
        ],
        skills: &[
            ("MIND/Awareness/Alertness", 3),
            ("MIND/Social Knowledge/Mythology", 4),
            ("MIND/Willpower/Mental Fortitude", 3),
            ("SPIRIT/Attunement/Self", 4),
            ("SPIRIT/Attunement/Mind", 3),
        ],
        melee_weapons: &[("Staff", 1)],
        missile_weapons: &[],
    },

    Chartype { name: "Sports contender", body: 3, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 4), ("BODY/Endurance", 4), ("BODY/Melee Combat", 2),
            ("BODY/Missile Combat", 2), ("BODY/Sleight", 2), ("BODY/Strength", 3),
            ("MIND/Awareness", 2), ("MIND/Nature Knowledge", 1),
            ("MIND/Practical Knowledge", 2), ("MIND/Social Knowledge", 2), ("MIND/Willpower", 3),
            ("SPIRIT/Attunement", 1),
        ],
        skills: &[
            ("BODY/Athletics/Tumble", 3), ("BODY/Athletics/Jump", 4),
            ("BODY/Endurance/Fortitude", 4), ("BODY/Endurance/Running", 4),
            ("BODY/Strength/Wield Weapon", 2),
            ("MIND/Social Knowledge/Performance", 2),
            ("MIND/Willpower/Mental Fortitude", 3),
        ],
        melee_weapons: &[("Unarmed", 3)],
        missile_weapons: &[("Throwing", 2)],
    },

    Chartype { name: "Summoner", body: 1, mind: 2, spirit: 3,
        attributes: &[
            ("BODY/Athletics", 1), ("BODY/Endurance", 1), ("BODY/Sleight", 2),
            ("BODY/Strength", 1),
            ("MIND/Awareness", 3), ("MIND/Nature Knowledge", 3),
            ("MIND/Practical Knowledge", 3), ("MIND/Social Knowledge", 2), ("MIND/Willpower", 4),
            ("SPIRIT/Attunement", 4), ("SPIRIT/Casting", 4),
        ],
        skills: &[
            ("MIND/Nature Knowledge/Magick Rituals", 4),
            ("MIND/Willpower/Mental Fortitude", 4),
            ("SPIRIT/Attunement/Death", 3),
            ("SPIRIT/Attunement/Mind", 3),
            ("SPIRIT/Casting/Range", 3),
        ],
        melee_weapons: &[("Staff", 1), ("Dagger", 1)],
        missile_weapons: &[],
    },

    Chartype { name: "Tailor", body: 1, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 1), ("BODY/Endurance", 1), ("BODY/Sleight", 4),
            ("BODY/Strength", 1),
            ("MIND/Awareness", 3), ("MIND/Nature Knowledge", 2),
            ("MIND/Practical Knowledge", 3), ("MIND/Social Knowledge", 3), ("MIND/Willpower", 2),
            ("SPIRIT/Attunement", 1),
        ],
        skills: &[
            ("BODY/Sleight/Crafts", 4),
            ("MIND/Practical Knowledge/Mathematics", 2),
            ("MIND/Social Knowledge/Trading", 3),
        ],
        melee_weapons: &[],
        missile_weapons: &[],
    },

    Chartype { name: "Tanner", body: 2, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 1), ("BODY/Endurance", 3), ("BODY/Melee Combat", 1),
            ("BODY/Sleight", 3), ("BODY/Strength", 2),
            ("MIND/Awareness", 2), ("MIND/Nature Knowledge", 3),
            ("MIND/Practical Knowledge", 3), ("MIND/Social Knowledge", 1), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Endurance/Fortitude", 3),
            ("BODY/Sleight/Crafts", 3),
            ("MIND/Nature Knowledge/Animal Handling", 3),
            ("MIND/Practical Knowledge/Crafts", 4),
        ],
        melee_weapons: &[("Club", 1), ("Knife", 2)],
        missile_weapons: &[],
    },

    // Race templates — same shape as the human chartypes, used by
    // the encounter generator when a non-human race is rolled.
    // Elves: MIND 3 baseline (wiki: "wise and spiritual / keen senses"),
    // SPIRIT 1 (innate magic). Wide skill spread reflects centuries
    // of lived experience — Amar-Tools class_town.rb scales elf age 3×.
    Chartype { name: "Elf: Warrior", body: 2, mind: 3, spirit: 1,
        attributes: &[
            ("BODY/Strength", 2), ("BODY/Endurance", 2), ("BODY/Athletics", 4),
            ("BODY/Melee Combat", 3), ("BODY/Missile Combat", 4),
            ("MIND/Intelligence", 3), ("MIND/Awareness", 4), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Missile Combat/Bow", 3),
            ("BODY/Melee Combat/Sword", 2),
            ("BODY/Athletics/Dodge", 2), ("BODY/Athletics/Balance", 2),
            ("MIND/Awareness/Alertness", 2),
        ],
        melee_weapons: &[("Sword", 2)],
        missile_weapons: &[("Bow", 4)],
    },
    Chartype { name: "Elf: Archer", body: 2, mind: 3, spirit: 1,
        attributes: &[
            ("BODY/Athletics", 4), ("BODY/Missile Combat", 5),
            ("MIND/Intelligence", 2), ("MIND/Awareness", 4),
        ],
        skills: &[
            ("BODY/Missile Combat/Bow", 4),
            ("BODY/Athletics/Hide", 2), ("BODY/Athletics/Move Quietly", 2),
            ("MIND/Awareness/Alertness", 3),
        ],
        melee_weapons: &[("Sword", 2)],
        missile_weapons: &[("Bow", 5)],
    },
    // New: Elf: Ranger — direct port of Amar-Tools' race_templates.rb.
    // Distinct from Archer: wilderness mastery (Tracking, Survival,
    // Move Quietly) over pure marksmanship.
    Chartype { name: "Elf: Ranger", body: 2, mind: 3, spirit: 1,
        attributes: &[
            ("BODY/Athletics", 4), ("BODY/Missile Combat", 4),
            ("MIND/Nature Knowledge", 3), ("MIND/Practical Knowledge", 3),
            ("MIND/Awareness", 4),
        ],
        skills: &[
            ("BODY/Missile Combat/Bow", 3),
            ("BODY/Athletics/Move Quietly", 3),
            ("BODY/Athletics/Hide", 2),
            ("MIND/Awareness/Tracking", 3),
            ("MIND/Practical Knowledge/Survival Lore", 2),
            ("MIND/Nature Knowledge/Plant Lore", 2),
        ],
        melee_weapons: &[("Sword", 2), ("Dagger", 2)],
        missile_weapons: &[("Bow", 4)],
    },
    Chartype { name: "Elf: Wizard", body: 1, mind: 3, spirit: 3,
        attributes: &[
            ("BODY/Athletics", 3),
            ("MIND/Intelligence", 4), ("MIND/Nature Knowledge", 4),
            ("SPIRIT/Casting", 3), ("SPIRIT/Attunement", 3),
        ],
        skills: &[
            ("MIND/Nature Knowledge/Magick Rituals", 2),
            ("SPIRIT/Attunement/Life", 2), ("SPIRIT/Attunement/Mind", 2),
        ],
        melee_weapons: &[("Staff", 1)],
        missile_weapons: &[("Bow", 2)],
    },
    Chartype { name: "Elf: Worker", body: 1, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 3),
            ("MIND/Awareness", 2), ("MIND/Practical Knowledge", 2),
        ],
        skills: &[
            ("BODY/Athletics/Move Quietly", 1),
            ("MIND/Awareness/Alertness", 1),
        ],
        melee_weapons: &[("Knife", 1)],
        missile_weapons: &[],
    },
    // Dwarves: wiki "Dwarf +1 BODY characteristic". Bake the bonus
    // into BODY directly (so a Dwarf Warrior is BODY 3 vs Human's 2).
    Chartype { name: "Dwarf: Warrior", body: 3, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Strength", 4), ("BODY/Endurance", 5),
            ("BODY/Melee Combat", 4), ("BODY/Missile Combat", 2),
            ("MIND/Practical Knowledge", 3), ("MIND/Willpower", 4),
        ],
        skills: &[
            ("BODY/Melee Combat/Axe", 3), ("BODY/Melee Combat/Shield", 2),
            ("BODY/Endurance/Fortitude", 2),
        ],
        melee_weapons: &[("Axe", 3), ("Shield", 2), ("Hammer", 2)],
        missile_weapons: &[("Throwing", 1)],
    },
    Chartype { name: "Dwarf: Smith", body: 3, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Strength", 5), ("BODY/Endurance", 4), ("BODY/Sleight", 3),
            ("MIND/Intelligence", 2), ("MIND/Practical Knowledge", 4),
            ("MIND/Nature Knowledge", 3),
        ],
        skills: &[
            ("BODY/Melee Combat/Hammer", 2),
            ("BODY/Strength/Weight Lifting", 2),
            ("BODY/Sleight/Crafts", 3),
            ("MIND/Practical Knowledge/Smithing", 4),
            ("MIND/Nature Knowledge/Metals", 3),
        ],
        melee_weapons: &[("Hammer", 3), ("Axe", 1)],
        missile_weapons: &[],
    },
    Chartype { name: "Dwarf: Guard", body: 3, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Strength", 4), ("BODY/Endurance", 5), ("BODY/Melee Combat", 3),
            ("MIND/Awareness", 3), ("MIND/Willpower", 4),
        ],
        skills: &[
            ("BODY/Melee Combat/Spear", 2), ("BODY/Melee Combat/Shield", 3),
            ("MIND/Awareness/Alertness", 3),
        ],
        melee_weapons: &[("Spear", 3), ("Shield", 3), ("Axe", 1)],
        missile_weapons: &[],
    },
    Chartype { name: "Dwarf: Worker", body: 3, mind: 1, spirit: 0,
        attributes: &[("BODY/Strength", 3), ("BODY/Endurance", 4)],
        skills: &[],
        melee_weapons: &[("Hammer", 1)],
        missile_weapons: &[],
    },
    // Araxi (Amar-Tools "Araxi: Warrior" had BODY 4 — savage / strong).
    Chartype { name: "Araxi: Warrior", body: 4, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Strength", 4), ("BODY/Endurance", 3), ("BODY/Athletics", 3),
            ("BODY/Melee Combat", 4),
            ("MIND/Awareness", 3), ("MIND/Willpower", 2),
        ],
        skills: &[
            ("BODY/Melee Combat/Unarmed", 3),
            ("BODY/Athletics/Hide", 2), ("MIND/Awareness/Tracking", 2),
        ],
        melee_weapons: &[("Club", 3), ("Axe", 2)],
        missile_weapons: &[],
    },
    Chartype { name: "Araxi: Hunter", body: 4, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Endurance", 4), ("BODY/Athletics", 4),
            ("BODY/Melee Combat", 3), ("BODY/Missile Combat", 3),
            ("MIND/Awareness", 4),
        ],
        skills: &[
            ("BODY/Athletics/Hide", 3), ("MIND/Awareness/Tracking", 3),
        ],
        melee_weapons: &[("Spear", 3)],
        missile_weapons: &[("Bow", 3)],
    },
    Chartype { name: "Araxi: Worker", body: 3, mind: 1, spirit: 0,
        attributes: &[("BODY/Strength", 3), ("BODY/Endurance", 3)],
        skills: &[],
        melee_weapons: &[("Club", 1)],
        missile_weapons: &[],
    },
    // ---- Race templates added to match Amar-Tools demographics. ----
    // Trolls: BODY 5 (very large). The Monster: Troll encounters use
    // the EncStats path — these templates are for humanoid Troll
    // encounters that need a chartype.
    Chartype { name: "Troll: Warrior", body: 5, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Strength", 6), ("BODY/Endurance", 5), ("BODY/Melee Combat", 3),
            ("MIND/Willpower", 1),
        ],
        skills: &[
            ("BODY/Melee Combat/Club", 3), ("BODY/Strength/Carrying", 3),
        ],
        melee_weapons: &[("Club", 3)],
        missile_weapons: &[("Throwing", 1)],
    },
    // Ogres: large, strong, simple.
    Chartype { name: "Ogre: Warrior", body: 4, mind: 1, spirit: 0,
        attributes: &[
            ("BODY/Strength", 5), ("BODY/Endurance", 4), ("BODY/Melee Combat", 3),
            ("MIND/Awareness", 2),
        ],
        skills: &[
            ("BODY/Melee Combat/Club", 2),
            ("BODY/Melee Combat/Unarmed", 2),
        ],
        melee_weapons: &[("Club", 3)],
        missile_weapons: &[],
    },
    // Lizardmen: amphibious, scaly, decent fighters.
    Chartype { name: "Lizard Man: Warrior", body: 3, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Strength", 3), ("BODY/Endurance", 3), ("BODY/Athletics", 3),
            ("BODY/Melee Combat", 3),
            ("MIND/Awareness", 3),
        ],
        skills: &[
            ("BODY/Melee Combat/Spear", 2),
            ("BODY/Athletics/Swim", 3), ("BODY/Athletics/Hide", 2),
        ],
        melee_weapons: &[("Spear", 3)],
        missile_weapons: &[],
    },
    // Goblins: small, sneaky, weak. Wiki: not race-bonus'd.
    Chartype { name: "Goblin: Warrior", body: 2, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Strength", 2), ("BODY/Endurance", 2), ("BODY/Athletics", 3),
            ("BODY/Melee Combat", 2), ("BODY/Missile Combat", 2),
            ("MIND/Awareness", 3),
        ],
        skills: &[
            ("BODY/Melee Combat/Sword", 1),
            ("BODY/Athletics/Hide", 2), ("BODY/Athletics/Move Quietly", 2),
        ],
        melee_weapons: &[("Short Sword", 2), ("Knife", 2)],
        missile_weapons: &[("Bow", 2)],
    },
    Chartype { name: "Goblin: Thief", body: 2, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Athletics", 4), ("BODY/Sleight", 3),
            ("MIND/Awareness", 3), ("MIND/Practical Knowledge", 2),
        ],
        skills: &[
            ("BODY/Athletics/Hide", 3), ("BODY/Athletics/Move Quietly", 3),
            ("BODY/Sleight/Pick Pockets", 2),
            ("MIND/Awareness/Alertness", 2),
        ],
        melee_weapons: &[("Knife", 3)],
        missile_weapons: &[("Throwing", 2)],
    },
    // Centaurs: half-horse → strong, fast, decent ranged.
    Chartype { name: "Centaur: Warrior", body: 4, mind: 2, spirit: 0,
        attributes: &[
            ("BODY/Strength", 4), ("BODY/Endurance", 4), ("BODY/Athletics", 3),
            ("BODY/Melee Combat", 3), ("BODY/Missile Combat", 3),
            ("MIND/Nature Knowledge", 2),
        ],
        skills: &[
            ("BODY/Melee Combat/Spear", 2),
            ("BODY/Missile Combat/Bow", 2),
            ("BODY/Endurance/Running", 3),
        ],
        melee_weapons: &[("Spear", 2)],
        missile_weapons: &[("Bow", 3)],
    },
    Chartype { name: "Centaur: Ranger", body: 3, mind: 2, spirit: 1,
        attributes: &[
            ("BODY/Athletics", 4), ("BODY/Missile Combat", 4),
            ("MIND/Nature Knowledge", 3), ("MIND/Awareness", 3),
        ],
        skills: &[
            ("BODY/Missile Combat/Bow", 3),
            ("BODY/Endurance/Running", 3),
            ("MIND/Awareness/Tracking", 2),
            ("MIND/Nature Knowledge/Plant Lore", 2),
        ],
        melee_weapons: &[("Spear", 1)],
        missile_weapons: &[("Bow", 4)],
    },
    // Faeries: tiny, magical, can fly. Athletics is sky-high to cover
    // flight via the canonical attribute (SPIRIT/Innate was dropped
    // from the universal list per the user's earlier preference —
    // it's a rare per-race ability, lives in a free-form slot when
    // a player wants it). Plant Lore + nature affinity per the wiki
    // ("from a realm purer than the mortal world / love art / mimic
    // human creativity / kinship with Anashina").
    Chartype { name: "Faerie: Mage", body: 1, mind: 3, spirit: 4,
        attributes: &[
            ("BODY/Athletics", 5),
            ("MIND/Intelligence", 3), ("MIND/Nature Knowledge", 4),
            ("SPIRIT/Casting", 4), ("SPIRIT/Attunement", 4),
        ],
        skills: &[
            ("BODY/Athletics/Dodge", 3), ("BODY/Athletics/Balance", 3),
            ("MIND/Nature Knowledge/Plant Lore", 3),
            ("SPIRIT/Attunement/Life", 3),
        ],
        melee_weapons: &[("Knife", 1)],
        missile_weapons: &[],
    },
];

pub fn chartype(name: &str) -> Option<&'static Chartype> {
    CHARTYPES.iter().find(|c| c.name == name)
}

// ---------------------------------------------------------------- Religions

/// A god of Amar. Mirrors Amar-Tools' `$ReligionTable` (which itself
/// follows d6gaming.org/Mythology). Domain is the elemental /
/// thematic sphere the god rules; portfolio is the prose summary GMs
/// reach for in play.
#[derive(Debug, Clone, Copy)]
pub struct God {
    pub name: &'static str,
    pub domain: &'static str,
    pub portfolio: &'static str,
    pub alignment: &'static str,
}

/// 21 gods from the Amar-Tools religions port — the primary five
/// (Alesia/Ikalio/Shalissa/Walmaer/Ielina) plus the 16 lesser gods
/// every character type can worship.
pub const GODS: &[God] = &[
    God { name: "Alesia",       domain: "Earth",        portfolio: "Stability, Protection, Agriculture",   alignment: "Good" },
    God { name: "Ikalio",       domain: "Fire",         portfolio: "Creativity, Passion, Thought",         alignment: "Neutral" },
    God { name: "Shalissa",     domain: "Wind/Air",     portfolio: "Freedom, Speed, Adventure",            alignment: "Neutral" },
    God { name: "Walmaer",      domain: "Water",        portfolio: "Sea, Rivers, Maritime",                alignment: "Neutral" },
    God { name: "Ielina",       domain: "Moon/Time",    portfolio: "Perception, Wisdom, Time",             alignment: "Neutral" },
    God { name: "Cal Amae",     domain: "Good",         portfolio: "Good Deeds, Heroism, Protection",      alignment: "Good" },
    God { name: "Taroc",        domain: "War",          portfolio: "Battle, Conflict, Strategy",           alignment: "Neutral" },
    God { name: "Fal Munir",    domain: "Knowledge",    portfolio: "Learning, Wisdom, Research",           alignment: "Neutral" },
    God { name: "Elesi",        domain: "Creation",     portfolio: "Art, Craftsmanship, Beauty",           alignment: "Good" },
    God { name: "Anashina",     domain: "Nature",       portfolio: "Wilderness, Animals, Plants",          alignment: "Neutral" },
    God { name: "Gwendyll",     domain: "Royalty",      portfolio: "Leadership, Nobility, Authority",      alignment: "Neutral" },
    God { name: "MacGillan",    domain: "Royalty (M)",  portfolio: "Kingship, command, war-banner",        alignment: "Neutral" },
    God { name: "Mailatroz",    domain: "Trade",        portfolio: "Commerce, Wealth, Business",           alignment: "Neutral" },
    God { name: "Juba",         domain: "Entertainment",portfolio: "Joy, Performance, Festivity",          alignment: "Good" },
    God { name: "Kraagh",       domain: "Death",        portfolio: "Death, Reincarnation, Afterlife",      alignment: "Neutral" },
    God { name: "Mestronorpha", domain: "Evil",         portfolio: "Darkness, Corruption, Malice",         alignment: "Evil" },
    God { name: "Tsankili",     domain: "Thievery",     portfolio: "Trickery, Stealth, Cunning",           alignment: "Neutral" },
    God { name: "Man Peggon",   domain: "Strength",     portfolio: "Physical Power, Athletics",            alignment: "Neutral" },
    God { name: "Maleko",       domain: "Inner Strength",portfolio: "Meditation, Self-Control, Discipline",alignment: "Good" },
    God { name: "Recolar",      domain: "Sports",       portfolio: "Competition, Athletics, Games",        alignment: "Neutral" },
    God { name: "Liandra",      domain: "Hope",         portfolio: "Dreams, Optimism, Inspiration",        alignment: "Good" },
    God { name: "Moltan",       domain: "Underworld",   portfolio: "Hidden depths, secrets, gloom",        alignment: "Evil" },
];

/// Per-chartype deity weights, modelled after Amar-Tools'
/// `$CharacterReligions`. Each entry is the (god_name, weight) list
/// used to pick a deity for an NPC of that type. The "nobility"
/// pseudo-name picks MacGillan for male NPCs, Gwendyll for female.
/// "any" falls back to `RANDOM_DEITY_WEIGHTS` below.
///
/// Multiple gods listed → reasonable diversity. The first god is the
/// most common (e.g. fire wizards lean Ikalio but a few worship Taroc
/// or Elesi instead).
pub const CHARTYPE_RELIGIONS: &[(&str, &[&str])] = &[
    // Wizards by element
    ("Wizard (water)", &["Walmaer", "Walmaer", "Walmaer", "Ielina", "Alesia"]),
    ("Wizard (fire)",  &["Ikalio",  "Ikalio",  "Ikalio",  "Taroc",  "Elesi"]),
    ("Wizard (air)",   &["Shalissa","Shalissa","Shalissa","Ielina", "Anashina"]),
    ("Wizard (earth)", &["Alesia",  "Alesia",  "Alesia",  "Anashina","Ikalio"]),
    ("Wizard (prot.)", &["Alesia",  "Cal Amae","Gwendyll","MacGillan"]),
    // Magic users
    ("Mage",           &["Ikalio", "Elesi", "Ielina", "Taroc", "Shalissa", "Alesia"]),
    ("Witch (white)",  &["Cal Amae", "Liandra", "Elesi", "Gwendyll", "Alesia"]),
    ("Witch (black)",  &["Mestronorpha", "Kraagh", "Moltan"]),
    ("Sorcerer",       &["Mestronorpha", "Kraagh", "Moltan", "Tsankili"]),
    ("Summoner",       &["Kraagh", "Ielina", "Mestronorpha"]),
    ("Seer",           &["Ielina", "Ielina", "Fal Munir"]),
    ("Priest",         &["Cal Amae", "Alesia", "Shalissa", "Ielina", "Elesi", "Liandra", "Walmaer", "Ikalio"]),
    ("Monk",           &["Maleko", "Ielina", "Fal Munir", "Cal Amae"]),
    // Warriors
    ("Warrior",        &["Taroc", "Taroc", "Taroc", "Recolar", "Man Peggon", "Cal Amae"]),
    ("Soldier",        &["Taroc", "Taroc", "Cal Amae", "nobility"]),
    ("Guard",          &["Taroc", "Alesia", "Cal Amae", "nobility"]),
    ("Body guard",     &["Taroc", "Man Peggon", "nobility", "Cal Amae"]),
    ("Gladiator",      &["Taroc", "Recolar", "Man Peggon", "Juba"]),
    ("Berserker",      &["Man Peggon", "Taroc", "Anashina"]),
    ("Barbarian",      &["Man Peggon", "Anashina", "Taroc", "Kraagh"]),
    // Nature
    ("Ranger",         &["Anashina", "Anashina", "Alesia", "Shalissa"]),
    ("Hunter",         &["Anashina", "Anashina", "Taroc"]),
    ("Tracker",        &["Anashina", "Shalissa", "Taroc"]),
    // Rogues
    ("Thief",          &["Tsankili", "Tsankili", "None", "Juba"]),
    ("Assassin",       &["Tsankili", "Mestronorpha", "Kraagh", "None"]),
    ("Highwayman",     &["Tsankili", "None", "Taroc"]),
    ("Scout",          &["Shalissa", "Anashina", "Tsankili"]),
    // Scholars
    ("Scholar",        &["Fal Munir", "Fal Munir", "Elesi", "Ielina"]),
    ("Sage",           &["Fal Munir", "Ielina", "Elesi", "any"]),
    // Social
    ("Noble",          &["nobility", "nobility", "Taroc", "Alesia", "Ikalio", "Shalissa", "Walmaer"]),
    ("Merchant",       &["Mailatroz", "Mailatroz", "Juba", "Alesia", "Walmaer"]),
    ("Bard",           &["Juba", "Juba", "Shalissa"]),
    // Default
    ("Commoner",       &["Alesia", "Ikalio", "Shalissa", "Walmaer", "Cal Amae", "Mailatroz", "None", "any"]),
    ("Farmer",         &["Alesia", "Alesia", "Cal Amae"]),
    ("Sailor",         &["Walmaer", "Walmaer", "Shalissa"]),
    ("Smith",          &["Elesi", "Mailatroz", "Alesia"]),
];

/// Weighted pool for the "any" deity placeholder. Amar-Tools used
/// "None" weight 6 (~9% atheism); Amar lore treats godlessness as
/// rare, so we drop it to 1 (~1.6% of the "any" rolls land on
/// "None"). The NPC builder then applies an additional ~5% filter,
/// so combined atheism rate across all NPCs sits near 5%.
pub const RANDOM_DEITY_WEIGHTS: &[(&str, u32)] = &[
    ("Alesia", 4), ("Anashina", 3), ("Cal Amae", 2), ("Elesi", 1),
    ("Fal Munir", 2), ("Gwendyll", 2), ("Ielina", 3), ("Ikalio", 3),
    ("Juba", 3), ("Kraagh", 2), ("Liandra", 1), ("MacGillan", 2),
    ("Mailatroz", 2), ("Maleko", 2), ("Man Peggon", 1),
    ("Mestronorpha", 1), ("Moltan", 3), ("Recolar", 3), ("Shalissa", 3),
    ("Taroc", 5), ("Tsankili", 2), ("Walmaer", 5), ("None", 1),
];

pub fn god(name: &str) -> Option<&'static God> {
    GODS.iter().find(|g| g.name == name)
}
