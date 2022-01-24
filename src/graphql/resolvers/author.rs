use super::Post;
use crate::{
    entity::{
        posts_authors::{self, Entity as PostsAuthors},
        posts_data,
    },
    graphql::types::PostCursor,
    select_columns,
    utils::{create_paginated_posts, db_error, Maybe},
};
use async_graphql::{
    connection::{Connection, EmptyFields},
    ComplexObject, Context, Error, Object, Result, SimpleObject,
};
use sea_orm::{prelude::*, query::QuerySelect, Condition, DatabaseTransaction, FromQueryResult};
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
    async fn image(&self) -> Result<Option<String>> {
        if let Some(ref image) = *self.image {
            if let Some(ref image) = image {
                Ok(Some(format!(
                    "https://backend.verseghy-gimnazium.net/storage/authors_images/{}",
                    image
                )))
            } else {
                Ok(None)
            }
        } else {
            Err(Error::new("Database error: image not selected"))
        }
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

        create_paginated_posts(after, before, first, last, ctx, db, condition).await
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
            .map_err(db_error)
    }
}
