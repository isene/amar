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
    if first == 6 {
        let mut total: i32 = 6;
        let mut prev = 6u8;
        loop {
            let r = rng.d6();
            sequence.push(r);
            if prev == 6 && r == 6 {
                total += 1;
                return Roll { total, outcome: Outcome::Critical, sequence };
            }
            match r {
                4..=6 => total += 1,
                _ => return Roll { total, outcome: Outcome::Normal, sequence },
            }
            prev = r;
        }
    }
    let mut total: i32 = 1;
    let mut prev = 1u8;
    loop {
        let r = rng.d6();
        sequence.push(r);
        if prev == 1 && r == 1 {
            total -= 1;
            return Roll { total, outcome: Outcome::Fumble, sequence };
        }
        match r {
            1..=3 => total -= 1,
            _ => return Roll { total, outcome: Outcome::Normal, sequence },
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
    }

    #[test]
    fn double_six_critical() {
        // 6, 6 anywhere -> Critical
        let mut q = QueueRng(vec![6, 6]);
        let r = o6(&mut q);
        assert_eq!(r.outcome, Outcome::Critical);
        assert_eq!(r.total, 7);
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
    fn double_one_fumble() {
        // 1, 1 anywhere -> Fumble
        let mut q = QueueRng(vec![1, 1]);
        let r = o6(&mut q);
        assert_eq!(r.outcome, Outcome::Fumble);
        assert_eq!(r.total, 0);
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
        // Skill 0, DR 100, but 6,6 -> still success.
        let mut q = QueueRng(vec![6, 6]);
        let res = check(&mut q, 0, 100);
        assert!(res.success);
        // Skill 100, DR 0, but 1,1 -> still fail.
        let mut q = QueueRng(vec![1, 1]);
        let res = check(&mut q, 100, 0);
        assert!(!res.success);
    }
}
