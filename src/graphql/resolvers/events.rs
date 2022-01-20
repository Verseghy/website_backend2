use crate::{
    entity::events_data::{Column, Entity as EventsData},
    graphql::types::DateTime,
    select_columns,
    utils::Maybe,
};
use async_graphql::{Context, Error, Object, Result, SimpleObject};
use chrono::NaiveDate;
use sea_orm::{
    prelude::*,
    query::{Order, QueryOrder, QuerySelect},
    DatabaseTransaction, FromQueryResult,
};
use std::{ops::Deref, sync::Arc};

#[derive(SimpleObject, Debug, FromQueryResult)]
pub struct Event {
    id: Maybe<u32>,
    date_from: Maybe<DateTime>,
    date_to: Maybe<DateTime>,
    title: Maybe<String>,
    description: Maybe<Option<String>>,
    color: Maybe<Option<String>>,
}

#[derive(Default)]
pub struct EventsQuery;

#[Object]
impl EventsQuery {
    async fn events(&self, ctx: &Context<'_>, year: i32, month: u32) -> Result<Vec<Event>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = EventsData::find().select_only();

        select_columns!(ctx, query, Column);

        let start = NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| Error::new("invalid date"))?
            .and_hms(0, 0, 0);

        let end = if month < 12 {
            NaiveDate::from_ymd_opt(year, month + 1, 1).ok_or_else(|| Error::new("invalid date"))?
        } else {
            NaiveDate::from_ymd_opt(year + 1, 1, 1).ok_or_else(|| Error::new("invalid date"))?
        }
        .and_hms(0, 0, 0);

        query
            .filter(Column::DateTo.gte(start))
            .filter(Column::DateTo.lt(end))
            .order_by(Column::DateFrom, Order::Asc)
            .into_model::<Event>()
            .all(db.deref())
            .await
            .map_err(|err| Error::new(format!("database error: {:?}", err)))
    }
}
