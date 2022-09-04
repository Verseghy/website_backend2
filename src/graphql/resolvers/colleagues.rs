use crate::{
    entity::colleagues_data::{Column, Entity as ColleaguesData},
    select_columns,
    utils::{db_error, Maybe},
};
use async_graphql::{ComplexObject, Context, Object, Result, SimpleObject};
use prometheus::{labels, IntCounterVec};
use sea_orm::{
    prelude::*,
    query::{QueryOrder, QuerySelect},
    DatabaseTransaction, FromQueryResult,
};
use std::{ops::Deref, sync::Arc};

#[derive(SimpleObject, Debug, FromQueryResult)]
#[graphql(complex)]
pub struct Colleague {
    pub id: Maybe<u32>,
    pub name: Maybe<String>,
    pub jobs: Maybe<Option<String>>,
    pub subjects: Maybe<Option<String>>,
    pub roles: Maybe<Option<String>>,
    pub awards: Maybe<Option<String>>,
    #[graphql(skip)]
    pub image: Maybe<Option<String>>,
    pub category: Maybe<u16>,
}

#[ComplexObject]
impl Colleague {
    async fn image(&self) -> Result<Option<String>> {
        if let Some(Some(ref image)) = *self.image {
            Ok(Some(format!(
                "https://backend.verseghy-gimnazium.net/storage/colleagues_images/{}",
                image
            )))
        } else {
            Ok(None)
        }
    }
}

#[derive(Default)]
pub struct ColleaguesQuery;

#[Object]
impl ColleaguesQuery {
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
