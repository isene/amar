//! Combat tab state machine — Amar RPG rules engine driving the
//! per-participant initiative + status + roll bookkeeping.
//!
//! `CombatState` is the whole-fight snapshot persisted on the
//! `Campaign`. The Combat tab renders cards from `participants` and
//! a roll log from `log`. All status modifiers are summed in
//! `effective_modifier()` so the o/d/D rolls have a single source of
//! truth; auto-derived statuses (Half-Action / Quarter-Action from
//! BP threshold, Endurance drain from round count) are computed on
//! the fly against the live `Character` rather than stored, so a
//! manual BP edit takes effect on the very next roll.
//!
//! Cross-source tagging — the pool the GM builds up by pressing `t`
//! on any encounter / NPC / monster row — lives in `TagPool` on the
//! same Campaign. Press `C` from anywhere with `tagged.is_empty() ==
//! false` to launch combat populated from the pool + all PCs.

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

use crate::store::CombatRef;

/// Whole-fight state. `round` starts at 1 the moment the first
/// initiative is rolled (or the tab is opened from a tag pool). `log`
/// is capped at 100 entries — older rolls get truncated on push so
/// the right pane render stays cheap.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CombatState {
    #[serde(default = "default_round")]
    pub round: u32,
    #[serde(default)]
    pub lighting: Lighting,
    #[serde(default)]
    pub participants: Vec<Participant>,
    #[serde(default)]
    pub selected: usize,
    #[serde(default)]
    pub log: Vec<RollEntry>,
}

fn default_round() -> u32 { 1 }

impl CombatState {
    pub fn new() -> Self { Self { round: 1, ..Self::default() } }

    /// Cap log at 100 entries. Roll log is append-only during a fight
    /// but the GM rarely scrolls back beyond the current round.
    pub fn push_log(&mut self, entry: RollEntry) {
        self.log.push(entry);
        if self.log.len() > 100 {
            let drop = self.log.len() - 100;
            self.log.drain(0..drop);
        }
    }
}

/// One combatant. References into `Campaign.pcs` / `.npcs` via
/// `r: CombatRef` so we don't duplicate base stats — the live
/// Character row is the source of truth. Combat overlay data
/// (weapon choice, statuses, init result for this round) lives here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub r: CombatRef,
    /// Initiative with current weapon: O6 + WeaponIni + ReactionSpeed − Status.
    /// `None` until rolled this round; cleared at `r` (next round).
    #[serde(default)]
    pub init: Option<i32>,
    /// Same roll but without the weapon's I — what counts if this
    /// participant decides to do something other than attack.
    #[serde(default)]
    pub init_noweapon: Option<i32>,
    /// Index into the live Character's weapons list. Defaults to 0.
    #[serde(default)]
    pub selected_weapon: usize,
    /// GM-toggled statuses (Stunned / Unaware / etc.). Independent
    /// of round counter; cleared explicitly via the `s` menu.
    #[serde(default)]
    pub manual_statuses: Vec<ManualStatus>,
    /// Crit / fumble effects with a round counter. Tick down on
    /// `r`; removed when `rounds_left` hits 0.
    #[serde(default)]
    pub timed_statuses: Vec<TimedStatus>,
    /// Encumbrance tier: 0 → 0, 1 → −1, 2 → −3, 3 → −5.
    #[serde(default)]
    pub encumbrance_tier: u8,
    /// Movement-this-turn. Reset to `None` on `r` (next round).
    #[serde(default)]
    pub movement: Movement,
    /// "Doing something other than attack" — affects which init
    /// total is used for sorting. Reset on `r`.
    #[serde(default)]
    pub non_attack: bool,
    /// Arbitrary GM-edited combat-only field overrides. Lets the
    /// user edit any of the displayed numbers (BP, MD, Awareness,
    /// etc.) without touching the underlying Character sheet.
    /// Looked up by short field key (`"bp"`, `"md"`, `"awr"`, …).
    #[serde(default)]
    pub overrides: BTreeMap<String, i32>,
}

impl Participant {
    pub fn new(r: CombatRef) -> Self {
        Self {
            r, init: None, init_noweapon: None,
            selected_weapon: 0,
            manual_statuses: Vec::new(),
            timed_statuses: Vec::new(),
            encumbrance_tier: 0,
            movement: Movement::None,
            non_attack: false,
            overrides: BTreeMap::new(),
        }
    }
}

