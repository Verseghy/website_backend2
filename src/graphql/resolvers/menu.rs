use crate::entity::{
    menu_items::{Column, Entity as MenuItems},
    pages,
};
use crate::select_columns;
use crate::utils::{db_error, Maybe};
use async_graphql::{ComplexObject, Context, Object, Result, SimpleObject};
use sea_orm::{
    entity::prelude::*,
    query::{QueryOrder, QuerySelect},
    DatabaseTransaction, FromQueryResult, Order,
};
use std::{ops::Deref, sync::Arc};

#[derive(SimpleObject, Debug)]
#[graphql(complex)]
pub struct MenuItem {
    #[graphql(skip)]
    id: Maybe<u32>,
    name: Maybe<String>,
    r#type: Maybe<String>,
    link: Maybe<Option<String>>,
    #[graphql(skip)]
    page_id: Option<u32>,
}

impl FromQueryResult for MenuItem {
    fn from_query_result(row: &QueryResult, pre: &str) -> Result<Self, DbErr> {
        Ok(Self {
            id: row.try_get(pre, "id")?,
            name: row.try_get(pre, "name")?,
            r#type: row.try_get(pre, "type")?,
            link: row.try_get(pre, "link")?,
            page_id: row.try_get(pre, "page_id")?,
        })
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
enum QuerySlug {
    Slug,
}

#[ComplexObject]
impl MenuItem {
    async fn slug(&self, ctx: &Context<'_>) -> Result<Option<String>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        Ok(pages::Entity::find()
            .select_only()
            .column(pages::Column::Slug)
            .filter(pages::Column::Id.eq(self.page_id))
            .into_values::<_, QuerySlug>()
            .one(db.deref())
            .await
            .map_err(db_error)?
            .map(|(slug,)| slug))
    }

    async fn children(&self, ctx: &Context<'_>) -> Result<Vec<MenuItem>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = MenuItems::find().select_only();

        select_columns!(ctx, query, Column);
        select_columns!(ctx, query,
            "link" => Column::Type,
            "link" => Column::PageId,
            "children" => Column::Id);

        query
            .filter(Column::ParentId.eq(self.id.unwrap()))
            .order_by(Column::Lft, Order::Asc)
            .into_model::<MenuItem>()
            .all(db.deref())
            .await
            .map_err(db_error)
    }
}

#[derive(Default)]
pub struct MenuQuery;

#[Object]
impl MenuQuery {
    async fn menu(&self, ctx: &Context<'_>) -> Result<Vec<MenuItem>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = MenuItems::find().select_only();

        select_columns!(ctx, query, Column);
        select_columns!(ctx, query,
            "link" => Column::Type,
            "link" => Column::PageId,
            "children" => Column::Id);

        query
            .filter(Column::ParentId.is_null())
            .order_by(Column::Lft, Order::Asc)
            .into_model::<MenuItem>()
            .all(db.deref())
            .await
            .map_err(db_error)
    }
}
