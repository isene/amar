//! The Amar calendar — 13 months, 4 weeks per month, 7 days per week.
//! Year length: 13 × 4 × 7 = 364 days. Source: d6gaming.org/Campaign_tracker.

pub const MONTHS: [&str; 13] = [
    "Cal Amae", "Elesi", "Anashina", "Gwendyll", "MacGillan", "Juba", "Taroc",
    "Man Peggon", "Maleko", "Fal Munir", "Moltan", "Kraagh", "Mestronorpha",
];

pub const WEEKS: [&str; 4] = ["InIelina", "UrIelina", "AlIelina", "DeIelina"];

pub const DAYS: [&str; 7] = [
    "Recolar", "Mailatroz", "Ztenasi", "Staari",
    "Tsankili", "Fooradur", "Liandra",
];

pub const DAYS_PER_MONTH: u32 = 28;
pub const DAYS_PER_YEAR: u32 = 364;

/// Colour for each month name (index = month - 1), matching that month-god's
/// colour on the Gods-of-Amar wheel/graph (d6gaming.org).
pub const MONTH_COLORS: [u8; 13] = [
    231, 254, 34, 200, 99, 202, 250, 130, 179, 220, 214, 244, 240,
];

/// The colour of month `m` (1-13) — its god's colour.
pub fn month_color(m: u32) -> u8 {
    MONTH_COLORS[(m.clamp(1, 13) - 1) as usize]
}

/// A holy / special day: a specific date honouring a god. Colours from the
/// Gods-of-Amar wheel/graph; dates, domains and effects from the Mythology
/// wiki (Overview of the Gods and their Worshipping).
pub struct SpecialDay {
    pub month: u32, // 1-13
    pub day: u32,   // 1-28
    pub god: &'static str,
    /// xterm-256 colour for the day's background.
    pub color: u8,
    /// Contrasting foreground on that background.
    pub text: u8,
    pub domain: &'static str,
    /// The power/skill the god grants its followers on this day.
    pub power: &'static str,
}

pub const SPECIAL_DAYS: &[SpecialDay] = &[
    // ── The thirteen month-gods (each month is named after one) ──
    SpecialDay { month: 1,  day: 9,  god: "Cal Amae",     color: 231, text: 16,  domain: "Good Deeds",            power: "Melee Defense" },
    SpecialDay { month: 2,  day: 2,  god: "Elesi",        color: 254, text: 16,  domain: "Creation & Art",        power: "Life Magick" },
    SpecialDay { month: 3,  day: 4,  god: "Anashina",     color: 34,  text: 231, domain: "Nature",                power: "Missile Weapons" },
    SpecialDay { month: 4,  day: 12, god: "Gwendyll",     color: 200, text: 16,  domain: "Queen of the Gods",     power: "Social Skills" },
    SpecialDay { month: 5,  day: 13, god: "MacGillan",    color: 99,  text: 231, domain: "King of the Gods",      power: "Leadership" },
    SpecialDay { month: 6,  day: 10, god: "Juba",         color: 202, text: 231, domain: "Entertainment",         power: "Music & Dance" },
    SpecialDay { month: 7,  day: 11, god: "Taroc",        color: 250, text: 16,  domain: "War",                   power: "Melee Skills" },
    SpecialDay { month: 8,  day: 5,  god: "Man Peggon",   color: 130, text: 231, domain: "Strength",              power: "Strength" },
    SpecialDay { month: 9,  day: 1,  god: "Maleko",       color: 179, text: 16,  domain: "Inner Strength",        power: "Endurance" },
    SpecialDay { month: 10, day: 7,  god: "Fal Munir",    color: 220, text: 16,  domain: "Knowledge & Wisdom",    power: "Learning" },
    SpecialDay { month: 11, day: 3,  god: "Moltan",       color: 214, text: 16,  domain: "Judgement",             power: "Awareness" },
    SpecialDay { month: 12, day: 8,  god: "Kraagh",       color: 244, text: 16,  domain: "Death & Reincarnation", power: "Black Magick" },
    SpecialDay { month: 13, day: 6,  god: "Mestronorpha", color: 240, text: 231, domain: "Evil Deeds",            power: "Black Magick" },
    // ── The five elemental gods (their days fall at the quarters of the year) ──
    SpecialDay { month: 1,  day: 1,  god: "Walmaer",      color: 19,  text: 231, domain: "Water & Seas",          power: "Water Magick" },
    SpecialDay { month: 4,  day: 8,  god: "Alesia",       color: 88,  text: 231, domain: "Earth",                 power: "Earth & Protection Magick" },
    SpecialDay { month: 7,  day: 15, god: "Ikalio",       color: 208, text: 16,  domain: "Fire & Sun",            power: "Fire Magick" },
    SpecialDay { month: 10, day: 22, god: "Shalissa",     color: 117, text: 16,  domain: "Wind & Freedom",        power: "Air Magick" },
    SpecialDay { month: 13, day: 28, god: "Ielina",       color: 189, text: 16,  domain: "Moon & Time",           power: "Perception Magick" },
];

