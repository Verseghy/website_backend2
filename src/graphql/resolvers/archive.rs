use super::Post;
use crate::{
    entity::posts_data::{Column, Entity as PostsData},
    select_columns,
    utils::db_error,
};
use async_graphql::{Context, Error, Object, Result, SimpleObject};
use chrono::NaiveDate;
use sea_orm::{
    entity::prelude::*,
    query::{Order, QueryOrder, QuerySelect},
    DatabaseTransaction, FromQueryResult,
};
use sea_query::Expr;
use std::{ops::Deref, sync::Arc};

#[derive(SimpleObject, Debug, FromQueryResult)]
pub struct Info {
    count: i32,
    year: i32,
    month: i32,
}

#[derive(Debug)]
pub struct Archive;

#[Object]
impl Archive {
    async fn posts(&self, ctx: &Context<'_>, year: i32, month: u32) -> Result<Vec<Post>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = PostsData::find().select_only();

        select_columns!(ctx, query, Column);
        select_columns!(ctx, query,
            "author" => Column::AuthorId,
            "labels" => Column::Id);

        let start = NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| Error::new("invalid date"))?
            .and_hms(0, 0, 0);

        let end = if month < 12 {
            NaiveDate::from_ymd_opt(year, month + 1, 1)
        } else {
            NaiveDate::from_ymd_opt(year + 1, 1, 1)
        }
        .ok_or_else(|| Error::new("invalid date"))?
        .and_hms(0, 0, 0);

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
    async fn archive(&self) -> Result<Archive> {
        Ok(Archive)
    }
}
