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

    pub fn advance(&self, days: i64) -> Self {
        let total = self.year as i64 * DAYS_PER_YEAR as i64 + self.day_of_year as i64 + days;
        let year = (total.div_euclid(DAYS_PER_YEAR as i64)) as u32;
        let doy = (total.rem_euclid(DAYS_PER_YEAR as i64)) as u32;
        Self { year, day_of_year: doy }
    }

    /// Header format requested by the user:
    ///   `Liandra, DeIelina (28) Juba — Year-354`
    /// Day-name, week-name, day-of-month in parens, month-name, year.
    pub fn fmt_header(&self) -> String {
        format!("{}, {} ({}) {} - Year-{}",
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
    fn user_example_header_formats_correctly() {
        // The user specified the header should read:
        // "Liandra, DeIelina (28) Juba - Year-354"
        // Liandra is day-of-week 7; DeIelina is week 4; Juba is month 6;
        // day-of-month 28 puts us at month_offset = 5 * 28 + 27 = 167.
        let d = AmarDate::from_ymd(354, 6, 28);
        assert_eq!(d.day_name(), "Liandra");
        assert_eq!(d.week_name(), "DeIelina");
        assert_eq!(d.month_name(), "Juba");
        assert_eq!(d.fmt_header(), "Liandra, DeIelina (28) Juba - Year-354");
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