/// The special day on a given date (month 1-13, day 1-28), if any.
pub fn special_day(month: u32, day: u32) -> Option<&'static SpecialDay> {
    SPECIAL_DAYS.iter().find(|s| s.month == month && s.day == day)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AmarDate {
    pub year: u32,
    pub day_of_year: u32,
}

impl AmarDate {
    pub fn new(year: u32, day_of_year: u32) -> Self {
        Self { year, day_of_year }
    }

    pub fn from_ymd(year: u32, month: u32, day_of_month: u32) -> Self {
        let m = month.clamp(1, 13);
        let d = day_of_month.clamp(1, DAYS_PER_MONTH);
        Self { year, day_of_year: (m - 1) * DAYS_PER_MONTH + (d - 1) }
    }

    pub fn month(&self) -> u32 { self.day_of_year / DAYS_PER_MONTH + 1 }

    pub fn day_of_month(&self) -> u32 { self.day_of_year % DAYS_PER_MONTH + 1 }

    pub fn week_of_month(&self) -> u32 { (self.day_of_month() - 1) / 7 + 1 }

    pub fn day_of_week(&self) -> u32 { (self.day_of_year % 7) + 1 }

    pub fn month_name(&self) -> &'static str { MONTHS[(self.month() - 1) as usize] }

    pub fn week_name(&self) -> &'static str { WEEKS[(self.week_of_month() - 1) as usize] }

    pub fn day_name(&self) -> &'static str { DAYS[(self.day_of_week() - 1) as usize] }

    /// The special (holy) day falling on this exact date, if any.
    pub fn special_day(&self) -> Option<&'static SpecialDay> {
        special_day(self.month(), self.day_of_month())
    }

    pub fn advance(&self, days: i64) -> Self {
        let total = self.year as i64 * DAYS_PER_YEAR as i64 + self.day_of_year as i64 + days;
        let year = (total.div_euclid(DAYS_PER_YEAR as i64)) as u32;
        let doy = (total.rem_euclid(DAYS_PER_YEAR as i64)) as u32;
        Self { year, day_of_year: doy }
    }

    /// Compact header date: `15 Juba, Year 354`. Used everywhere we
    /// show the current date (top header, Campaign sub-titles, the
    /// Calendar section). Day-name and week-name are dropped — they
    /// add visual noise without much information for the GM.
    pub fn fmt_header(&self) -> String {
        format!("{} {}, Year {}",
            self.day_of_month(), self.month_name(), self.year)
    }

    /// Full ceremonial date: `Liandra, DeIelina (15) Juba, Year 354`.
    /// Available for the Calendar section's detail view and any
    /// future "long form" usage.
    pub fn fmt_long(&self) -> String {
        format!("{}, {} ({}) {}, Year {}",
            self.day_name(),
            self.week_name(),
            self.day_of_month(),
            self.month_name(),
            self.year)
    }
}

impl Default for AmarDate {
    fn default() -> Self { Self::from_ymd(354, 1, 1) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_one_is_recolar_inielina_cal_amae() {
        let d = AmarDate::from_ymd(354, 1, 1);
        assert_eq!(d.day_name(), "Recolar");
        assert_eq!(d.week_name(), "InIelina");
        assert_eq!(d.month_name(), "Cal Amae");
        assert_eq!(d.day_of_month(), 1);
    }

    #[test]
    fn day_28_is_in_week_4() {
        let d = AmarDate::from_ymd(354, 1, 28);
        assert_eq!(d.week_name(), "DeIelina");
        assert_eq!(d.day_of_month(), 28);
    }

    #[test]
    fn header_short_form_is_day_month_year() {
        let d = AmarDate::from_ymd(354, 6, 15);
        assert_eq!(d.fmt_header(), "15 Juba, Year 354");
    }

    #[test]
    fn long_form_keeps_day_and_week_names() {
        let d = AmarDate::from_ymd(354, 6, 28);
        assert_eq!(d.day_name(), "Liandra");
        assert_eq!(d.week_name(), "DeIelina");
        assert_eq!(d.month_name(), "Juba");
        assert_eq!(d.fmt_long(), "Liandra, DeIelina (28) Juba, Year 354");
    }

    #[test]
    fn advance_wraps_year() {
        let d = AmarDate::from_ymd(354, 13, 28); // last day of year 354
        let next = d.advance(1);
        assert_eq!(next.year, 355);
        assert_eq!(next.day_of_year, 0);
    }

    #[test]
    fn advance_negative_wraps_back() {
        let d = AmarDate::from_ymd(354, 1, 1);
        let prev = d.advance(-1);
        assert_eq!(prev.year, 353);
        assert_eq!(prev.day_of_year, DAYS_PER_YEAR - 1);
    }
}
