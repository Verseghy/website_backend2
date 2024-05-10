use async_graphql::{InputValueError, InputValueResult, Scalar, ScalarType, Value};
use chrono::{DateTime as ChronoDateTime, NaiveDateTime};
use core::str::FromStr;
use sea_orm::{QueryResult, TryGetError, TryGetable};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DateTime(pub NaiveDateTime);

#[Scalar]
impl ScalarType for DateTime {
    fn parse(value: Value) -> InputValueResult<DateTime> {
        if let Value::String(value) = value {
            let date = NaiveDateTime::from_str(&value);
            if let Ok(date) = date {
                Ok(DateTime(date))
            } else {
                Err(InputValueError::custom("Wrong date format"))
            }
        } else {
            Err(InputValueError::expected_type(value))
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
