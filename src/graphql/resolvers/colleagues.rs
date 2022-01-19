use crate::{
    entity::colleagues_data::{Column, Entity as ColleaguesData},
    select_columns,
    utils::Maybe,
};
use async_graphql::{Context, Error, Object, Result, SimpleObject};
use sea_orm::{prelude::*, query::QuerySelect, DatabaseTransaction, FromQueryResult};
use std::{ops::Deref, sync::Arc};

#[derive(SimpleObject, Debug, FromQueryResult)]
pub struct Colleague {
    pub id: Maybe<u32>,
    pub name: Maybe<String>,
    pub jobs: Maybe<Option<String>>,
    pub subjects: Maybe<Option<String>>,
    pub roles: Maybe<Option<String>>,
    pub awards: Maybe<Option<String>>,
    pub image: Maybe<Option<String>>,
    pub category: Maybe<u16>,
}

#[derive(Default)]
pub struct ColleaguesQuery;

#[Object]
impl ColleaguesQuery {
    async fn colleagues(&self, ctx: &Context<'_>) -> Result<Vec<Colleague>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = ColleaguesData::find().select_only();

        select_columns!(ctx, query, Column);

        let mut res = query
            .into_model::<Colleague>()
            .all(db.deref())
            .await
            .map_err(|_| Error::new("database error"))?;

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