/// Global lighting condition — applies to every participant's rolls.
/// Modifiers cribbed from the wiki "Defender's condition penalties"
/// table (indexb6e4.html).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Lighting {
    #[default]
    Bright,
    /// Twilight / torchlight: −1 Off, −2 Def.
    Twilight,
    /// Moonlight (full moon): −2 Off, −4 Def.
    Moonlight,
    /// Starlight: −3 Off, −6 Def.
    Starlight,
}

impl Lighting {
    pub fn label(self) -> &'static str {
        match self {
            Lighting::Bright    => "bright",
            Lighting::Twilight  => "twilight",
            Lighting::Moonlight => "moonlight",
            Lighting::Starlight => "starlight",
        }
    }
    pub fn off_def(self) -> (i32, i32) {
        match self {
            Lighting::Bright    => (0, 0),
            Lighting::Twilight  => (-1, -2),
            Lighting::Moonlight => (-2, -4),
            Lighting::Starlight => (-3, -6),
        }
    }
    pub fn next(self) -> Lighting {
        match self {
            Lighting::Bright    => Lighting::Twilight,
            Lighting::Twilight  => Lighting::Moonlight,
            Lighting::Moonlight => Lighting::Starlight,
            Lighting::Starlight => Lighting::Bright,
        }
    }
}

/// Per-turn movement choice. Off/Def penalties from the wiki
/// "Movement in Combat" table.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Movement {
    #[default]
    None,
    /// Run x2: −10 Off, −5 Def.
    Run,
    /// Move x1: −5 Off, −2 Def.
    Move,
    /// Move & Fight x½: −3 Off, −1 Def.
    MoveAndFight,
    /// Move in Melee x¼: −1 Off, −1 Def.
    MoveInMelee,
    /// Disengage x1: no Off, −5 Def.
    Disengage,
}

impl Movement {
    pub fn label(self) -> &'static str {
        match self {
            Movement::None         => "—",
            Movement::Run          => "Run",
            Movement::Move         => "Move",
            Movement::MoveAndFight => "Move&Fight",
            Movement::MoveInMelee  => "Move(melee)",
            Movement::Disengage    => "Disengage",
        }
    }
    /// (off_mod, def_mod). Disengage returns (i32::MIN, -5) to
    /// signal "no attack" via the off_mod; callers should check.
    pub fn off_def(self) -> (i32, i32) {
        match self {
            Movement::None         => (0, 0),
            Movement::Run          => (-10, -5),
            Movement::Move         => (-5, -2),
            Movement::MoveAndFight => (-3, -1),
            Movement::MoveInMelee  => (-1, -1),
            Movement::Disengage    => (i32::MIN, -5),
        }
    }
    pub fn next(self) -> Movement {
        match self {
            Movement::None         => Movement::Run,
            Movement::Run          => Movement::Move,
            Movement::Move         => Movement::MoveAndFight,
            Movement::MoveAndFight => Movement::MoveInMelee,
            Movement::MoveInMelee  => Movement::Disengage,
            Movement::Disengage    => Movement::None,
        }
    }
}

/// GM-toggled situational status. Off/Def per wiki "Defender's
/// condition penalties". Move penalties not stored here — they're
/// looked up via `off_def_move()` when relevant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManualStatus {
    PartiallyUnaware,
    Stunned,
    Unaware,
    Immobilized,
}

impl ManualStatus {
    pub fn label(self) -> &'static str {
        match self {
            ManualStatus::PartiallyUnaware => "Partially unaware",
            ManualStatus::Stunned          => "Stunned",
            ManualStatus::Unaware          => "Unaware",
            ManualStatus::Immobilized      => "Immobilized",
        }
    }
    pub fn off_def(self) -> (i32, i32) {
        match self {
            ManualStatus::PartiallyUnaware => (0, -5),
            ManualStatus::Stunned          => (-3, -3),
            // "X" Off means cannot attack — encoded as i32::MIN
            // so the o-roll path can detect and decline.
            ManualStatus::Unaware          => (i32::MIN, -10),
            ManualStatus::Immobilized      => (i32::MIN, -5),
        }
    }
    pub fn all() -> [ManualStatus; 4] {
        [ManualStatus::PartiallyUnaware, ManualStatus::Stunned,
         ManualStatus::Unaware, ManualStatus::Immobilized]
    }
}

/// Round-counted effect — typically from a crit (penalty on opponent)
/// or fumble (penalty on self), but the GM can add free-form ones via
/// the `s` menu's "Custom" option.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimedStatus {
    pub label: String,
    #[serde(default)]
    pub off: i32,
    #[serde(default)]
    pub def: i32,
    #[serde(default)]
    pub dam: i32,
    /// Rounds remaining. Ticked down on `r`; removed at 0.
    pub rounds_left: u32,
}

