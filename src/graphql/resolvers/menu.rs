use crate::entity::{
    menu_items::{Column, Entity as MenuItems},
    pages,
};
use crate::select_columns;
use crate::utils::{Maybe, db_error};
use async_graphql::{ComplexObject, Context, Object, Result, SimpleObject};
use prometheus::{IntCounterVec, labels};
use sea_orm::{
    DatabaseTransaction, FromQueryResult, Order,
    entity::prelude::*,
    query::{QueryOrder, QuerySelect},
};
use std::{ops::Deref, sync::Arc};

/// A navigation menu item.
#[derive(SimpleObject, Debug, FromQueryResult)]
#[graphql(complex)]
pub struct MenuItem {
    #[graphql(skip)]
    id: Maybe<u32>,
    /// Display name of the menu item.
    name: Maybe<String>,
    /// Item type (e.g., "page", "link", "separator").
    r#type: Maybe<String>,
    /// External URL for link-type items.
    link: Maybe<Option<String>>,
    #[graphql(skip)]
    page_id: Option<u32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
enum QuerySlug {
    Slug,
}

#[ComplexObject]
impl MenuItem {
    /// Page slug for page-type items (used to construct internal links).
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

    /// Nested child menu items (for dropdown menus).
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
    /// Retrieve the navigation menu structure.
    ///
    /// Returns top-level menu items. Use the `children` field to access nested items.
    async fn menu(&self, ctx: &Context<'_>) -> Result<Vec<MenuItem>> {
        ctx.data_unchecked::<IntCounterVec>()
            .with(&labels! {"resource" => "menu"})
            .inc();

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
