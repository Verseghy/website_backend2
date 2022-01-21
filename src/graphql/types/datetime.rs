use async_graphql::{InputValueError, InputValueResult, Scalar, ScalarType, Value};
use chrono::NaiveDateTime;
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
    fn try_get(res: &QueryResult, pre: &str, col: &str) -> Result<Self, TryGetError> {
        Ok(DateTime(NaiveDateTime::try_get(res, pre, col)?))
    }
}

impl Default for DateTime {
    fn default() -> Self {
        DateTime(NaiveDateTime::from_timestamp(0, 0))
    }
}