/// One roll on the log. The breakdown (`base + status_mod + o6`) is
/// stored so the render can show why the total is what it is.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollEntry {
    pub round: u32,
    pub name: String,
    pub weapon: Option<String>,
    pub kind: RollKind,
    /// The exploding-d6 result added to the roll.
    pub o6: i32,
    /// Base value before status: weapon mod (off/def/dam) for o/d/D,
    /// or `WeaponIni + ReactionSpeed` for Init.
    pub base: i32,
    /// Net Status modifier applied to this roll.
    pub status_mod: i32,
    pub total: i32,
    /// Optional flavour text: "Critical!", "Fumble!", "(no-weapon
    /// init: 11)".
    #[serde(default)]
    pub extra: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RollKind { Init, Off, Def, Dam }

impl RollKind {
    pub fn short(self) -> &'static str {
        match self {
            RollKind::Init => "i",
            RollKind::Off  => "o",
            RollKind::Def  => "d",
            RollKind::Dam  => "D",
        }
    }
}

/// Cross-source tag pool. Filled by the `t` toggle across any
/// browsable view (Campaign PCs, NPCs, …); drained when `C` launches
/// a combat. Persisted on the Campaign so the pool survives a quit.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TagPool {
    #[serde(default)]
    pub refs: Vec<CombatRef>,
}

impl TagPool {
    pub fn contains(&self, r: &CombatRef) -> bool { self.refs.contains(r) }

    /// Toggle: insert if absent, remove if present. Returns the new
    /// membership state (true = now tagged).
    pub fn toggle(&mut self, r: CombatRef) -> bool {
        if let Some(pos) = self.refs.iter().position(|x| *x == r) {
            self.refs.remove(pos);
            false
        } else {
            self.refs.push(r);
            true
        }
    }

    pub fn clear(&mut self) { self.refs.clear(); }
    pub fn len(&self) -> usize { self.refs.len() }
    pub fn is_empty(&self) -> bool { self.refs.is_empty() }
}

/// Encumbrance tier → status modifier. Wiki rule:
///   ≤ 2× Strength → 0, ≤ 5× → −1, ≤ 10× → −3, ≤ 20× → −5.
pub fn encumbrance_modifier(tier: u8) -> i32 {
    match tier {
        0 => 0,
        1 => -1,
        2 => -3,
        _ => -5,
    }
}

/// Endurance-drain tier in effect at `round` for a participant with
/// the given endurance score. Wiki rule: −1 every `endurance` rounds.
/// Endurance ≤ 0 short-circuits to 0 — degenerate sheets shouldn't
/// stack infinite drain.
pub fn endurance_drain_tier(round: u32, endurance: i32) -> i32 {
    if endurance <= 0 { return 0; }
    (round as i32 - 1) / endurance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tagpool_toggle_round_trip() {
        let mut p = TagPool::default();
        let r = CombatRef::Pc(3);
        assert!(p.toggle(r));   // added
        assert!(p.contains(&r));
        assert!(!p.toggle(r));  // removed
        assert!(!p.contains(&r));
    }

    #[test]
    fn endurance_drain_increments_every_n_rounds() {
        // Endurance 4 → −1 starting round 5, −2 round 9, etc.
        assert_eq!(endurance_drain_tier(1, 4), 0);
        assert_eq!(endurance_drain_tier(4, 4), 0);
        assert_eq!(endurance_drain_tier(5, 4), 1);
        assert_eq!(endurance_drain_tier(8, 4), 1);
        assert_eq!(endurance_drain_tier(9, 4), 2);
    }

    #[test]
    fn encumbrance_tiers_match_wiki() {
        assert_eq!(encumbrance_modifier(0),  0);
        assert_eq!(encumbrance_modifier(1), -1);
        assert_eq!(encumbrance_modifier(2), -3);
        assert_eq!(encumbrance_modifier(3), -5);
    }

    #[test]
    fn lighting_cycle() {
        let mut l = Lighting::Bright;
        l = l.next(); assert_eq!(l, Lighting::Twilight);
        l = l.next(); assert_eq!(l, Lighting::Moonlight);
        l = l.next(); assert_eq!(l, Lighting::Starlight);
        l = l.next(); assert_eq!(l, Lighting::Bright);
    }
}
