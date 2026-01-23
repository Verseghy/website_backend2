use crate::select_columns;
use crate::utils::Maybe;
use crate::{
    entity::{
        canteen_data::{self, Entity as CanteenData},
        canteen_menus::{self, Entity as CanteenMenus},
        canteen_pivot_menus_data,
    },
    graphql::types::Date,
    utils::db_error,
};
use async_graphql::{ComplexObject, Context, Object, Result, SimpleObject};
use chrono::{NaiveDate, Weekday};
use prometheus::{IntCounterVec, labels};
use sea_orm::{
    DatabaseTransaction, FromQueryResult, JoinType,
    entity::prelude::*,
    query::{Order, QueryOrder, QuerySelect},
};
use std::{ops::Deref, sync::Arc};

/// A canteen menu item.
#[derive(SimpleObject, Debug, FromQueryResult)]
pub struct Menu {
    /// Unique identifier.
    id: Maybe<u32>,
    /// Menu item description.
    menu: Maybe<String>,
    /// Menu type identifier (e.g., main course, soup, dessert).
    r#type: Maybe<u16>,
}

/// A day's canteen information.
#[derive(SimpleObject, Debug, FromQueryResult)]
#[graphql(complex)]
pub struct Canteen {
    /// Unique identifier.
    id: Maybe<u32>,
    /// The date for this canteen menu.
    date: Maybe<Date>,
}

#[ComplexObject]
impl Canteen {
    /// Available menu items for this day.
    async fn menus(&self, ctx: &Context<'_>) -> Result<Vec<Menu>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = CanteenMenus::find().select_only();

        select_columns!(ctx, query, canteen_menus::Column);

        query
            .filter(canteen_pivot_menus_data::Column::DataId.eq(self.id.deref().unwrap()))
            .join_rev(
                JoinType::Join,
                canteen_pivot_menus_data::Relation::Menu.def(),
            )
            .order_by(canteen_menus::Column::Type, Order::Asc)
            .into_model::<Menu>()
            .all(db.deref())
            .await
            .map_err(db_error)
    }
}

#[derive(Default)]
pub struct CanteenQuery;

#[Object]
impl CanteenQuery {
    /// Retrieve canteen menus for a specific ISO week.
    ///
    /// Returns canteen data for each day of the specified week (Monday through Sunday).
    async fn canteen(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The ISO year.")] year: i32,
        #[graphql(desc = "The ISO week number (1-53).")] week: i32,
    ) -> Result<Vec<Canteen>> {
        ctx.data_unchecked::<IntCounterVec>()
            .with(&labels! {"resource" => "canteen"})
            .inc();

        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = CanteenData::find().select_only();

        select_columns!(ctx, query, canteen_data::Column);
        select_columns!(ctx, query, "menus" => canteen_data::Column::Id);

        let start = NaiveDate::from_isoywd_opt(year, week as u32, Weekday::Mon).unwrap();
        let end = NaiveDate::from_isoywd_opt(year, week as u32, Weekday::Sun).unwrap();

        query
            .filter(canteen_data::Column::Date.lte(end))
            .filter(canteen_data::Column::Date.gte(start))
            .order_by(canteen_data::Column::Date, Order::Asc)
            .into_model::<Canteen>()
            .all(db.deref())
            .await
            .map_err(db_error)
    }
}
