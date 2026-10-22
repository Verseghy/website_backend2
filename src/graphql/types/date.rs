use async_graphql::{Description, InputValueError, InputValueResult, Scalar, ScalarType, Value};
use chrono::NaiveDate;
use core::str::FromStr;
use sea_orm::{QueryResult, TryGetError, TryGetable};

/// A date without time information. Format: YYYY-MM-DD (e.g., "2024-01-15").
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Description)]
pub struct Date(pub NaiveDate);

#[Scalar(use_type_description)]
impl ScalarType for Date {
    fn parse(value: Value) -> InputValueResult<Date> {
        if let Value::String(value) = value {
            let date = NaiveDate::from_str(&value);
            if let Ok(date) = date {
                Ok(Date(date))
            } else {
                Err(InputValueError::custom("Wrong date format"))
            }
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.format("%Y-%m-%d").to_string())
    }
}

impl TryGetable for Date {
    fn try_get_by<I: sea_orm::ColIdx>(res: &QueryResult, index: I) -> Result<Self, TryGetError> {
        Ok(Date(NaiveDate::try_get_by(res, index)?))
    }
}

impl Default for Date {
    fn default() -> Self {
        Date(NaiveDate::from_ymd_opt(1970, 1, 1).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::Pos;
    use proptest::prelude::*;

    fn date(year: i32, month: u32, day: u32) -> Date {
        Date(NaiveDate::from_ymd_opt(year, month, day).unwrap())
    }

    fn parse_error(value: Value) -> String {
        Date::parse(value)
            .unwrap_err()
            .into_server_error(Pos::default())
            .message
    }

    #[test]
    fn parses_iso_dates() {
        assert_eq!(
            Date::parse(Value::String("2024-01-15".to_owned())).unwrap(),
            date(2024, 1, 15)
        );
    }

    #[test]
    fn serializes_as_iso_date() {
        assert_eq!(
            date(2024, 1, 15).to_value(),
            Value::String("2024-01-15".to_owned())
        );
    }

    #[test]
    fn rejects_strings_that_are_not_valid_dates() {
        for input in [
            "",
            "15.01.2024",
            "2024-13-01",
            "2023-02-29",
            "2024-01-15 10:00:00",
        ] {
            let message = parse_error(Value::String(input.to_owned()));

            assert!(
                message.contains("Wrong date format"),
                "{input:?}: {message}"
            );
        }
    }

    #[test]
    fn rejects_values_that_are_not_strings() {
        let message = parse_error(Value::Number(20240115.into()));

        assert!(message.contains("Expected input type"), "{message}");
    }

    #[test]
    fn defaults_to_the_unix_epoch() {
        assert_eq!(Date::default(), date(1970, 1, 1));
    }

    proptest! {
        #[test]
        fn round_trips_through_a_graphql_value(days in 1..=3_652_059i32) {
            // Day 1 is 0001-01-01 and day 3 652 059 is 9999-12-31.
            let date = Date(NaiveDate::from_num_days_from_ce_opt(days).unwrap());

            prop_assert_eq!(Date::parse(date.to_value()).unwrap(), date);
        }
    }
}
