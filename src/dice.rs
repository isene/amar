//! O6 — open-ended d6 system used by the Amar RPG (d6gaming.org).
//!
//! Roll procedure (per the wiki):
//!   - Roll a d6.
//!   - On a 6: reroll, +1 per 4/5/6, stop on 1/2/3.
//!   - On a 1: reroll, -1 per 1/2/3, stop on 4/5/6.
//!   - Two consecutive 6s anywhere -> Critical.
//!   - Two consecutive 1s anywhere -> Fumble.
//!
//! All resolution is `O6 + skill_total` against a Difficulty Rating (DR).
//! Combat uses opposed `attacker offense + O6` vs `defender defense + O6`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Normal,
    Critical,
    Fumble,
}

#[derive(Debug, Clone)]
pub struct Roll {
    pub total: i32,
    pub outcome: Outcome,
    pub sequence: Vec<u8>,
}

pub trait Rng {
    fn d6(&mut self) -> u8;
}

pub struct StdRng {
    state: u64,
}

impl StdRng {
    pub fn new(seed: u64) -> Self {
        let s = if seed == 0 { 0x9e3779b97f4a7c15 } else { seed };
        Self { state: s }
    }
    pub fn from_time() -> Self {
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0xdeadbeef);
        Self::new(secs)
    }
}

impl Rng for StdRng {
    fn d6(&mut self) -> u8 {
        // SplitMix64 step, then map low bits to 1..=6.
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        ((z >> 32) % 6 + 1) as u8
    }
}

pub fn o6(rng: &mut impl Rng) -> Roll {
    let mut sequence = Vec::with_capacity(2);
    let first = rng.d6();
    sequence.push(first);
    if first >= 2 && first <= 5 {
        return Roll { total: first as i32, outcome: Outcome::Normal, sequence };
    }
    // Snake-eyes (Critical for up, Fumble for down) are FLAGS, not
    // terminators. The cascade keeps going until the proper stop
    // roll lands (1/2/3 for up, 4/5/6 for down). Total accumulates
    // through the whole cascade. So a sequence like
    //   6,5,4,6,6,4,2 -> 11 Critical
    //   1,1,2,3,1,1,2,1,6 -> -6 Fumble
    // is possible — the snake-eyes set the outcome flag once, then
    // the cascade continues.
    if first == 6 {
        let mut total: i32 = 6;
        let mut prev = 6u8;
        let mut outcome = Outcome::Normal;
        loop {
            let r = rng.d6();
            sequence.push(r);
            if prev == 6 && r == 6 {
                outcome = Outcome::Critical;
            }
            match r {
                4..=6 => total += 1,
                _ => return Roll { total, outcome, sequence },
            }
            prev = r;
        }
    }
    // first == 1: down cascade.
    let mut total: i32 = 1;
    let mut prev = 1u8;
    let mut outcome = Outcome::Normal;
    loop {
        let r = rng.d6();
        sequence.push(r);
        if prev == 1 && r == 1 {
            outcome = Outcome::Fumble;
        }
        match r {
            1..=3 => total -= 1,
            _ => return Roll { total, outcome, sequence },
        }
        prev = r;
    }
}

pub fn check(rng: &mut impl Rng, skill_total: i32, dr: i32) -> CheckResult {
    let r = o6(rng);
    let final_total = r.total + skill_total;
    let success = match r.outcome {
        Outcome::Critical => true,
        Outcome::Fumble => false,
        Outcome::Normal => final_total >= dr,
    };
    CheckResult { roll: r, final_total, dr, success }
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub roll: Roll,
    pub final_total: i32,
    pub dr: i32,
    pub success: bool,
}

// ---------------------------------------------------------------- Critical
// and Fumble tables (data/lore/character.md + combat.md, wiki canon).
//
// The tables apply identically to combat attack rolls and to general
// skill rolls. Procedure: on Critical or Fumble, roll a category
// (1-6) and then a specific result (1-6) within that category. If
// the category is 6 (Critical) or 1 (Fumble), the rule says "roll
// twice on this table, ignoring subsequent 6s/1s, +/-1 mark" — we
// surface the recursion in the result.

/// One entry on the critical/fumble tables — its category + entry
/// numbers (so the GM sees what was rolled) plus the description.
#[derive(Debug, Clone)]
pub struct TableHit {
    pub category: u8,
    pub category_name: &'static str,
    pub entry: u8,
    pub description: String,
}

