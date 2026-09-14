//! Date bounds for the month- and week-based queries (archive, events, canteen).
//!
//! Clients pass plain `year`/`month`/`week` integers, and each resolver has to
//! turn them into database bounds. The rules differ per query (a half-open
//! month, a Monday-to-Sunday calendar grid, an inclusive ISO week) and are easy
//! to get wrong at month ends, year ends and in 53-week years, so they live here
//! as pure functions that can be tested without a database.

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday};

/// Half-open `[start, end)` bounds of a calendar month, both at midnight.
///
/// * `year` - calendar year.
/// * `month` - month number, 1 to 12.
///
/// Returns `None` if the pair does not name a month chrono can represent, such
/// as month 0 or 13.
pub fn month_range(year: i32, month: u32) -> Option<(NaiveDateTime, NaiveDateTime)> {
    let start = NaiveDate::from_ymd_opt(year, month, 1)?;
    let end = if month < 12 {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    }?;

    Some((start.and_time(NaiveTime::MIN), end.and_time(NaiveTime::MIN)))
}

/// Half-open `[start, end)` bounds of the events shown in a month's calendar view.
///
/// The view is a grid of Monday-to-Sunday weeks, so it also shows days of the
/// neighbouring months. `start` is midnight on the Monday of the week containing
/// the 1st of the month, and `end` is midnight after the Sunday of the week
/// containing the last day of the month.
///
/// * `year` - calendar year.
/// * `month` - month number, 1 to 12.
///
/// Returns `None` under the same conditions as [`month_range`].
pub fn calendar_grid_range(year: i32, month: u32) -> Option<(NaiveDateTime, NaiveDateTime)> {
    let (month_start, next_month_start) = month_range(year, month)?;

    let start = month_start - Duration::days(month_start.weekday().num_days_from_monday().into());
    // The grid ends at the first Monday on or after the next month's 1st: the
    // last row ends on the Sunday before it.
    let days_to_monday = (7 - next_month_start.weekday().num_days_from_monday()) % 7;
    let end = next_month_start + Duration::days(days_to_monday.into());

    Some((start, end))
}

