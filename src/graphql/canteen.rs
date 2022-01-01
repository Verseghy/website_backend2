use super::utils::{Date, Optional};
use crate::entity::{
    canteen_data::{self, Entity as CanteenData},
    canteen_menus::{self, Entity as CanteenMenus},
    canteen_pivot_menus_data,
};
use async_graphql::{ComplexObject, Context, Error, Object, Result, SimpleObject};
use chrono::{NaiveDate, Weekday};
use sea_orm::{
    entity::prelude::*,
    query::{Order, QueryOrder, QuerySelect},
    DatabaseConnection, FromQueryResult, JoinType,
};

#[derive(SimpleObject, Debug)]
pub struct Menu {
    id: Optional<u32>,
    menu: Optional<String>,
    r#type: Optional<u16>,
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
    id: u32,
    date: Optional<Date>,
}

#[ComplexObject]
impl Canteen {
    async fn menus(&self, ctx: &Context<'_>) -> Result<Vec<Menu>> {
        let db: &DatabaseConnection = ctx.data().unwrap();

        let mut query = CanteenMenus::find().select_only();

        if ctx.look_ahead().field("id").exists() {
            query = query.column(canteen_menus::Column::Id);
        }
        if ctx.look_ahead().field("menu").exists() {
            query = query.column(canteen_menus::Column::Menu);
        }
        if ctx.look_ahead().field("type").exists() {
            query = query.column(canteen_menus::Column::Type);
        }

        Ok(query
            .filter(canteen_pivot_menus_data::Column::DataId.eq(self.id))
            .join_rev(
                JoinType::Join,
                canteen_pivot_menus_data::Relation::Menu.def(),
            )
            .order_by(canteen_menus::Column::Type, Order::Asc)
            .into_model::<Menu>()
            .all(db)
            .await
            .map_err(|_| Error::new("database error"))?)
    }
}

#[derive(Default)]
pub struct CanteenQuery;

#[Object]
impl CanteenQuery {
    async fn canteen(&self, ctx: &Context<'_>, year: i32, week: i32) -> Result<Vec<Canteen>> {
        let db: &DatabaseConnection = ctx.data().unwrap();

        let mut query = CanteenData::find()
            .select_only()
            .column(canteen_data::Column::Id);

        if ctx.look_ahead().field("date").exists() {
            query = query.column(canteen_data::Column::Date);
        }

        let start = NaiveDate::from_isoywd_opt(year, week as u32, Weekday::Mon).unwrap();
        let end = NaiveDate::from_isoywd_opt(year, week as u32, Weekday::Sun).unwrap();

        Ok(query
            .filter(canteen_data::Column::Date.lte(end))
            .filter(canteen_data::Column::Date.gte(start))
            .order_by(canteen_data::Column::Date, Order::Asc)
            .into_model::<Canteen>()
            .all(db)
            .await
            .map_err(|_| Error::new("database error"))?)
    }
}
