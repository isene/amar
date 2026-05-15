#!/usr/bin/env python3
"""Populate the 10 NPCs in ThePortal/TheShatteredHourglass with the
stat blocks from the adventure markdown.

Run:  python3 scripts/populate_shattered_hourglass.py

Idempotent: re-running overwrites the fields it sets. Does NOT touch
fields the script doesn't know about (portrait_path, etc.), so the
auto-imported portraits stay attached.

The match key is the NPC's *last-name segment* (Vel, Vael, Coleth,
Macch, Grenn, Vross, Halroyd, Porenson, Dovennan, Rat) so the
JSON-side titles in parens (e.g. "(The Loyal Apprentice)") don't
prevent the match, and so "Mirenna Coleth" matches the md's "Miren
Coleth" stat block.
"""

import json
from pathlib import Path

CAMP_PATH = Path.home() / ".amar/campaigns/ThePortal/campaign.json"

# Canonical attribute parents (matches src/pc.rs::ATTRIBUTES).
ATTR_PARENT = {
    "Strength":             "BODY",
    "Endurance":            "BODY",
    "Athletics":            "BODY",
    "Melee Combat":         "BODY",
    "Missile Combat":       "BODY",
    "Sleight":              "BODY",
    "Nature Knowledge":     "MIND",
    "Social Knowledge":     "MIND",
    "Practical Knowledge":  "MIND",
    "Awareness":            "MIND",
    "Willpower":            "MIND",
    "Casting":              "SPIRIT",
    "Attunement":           "SPIRIT",
    "Worship":              "SPIRIT",
}

# Canonical skill → attribute mapping (subset used here). Anything
# *not* in this map goes into open_skills under the explicit parent.
SKILL_PARENT = {
    # BODY
    "Wield Weapon":         "Strength",
    "Carrying":             "Strength",
    "Fortitude":            "Endurance",
    "Combat Tenacity":      "Endurance",
    "Running":              "Endurance",
    "Poison Resistance":    "Endurance",
    "Hide":                 "Athletics",
    "Move Quietly":         "Athletics",
    "Climb":                "Athletics",
    "Swim":                 "Athletics",
    "Jump":                 "Athletics",
    "Dodge":                "Athletics",
    "Pick Pockets":         "Sleight",
    "Disarm Traps":         "Sleight",
    # MIND
    "Medical Lore":         "Nature Knowledge",
    "Magick Rituals":       "Nature Knowledge",
    "Alchemy":              "Nature Knowledge",
    "Social Lore":          "Social Knowledge",
    "Spoken Language":      "Social Knowledge",
    "Literacy":             "Social Knowledge",
    "Mythology":            "Social Knowledge",
    "Legend Lore":          "Social Knowledge",
    "Survival Lore":        "Practical Knowledge",
    "Set Traps":            "Practical Knowledge",
    "Ambush":               "Practical Knowledge",
    "Reaction Speed":       "Awareness",
    "Alertness":            "Awareness",
    "Tracking":             "Awareness",
    "Detect Traps":         "Awareness",
    "Pain Tolerance":       "Willpower",
    "Mental Fortitude":     "Willpower",
    # SPIRIT (Casting sub-skills)
    "Range":                "Casting",
    "Duration":             "Casting",
    "Area of Effect":       "Casting",
    "Weight":               "Casting",
    "Number of Targets":    "Casting",
}

# Per-NPC stat data, transcribed from
# /home/geir/Main/G/AMAR/ThePortal/Amaron/TheShatteredHourglass/
# TheShatteredHourglass.md
#
# Each "skills" entry is the *displayed total* exactly as printed in
# the md. The loader converts each total to the bare skill rank by
# subtracting characteristic + attribute (so the rendered total
# matches the md when amar adds char+attr+rank back). Skills not in
# SKILL_PARENT land in open_skills with an explicit parent attr.

