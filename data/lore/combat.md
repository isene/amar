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

## Secondary Weapons

+1 Off and/or Def per extra weapon beyond primary. Shields don't count as extras. Off-hand weapon: -3 penalty; +2 Strength requirement.

**Unarmed bonus**: +1 Off and Def per full 5 in Unarmed.
**Dodge bonus**: +1 Def per full 5 in Dodge.

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
