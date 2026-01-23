use crate::entity::pages::{Column, Entity as Pages};
use crate::select_columns;
use crate::utils::{Maybe, db_error};
use async_graphql::{Context, Object, Result, SimpleObject};
use prometheus::{IntCounterVec, labels};
use sea_orm::{DatabaseTransaction, FromQueryResult, entity::prelude::*, query::QuerySelect};
use std::{ops::Deref, sync::Arc};

/// A static page.
#[derive(SimpleObject, Debug, FromQueryResult)]
pub struct Page {
    /// Unique identifier.
    id: Maybe<u32>,
    /// Template name used to render this page.
    template: Maybe<String>,
    /// Internal page name.
    name: Maybe<String>,
    /// Page title for display.
    title: Maybe<String>,
    /// Page content (HTML or markdown).
    content: Maybe<String>,
    /// Additional structured data as JSON.
    extras: Maybe<Json>,
}

#[derive(Default)]
pub struct PagesQuery;

#[Object]
impl PagesQuery {
    /// Retrieve a page by its URL slug.
    async fn page(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The page's URL slug (e.g., \"about\", \"contact\").")] slug: String,
    ) -> Result<Option<Page>> {
        ctx.data_unchecked::<IntCounterVec>()
            .with(&labels! {"resource" => "page"})
            .inc();

        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = Pages::find().select_only();

        select_columns!(ctx, query, Column);

        query
            .filter(Column::Slug.eq(slug))
            .into_model::<Page>()
            .one(db.deref())
            .await
            .map_err(db_error)
    }
}