NPCS = {
    # ---- KEY NPCs --------------------------------------------------
    "Vel": {  # Priestess Marienna Vel
        "gender": "F", "age": 34, "level": 3, "race": "Human", "size": 3.0,
        "ch": {"BODY": 1, "MIND": 2, "SPIRIT": 2},
        "attrs": {"Strength": 1, "Endurance": 2, "Awareness": 5, "Willpower": 3, "Worship": 5},
        "skills_total": {
            "Mythology": 7, "Literacy": 6, "Mental Fortitude": 9,
            "Reaction Speed": 8,
        },
        "open_skills": [
            ("MIND", "Awareness",        "Sense Emotions", 9),
            ("MIND", "Awareness",        "Sense Magick",   9),
        ],
        "weapons": [
            {"name": "Staff", "kind": "Melee", "skill_name": "Staff",
             "init": 6, "off_mod": 0, "def_mod": 2, "damage": -1, "hp": 7,
             "two_handed": True},
        ],
        "armor_name": "Robes", "armor_ap": 0,
        "bp": 7, "mf": 9,
        "money_sp": 15,
        "equipment": ["Ceremonial robes", "moon pendant (silver, 25sp)", "prayer beads"],
        "description": (
            "Calm and perceptive; speaks in measured tones; often pauses as if listening "
            "to something others cannot hear. Has kind eyes but carries worry about the "
            "failing Hourglass.\n\n"
            "Motivation: Protect the sacred bond between Ielina and her followers; "
            "restore the Hourglass of Ages before the disruption spreads.\n\n"
            "Connection: Party's contact; can provide healing (once per day, heals 1d6 BP) "
            "and guidance between delves into the catacombs."
        ),
    },
    "Vael": {  # Corusath Vael — the antagonist
        "gender": "M", "age": 47, "level": 4, "race": "Human", "size": 3.0,
        "ch": {"BODY": 0, "MIND": 2, "SPIRIT": 2},
        "attrs": {"Strength": 1, "Endurance": 1, "Awareness": 2, "Casting": 3, "Attunement": 2},
        "skills_total": {
            "Magick Rituals": 8, "Literacy": 7, "Duration": 9,
        },
        "open_skills": [
            ("MIND",   "Practical Knowledge", "Problem Solving", 8),
            ("MIND",   "Practical Knowledge", "Intelligence",    4),  # md "Int:4"
            ("SPIRIT", "Attunement",          "Self",            8),
        ],
        "weapons": [
            {"name": "Dagger", "kind": "Melee", "skill_name": "Knife",
             "init": 2, "off_mod": -2, "def_mod": -3, "damage": -1, "hp": 8,
             "two_handed": False},
        ],
        "armor_name": "None", "armor_ap": 0,
        "bp": 6, "mf": 5,
        "money_sp": 2,
        "equipment": ["Tattered academy robes", "notebook filled with equations",
                      "cracked crystal focus"],
        "spells": [
            {"name": "Time Slip",      "domain": "Time", "effects": "Delays single action by 2s; disrupts initiative"},
            {"name": "Temporal Echo",  "domain": "Time", "effects": "Creates brief image of past event in 3m radius"},
            {"name": "Slow",           "domain": "Time", "effects": "Target moves at half speed for 1 min; opposed MIND roll"},
        ],
        "description": (
            "Obsessive and brilliant; mutters calculations constantly; has dark circles "
            "under eyes from lack of sleep; genuinely believes he's close to a breakthrough "
            "that will vindicate his expulsion. Fidgets with his crystal focus when nervous.\n\n"
            "Motivation: Prove the Academy wrong; master time magic; restore his reputation.\n\n"
            "Connection: Final encounter; will try to bargain rather than fight. If cornered, "
            "may threaten to collapse the rift (which would destroy several city blocks)."
        ),
    },
    "Coleth": {  # Mirenna / Miren Coleth — Corusath's apprentice
        "gender": "F", "age": 22, "level": 2, "race": "Human", "size": 3.0,
        "ch": {"BODY": 1, "MIND": 1, "SPIRIT": 0},
        "attrs": {"Strength": 1, "Endurance": 1, "Awareness": 2},
        "skills_total": {
            "Move Quietly": 6, "Climb": 6, "Pick Pockets": 6, "Literacy": 5,
        },
        "open_skills": [
            ("MIND", "Practical Knowledge", "Innovation",   5),
            ("MIND", "Practical Knowledge", "Intelligence", 2),  # md "Int:2"
        ],
        "weapons": [
            {"name": "Knife", "kind": "Melee", "skill_name": "Knife",
             "init": 2, "off_mod": -2, "def_mod": -3, "damage": -2, "hp": 8,
             "two_handed": False},
        ],
        "armor_name": "Leather", "armor_ap": 1,
        "bp": 6, "mf": 1,
        "money_sp": 12,
        "equipment": ["Lockpicks", "rope (15m)", "stolen academy supplies",
                      "smoke bomb (obscures 3m radius)", "stolen academy keys"],
        "description": (
            "Loyal to Corusath (her mentor and father figure); conflicted about the danger; "
            "young and in over her head. Bites her nails when nervous. Wants desperately "
            "to prove herself worthy of his teaching.\n\n"
            "Motivation: Help her mentor succeed; prove herself; avoid going back to "
            "poverty on the streets.\n\n"
            "Connection: Can be encountered stealing supplies at academy or entering the "
            "catacombs through the junction. Might betray Corusath if shown the temporal "
            "sickness affecting the innocent, or promised protection."
        ),
    },
    "Dovennan": {  # Brother Alric Dovennan
        "gender": "M", "age": 58, "level": 2, "race": "Human", "size": 3.0,
        "ch": {"BODY": 1, "MIND": 1, "SPIRIT": 1},
        "attrs": {"Strength": 2, "Endurance": 2, "Awareness": 2, "Worship": 1},
        "skills_total": {
            "Legend Lore": 5, "Social Lore": 5, "Literacy": 4,
        },
        "open_skills": [
            ("MIND", "Awareness", "Sense of Direction", 6),
            ("MIND", "Practical Knowledge", "Intelligence", 1),
        ],
        "weapons": [
            {"name": "Walking stick", "kind": "Melee", "skill_name": "Staff",
             "init": 6, "off_mod": 0, "def_mod": 2, "damage": -1, "hp": 7,
             "two_handed": False},
        ],
        "armor_name": "Simple robes", "armor_ap": 0,
        "bp": 7, "mf": 1,
        "money_sp": 3,
        "equipment": ["Lantern", "keys to temple areas",
                      "old maps (partially inaccurate)", "wine flask"],
        "description": (
            "Kindly but forgetful; has maintained the temple for 30 years; knows secret "
            "passages but can't always remember which is which. Smells of incense and wine. "
            "Treats the young people (PCs) like grandchildren.\n\n"
            "Motivation: Protect the temple; help the young people stay safe; preserve "
            "the history he's guarded for decades.\n\n"
            "Connection: Provides information about the catacombs' layout and history; "
            "knows about the sealed entrance beneath the temple."
        ),
    },
    "Macch": {  # Guard Captain Vorian Macch
        "gender": "M", "age": 41, "level": 3, "race": "Human", "size": 3.0,
        "ch": {"BODY": 2, "MIND": 1, "SPIRIT": 0},
        "attrs": {"Strength": 3, "Endurance": 2, "Awareness": 2, "Willpower": 2,
                  "Melee Combat": 0},
        "skills_total": {
            "Sword": 10, "Shield": 7, "Alertness": 6, "Social Lore": 5,
        },
        "open_skills": [
            ("MIND", "Awareness", "Sense Ambush", 6),
        ],
        "weapons": [
            {"name": "Longsword", "kind": "Melee", "skill_name": "Sword",
             "init": 5, "off_mod": 0, "def_mod": 0, "damage": -1, "hp": 12,
             "two_handed": False},
            {"name": "Shield", "kind": "Melee", "skill_name": "Shield",
             "init": 0, "off_mod": 0, "def_mod": 2, "damage": 0, "hp": 10,
             "two_handed": False},
        ],
        "armor_name": "Chainmail", "armor_ap": 3,
        "bp": 9, "mf": 5,
        "money_sp": 25,
        "equipment": ["Guard whistle", "manacles", "rope", "city watch badge"],
        "description": (
            "Dutiful and suspicious; doesn't like \"adventurer types\" mucking about in "
            "temple business. Loyal to Commander Seillan Torthal. Short-tempered but fair.\n\n"
            "Motivation: Maintain order in Amaron; prevent panic about magic under the city.\n\n"
            "Connection: May investigate if PCs are too public about their activities. "
            "Can be an obstacle or reluctant ally depending on PC approach."
        ),
    },
    "Grenn": {  # Thelia Grenn — informant
        "gender": "F", "age": 28, "level": 2, "race": "Human", "size": 3.0,
        "ch": {"BODY": 1, "MIND": 2, "SPIRIT": 0},
        "attrs": {"Strength": 1, "Endurance": 1, "Awareness": 3},
        "skills_total": {
            "Social Lore": 7, "Move Quietly": 6,
        },
        "open_skills": [
            ("MIND", "Awareness", "Listening",      8),
            ("MIND", "Awareness", "Sense Emotions", 7),
            ("MIND", "Practical Knowledge", "Intelligence", 2),
        ],
        "weapons": [
            {"name": "Concealed knife", "kind": "Melee", "skill_name": "Knife",
             "init": 2, "off_mod": -2, "def_mod": -3, "damage": -2, "hp": 8,
             "two_handed": False},
        ],
        "armor_name": "None", "armor_ap": 0,
        "bp": 6, "mf": 1,
        "money_sp": 15,
        "equipment": ["Various disguises", "notebook (coded)", "messenger bag"],
        "description": (
            "Sharp-eyed and quick-witted; sells information to highest bidder; neutral "
            "alignment but can be bought. Has contacts throughout Amaron's underworld.\n\n"
            "Motivation: Make money; stay alive; know everything happening in the city.\n\n"
            "Connection: Might approach PCs if they're asking questions around the temple "
            "district. Knows about Miren's supply runs (costs 25sp) and rumors about "
            "\"strange sounds\" from below the old quarter (costs 5sp)."
        ),
    },
    "Rat": {  # Torik The Rat Catcher
        "gender": "M", "age": 54, "level": 1, "race": "Human", "size": 3.0,
        "ch": {"BODY": 1, "MIND": 1, "SPIRIT": 0},
        "attrs": {"Strength": 2, "Endurance": 2, "Awareness": 2},
        "skills_total": {
            "Survival Lore": 5, "Set Traps": 5,
        },
        "open_skills": [
            ("MIND", "Nature Knowledge", "Animal Handling",   4),
            ("MIND", "Awareness",        "Sense of Direction", 6),
            ("MIND", "Practical Knowledge", "Intelligence",    1),
        ],
        "weapons": [
            {"name": "Spear", "kind": "Melee", "skill_name": "Spear",
             "init": 6, "off_mod": 0, "def_mod": 1, "damage": -1, "hp": 7,
             "two_handed": True},
        ],
        "armor_name": "Leather scraps", "armor_ap": 1,
        "bp": 7, "mf": 0,
        "money_sp": 8,
        "equipment": ["Rat traps", "poison bait", "lantern",
                      "sewer maps (accurate)", "rope"],
        "description": (
            "Gruff and practical; has worked Amaron's sewers for 30 years; knows the "
            "junction where Miren passes. Missing two fingers on left hand from rat bite.\n\n"
            "Motivation: Do his job; avoid trouble; supplement income however possible.\n\n"
            "Connection: Can be found near the junction if PCs explore sewers. Will "
            "provide information about \"the young woman who sneaks through\" for 3sp "
            "or a favor. Can guide PCs through sewers if paid well (10sp)."
        ),
    },
    "Vross": {  # Scholar Nalien Vross
        "gender": "M", "age": 35, "level": 2, "race": "Human", "size": 3.0,
        "ch": {"BODY": 0, "MIND": 2, "SPIRIT": 1},
        "attrs": {"Strength": 1, "Endurance": 1, "Awareness": 2, "Worship": 1},
        "skills_total": {
            "Literacy": 8, "Legend Lore": 8, "Mythology": 7,
        },
        "open_skills": [
            ("MIND", "Practical Knowledge", "Problem Solving", 7),
            ("MIND", "Practical Knowledge", "Intelligence",    3),
        ],
        "weapons": [],
        "armor_name": "None", "armor_ap": 0,
        "bp": 6, "mf": 1,
        "money_sp": 30,
        "equipment": ["Books (4, worth 80sp)", "scrolls", "research notes",
                      "magnifying glass"],
        "description": (
            "Enthusiastic about history; particularly interested in pre-kingdom Amaron; "
            "would LOVE to explore the catacombs if it weren't so dangerous. Speaks "
            "quickly when excited.\n\n"
            "Motivation: Discover historical truths; publish findings; earn recognition.\n\n"
            "Connection: Can be found at Amaron library researching catacomb history. "
            "Provides valuable background: the lower catacombs pre-date the kingdom by at "
            "least 500 years and may have been built by an unknown civilization that "
            "worshipped time/cycles."
        ),
    },
    "Halroyd": {  # Enna Halroyd
        "gender": "F", "age": 19, "level": 1, "race": "Human", "size": 3.0,
        "ch": {"BODY": 1, "MIND": 1, "SPIRIT": 1},
        "attrs": {"Strength": 1, "Endurance": 1, "Awareness": 2, "Willpower": 2, "Worship": 2},
        "skills_total": {
            "Mythology": 5, "Literacy": 4,
        },
        "open_skills": [
            ("MIND", "Awareness", "Sense Magick", 6),
            ("MIND", "Willpower", "Meditation",   5),
        ],
        "weapons": [],
        "armor_name": "Robes", "armor_ap": 0,
        "bp": 6, "mf": 3,
        "money_sp": 3,
        "equipment": ["Prayer beads", "moon water (holy water equivalent)",
                      "meditation cushion"],
        "description": (
            "Young and earnest; recently became an initiate; terrified by the Hourglass "
            "failing. Looks up to Priestess Marienna. Stammers when nervous.\n\n"
            "Motivation: Serve Ielina faithfully; help restore the Hourglass; prove "
            "herself to the High Priestess.\n\n"
            "Connection: Can assist PCs with minor blessings or information about Ielina's "
            "teachings. If PCs are kind to her, she'll secretly follow them once (out of "
            "curiosity and concern), potentially providing aid at a crucial moment."
        ),
    },
    "Porenson": {  # Instructor Vael Porenson — the rival
        "gender": "M", "age": 52, "level": 5, "race": "Human", "size": 3.0,
        "ch": {"BODY": 1, "MIND": 3, "SPIRIT": 2},
        "attrs": {"Strength": 1, "Endurance": 1, "Awareness": 2, "Casting": 4, "Attunement": 3},
        "skills_total": {
            "Magick Rituals": 10, "Literacy": 10,
        },
        "open_skills": [
            ("MIND",   "Practical Knowledge", "Problem Solving", 11),
            ("MIND",   "Practical Knowledge", "Intelligence",    5),
            ("SPIRIT", "Attunement",          "Fire",            10),
        ],
        "weapons": [
            {"name": "Staff", "kind": "Melee", "skill_name": "Staff",
             "init": 6, "off_mod": 0, "def_mod": 2, "damage": -1, "hp": 7,
             "two_handed": True},
        ],
        "armor_name": "Robes", "armor_ap": 0,
        "bp": 6, "mf": 7,
        "money_sp": 75,
        "equipment": ["Academy robes (quality)", "spell focus (staff)",
                      "research notes", "rare books"],
        "spells": [
            {"name": "Fireball",     "domain": "Fire"},
            {"name": "Shield",       "domain": "Self"},
            {"name": "Detect Magic", "domain": "Self"},
            {"name": "Dispel",       "domain": "Self"},
        ],
        "description": (
            "Brilliant but arrogant; holds grudge against Corusath for \"proving me wrong\" "
            "years ago during academic debate. Wants to see him humiliated publicly.\n\n"
            "Motivation: Capture Corusath personally to prove superiority; salvage "
            "Corusath's research for himself; enhance own reputation.\n\n"
            "Connection: Triggered if PCs investigate Academica Magicka. Pretends to help "
            "but actually wants Corusath for himself; arrives at the lab during/after the "
            "confrontation and gloats. May attack Corusath if PCs try to take him alive."
        ),
    },
}


