use rust_extensions::date_time::{DateTimeAsMicroseconds, DateTimeStruct, TimeStruct};

pub const SECONDS_PER_DAY: i64 = 24 * 60 * 60;

const MICROS_PER_SECOND: i64 = 1_000_000;

/// A UTC day as `YYYYMMDD` - what the day files are named by
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DayKey(u32);

impl DayKey {
    /// The day the unix second falls into
    pub fn of(unix_seconds: i64) -> Self {
        let date: DateTimeStruct =
            DateTimeAsMicroseconds::new(unix_seconds * MICROS_PER_SECOND).into();

        Self(date.year as u32 * 10_000 + date.month * 100 + date.day)
    }

    /// The unix second the day starts at
    pub fn start(&self) -> i64 {
        // A key is made of a real date only - by `of`, or by `parse`, which checks it
        self.start_or_none().unwrap()
    }

    fn start_or_none(&self) -> Option<i64> {
        let date = DateTimeStruct {
            year: (self.0 / 10_000) as i32,
            month: self.0 / 100 % 100,
            day: self.0 % 100,
            time: TimeStruct {
                hour: 0,
                min: 0,
                sec: 0,
                micros: 0,
            },
            dow: None,
        };

        Some(date.to_unix_microseconds()? / MICROS_PER_SECOND)
    }

    /// The days the seconds `from .. to` are in - `to` itself is not one of them
    pub fn range(from: i64, to: i64) -> Vec<Self> {
        let mut result = Vec::new();

        if to <= from {
            return result;
        }

        let last = Self::of(to - 1);
        let mut day = Self::of(from);

        while day <= last {
            result.push(day);
            day = Self::of(day.start() + SECONDS_PER_DAY);
        }

        result
    }

    /// `None` when it is not eight digits of a date there is
    pub fn parse(src: &str) -> Option<Self> {
        if src.len() != 8 || !src.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }

        let result = Self(src.parse().ok()?);
        result.start_or_none()?;
        Some(result)
    }
}

impl std::fmt::Display for DayKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:08}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-10-12T00:00:00Z
    const OCT_12: i64 = 1_791_763_200;

    #[test]
    fn a_second_falls_into_its_utc_day() {
        assert_eq!(DayKey::of(OCT_12).to_string(), "20261012");
        assert_eq!(DayKey::of(OCT_12 + SECONDS_PER_DAY - 1).to_string(), "20261012");
        assert_eq!(DayKey::of(OCT_12 - 1).to_string(), "20261011");
        assert_eq!(DayKey::of(OCT_12).start(), OCT_12);
    }

    #[test]
    fn a_range_names_every_day_it_touches_and_not_the_one_it_ends_at() {
        let days = |from, to| -> Vec<String> {
            DayKey::range(from, to)
                .iter()
                .map(|day| day.to_string())
                .collect()
        };

        assert_eq!(days(OCT_12, OCT_12 + SECONDS_PER_DAY), ["20261012"]);
        assert_eq!(
            days(OCT_12 - 1, OCT_12 + SECONDS_PER_DAY + 1),
            ["20261011", "20261012", "20261013"]
        );
        assert_eq!(days(OCT_12 + 10, OCT_12 + 11), ["20261012"]);
        assert!(days(OCT_12, OCT_12).is_empty());
        assert!(days(OCT_12, OCT_12 - 5).is_empty());
    }

    #[test]
    fn a_range_goes_over_the_end_of_a_month() {
        let oct_31 = DayKey::parse("20261031").unwrap().start();

        let days: Vec<String> = DayKey::range(oct_31, oct_31 + 2 * SECONDS_PER_DAY)
            .iter()
            .map(|day| day.to_string())
            .collect();

        assert_eq!(days, ["20261031", "20261101"]);
    }

    #[test]
    fn only_a_real_date_is_a_day() {
        assert_eq!(DayKey::parse("20261012"), Some(DayKey::of(OCT_12)));
        assert_eq!(DayKey::parse("20261301"), None);
        assert_eq!(DayKey::parse("20260230"), None);
        assert_eq!(DayKey::parse("2026101"), None);
        assert_eq!(DayKey::parse("2026-10-12"), None);
        assert_eq!(DayKey::parse("+2026101"), None);
    }
}