const CRIT_CATS: [&str; 6] = [
    "Impression",
    "Side effect",
    "Increased effect",
    "Added effect",
    "Special",
    "Roll twice (no 6s) + 1 mark",
];

const FUMBLE_CATS: [&str; 6] = [
    "Roll twice (no 1s) - 1 mark",
    "Special",
    "Unwanted effect",
    "Stun effect",
    "Added effect",
    "Impression",
];

const CRIT_TABLE: [[&str; 6]; 5] = [
    // 1 - Impression
    [
        "Looks really cool",
        "Impressive — adjacent friends get +1 next round",
        "Very impressive — adjacent friends get +1 next D rounds",
        "Fearsome — foe rolls on Fear Table with +9 adjustment",
        "Awesome — foe rolls on Fear Table with +6 adjustment",
        "Wild — foe rolls on Fear Table with +3 adjustment",
    ],
    // 2 - Side effect
    [
        "Opponent off balance — Status -1 next round",
        "Opponent confused — Status -3 next round",
        "Opponent stunned — Status -3 for 3 rounds",
        "Opponent staggered — Status -D for D rounds",
        "Opponent reeling — Status -O for O rounds",
        "Opponent shocked — Status -(O+3) for the rest of the fight",
    ],
    // 3 - Increased effect
    [
        "Good hit — +1 damage",
        "Tough hit — +3 damage",
        "Great hit — +(D+1) damage",
        "Greater hit — +(O+2) damage",
        "Power hit — double damage (after AP)",
        "Opportunity found — immediate free attack",
    ],
    // 4 - Added effect
    [
        "Foe knocked down on failed Tumble DR 8",
        "Foe knocked down on failed Tumble DR 12",
        "Roll for disarming the opponent",
        "Damage also done to opponent's weapon",
        "Damage also done to opponent's weapon (double damage to weapon)",
        "Opponent loses equipment (GM's discretion)",
    ],
    // 5 - Special
    [
        "Bleeding — -1 BP per minute",
        "Bleeding — -1 BP per round",
        "Muscle strained — opponent Status -3 until Medical Lore DR 8",
        "Disable special location (eye, finger, …) — Medical Lore DR 8 to fix",
        "Disable special location — Medical Lore DR 12 to fix",
        "Opponent faints — Medical Lore DR 8 to awaken",
    ],
];

const FUMBLE_TABLE: [[&str; 6]; 5] = [
    // 2 - Special  (cat 1 is the recursive one)
    [
        "Lose next attack; opponent gets +10 to next attack",
        "Hit self",
        "Hit nearest friend",
        "Hit nearest friend, half damage",
        "Obstruct nearest friend — friend Status -3 next round",
        "Muscle strained — Status -3 until Medical Lore DR 8",
    ],
    // 3 - Unwanted effect
    [
        "Lose equipment (GM's discretion)",
        "Damage to own weapon",
        "Weapon stuck — Strength DR 10 to free",
        "Lose weapon — no attack until retrieved, -5 defense",
        "Fall on failed Tumble DR 12",
        "Fall on failed Tumble DR 8",
    ],
    // 4 - Stun effect
    [
        "Shocked — Status -(O+3) for rest of fight",
        "Reeling — Status -O for O rounds",
        "Staggered — Status -D for D rounds",
        "Stunned — Status -3 for 3 rounds",
        "Confused — Status -3 next round",
        "Off balance — Status -1 next round",
    ],
    // 5 - Added effect
    [
        "Very fatigued — Endurance -3 for rest of fight (min 1)",
        "Very tired — Strength -3 for rest of fight (min 1)",
        "Very dazed — Reaction Speed and Awareness -3 for rest of fight",
        "Fatigued — Endurance -1 for rest of fight (min 1)",
        "Tired — Strength -1 for rest of fight (min 1)",
        "Dazed — Reaction Speed and Awareness -1 for rest of fight",
    ],
    // 6 - Impression
    [
        "Terrible for morale — friends -1 to all rolls for next D rounds",
        "Very bad for morale — friends -1 to all rolls for rest of round",
        "Bad for morale — friends -1 to attack for rest of round",
        "You make a fool of yourself — laughter is heard",
        "Botched it — giggles are heard",
        "Awkward looking",
    ],
];

