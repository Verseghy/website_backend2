use crate::{
    Config,
    entity::colleagues_data::{Column, Entity as ColleaguesData},
    select_columns,
    utils::{Maybe, db_error},
};
use async_graphql::{ComplexObject, Context, Object, Result, SimpleObject};
use prometheus::{IntCounterVec, labels};
use sea_orm::{
    DatabaseTransaction, FromQueryResult,
    prelude::*,
    query::{QueryOrder, QuerySelect},
};
use std::{ops::Deref, sync::Arc};

/// A staff member or colleague.
#[derive(SimpleObject, Debug, FromQueryResult)]
#[graphql(complex)]
pub struct Colleague {
    /// Unique identifier.
    pub id: Maybe<u32>,
    /// Full name (may include title like "Dr.").
    pub name: Maybe<String>,
    /// Job titles or positions.
    pub jobs: Maybe<Option<String>>,
    /// Teaching subjects.
    pub subjects: Maybe<Option<String>>,
    /// Additional roles or responsibilities.
    pub roles: Maybe<Option<String>>,
    /// Awards and recognitions.
    pub awards: Maybe<Option<String>>,
    #[graphql(skip)]
    pub image: Maybe<Option<String>>,
    /// Category identifier for grouping colleagues.
    pub category: Maybe<u16>,
}

#[ComplexObject]
impl Colleague {
    /// Profile image URL.
    async fn image(&self, ctx: &Context<'_>) -> Result<Option<String>> {
        let config = ctx.data_unchecked::<Config>();

        let Some(Some(ref image)) = *self.image else {
            return Ok(None);
        };

        Ok(Some(format!(
            "{}/colleagues_images/{}",
            config.storage_base_url, image
        )))
    }
}

#[derive(Default)]
pub struct ColleaguesQuery;

#[Object]
impl ColleaguesQuery {
    /// Retrieve all colleagues, sorted alphabetically by name.
    ///
    /// Names with "Dr." prefix are sorted by the name portion (ignoring the title).
    async fn colleagues(&self, ctx: &Context<'_>) -> Result<Vec<Colleague>> {
        ctx.data_unchecked::<IntCounterVec>()
            .with(&labels! {"resource" => "colleagues"})
            .inc();

        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = ColleaguesData::find().select_only();

        select_columns!(ctx, query, Column);

        let mut res = query
            .order_by_asc(Column::Name)
            .into_model::<Colleague>()
            .all(db.deref())
            .await
            .map_err(db_error)?;

        if ctx.look_ahead().field("name").exists() {
            res.sort_by(|a, b| {
                let a = a.name.as_ref().unwrap();
                let b = b.name.as_ref().unwrap();

                let a_name = a.strip_prefix("Dr. ").unwrap_or(a);
                let b_name = a.strip_prefix("Dr. ").unwrap_or(b);

                a_name.cmp(b_name)
            });
        }

        Ok(res)
    }
}
