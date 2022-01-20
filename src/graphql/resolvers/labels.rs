use super::Post;
use crate::{
    entity::{
        posts_data::{self, Entity as PostsData},
        posts_labels::{self, Entity as PostsLabels},
        posts_pivot_labels_data,
    },
    select_columns,
    utils::Maybe,
};
use async_graphql::{ComplexObject, Context, Error, Object, Result, SimpleObject};
use sea_orm::{
    prelude::*,
    query::{JoinType, Order, QueryOrder, QuerySelect},
    DatabaseTransaction, FromQueryResult,
};
use std::{ops::Deref, sync::Arc};

#[derive(SimpleObject, Debug, FromQueryResult)]
#[graphql(complex)]
pub struct Label {
    pub id: Maybe<u32>,
    pub name: Maybe<String>,
    pub color: Maybe<String>,
}

#[ComplexObject]
impl Label {
    async fn posts(&self, ctx: &Context<'_>) -> Result<Vec<Post>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = PostsData::find().select_only();

        select_columns!(ctx, query, posts_data::Column);
        select_columns!(ctx, query,
            "author" => posts_data::Column::AuthorId,
            "labels" => posts_data::Column::Id);

        query
            .filter(posts_pivot_labels_data::Column::LabelsId.eq(self.id.deref().unwrap()))
            .join_rev(
                JoinType::Join,
                posts_pivot_labels_data::Relation::Posts.def(),
            )
            .order_by(posts_data::Column::Id, Order::Desc)
            .into_model::<Post>()
            .all(db.deref())
            .await
            .map_err(|err| Error::new(format!("database error: {:?}", err)))
    }
}

#[derive(Default)]
pub struct LabelQuery;

#[Object]
impl LabelQuery {
    pub async fn label(&self, ctx: &Context<'_>, id: u32) -> Result<Option<Label>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = PostsLabels::find().select_only();

        select_columns!(ctx, query, posts_labels::Column);
        select_columns!(ctx, query, "posts" => posts_labels::Column::Id);

        query
            .filter(posts_labels::Column::Id.eq(id))
            .into_model::<Label>()
            .one(db.deref())
            .await
            .map_err(|err| Error::new(format!("database error: {:?}", err)))
    }
}
