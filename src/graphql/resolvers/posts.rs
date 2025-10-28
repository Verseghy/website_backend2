use super::{Author, Label};
use crate::{
    entity::{
        posts_authors::{self, Entity as PostsAuthors},
        posts_data::{self, Entity as PostsData},
        posts_labels::{self, Entity as PostsLabels},
        posts_pivot_labels_data,
    },
    graphql::types::{Date, PostCursor},
    select_columns,
    utils::{create_paginated_posts, db_error, Maybe},
};
use async_graphql::{
    connection::{Connection, EmptyFields},
    ComplexObject, Context, Error, Object, Result, SimpleObject,
};
use prometheus::{labels, IntCounterVec};
use sea_orm::{
    prelude::*,
    query::{JoinType, Order, QueryOrder, QuerySelect},
    Condition, DatabaseTransaction, FromQueryResult,
};
use std::{ops::Deref, sync::Arc};

#[derive(SimpleObject, Debug, FromQueryResult)]
#[graphql(complex)]
pub struct Post {
    pub id: Maybe<u32>,
    pub title: Maybe<String>,
    pub color: Maybe<String>,
    pub description: Maybe<Option<String>>,
    pub content: Maybe<String>,
    #[graphql(skip)]
    pub index_image: Maybe<String>,
    #[graphql(skip)]
    pub author_id: Maybe<u32>,
    #[graphql(skip)]
    pub images: Maybe<serde_json::Value>,
    pub date: Maybe<Date>,
}

#[ComplexObject]
impl Post {
    async fn index_image(&self) -> Result<String> {
        if let Some(ref image) = *self.index_image {
            Ok(format!(
                "https://backend.microshift.verseghy-gimnazium.net/storage/posts_images/{}",
                image
            ))
        } else {
            Err(Error::new("No index image found"))
        }
    }

    async fn images(&self) -> Result<Vec<String>> {
        match &*self.images {
            Some(Json::Array(arr)) => Ok(arr
                .iter()
                .filter(|elem| elem.is_string())
                .map(|elem| {
                    format!(
                        "https://backend.microshift.verseghy-gimnazium.net/storage/posts_images/{}",
                        elem.as_str().unwrap()
                    )
                })
                .collect()),
            Some(Json::Object(map)) => Ok(map
                .values()
                .filter(|elem| elem.is_string())
                .map(|elem| {
                    format!(
                        "https://backend.microshift.verseghy-gimnazium.net/storage/posts_images/{}",
                        elem.as_str().unwrap()
                    )
                })
                .collect()),
            _ => Err(Error::new("invalid data in database")),
        }
    }

    async fn author(&self, ctx: &Context<'_>) -> Result<Option<Author>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = PostsAuthors::find().select_only();

        select_columns!(ctx, query, posts_authors::Column);
        select_columns!(ctx, query, "posts" => posts_authors::Column::Id);

        query
            .filter(posts_authors::Column::Id.eq(self.author_id.unwrap()))
            .into_model::<Author>()
            .one(db.deref())
            .await
            .map_err(db_error)
    }

    async fn labels(&self, ctx: &Context<'_>) -> Result<Vec<Label>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = PostsLabels::find().select_only();

        select_columns!(ctx, query, posts_labels::Column);
        select_columns!(ctx, query, "posts" => posts_labels::Column::Id);

        query
            .filter(posts_pivot_labels_data::Column::PostsId.eq(self.id.deref().unwrap()))
            .join_rev(
                JoinType::Join,
                posts_pivot_labels_data::Relation::Labels.def(),
            )
            .order_by(posts_labels::Column::Id, Order::Desc)
            .into_model::<Label>()
            .all(db.deref())
            .await
            .map_err(db_error)
    }
}

#[derive(Default)]
pub struct PostsQuery;

#[Object]
impl PostsQuery {
    async fn posts(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = false)] featured: bool,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
    ) -> Result<Connection<PostCursor, Post, EmptyFields, EmptyFields>> {
        ctx.data_unchecked::<IntCounterVec>()
            .with(&labels! {"resource" => "posts"})
            .inc();

        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let condition = {
            let condition = if featured {
                Some(posts_data::Column::Featured.eq(true))
            } else {
                None
            };

            Condition::all().add_option(condition)
        };

        create_paginated_posts(after, before, first, last, ctx, db, condition, None).await
    }

    async fn search(
        &self,
        ctx: &Context<'_>,
        term: String,
        after: Option<String>,
        before: Option<String>,
        first: Option<i32>,
        last: Option<i32>,
    ) -> Result<Connection<PostCursor, Post, EmptyFields, EmptyFields>> {
        ctx.data_unchecked::<IntCounterVec>()
            .with(&labels! {"resource" => "search"})
            .inc();

        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let condition = Condition::any()
            .add(posts_data::Column::Content.like(format!("%{}%", term).as_str()))
            .add(posts_data::Column::Description.like(format!("%{}%", term).as_str()))
            .add(posts_data::Column::Title.like(format!("%{}%", term).as_str()));

        create_paginated_posts(after, before, first, last, ctx, db, condition, None).await
    }

    async fn post(
        &self,
        ctx: &Context<'_>,
        id: u32,
        token: Option<String>,
    ) -> Result<Option<Post>> {
        ctx.data_unchecked::<IntCounterVec>()
            .with(&labels! {"resource" => "post"})
            .inc();

        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = PostsData::find().select_only();

        select_columns!(ctx, query, posts_data::Column);
        select_columns!(ctx, query,
            "author" => posts_data::Column::AuthorId,
            "labels" => posts_data::Column::Id);

        if let Some(token) = token {
            query = query
                .filter(posts_data::Column::Published.eq(false))
                .filter(posts_data::Column::PreviewToken.eq(token))
        } else {
            query = query.filter(posts_data::Column::Published.eq(true))
        }

        query
            .filter(posts_data::Column::Id.eq(id))
            .order_by(posts_data::Column::Id, Order::Desc)
            .into_model::<Post>()
            .one(db.deref())
            .await
            .map_err(db_error)
    }
}
