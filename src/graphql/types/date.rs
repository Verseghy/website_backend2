use async_graphql::{InputValueError, InputValueResult, Scalar, ScalarType, Value};
use chrono::NaiveDate;
use core::str::FromStr;
use sea_orm::{QueryResult, TryGetError, TryGetable};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date(pub NaiveDate);

#[Scalar]
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