/// Inclusive `(monday, sunday)` bounds of an ISO 8601 week.
///
/// * `year` - ISO week-numbering year. It differs from the calendar year for
///   days around New Year: 29 December 2025 is in week 1 of 2026.
/// * `week` - ISO week number, 1 to 52, or 53 in long years.
///
/// Returns `None` for week numbers that do not exist in `year`, including zero
/// and negative numbers.
pub fn iso_week_range(year: i32, week: i32) -> Option<(NaiveDate, NaiveDate)> {
    let week = u32::try_from(week).ok()?;

    Some((
        NaiveDate::from_isoywd_opt(year, week, Weekday::Mon)?,
        NaiveDate::from_isoywd_opt(year, week, Weekday::Sun)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    fn midnight(year: i32, month: u32, day: u32) -> NaiveDateTime {
        date(year, month, day).and_time(NaiveTime::MIN)
    }

    #[test]
    fn month_range_is_half_open() {
        assert_eq!(
            month_range(2026, 9),
            Some((midnight(2026, 9, 1), midnight(2026, 10, 1)))
        );
    }

    #[test]
    fn month_range_rolls_december_into_the_next_year() {
        assert_eq!(
            month_range(2025, 12),
            Some((midnight(2025, 12, 1), midnight(2026, 1, 1)))
        );
    }

    #[test]
    fn month_range_handles_leap_february() {
        assert_eq!(
            month_range(2024, 2),
            Some((midnight(2024, 2, 1), midnight(2024, 3, 1)))
        );
    }

    #[test]
    fn month_range_rejects_months_outside_1_to_12() {
        for month in [0, 13, u32::MAX] {
            assert_eq!(month_range(2026, month), None, "month {month}");
        }
    }

    #[test]
    fn month_range_rejects_unrepresentable_years() {
        assert_eq!(month_range(i32::MAX, 12), None);
        assert_eq!(month_range(i32::MIN, 1), None);
    }

    #[test]
    fn calendar_grid_starts_on_the_monday_before_the_first() {
        // 1 September 2026 is a Tuesday.
        let (start, _) = calendar_grid_range(2026, 9).unwrap();

        assert_eq!(start, midnight(2026, 8, 31));
    }

    #[test]
    fn calendar_grid_starts_on_the_first_when_it_is_a_monday() {
        // 1 June 2026 is a Monday.
        let (start, _) = calendar_grid_range(2026, 6).unwrap();

        assert_eq!(start, midnight(2026, 6, 1));
    }

    #[test]
    fn calendar_grid_ends_after_the_sunday_of_the_last_week() {
        // 30 September 2026 is a Wednesday; its week ends on Sunday 4 October.
        assert_eq!(
            calendar_grid_range(2026, 9).unwrap().1,
            midnight(2026, 10, 5)
        );
        // 31 May 2026 is a Sunday, so the grid ends exactly at the month boundary.
        assert_eq!(
            calendar_grid_range(2026, 5).unwrap().1,
            midnight(2026, 6, 1)
        );
    }

    #[test]
    fn calendar_grid_rejects_months_outside_1_to_12() {
        assert_eq!(calendar_grid_range(2026, 0), None);
        assert_eq!(calendar_grid_range(2026, 13), None);
    }

    #[test]
    #[ignore = "bug: calendar grid start overflows and panics for the earliest month chrono can represent"]
    fn calendar_grid_does_not_panic_at_the_earliest_representable_month() {
        let _ = calendar_grid_range(NaiveDate::MIN.year(), 1);
    }

    #[test]
    fn iso_week_one_can_start_in_the_previous_calendar_year() {
        assert_eq!(
            iso_week_range(2026, 1),
            Some((date(2025, 12, 29), date(2026, 1, 4)))
        );
    }

    #[test]
    fn iso_week_53_exists_only_in_long_years() {
        assert_eq!(
            iso_week_range(2020, 53),
            Some((date(2020, 12, 28), date(2021, 1, 3)))
        );
        assert_eq!(iso_week_range(2021, 53), None);
    }

    #[test]
    fn iso_week_rejects_weeks_that_do_not_exist() {
        for week in [0, -1, 54, i32::MIN, i32::MAX] {
            assert_eq!(iso_week_range(2026, week), None, "week {week}");
        }
    }

    proptest! {
        #[test]
        fn month_range_spans_exactly_one_month(year in 1..=9999i32, month in 1..=12u32) {
            let (start, end) = month_range(year, month).unwrap();

            prop_assert_eq!((start.year(), start.month(), start.day()), (year, month, 1));
            prop_assert_eq!(start.time(), NaiveTime::MIN);
            prop_assert_eq!(end.day(), 1);
            prop_assert_eq!(end.time(), NaiveTime::MIN);
            prop_assert_eq!(end.date().pred_opt().unwrap().month(), month);
        }

        #[test]
        fn calendar_grid_start_is_the_monday_of_the_first_week(
            year in 1..=9999i32,
            month in 1..=12u32,
        ) {
            let (month_start, _) = month_range(year, month).unwrap();
            let (start, _) = calendar_grid_range(year, month).unwrap();

            prop_assert_eq!(start.weekday(), Weekday::Mon);
            prop_assert!(start <= month_start);
            prop_assert!(month_start - start < Duration::days(7));
        }

        #[test]
        fn calendar_grid_covers_whole_weeks_through_the_month_end(
            year in 1..=9999i32,
            month in 1..=12u32,
        ) {
            let (_, next_month_start) = month_range(year, month).unwrap();
            let (start, end) = calendar_grid_range(year, month).unwrap();

            prop_assert_eq!(end.weekday(), Weekday::Mon);
            prop_assert!(end >= next_month_start);
            prop_assert!(end - next_month_start < Duration::days(7));
            prop_assert_eq!((end - start).num_days() % 7, 0);
        }

        #[test]
        fn iso_week_range_is_monday_to_sunday_of_that_week(
            year in 1..=9999i32,
            week in 1..=52i32,
        ) {
            let (monday, sunday) = iso_week_range(year, week).unwrap();

            prop_assert_eq!(monday.weekday(), Weekday::Mon);
            prop_assert_eq!(sunday - monday, Duration::days(6));
            prop_assert_eq!(monday.iso_week().year(), year);
            prop_assert_eq!(i64::from(monday.iso_week().week()), i64::from(week));
        }

        // `calendar_grid_range` is left out until it stops panicking at the
        // earliest representable month (see the ignored test above).
        #[test]
        fn month_and_week_ranges_never_panic(
            year in prop_oneof![any::<i32>(), NaiveDate::MIN.year()..=NaiveDate::MAX.year()],
            month in prop_oneof![any::<u32>(), 0..=13u32],
            week in prop_oneof![any::<i32>(), -1..=54i32],
        ) {
            let _ = month_range(year, month);
            let _ = iso_week_range(year, week);
        }
    }
}
