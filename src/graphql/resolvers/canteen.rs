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
use sea_orm::{
    entity::prelude::*,
    query::{Order, QueryOrder, QuerySelect},
    DatabaseTransaction, FromQueryResult, JoinType,
};
use std::{ops::Deref, sync::Arc};

#[derive(SimpleObject, Debug)]
pub struct Menu {
    id: Maybe<u32>,
    menu: Maybe<String>,
    r#type: Maybe<u16>,
}

impl FromQueryResult for Menu {
    fn from_query_result(res: &QueryResult, pre: &str) -> Result<Self, DbErr> {
        Ok(Menu {
            id: res.try_get(pre, "id")?,
            menu: res.try_get(pre, "menu")?,
            r#type: res.try_get(pre, "type")?,
        })
    }
}

#[derive(SimpleObject, Debug, FromQueryResult)]
#[graphql(complex)]
pub struct Canteen {
    id: Maybe<u32>,
    date: Maybe<Date>,
}

#[ComplexObject]
impl Canteen {
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
    async fn canteen(&self, ctx: &Context<'_>, year: i32, week: i32) -> Result<Vec<Canteen>> {
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
