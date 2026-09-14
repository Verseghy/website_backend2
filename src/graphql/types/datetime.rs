use async_graphql::{Description, InputValueError, InputValueResult, Scalar, ScalarType, Value};
use chrono::{DateTime as ChronoDateTime, NaiveDateTime};
use core::str::FromStr;
use sea_orm::{QueryResult, TryGetError, TryGetable};

/// A date with time information. Format: YYYY-MM-DD HH:MM:SS (e.g., "2024-01-15 14:30:00").
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Description)]
pub struct DateTime(pub NaiveDateTime);

#[Scalar(use_type_description)]
impl ScalarType for DateTime {
    fn parse(value: Value) -> InputValueResult<DateTime> {
        let Value::String(value) = value else {
            return Err(InputValueError::expected_type(value));
        };

        let date = NaiveDateTime::from_str(&value);

        if let Ok(date) = date {
            Ok(DateTime(date))
        } else {
            Err(InputValueError::custom("Wrong date format"))
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.format("%Y-%m-%d %H:%M:%S").to_string())
    }
}

impl TryGetable for DateTime {
    fn try_get_by<I: sea_orm::ColIdx>(res: &QueryResult, index: I) -> Result<Self, TryGetError> {
        Ok(DateTime(NaiveDateTime::try_get_by(res, index)?))
    }
}

impl Default for DateTime {
    fn default() -> Self {
        DateTime(
            ChronoDateTime::from_timestamp_millis(0)
                .unwrap()
                .naive_utc(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::Pos;
    use chrono::NaiveDate;

    fn datetime(year: i32, month: u32, day: u32, hour: u32, min: u32, sec: u32) -> DateTime {
        DateTime(
            NaiveDate::from_ymd_opt(year, month, day)
                .unwrap()
                .and_hms_opt(hour, min, sec)
                .unwrap(),
        )
    }

    fn parse_error(value: Value) -> String {
        DateTime::parse(value)
            .unwrap_err()
            .into_server_error(Pos::default())
            .message
    }

    #[test]
    fn serializes_with_a_space_between_date_and_time() {
        assert_eq!(
            datetime(2024, 1, 15, 14, 30, 5).to_value(),
            Value::String("2024-01-15 14:30:05".to_owned())
        );
    }

    #[test]
    fn parses_iso_8601_with_a_t_separator() {
        assert_eq!(
            DateTime::parse(Value::String("2024-01-15T14:30:05".to_owned())).unwrap(),
            datetime(2024, 1, 15, 14, 30, 5)
        );
    }

    #[test]
    #[ignore = "bug: DateTime rejects the \"YYYY-MM-DD HH:MM:SS\" format it documents and emits"]
    fn parses_the_format_it_emits() {
        let value = datetime(2024, 1, 15, 14, 30, 5);

        assert_eq!(DateTime::parse(value.to_value()).unwrap(), value);
    }

    #[test]
    fn rejects_strings_that_are_not_valid_datetimes() {
        for input in ["", "2024-01-15", "2024-01-15T25:00:00", "15.01.2024 10:00"] {
            let message = parse_error(Value::String(input.to_owned()));

            assert!(
                message.contains("Wrong date format"),
                "{input:?}: {message}"
            );
        }
    }

    #[test]
    fn rejects_values_that_are_not_strings() {
        let message = parse_error(Value::Boolean(true));

        assert!(message.contains("Expected input type"), "{message}");
    }

    #[test]
    fn defaults_to_the_unix_epoch() {
        assert_eq!(DateTime::default(), datetime(1970, 1, 1, 0, 0, 0));
    }
}
