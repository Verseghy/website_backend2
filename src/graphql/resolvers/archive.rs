use super::Post;
use crate::{
    entity::posts_data::{Column, Entity as PostsData},
    select_columns,
    utils::db_error,
};
use async_graphql::{Context, Error, Object, Result, SimpleObject};
use chrono::NaiveDate;
use prometheus::{IntCounterVec, labels};
use sea_orm::{
    DatabaseTransaction, FromQueryResult,
    entity::prelude::*,
    query::{Order, QueryOrder, QuerySelect},
    sea_query::Expr,
};
use std::{ops::Deref, sync::Arc};

/// Summary information about posts in a specific month.
#[derive(SimpleObject, Debug, FromQueryResult)]
pub struct Info {
    /// Number of posts in this month.
    count: i32,
    /// Year.
    year: i32,
    /// Month (1-12).
    month: i32,
}

/// Container for accessing archived posts by year and month.
#[derive(Debug)]
pub struct Archive;

#[Object]
impl Archive {
    /// Retrieve posts from a specific year and month.
    async fn posts(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The year.")] year: i32,
        #[graphql(desc = "The month (1-12).")] month: u32,
    ) -> Result<Vec<Post>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = PostsData::find().select_only();

        select_columns!(ctx, query, Column);
        select_columns!(ctx, query,
            "author" => Column::AuthorId,
            "labels" => Column::Id);

        let start = NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| Error::new("invalid date"))?
            .and_hms_opt(0, 0, 0)
            .unwrap();

        let end = if month < 12 {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        }
        .ok_or_else(|| Error::new("invalid date"))?
        .and_hms_opt(0, 0, 0)
        .unwrap();

        query
            .filter(Column::Date.gte(start))
            .filter(Column::Date.lt(end))
            .filter(Column::Published.eq(true))
            .order_by(Column::Date, Order::Desc)
            .into_model::<Post>()
            .all(db.deref())
            .await
            .map_err(db_error)
    }

    /// Get a summary of post counts grouped by year and month.
    ///
    /// Returns entries sorted by date in descending order (newest first).
    async fn info(&self, ctx: &Context<'_>) -> Result<Vec<Info>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();

        PostsData::find()
            .select_only()
            .column_as(Column::Id.count(), "count")
            .column_as(
                Expr::cust(format!("YEAR({})", Column::Date.to_string()).as_str()),
                "year",
            )
            .column_as(
                Expr::cust(format!("MONTH({})", Column::Date.to_string()).as_str()),
                "month",
            )
            .filter(Column::Date.is_not_null())
            .filter(Column::Published.eq(true))
            .group_by(Expr::cust("year"))
            .group_by(Expr::cust("month"))
            .order_by(Expr::cust("year"), Order::Desc)
            .order_by(Expr::cust("month"), Order::Desc)
            .into_model::<Info>()
            .all(db.deref())
            .await
            .map_err(db_error)
    }
}

#[derive(Default)]
pub struct ArchiveQuery;

#[Object]
impl ArchiveQuery {
    /// Access the post archive.
    ///
    /// Use `posts(year, month)` to retrieve posts from a specific month,
    /// or `info` to get a summary of all available months.
    async fn archive(&self, ctx: &Context<'_>) -> Result<Archive> {
        ctx.data_unchecked::<IntCounterVec>()
            .with(&labels! {"resource" => "archive"})
            .inc();

        Ok(Archive)
    }
}
