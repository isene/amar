# Combat — Quick Reference

Source: [d6gaming.org/Combat](https://d6gaming.org/index.php/Combat)

## The O6 Roll

Open-ended d6:

1. Roll a d6.
2. On a **6**: reroll, +1 per 4/5/6, stop on 1/2/3.
3. On a **1**: reroll, -1 per 1/2/3, stop on 4/5/6.
4. Two consecutive 6s anywhere → **Critical**.
5. Two consecutive 1s anywhere → **Fumble**.

## Action Resolution

```
O6 + skill_total  vs.  Difficulty Rating (DR)
```

Skill total = Characteristic + Attribute + Skill rank.

A round is **6 seconds**; actions resolve second-by-second (low to high). Initiative is rolled each round.

## Initiative

```
Initiative = weapon Init + Reaction Speed (skill total) + O6
```

Modifiers:

| Situation | Modifier |
|---|---|
| Single-shot missile | +0 |
| Two-shot missile | first at +5, second at +0 |
| Ready aimed missile | +10 |
| Spells / innate magic | 0 / 10 |
| Moving and fighting | -5 per extra action |
| Buying initiative (optional) | -1 Off per +1 init |
| Fast shooting (optional) | -1 Off per +2 init |

## Attack

```
Attacker:  O6 + offense_total
Defender:  O6 + defense_total
If attacker > defender: hit
```

## Damage

```
Damage = weapon DAM + DB + O6 - armor AP
```

Damage Bonus: `DB = (SIZE + Wield Weapon) / 3`.

Armor Points are subtracted from incoming damage *before* applying to Body Points.

## Wound Thresholds

| Threshold | Condition | Penalty |
|---|---|---|
| BP ≤ 1/2 of max | Wounded | -2 to all rolls |
| BP ≤ 1/4 of max | Heavily Wounded | -4 to all rolls |
| BP = 0 | Helpless | — |
| BP < 0 | Unconscious | — |
| BP < -BP_max | Dead | — |

**Bleeding**: roll d6 every minute; on a 1, lose 1 BP.

## Vital vs Non-Vital Locations

**Vital locations (head, body):**
- Damage = BP in one blow → helpless
- Damage = 2 × BP in one blow → dead

**Non-vital locations (limbs):**
- Damage = BP in one blow → location unusable
- Damage = 2 × BP in one blow → severed / crushed (magical healing required)

Roll d6 for hit location; on shared numbers, roll again: 1-3 = left, 4-6 = right.

Human distribution: 50% of BP in head + arms, 80% in body + legs.

## Combat Stances

| Stance | Off | Def |
|---|---|---|
| Fighting Offensively | +3 | -5 |
| Fighting Defensively | -5 | +3 |
| Only Defending | — | +5 |
| Power Hit | -5 | (add DB again to damage) |

Berserker advantage changes Offensive to +4 / -5.

## Multiple Opponents (positional defense penalty)

| Position | Defensive penalty |
|---|---|
| Side | -1 |
| Rear-side | -4 |
| Directly behind | -7 |

## Light Conditions

| Condition | Off | Def |
|---|---|---|
| Twilight / torchlight | -1 | -2 |
| Full moonlight | -2 | -4 |
| Starlight | -3 | -6 |
| Darkness | -5 | -10 |

Light source radii: candle 1 m, torch 5 m, lantern 7 m. Within radius = torchlight; ×2 = moonlight; ×4 = starlight.

## Condition Modifiers

| Condition | Off | Def | Movement |
|---|---|---|---|
| Half BP (Wounded) | -2 | -2 | normal |
| Quarter BP | -4 | -4 | normal |
| Partially unaware | 0 | -5 | normal |
| Fully unaware | X | -10 | normal |
| Immobilized | X | -5 | none |
| Down on knee | -3 | -3 | ×1/10 |
| Down on ground | -5 | -5 | ×1/10 |
| Slippery ground | -2 | -2 | ×3/4 |

## Movement in Combat

```
Human combat movement = 12 m / round  (0.4 × base of 30 m)
Water = 2 m / round
Run = ×2
Move & fight = ×1/2
Move in melee = ×1/4
Disengage = ×1
```

You can always move 1 m per round without penalty.

## Multiple Attacks (optional)

-5 Offensive per extra attack. Second attack on second 6, third on second 5, etc.

## Weapon Combinations

A weapon combination (Longsword & Shield, Two Knives, Whip/Buckler) is ONE
skill of its own, trained and ranked separately from the solo weapon. The
sheet folds the pair's modifiers into that single row.

**Dodge bonus**: +1 Def per full 5 in Dodge (Athletics).
**Pure dodging**: forgo attacks; defence = Dodge total - 2.

## Weapon Familiarity

| Situation | Penalty |
|---|---|
| Same weapon type | -1 |
| Same weapon category | -3 |
| Different category | -5 |

## Strength Requirements

| Below requirement by | Penalty |
|---|---|
| 1 | -1 / -1 |
| 2 | -3 / -3 |
| 3 | -5 / -5 |
| 4+ | Cannot use |

## Missile DRs

| Range | DR |
|---|---|
| Half range | 5 |
| Normal range | 10 |
| Double range | 15 |
| Very long range | 20 |
| Maximum range | 25 |

Modifiers: +1 per Size, -3 hit-location aim, -5 missile firing penalty floor, dodge missiles -2 + 1 per full 5 in Dodge.

Shield AP vs missiles: round shield +1 AP, kite shield +2 AP.

## Grapple

1. Make an Unarmed attack.
2. Grapple Roll = O6 + 2 × DB. Beat defender's roll → grapple holds.
3. Each subsequent round: both roll Strength + O6; attacker > defender → grapple continues.

## Disarm / Break Weapon

- Disarm 1H: damage > opponent's Strength.
- Disarm 2H: damage > 2 × opponent's Strength.
- Break weapon: damage > weapon HP.

## Critical and Fumble Tables

The same tables apply to combat attack rolls and to general skill rolls — any open-ended roll that lands two consecutive 6s (Critical) or two consecutive 1s (Fumble). For combat the most common categories are Side effect (2), Increased effect (3), and Added effect (4).

Procedure: roll once for the **category** (1-6), then once for the **specific result** (1-6). If a result is inapplicable to the situation, pick the row above on the Critical table or the row below on the Fumble table.

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
| Critical failure | Heart attack (Endurance check or unconscious) |

Negative effects accumulate; roll each round while the source is present.
