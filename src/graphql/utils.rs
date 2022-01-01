use async_graphql::{
    async_trait::async_trait, parser::types::Field, registry::Registry, ContextSelectionSet,
    InputValueError, InputValueResult, OutputType, Positioned, Scalar, ScalarType, ServerResult,
    Value,
};
use chrono::NaiveDate;
use core::str::FromStr;
use sea_orm::{QueryResult, TryGetError, TryGetable};
use std::borrow::Cow;

#[derive(Debug)]
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
    fn try_get(res: &QueryResult, pre: &str, col: &str) -> Result<Self, TryGetError> {
        Ok(Date(NaiveDate::try_get(res, pre, col)?))
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum Optional<T> {
    Some(T),
    None,
}

impl<T: TryGetable> TryGetable for Optional<T> {
    fn try_get(res: &QueryResult, pre: &str, col: &str) -> Result<Self, TryGetError> {
        match T::try_get(res, pre, col) {
            Ok(value) => Ok(Optional::Some(value)),
            Err(_) => Ok(Optional::None),
        }
    }
}

#[async_trait]
impl<T: OutputType + Sync> OutputType for Optional<T> {
    fn type_name() -> Cow<'static, str> {
        T::type_name()
    }

    fn qualified_type_name() -> String {
        T::type_name().to_string()
    }

    fn create_type_info(registry: &mut Registry) -> String {
        T::create_type_info(registry);
        T::type_name().to_string()
    }

    async fn resolve(
        &self,
        ctx: &ContextSelectionSet<'_>,
        field: &Positioned<Field>,
    ) -> ServerResult<Value> {
        if let Optional::Some(inner) = self {
            match OutputType::resolve(inner, ctx, field).await {
                Ok(value) => Ok(value),
                Err(err) => {
                    ctx.add_error(err);
                    Ok(Value::Null)
                }
            }
        } else {
            Ok(Value::Null)
        }
    }
}
