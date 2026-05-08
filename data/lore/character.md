# The Character

Source: [d6gaming.org/The_Character](https://d6gaming.org/index.php/The_Character)

## Three Tiers

```
Characteristic (Tier 1)  →  Attribute (Tier 2)  →  Skill (Tier 3)
```

**Total Skill Value** for a roll = Skill rank + Attribute rank + Characteristic rank. All checks use the Total. Attribute-only or Characteristic-only rolls do not exist.

| Tier | Range | Mean (age 15-30) |
|---|---|---|
| Characteristic | 0-3 | 1 |
| Attribute | 0-5 | 2 |
| Skill | 0-8+ | varies |

## Characteristics

| Code | Domain |
|---|---|
| **BODY** | Strength, endurance, agility |
| **MIND** | Intelligence, knowledge, awareness, willpower |
| **SPIRIT** | Spellcasting and divine connection |

## Attributes by Characteristic

### BODY
- **Strength** — Carrying, Weight Lifting, Wield Weapon
- **Endurance** — Fortitude, Combat Tenacity, Running, Poison Resistance
- **Athletics** — Hide, Move Quietly, Climb, Swim, Ride, Jump, Balance, Tumble
- **Melee Combat** — weapon-specific skills
- **Missile Combat** — weapon-specific skills
- **Sleight** — Pick Pockets, Stage Magic, Disarm Traps

### MIND
- **Intelligence** — Innovation, Problem Solving
- **Nature Knowledge** — Medical Lore, Plant Lore, Animal Lore, Animal Handling, Magick Rituals, Alchemy
- **Social Knowledge** — Social Lore, Spoken Language, Literacy, Mythology, Legend Lore
- **Practical Knowledge** — Survival Lore, Set Traps, Ambush, crafting
- **Awareness** — Reaction Speed, Alertness, Tracking, Detect Traps, Sense Emotions, Sense Ambush, Sense of Direction, Sense Magick, Listening
- **Willpower** — Pain Tolerance, Courage, Hold Breath, Mental Fortitude

### SPIRIT
- **Casting** — Range, Duration, Area of Effect, Weight, Number of Targets
- **Attunement** — Self, Fire, Water, Air, Earth, Life, Death, Mind, Body, others
- **Innate** — Flying, Camouflage, Shape Shifting, race-specific abilities
- **Worship** — specific gods and major entities

## Derived Stats

```
Body Points (BP)    = SIZE × 2 + (Fortitude / 3)
Damage Bonus (DB)   = (SIZE + Wield Weapon) / 3
Magick Defense (MD) = (Mental Fortitude + Attunement Self) / 3
```

## Wound Thresholds

| Threshold | Condition | Penalty |
|---|---|---|
| BP ≤ 1/2 max | Wounded | -2 to all rolls |
| BP ≤ 1/4 max | Heavily Wounded | -4 to all rolls |
| 0 BP | Helpless | — |
| Negative BP | Unconscious | — |
| BP < -BP_max | Dead | — |

**Bleeding**: roll d6 every minute; on a 1, lose 1 BP.

## Size Table

SIZE is determined from weight, not chosen. amar uses the wiki's
**Half-Size Points** table (the granular optional rule) for both PCs
and NPCs, so a 75 kg adult is SIZE 3.5, a 90 kg adult is SIZE 3.5
still, and a 110 kg adult is SIZE 4.0.

### Half-Size Table (granular, up to 499 kg)

| SIZE | Weight (kg) | SIZE | Weight (kg) | SIZE | Weight (kg) |
|---|---|---|---|---|---|
| 0.5 | < 10 | 3.5 | 75-99 | 6.5 | 263-299 |
| 1 | 10-14 | 4 | 100-124 | 7 | 300-349 |
| 1.5 | 15-19 | 4.5 | 125-149 | 7.5 | 350-399 |
| 2 | 20-34 | 5 | 150-187 | 8 | 400-449 |
| 2.5 | 35-49 | 5.5 | 188-224 | 8.5 | 450-499 |
| 3 | 50-74 | 6 | 225-262 | | |

### Whole-Size Table (above 499 kg)

| SIZE | Weight (kg) |
|---|---|
| 9 | 500-599 |
| 10 | 600-724 |
| 11 | 725-849 |
| 12 | 850-999 |
| 13 | 1000-1149 |
| 14 | 1150-1299 |
| 15 | 1300-1449 |
| 16 | 1450-1599 |
| +1 | per +200 kg above 1600 |

A lean adult human is SIZE 3 (50-74 kg); an average adult is SIZE 3.5 (75-99 kg).

## Skill Rating Scale

| Rating | Description |
|---|---|
| 0 | Untrained |
| 1 | Novice |
| 2 | Trained some |
| 4 | Competent |
| 5 | Well trained |
| 7 | Professional |
| 12 | Master |
| 16 | Legendary |

## The O6 Roll

Open-ended d6:

1. Roll a d6.
2. On a **6**: reroll, +1 per 4/5/6, stop on 1/2/3.
3. On a **1**: reroll, -1 per 1/2/3, stop on 4/5/6.
4. Two consecutive **6s** anywhere → **Critical**.
5. Two consecutive **1s** anywhere → **Fumble**.

**Skill check**: `O6 + Total Skill Value ≥ DR` → success.

A Critical is always a success (within reasonable limits — impossible actions cannot be done). A Fumble is always a failure. If a Critical attack is countered by a Critical defense, the hit is only a success if the attack was higher than the defense.

## Critical and Fumble Tables

When the open-ended roll lands **6,6** (Critical) or **1,1** (Fumble) on any roll — combat or skill — roll twice on the matching table: first for the **category** (1-6), then for the **specific result** within the category. If the result is inapplicable to the situation, pick the entry above on the Critical table or below on the Fumble table.

### Criticals

**1 — Impression**
| d6 | Result |
|---|---|
| 1 | Looks really cool |
| 2 | Impressive — adjacent friends get +1 next round |
| 3 | Very impressive — adjacent friends get +1 next D rounds |
| 4 | Fearsome — foe rolls on Fear Table with +9 adjustment |
| 5 | Awesome — foe rolls on Fear Table with +6 adjustment |
| 6 | Wild — foe rolls on Fear Table with +3 adjustment |

**2 — Side effect**
| d6 | Result |
|---|---|
| 1 | Opponent off balance — Status -1 next round |
| 2 | Opponent confused — Status -3 next round |
| 3 | Opponent stunned — Status -3 for 3 rounds |
| 4 | Opponent staggered — Status -D for D rounds |
| 5 | Opponent reeling — Status -O for O rounds |
| 6 | Opponent shocked — Status -(O+3) for the rest of the fight |

**3 — Increased effect**
| d6 | Result |
|---|---|
| 1 | Good hit — +1 damage |
| 2 | Tough hit — +3 damage |
| 3 | Great hit — +(D+1) damage |
| 4 | Greater hit — +(O+2) damage |
| 5 | Power hit — double damage (after AP) |
| 6 | Opportunity found — immediate free attack |

**4 — Added effect**
| d6 | Result |
|---|---|
| 1 | Foe knocked down on failed Tumble DR 8 |
| 2 | Foe knocked down on failed Tumble DR 12 |
| 3 | Roll for disarming the opponent |
| 4 | Damage also done to opponent's weapon |
| 5 | Damage also done to opponent's weapon (double damage to weapon) |
| 6 | Opponent loses equipment (GM's discretion) |

**5 — Special**
| d6 | Result |
|---|---|
| 1 | Bleeding — -1 BP per minute |
| 2 | Bleeding — -1 BP per round |
| 3 | Muscle strained — opponent Status -3 until Medical Lore DR 8 |
| 4 | Disable special location (eye, finger, …) — Medical Lore DR 8 to fix |
| 5 | Disable special location — Medical Lore DR 12 to fix |
| 6 | Opponent faints — Medical Lore DR 8 to awaken |

**6 — Roll twice on this table, ignoring any subsequent 6s, add one experience mark.**

### Fumbles

**1 — Roll twice on this table, ignoring any subsequent 1s, subtract one experience mark.**

**2 — Special**
| d6 | Result |
|---|---|
| 1 | Lose next attack; opponent gets +10 to next attack |
| 2 | Hit self |
| 3 | Hit nearest friend |
| 4 | Hit nearest friend, half damage |
| 5 | Obstruct nearest friend — friend Status -3 next round |
| 6 | Muscle strained — Status -3 until Medical Lore DR 8 |

**3 — Unwanted effect**
| d6 | Result |
|---|---|
| 1 | Lose equipment (GM's discretion) |
| 2 | Damage to own weapon |
| 3 | Weapon stuck — Strength DR 10 to free |
| 4 | Lose weapon — no attack until retrieved, -5 defense |
| 5 | Fall on failed Tumble DR 12 |
| 6 | Fall on failed Tumble DR 8 |

**4 — Stun effect**
| d6 | Result |
|---|---|
| 1 | Shocked — Status -(O+3) for rest of fight |
| 2 | Reeling — Status -O for O rounds |
| 3 | Staggered — Status -D for D rounds |
| 4 | Stunned — Status -3 for 3 rounds |
| 5 | Confused — Status -3 next round |
| 6 | Off balance — Status -1 next round |

**5 — Added effect**
| d6 | Result |
|---|---|
| 1 | Very fatigued — Endurance -3 for rest of fight (min 1) |
| 2 | Very tired — Strength -3 for rest of fight (min 1) |
| 3 | Very dazed — Reaction Speed and Awareness -3 for rest of fight |
| 4 | Fatigued — Endurance -1 for rest of fight (min 1) |
| 5 | Tired — Strength -1 for rest of fight (min 1) |
| 6 | Dazed — Reaction Speed and Awareness -1 for rest of fight |

**6 — Impression**
| d6 | Result |
|---|---|
| 1 | Terrible for morale — friends -1 to all rolls for next D rounds |
| 2 | Very bad for morale — friends -1 to all rolls for rest of round |
| 3 | Bad for morale — friends -1 to attack for rest of round |
| 4 | You make a fool of yourself — laughter is heard |
| 5 | Botched it — giggles are heard |
| 6 | Awkward looking |

In Psychic Combat: a Critical attack forces the defender to roll on the Fear Table; a Fumble forces the attacker.

## Fear & Courage

```
Roll: (MIND + Willpower + Courage) + O6  vs.  fear DR
```

| Miss by | Effect |
|---|---|
| 1 | -1 to actions this round |
| 2 | -1 to actions for two rounds |
| 3 | -1 to actions for three rounds |
| 4 | Frozen for 1 round |
| 5+ | Flee in panic |
| Critical Failure | Heart attack — Endurance check or unconscious |

Roll each round in presence of the fear source until the roll succeeds. Negative effects accumulate.

## Character Creation

1. **Characteristics**: Choose 1 in two of the three; the third stays at 0.
2. **Attributes**: Distribute the values **3, 2, 2, 1, 1, 1** among attributes under your chosen characteristics. Attributes under the 0-rated characteristic stay at 0.
3. **Skills**: Distribute **3, 2, 2, 2, 1, 1, 1, 1, 1**.
4. **Spoken Language**: every character starts with **2** in their native tongue (in addition to point distribution above).

## Advancement

Marks-based progression where credit cascades upward.

**Skills**:
- 1 mark for any successful attempt where a roll of 1+ is required.
- 2 marks for five consecutive attempts using the skill.
- 2 marks per week of training with a teacher.
- 1 mark per week of solo training (BODY skill); ½ mark for MIND/SPIRIT.

**Attributes**: 1 mark when a skill under it advances a level. ½ mark per week with a teacher (BODY); ¼ mark solo (MIND/SPIRIT).

**Characteristics**: 1 mark when an attribute under it advances. Half the attribute training rate.

**Mark conversion**: 3 marks at one tier = 1 mark at the tier above.

**Advancing to next level**: needs `5 × (N + 1)` marks to go from level N to N+1, then a d6 of 2+ (fails only on a 1).

**Combat experience**: critical hit = +1 mark, fumble = -1 mark.

## Encumbrance

Based on Carrying skill.

| Carried | Penalty |
|---|---|
| Up to 2× Carrying | None |
| Up to 5× Carrying | -1 |
| Up to 10× Carrying | -3 |
| Up to 20× Carrying | -5 |

Swimming multiplies all weight by 5.

## Playable Races

| Race | Modifier |
|---|---|
| Human | Balanced; 2 bonus skill points |
| Elf | +1 SPIRIT characteristic |
| Dwarf | +1 BODY characteristic |