# ──────────────────────────────────────────────────────────────────────


def default_attrs_zeroed():
    """Every canonical attribute set to 0 (the same shape Character::new_blank
    produces). The per-NPC `attrs` dict overrides only the non-zero ones."""
    return {name: 0 for name in ATTR_PARENT}


def build_hit_locations(armor_name: str, ap: int):
    return {
        loc: {"armor": armor_name, "ap": ap}
        for loc in ("Head", "Body", "L. Arm", "R. Arm", "L. Leg", "R. Leg")
    }


def apply(npc, stats):
    npc["gender"]      = stats["gender"]
    npc["age"]         = stats["age"]
    npc["level"]       = stats["level"]
    npc["race"]        = stats["race"]
    npc["size"]        = stats["size"]
    npc["characteristics"] = {"BODY": 0, "MIND": 0, "SPIRIT": 0}
    npc["characteristics"].update(stats["ch"])
    npc["attributes"] = default_attrs_zeroed()
    npc["attributes"].update(stats["attrs"])

    # Resolve "displayed total" skills into bare ranks under the
    # right attribute parent. Anything outside SKILL_PARENT is added
    # to open_skills with an explicit parent specified by the data.
    skills = {}
    for skill, total in stats.get("skills_total", {}).items():
        parent = SKILL_PARENT.get(skill)
        if not parent:
            # Unknown canonical-skill name → treat as a custom weapon
            # skill (Sword, Shield, Spear, Staff, Knife). These live
            # under Melee/Missile Combat.
            parent = "Melee Combat"
        attr_parent_char = ATTR_PARENT[parent]
        ch_rank   = npc["characteristics"][attr_parent_char]
        attr_rank = npc["attributes"].get(parent, 0)
        rank = total - ch_rank - attr_rank
        skills.setdefault(parent, {})[skill] = rank
    # Always at minimum the Unarmed + Spoken Language defaults that
    # `Character::new_blank` seeds — keep them around so the sheet's
    # base rows still show something for non-listed skills.
    skills.setdefault("Melee Combat", {}).setdefault("Unarmed", 0)
    skills.setdefault("Social Knowledge", {}).setdefault("Spoken Language", 2)
    npc["skills"] = skills

    # open_skills entries: list of dicts matching pc::OpenSkill.
    open_skills = []
    for parent_char, attr, name, total in stats.get("open_skills", []):
        ch_rank   = npc["characteristics"][parent_char]
        attr_rank = npc["attributes"].get(attr, 0)
        rank = total - ch_rank - attr_rank
        open_skills.append({
            "parent_char": parent_char,
            "attribute":   attr,
            "name":        name,
            "rank":        rank,
        })
    npc["open_skills"] = open_skills

    npc["bp_current"] = stats["bp"]
    npc["mf_current"] = stats["mf"]

    npc["hit_locations"] = build_hit_locations(stats["armor_name"], stats["armor_ap"])

    weapons = []
    for w in stats["weapons"]:
        weapons.append({
            "name":            w["name"],
            "kind":            w["kind"],
            "skill_name":      w["skill_name"],
            "two_handed":      w.get("two_handed", False),
            "init":            w["init"],
            "off_mod":         w["off_mod"],
            "def_mod":         w["def_mod"],
            "shots_per_round": 0,
            "damage":          w["damage"],
            "hp":              w["hp"],
            "range_m":         0,
            "xp":              0,
        })
    npc["weapons"] = weapons

    spells = []
    for sp in stats.get("spells", []):
        spells.append({
            "name":           sp["name"],
            "domain":         sp.get("domain", ""),
            "active_passive": "Active",
            "dr":             0, "cost": 0,
            "casting_time":   "", "distance": "", "duration": "",
            "area":           "", "cooldown": "",
            "effects":        sp.get("effects", ""),
        })
    npc["spells"] = spells

    npc["equipment"]   = stats["equipment"]
    npc["money_sp"]    = stats["money_sp"]
    npc["description"] = stats["description"]


