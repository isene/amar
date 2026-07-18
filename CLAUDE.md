# amar — companion-session data contract

amar (v0.1.49+) is designed to run WITH a companion Claude Code session in a
sibling glass window. The companion generates and modifies campaign data by
editing `~/.amar/campaigns/<name>/campaign.json` and `~/.amar/world.json`
directly — amar stat()s both files on every keypress and reloads on change,
so edits appear live. The in-app Forge/Inspire tabs are retired; the
companion session replaces them.

## World vs Campaign (v0.2.0)

Tabs: 1 World, 2 Campaign, 3 Combat, 4 Lore.

`~/.amar/world.json` holds what exists independently of any campaign:

```json
{"locations": [ ...Location objects... ],
 "npcs": [ ...FULL-form Character objects... ]}
```

- **World NPCs are the wiki's major personalities** (royals, barons, the
  Magick Circle, the Wayanah-party, King Gorm, Altira, …). They are always
  FULL character sheets — never sparse stubs. Calibrate old-system (0-20)
  stats via the Altira conversion on the wiki (legacy → 3-tier mapping).
- **Campaigns** (`~/.amar/campaigns/<name>/campaign.json`) hold the PCs,
  campaign-specific lesser NPCs, encounters, diary, adventures. Several
  campaigns can run concurrently; a campaign meets world NPCs by reference
  (copy into `npcs[]` only if the campaign needs a local variant).
- `locations` live in world.json only; `campaign.locations` is legacy and
  stays empty.

## Rules for editing campaign.json while amar runs

1. **Write atomically**: write to `campaign.json.tmp`, then rename over.
   amar may read at any keypress; it must never see half a file.
2. **Read-modify-write**: always re-read the file just before writing.
   amar saves on most GM actions, and your stale copy would clobber them.
3. **Never reorder or delete `saved_encounters` entries or their `npcs`
   arrays while `combat` is non-null** — live combats reference encounter
   NPCs by index (`EncounterNpc { enc_idx, npc_idx }`).

## Character forms: short vs full

`Character` (src/pc.rs) is ONE struct serving both forms; the difference is
how populated it is. The struct is `#[serde(default)]` and every character
is run through `Character::normalize()` on load, so a **sparse JSON object
is a valid character**: omitted fields default, and normalize fills weight/
SIZE/BP/MF, the Unarmed weapon, hit locations, Spoken Language 2, and sets
`active: true` (sparse marker: `size` absent/0).

### Short form (encounter NPCs — `saved_encounters[].item.npcs[]`)

The minimum useful injection. MUST always include the stealth/awareness
quartet — **Move Quietly, Hide (Athletics), Reaction Speed, Alertness
(Awareness)** — at rank ≥ 1; the encounter block displays their totals and
the GM rolls surprise/ambush straight off it.

```json
{
  "name": "Araxi Raider", "gender": "Male", "race": "Araxi", "level": 2,
  "characteristics": {"BODY": 1, "MIND": 0, "SPIRIT": 0},
  "attributes": {"Endurance": 2, "Athletics": 2, "Melee Combat": 2,
                 "Missile Combat": 1, "Awareness": 2},
  "skills": {
    "Athletics": {"Move Quietly": 2, "Hide": 2, "Dodge": 1},
    "Awareness": {"Reaction Speed": 1, "Alertness": 1},
    "Endurance": {"Fortitude": 1},
    "Melee Combat": {"Spear": 2}
  },
  "weapons": [{"name": "Spear", "kind": "Melee", "skill_name": "Spear",
               "init": 6, "off_mod": -1, "def_mod": -4, "damage": -2, "hp": 7}],
  "hit_locations": {"Body": {"armor": "Hide scraps", "ap": 1}},
  "weight_kg": 65
}
```

Wrap NPCs in an encounter + `Saved` envelope:

```json
{"id": <max existing id + 1>, "name": "...", "created_at": <unix>,
 "flavour": "one-line GM summary",
 "item": {"terrain_idx": 0, "day": true, "category": "...", "spec": "...",
          "count": <n>, "attitude": "HOSTILE", "npcs": [ ... ]}}
```

### Full form (campaign roster — `npcs[]` / `pcs[]`)

Everything the sheet renders: identity (age, height, birthplace,
description, clothing), the full 3-tier spread (canonical attribute/skill
names in src/pc.rs `ATTRIBUTES`/`SKILLS` — unlisted cells resolve to 0),
per-location armour, equipment, money_sp, notes, portrait_path (portraits
live in `~/.amar/campaigns/<name>/portraits/<Name>.png`, landscape
head-and-shoulders; sanitize name: non-alphanumeric → `_`).

### Upgrade path: short → full

When the GM asks to "upgrade" an encounter (NPC) to full form:

1. Re-read campaign.json.
2. **Copy** (never move) the NPC object out of
   `saved_encounters[i].item.npcs[j]` into `npcs[]` — the encounter keeps
   its short-form copy, so combat refs and the encounter block stay valid.
3. Enrich the copy in place: description, clothing, birthplace, age/height,
   fuller skill spread consistent with its level and role, equipment,
   money, per-location armour, `is_pc: false`. Optionally generate a
   portrait (gpt-image-1) into `portraits/` and set `portrait_path`.
4. De-duplicate by name: if `npcs[]` already has the name, update that
   entry instead of appending.
5. Write atomically. amar shows the new roster NPC on the next keypress.

Derived stats (BP/MD/DB/OFF/DEF/totals) are always computed by amar from
the three tiers — never inject computed values, only ranks.

## Weapon combinations (one row = one skill)

A weapon combination is ONE weapon row and ONE skill, named for the combo:
`"Longsword & Shield"`, `"Two Knives"`, `"Whip/Buckler"`. Fold the shield's
defence into the row's `def_mod` (sword def + shield def); Init/Off/Dam
come from the primary weapon. Never inject a bare shield row plus a
separate sword row for a character who fights with the pair — the
combination is its own skill, ranked separately from the solo weapon.

Synergies: DEF totals get +1 per 5 in Dodge (Athletics), computed by amar.
Pure dodging is a GM call (Dodge total − 2), no data needed. There is no
Unarmed synergy — the Melee Combat attribute already carries it.

## Locations

`world.json locations[]` holds the places of the world. Sparse-friendly:

```json
{"name": "Borgheim", "kind": "Dwarven mountain-capital",
 "description": "…multi-line prose…",
 "image": "/abs/path/to/map.png", "notes": "…", "created_at": <unix>}
```

`image` (optional) renders INLINE below the text via glow and opens
externally with →. In-app: `n` adds, ENTER edits description, `c` renames,
`D` deletes; `/` search covers locations.

## Build

`PATH="/usr/bin:$PATH" cargo build --release` (~/bin/cc shadows the C
compiler). `~/bin/amar` symlinks target/release/amar. Tests: `cargo test`.
Fe2O3 house rules apply (see ../CLAUDE.md): battery first, crust panes are
text-only, images via glow overlays.
