use super::Post;
use crate::{
    Config,
    entity::{
        posts_authors::{self, Entity as PostsAuthors},
        posts_data,
    },
    graphql::types::PostCursor,
    select_columns,
    utils::{Maybe, create_paginated_posts, db_error},
};
use async_graphql::{
    ComplexObject, Context, Error, Object, Result, SimpleObject,
    connection::{Connection, EmptyFields},
};
use prometheus::{IntCounterVec, labels};
use sea_orm::{Condition, DatabaseTransaction, FromQueryResult, prelude::*, query::QuerySelect};
use std::{ops::Deref, sync::Arc};

#[derive(SimpleObject, Debug, FromQueryResult)]
#[graphql(complex)]
pub struct Author {
    pub id: Maybe<u32>,
    pub name: Maybe<String>,
    pub description: Maybe<Option<String>>,
    #[graphql(skip)]
    pub image: Maybe<Option<String>>,
}

#[ComplexObject]
impl Author {
    async fn image(&self, ctx: &Context<'_>) -> Result<Option<String>> {
        let config = ctx.data_unchecked::<Config>();

        let Some(ref image) = *self.image else {
            return Err(Error::new("Database error: image not selected"));
        };

        let Some(image) = image else {
            return Ok(None);
        };

        Ok(Some(format!(
            "{}/authors_images/{}",
            config.storage_base_url, image
        )))
    }

    async fn posts(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = false)] featured: bool,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
    ) -> Result<Connection<PostCursor, Post, EmptyFields, EmptyFields>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let condition = {
            let condition = if featured {
                Some(posts_data::Column::Featured.eq(true))
            } else {
                None
            };

            Condition::all()
                .add_option(condition)
                .add(posts_data::Column::AuthorId.eq(self.id.unwrap()))
        };

        create_paginated_posts(after, before, first, last, ctx, db, condition, None).await
    }
}

#[derive(Default)]
pub struct AuthorsQuery;

#[Object]
impl AuthorsQuery {
    pub async fn author(&self, ctx: &Context<'_>, id: u32) -> Result<Option<Author>> {
        ctx.data_unchecked::<IntCounterVec>()
            .with(&labels! {"resource" => "author"})
            .inc();

        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = PostsAuthors::find().select_only();

        select_columns!(ctx, query, posts_authors::Column);
        select_columns!(ctx, query, "posts" => posts_authors::Column::Id);

        query
            .filter(posts_authors::Column::Id.eq(id))
            .into_model::<Author>()
            .one(db.deref())
            .await
            .map_err(db_error)
    }
}