def match_key(name: str) -> str | None:
    """Pick the NPC key for this JSON name. Match by last-name token
    or a distinctive substring so the parenthetical titles in the
    JSON don't break the lookup. Longer keys are tried first so
    "Porenson" wins over "Vael" for the rival NPC."""
    lower = name.lower()
    for key in sorted(NPCS, key=len, reverse=True):
        if key.lower() in lower:
            return key
    return None


def amar_is_running() -> bool:
    """amar holds the campaign in memory and writes it back on quit /
    on certain ops, so if it's running while we modify campaign.json,
    its in-memory (pre-populate) state will overwrite our changes on
    the next save. Detect + warn."""
    import subprocess
    try:
        out = subprocess.run(
            ["pgrep", "-af", "/release/amar"],
            capture_output=True, text=True,
        ).stdout
    except FileNotFoundError:
        return False
    return any("/release/amar" in ln for ln in out.splitlines())


def main():
    if amar_is_running():
        print("ERROR: amar appears to be running. Quit amar (q) first —")
        print("       otherwise it'll overwrite this script's changes on")
        print("       its next save (quit / promote / etc.).")
        raise SystemExit(1)
    with open(CAMP_PATH) as f:
        camp = json.load(f)

    populated = []
    skipped   = []
    for npc in camp["npcs"]:
        key = match_key(npc["name"])
        if key is None:
            skipped.append(npc["name"])
            continue
        apply(npc, NPCS[key])
        populated.append(f"{npc['name']:55s} → {key}")

    with open(CAMP_PATH, "w") as f:
        json.dump(camp, f, indent=2)

    print(f"Populated {len(populated)} NPC(s):")
    for line in populated:
        print(f"  {line}")
    if skipped:
        print(f"\nSkipped (no key matched):")
        for line in skipped:
            print(f"  {line}")


if __name__ == "__main__":
    main()
