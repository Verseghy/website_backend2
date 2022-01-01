use async_graphql::{
    async_trait::async_trait, parser::types::Field, registry::Registry, ContextSelectionSet,
    OutputType, Positioned, ServerResult, Value,
};
use sea_orm::{QueryResult, TryGetError, TryGetable};
use std::{borrow::Cow, ops::Deref};

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub struct Maybe<T>(pub Option<T>);

impl<T> Deref for Maybe<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: TryGetable> TryGetable for Maybe<T> {
    fn try_get(res: &QueryResult, pre: &str, col: &str) -> Result<Self, TryGetError> {
        match T::try_get(res, pre, col) {
            Ok(value) => Ok(Maybe(Some(value))),
            Err(_) => Ok(Maybe(None)),
        }
    }
}

#[async_trait]
impl<T: OutputType + Sync> OutputType for Maybe<T> {
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
        if let Some(inner) = self.deref() {
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
