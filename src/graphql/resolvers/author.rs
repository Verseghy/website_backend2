use super::Post;
use crate::{
    entity::{
        posts_authors::{self, Entity as PostsAuthors},
        posts_data::{self, Entity as PostsData},
    },
    select_columns,
    utils::Maybe,
};
use async_graphql::{ComplexObject, Context, Error, Object, Result, SimpleObject};
use sea_orm::{
    prelude::*,
    query::{Order, QueryOrder, QuerySelect},
    DatabaseTransaction, FromQueryResult,
};
use std::{ops::Deref, sync::Arc};

#[derive(SimpleObject, Debug, FromQueryResult)]
#[graphql(complex)]
pub struct Author {
    pub id: Maybe<u32>,
    pub name: Maybe<String>,
    pub description: Maybe<Option<String>>,
    pub image: Maybe<Option<String>>,
}

#[ComplexObject]
impl Author {
    async fn posts(&self, ctx: &Context<'_>) -> Result<Vec<Post>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = PostsData::find().select_only();

        select_columns!(ctx, query, posts_data::Column);
        select_columns!(ctx, query,
            "author" => posts_data::Column::AuthorId,
            "labels" => posts_data::Column::Id);

        query
            .filter(posts_data::Column::AuthorId.eq(self.id.unwrap()))
            .filter(posts_data::Column::Published.eq(true))
            .order_by(posts_data::Column::Id, Order::Desc)
            .into_model::<Post>()
            .all(db.deref())
            .await
            .map_err(|err| Error::new(format!("database error: {:?}", err)))
    }
}

#[derive(Default)]
pub struct AuthorsQuery;

#[Object]
impl AuthorsQuery {
    pub async fn author(&self, ctx: &Context<'_>, id: u32) -> Result<Option<Author>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = PostsAuthors::find().select_only();

        select_columns!(ctx, query, posts_authors::Column);
        select_columns!(ctx, query, "posts" => posts_authors::Column::Id);

        query
            .filter(posts_authors::Column::Id.eq(id))
            .into_model::<Author>()
            .one(db.deref())
            .await
            .map_err(|err| Error::new(format!("database error: {:?}", err)))
    }
}
