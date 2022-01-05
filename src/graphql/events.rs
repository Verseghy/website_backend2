use super::types::DateTime;
use crate::{
    entity::events_data::{Column, Entity as EventsData},
    select_columns,
    utils::Maybe,
};
use async_graphql::{Context, Error, Object, Result, SimpleObject};
use chrono::NaiveDate;
use sea_orm::{
    prelude::*,
    query::{Order, QueryOrder, QuerySelect},
    DatabaseConnection, FromQueryResult,
};

#[derive(SimpleObject, Debug, FromQueryResult)]
pub struct Event {
    id: Maybe<u32>,
    date_from: Maybe<DateTime>,
    date_to: Maybe<DateTime>,
    title: Maybe<String>,
    description: Maybe<String>,
    color: Maybe<String>,
    created_at: Maybe<DateTime>,
    updated_at: Maybe<DateTime>,
}

#[derive(Default)]
pub struct EventsQuery;

#[Object]
impl EventsQuery {
    async fn events(&self, ctx: &Context<'_>, year: i32, month: u32) -> Result<Vec<Event>> {
        let db: &DatabaseConnection = ctx.data().unwrap();
        let mut query = EventsData::find().select_only();

        select_columns!(ctx, query,
            "id" => Column::Id,
            "dateFrom" => Column::DateFrom,
            "dateTo" => Column::DateTo,
            "title" => Column::Title,
            "description" => Column::Description,
            "color" => Column::Color,
            "createdAt" => Column::CreatedAt,
            "updatedAt" => Column::UpdatedAt);

        let start = NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or(Error::new("invalid date"))?
            .and_hms(0, 0, 0);

        let end = if month < 12 {
            NaiveDate::from_ymd_opt(year, month + 1, 1).ok_or(Error::new("invalid date"))?
        } else {
            NaiveDate::from_ymd_opt(year + 1, 1, 1).ok_or(Error::new("invalid date"))?
        }
        .and_hms(0, 0, 0);

        Ok(query
            .filter(Column::DateTo.gte(start))
            .filter(Column::DateTo.lt(end))
            .order_by(Column::DateFrom, Order::Asc)
            .into_model::<Event>()
            .all(db)
            .await
            .map_err(|_| Error::new("database error"))?)
    }
}
