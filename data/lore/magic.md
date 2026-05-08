# Magic — Quick Reference

Source: [d6gaming.org/Magick](https://d6gaming.org/index.php/Magick)

## Three Categories

1. **Spell magick** — active casting via the Casting attribute.
2. **Ritual magick** — slow, ingredient-driven; uses Magick Rituals (Nature Knowledge).
3. **Alchemy** — potions and amulets; uses Alchemy (Nature Knowledge).

## Magical Aptitude

Only **1 in 50 humans** has Magical Aptitude (MA). Of those:

- 1 in 3 develop a **Womp** (magical focus)
- 1 in 2 of those learn spells

So roughly **1 in 300 humans** can actually cast.

## Spell Casting

### Requirements

```
SPIRIT >= 2  AND  (SPIRIT + Casting attribute) >= 5
```

### The Roll

```
O6 + SPIRIT + Attunement + Domain Skill  vs.  DR
```

Example: O6 (6,5,2 = 7) + SPIRIT 2 + Attunement 3 + Fire 4 = 16 vs DR 10 — succeeds with margin 6.

### Active Spell Capacity

Limited by **Mental Fortitude** (MIND → Willpower → Mental Fortitude). Activating a spell requires ~10 minutes of concentration. Recovery: 1 point per hour of rest, full recovery on 8 h sleep.

## Spell Domains

The wiki has the following domain categories:

- **Fire** (25 spells)
- **Water** (36)
- **Earth** (27)
- **Air** (41)
- **Life** (47)
- **Black** (26)
- **Ice** (16)
- **Lava** (3)
- **Magic** (18 — meta-domain for Detect / Dispel / Memorize / Recall etc.)
- **Perception** (20)
- **Protection** (12)
- **Summoning** (8)

**Death** is mentioned as an Attunement domain but the wiki's `Category:Death_Magick` page is currently empty; amar fills this gap with a small author-canon set marked accordingly.

Many spells belong to **multiple** domains; the caster picks which Attunement to add when rolling.

## Spell Property Schema

Every spell on the wiki carries:

| Field | Notes |
|---|---|
| Name | Display name |
| Domain | One or more |
| Encumbrance | Mental load — limits how many active spells |
| Cooldown | Minimum time before recasting |
| Casting Time | Rounds or minutes |
| Active / Passive | Type of casting focus |
| Restrictions | Special conditions |
| DR | Difficulty Rating to cast |
| Cost | Mental Fortitude consumed |
| Distance | Range |
| Duration | How long the effect lasts |
| Area of Effect | Area or targets affected |
| Effects | What it does |
| Receiving | Min Attunement required to receive in a transfer |
| Giving | Length of transfer ritual to give the spell |

## Acquiring Spells

Four methods:

1. **Direct Gift** — two-week ritual transfer from a caster who knows the spell.
2. **Reading a spell book** — first reader gains the spell.
3. **Divine grant** from a god.
4. **Ancient scroll or artifact** discovery.

Recipients must meet the spell's `Receiving` Attunement requirement.

## Ritual Magick

Skill: **Magick Rituals** (under MIND → Nature Knowledge).

```
O6 + Magick Rituals total  >=  DR
```

Rituals are slow, ingredient-driven, and can have very long durations. The wiki has 11 rituals — see the **Rituals** category.

## Alchemy (Potions)

Skill: **Alchemy** (under MIND → Nature Knowledge).

```
O6 + Alchemy total  >=  DR
```

Potions usually take ~1 hour to make and last ~1 hour. Each requires specific ingredients (often blood, herbs, sometimes a poem). The wiki has 9 potions — see the **Potions** category.

## Crystal of Spell Storing

The one wiki-canonical magic item:

- **Cost**: 20 gp
- Stores one spell; activates instantly at the start of a round.
- Stored spell is personal to the bearer.
- Dissipates after one week if unused.
- **Creator permanently sacrifices 5 MA**.
- Creation requires: 10-carat diamond, gift from a faerie, blessing from a creature with 15+ MA, a poem (2 min, DR 8 in Poem Recital), and one week of rest.

Other magic items in amar are author-canon (filling a wiki gap); see the Magic Items section.
