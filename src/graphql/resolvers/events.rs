use crate::{
    entity::events_data::{Column, Entity as EventsData},
    graphql::types::DateTime,
    select_columns,
    utils::{Maybe, db_error},
};
use async_graphql::{Context, Error, Object, Result, SimpleObject};
use chrono::{Datelike, Duration, NaiveDate};
use prometheus::{IntCounterVec, labels};
use sea_orm::{
    DatabaseTransaction, FromQueryResult,
    prelude::*,
    query::{Order, QueryOrder, QuerySelect},
};
use std::{ops::Deref, sync::Arc};

/// A calendar event.
#[derive(SimpleObject, Debug, FromQueryResult)]
pub struct Event {
    /// Unique identifier.
    id: Maybe<u32>,
    /// Event start date and time.
    date_from: Maybe<DateTime>,
    /// Event end date and time.
    date_to: Maybe<DateTime>,
    /// Event title.
    title: Maybe<String>,
    /// Event description.
    description: Maybe<Option<String>>,
    /// Display color for the event.
    color: Maybe<Option<String>>,
}

#[derive(Default)]
pub struct EventsQuery;

#[Object]
impl EventsQuery {
    /// Retrieve events for a specific month.
    ///
    /// Returns events that overlap with the calendar view of the specified month
    /// (including days from adjacent weeks that appear in the month's calendar grid).
    async fn events(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The year.")] year: i32,
        #[graphql(desc = "The month (1-12).")] month: u32,
    ) -> Result<Vec<Event>> {
        ctx.data_unchecked::<IntCounterVec>()
            .with(&labels! {"resource" => "events"})
            .inc();

        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = EventsData::find().select_only();

        select_columns!(ctx, query, Column);

        let start = {
            let start = NaiveDate::from_ymd_opt(year, month, 1)
                .ok_or_else(|| Error::new("invalid date"))?
                .and_hms_opt(0, 0, 0)
                .unwrap();

            start - Duration::days(start.weekday().num_days_from_monday().into())
        };

        let end = {
            let end = if month < 12 {
                NaiveDate::from_ymd_opt(year, month + 1, 1)
            } else {
                NaiveDate::from_ymd_opt(year + 1, 1, 1)
            }
            .ok_or_else(|| Error::new("invalid date"))?
            .and_hms_opt(0, 0, 0)
            .unwrap();

            end + Duration::days((6 - end.weekday().num_days_from_monday()).into())
        };

        query
            .filter(Column::DateTo.gte(start))
            .filter(Column::DateFrom.lt(end))
            .order_by(Column::DateFrom, Order::Asc)
            .into_model::<Event>()
            .all(db.deref())
            .await
            .map_err(db_error)
    }
}