/// Result of rolling on the critical or fumble table. `recursive`
/// is true when the original cat-6 (Critical) or cat-1 (Fumble)
/// trigger fired and `hits` therefore contains two sub-rolls
/// instead of one. The caller surfaces the recursion so the user
/// sees both the trigger ("Cat 1 — Roll twice on this table") and
/// the resolved sub-rolls.
#[derive(Debug, Clone)]
pub struct TableRoll {
    pub recursive: bool,
    pub hits: Vec<TableHit>,
}

/// Roll on the critical table. Cat 6 rolls twice (with subsequent
/// 6s ignored) and adds an XP mark — we surface that as
/// `recursive = true`. Returns 1 or 2 hits.
pub fn roll_critical(rng: &mut impl Rng) -> TableRoll {
    let cat = rng.d6();
    if cat == 6 {
        let mut hits = Vec::with_capacity(2);
        for _ in 0..2 {
            let mut c = rng.d6();
            while c == 6 { c = rng.d6(); }
            let e = rng.d6();
            hits.push(crit_entry(c, e));
        }
        return TableRoll { recursive: true, hits };
    }
    let entry = rng.d6();
    TableRoll { recursive: false, hits: vec![crit_entry(cat, entry)] }
}

/// Roll on the fumble table. Cat 1 rolls twice (with subsequent 1s
/// ignored) and subtracts an XP mark — same shape as roll_critical.
pub fn roll_fumble(rng: &mut impl Rng) -> TableRoll {
    let cat = rng.d6();
    if cat == 1 {
        let mut hits = Vec::with_capacity(2);
        for _ in 0..2 {
            let mut c = rng.d6();
            while c == 1 { c = rng.d6(); }
            let e = rng.d6();
            hits.push(fumble_entry(c, e));
        }
        return TableRoll { recursive: true, hits };
    }
    let entry = rng.d6();
    TableRoll { recursive: false, hits: vec![fumble_entry(cat, entry)] }
}

fn crit_entry(cat: u8, entry: u8) -> TableHit {
    let c = cat.clamp(1, 5);
    let e = entry.clamp(1, 6);
    TableHit {
        category: cat,
        category_name: CRIT_CATS[(cat as usize - 1).min(5)],
        entry: e,
        description: CRIT_TABLE[c as usize - 1][e as usize - 1].to_string(),
    }
}

