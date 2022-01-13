use super::{Author, Label};
use crate::{
    entity::{
        posts_authors::{self, Entity as PostsAuthors},
        posts_data::{self, Entity as PostsData},
        posts_labels::{self, Entity as PostsLabels},
        posts_pivot_labels_data,
    },
    graphql::types::DateTime,
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
pub struct Post {
    pub id: Maybe<u32>,
    pub title: Maybe<String>,
    pub color: Maybe<String>,
    pub description: Maybe<String>,
    pub content: Maybe<String>,
    pub index_image: Maybe<String>,
    #[graphql(skip)]
    pub author_id: Maybe<u32>,
    pub images: Maybe<Json>,
    pub date: Maybe<DateTime>,
}

#[ComplexObject]
impl Post {
    async fn author(&self, ctx: &Context<'_>) -> Result<Author> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = PostsAuthors::find().select_only();

        select_columns!(ctx, query, posts_authors::Column);
        select_columns!(ctx, query, "posts" => posts_authors::Column::Id);

        Ok(query
            .filter(posts_authors::Column::Id.eq(self.author_id.unwrap()))
            .into_model::<Author>()
            .one(db.deref())
            .await
            .map_err(|err| Error::new(format!("database error: {:?}", err)))?
            .unwrap())
    }

    async fn labels(&self, ctx: &Context<'_>) -> Result<Vec<Label>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = PostsLabels::find().select_only();

        select_columns!(ctx, query, posts_labels::Column);
        select_columns!(ctx, query, "labels" => posts_labels::Column::Id);

        Ok(query
            .filter(posts_pivot_labels_data::Column::PostsId.eq(self.id.deref().unwrap()))
            .join_rev(
                JoinType::Join,
                posts_pivot_labels_data::Relation::Labels.def(),
            )
            .order_by(posts_labels::Column::Id, Order::Desc)
            .into_model::<Label>()
            .all(db.deref())
            .await
            .map_err(|err| Error::new(format!("database error: {:?}", err)))?)
    }
}

#[derive(Default)]
pub struct PostsQuery;

#[Object]
impl PostsQuery {
    async fn post(&self, ctx: &Context<'_>, id: u32) -> Result<Option<Post>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = PostsData::find().select_only();

        select_columns!(ctx, query, posts_data::Column);
        select_columns!(ctx, query, "author" => posts_data::Column::AuthorId);

        Ok(query
            .filter(posts_data::Column::Id.eq(id))
            .order_by(posts_data::Column::Id, Order::Desc)
            .into_model::<Post>()
            .one(db.deref())
            .await
            .map_err(|err| Error::new(format!("database error: {:?}", err)))?)
    }
}
