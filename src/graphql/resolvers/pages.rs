use crate::entity::pages::{Column, Entity as Pages};
use crate::select_columns;
use crate::utils::Maybe;
use async_graphql::{Context, Error, Object, Result, SimpleObject};
use sea_orm::{entity::prelude::*, query::QuerySelect, DatabaseTransaction, FromQueryResult};
use std::{ops::Deref, sync::Arc};

#[derive(SimpleObject, Debug, FromQueryResult)]
pub struct Page {
    id: Maybe<u32>,
    template: Maybe<String>,
    name: Maybe<String>,
    title: Maybe<String>,
    content: Maybe<String>,
    extras: Maybe<Json>,
}

#[derive(Default)]
pub struct PagesQuery;

#[Object]
impl PagesQuery {
    async fn page(&self, ctx: &Context<'_>, slug: String) -> Result<Option<Page>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = Pages::find().select_only();

        select_columns!(ctx, query, Column);

        query
            .filter(Column::Slug.eq(slug))
            .into_model::<Page>()
            .one(db.deref())
            .await
            .map_err(|_| Error::new("database error"))
    }
}