fn fumble_entry(cat: u8, entry: u8) -> TableHit {
    let c = cat.clamp(2, 6);
    let e = entry.clamp(1, 6);
    TableHit {
        category: cat,
        category_name: FUMBLE_CATS[(cat as usize - 1).min(5)],
        entry: e,
        // FUMBLE_TABLE is 0-indexed for cats 2-6.
        description: FUMBLE_TABLE[c as usize - 2][e as usize - 1].to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic queue-driven RNG for reproducible test rolls.
    struct QueueRng(Vec<u8>);
    impl Rng for QueueRng {
        fn d6(&mut self) -> u8 {
            self.0.remove(0)
        }
    }

    #[test]
    fn middle_results_pass_through() {
        for v in 2..=5 {
            let mut q = QueueRng(vec![v]);
            let r = o6(&mut q);
            assert_eq!(r.total, v as i32);
            assert_eq!(r.outcome, Outcome::Normal);
            assert_eq!(r.sequence, vec![v]);
        }
    }

    #[test]
    fn six_then_stop_low_extends_normal() {
        // 6, 5, 3 -> 6 + 1 (for the 5) = 7, stop on the 3
        let mut q = QueueRng(vec![6, 5, 3]);
        let r = o6(&mut q);
        assert_eq!(r.total, 7);
        assert_eq!(r.outcome, Outcome::Normal);
        // Sequence MUST include the terminator (3), not just the
        // extending rolls. Anything less would mislead the GM
        // about what was actually rolled.
        assert_eq!(r.sequence, vec![6, 5, 3]);
    }

    #[test]
    fn up_terminator_always_in_sequence() {
        // Multi-step up cascade ending in 1, 2, or 3.
        for term in 1u8..=3 {
            let mut q = QueueRng(vec![6, 4, 5, term]);
            let r = o6(&mut q);
            assert_eq!(r.total, 8); // 6 + 1 + 1 = 8
            assert_eq!(r.outcome, Outcome::Normal);
            assert_eq!(r.sequence, vec![6, 4, 5, term]);
        }
    }

    #[test]
    fn down_terminator_always_in_sequence() {
        for term in 4u8..=6 {
            let mut q = QueueRng(vec![1, 2, 3, term]);
            let r = o6(&mut q);
            assert_eq!(r.total, -1); // 1 - 1 - 1 = -1
            assert_eq!(r.outcome, Outcome::Normal);
            assert_eq!(r.sequence, vec![1, 2, 3, term]);
        }
    }

    #[test]
    fn double_six_sets_critical_flag_then_cascade_continues() {
        // 6, 6, 2 -> snake-eyes flag Critical, second 6 still gives
        // +1 (it's a 4/5/6 result), then 2 terminates the cascade.
        // Total 6 + 1 = 7, outcome Critical.
        let mut q = QueueRng(vec![6, 6, 2]);
        let r = o6(&mut q);
        assert_eq!(r.outcome, Outcome::Critical);
        assert_eq!(r.total, 7);
        assert_eq!(r.sequence, vec![6, 6, 2]);
    }

    #[test]
    fn critical_extends_through_cascade() {
        // User example: 6,5,4,6,6,4,2 -> 11 Critical.
        // 6 (initial) +1 (5) +1 (4) +1 (6) +1 (6, snake-eyes flags
        // Critical) +1 (4) → 11, then 2 terminates.
        let mut q = QueueRng(vec![6, 5, 4, 6, 6, 4, 2]);
        let r = o6(&mut q);
        assert_eq!(r.total, 11);
        assert_eq!(r.outcome, Outcome::Critical);
        assert_eq!(r.sequence, vec![6, 5, 4, 6, 6, 4, 2]);
    }

    #[test]
    fn one_then_stop_high_extends_normal() {
        // 1, 2, 4 -> 1 - 1 (for the 2) = 0, stop on the 4
        let mut q = QueueRng(vec![1, 2, 4]);
        let r = o6(&mut q);
        assert_eq!(r.total, 0);
        assert_eq!(r.outcome, Outcome::Normal);
    }

    #[test]
    fn double_one_sets_fumble_flag_then_cascade_continues() {
        // 1, 1, 4 -> snake-eyes flag Fumble, second 1 still gives
        // -1 (it's a 1/2/3 result), then 4 terminates.
        // Total 1 - 1 = 0, outcome Fumble.
        let mut q = QueueRng(vec![1, 1, 4]);
        let r = o6(&mut q);
        assert_eq!(r.outcome, Outcome::Fumble);
        assert_eq!(r.total, 0);
        assert_eq!(r.sequence, vec![1, 1, 4]);
    }

    #[test]
    fn fumble_extends_through_cascade() {
        // User example: 1,1,2,3,1,1,2,1,6 -> -6 Fumble.
        // 1 (initial) -1 (1, snake-eyes flags Fumble) -1 (2) -1 (3)
        // -1 (1) -1 (1, second snake-eyes; flag stays) -1 (2) -1 (1)
        // → -6, then 6 terminates.
        let mut q = QueueRng(vec![1, 1, 2, 3, 1, 1, 2, 1, 6]);
        let r = o6(&mut q);
        assert_eq!(r.total, -6);
        assert_eq!(r.outcome, Outcome::Fumble);
        assert_eq!(r.sequence, vec![1, 1, 2, 3, 1, 1, 2, 1, 6]);
    }

    #[test]
    fn check_dr_pass_and_fail() {
        // Skill 5, DR 10. O6 = 5 -> 10 total -> success at exactly DR.
        let mut q = QueueRng(vec![5]);
        let res = check(&mut q, 5, 10);
        assert!(res.success);
        // Skill 5, DR 11. O6 = 5 -> 10 total -> fail.
        let mut q = QueueRng(vec![5]);
        let res = check(&mut q, 5, 11);
        assert!(!res.success);
    }

    #[test]
    fn critical_always_succeeds_fumble_always_fails() {
        // Skill 0, DR 100, but 6,6 (with terminator) -> still success.
        let mut q = QueueRng(vec![6, 6, 2]);
        let res = check(&mut q, 0, 100);
        assert!(res.success);
        // Skill 100, DR 0, but 1,1 (with terminator) -> still fail.
        let mut q = QueueRng(vec![1, 1, 4]);
        let res = check(&mut q, 100, 0);
        assert!(!res.success);
    }
}
