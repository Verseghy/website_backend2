use super::{Author, Label};
use crate::{
    Config,
    entity::{
        posts_authors::{self, Entity as PostsAuthors},
        posts_data::{self, Entity as PostsData},
        posts_labels::{self, Entity as PostsLabels},
        posts_pivot_labels_data,
    },
    graphql::types::{Date, PostCursor},
    select_columns,
    utils::{Maybe, create_paginated_posts, db_error},
};
use async_graphql::{
    ComplexObject, Context, Error, Object, Result, SimpleObject,
    connection::{Connection, EmptyFields},
};
use prometheus::{IntCounterVec, labels};
use sea_orm::{
    Condition, DatabaseTransaction, FromQueryResult,
    prelude::*,
    query::{JoinType, Order, QueryOrder, QuerySelect},
};
use std::{ops::Deref, sync::Arc};

/// A blog post or article.
#[derive(SimpleObject, Debug, FromQueryResult)]
#[graphql(complex)]
pub struct Post {
    /// Unique identifier.
    pub id: Maybe<u32>,
    /// Post title.
    pub title: Maybe<String>,
    /// Theme color for display.
    pub color: Maybe<String>,
    /// Short description or excerpt.
    pub description: Maybe<Option<String>>,
    /// Full post content.
    pub content: Maybe<String>,
    #[graphql(skip)]
    pub index_image: Maybe<String>,
    #[graphql(skip)]
    pub author_id: Maybe<u32>,
    #[graphql(skip)]
    pub images: Maybe<serde_json::Value>,
    /// Publication date.
    pub date: Maybe<Date>,
}

#[ComplexObject]
impl Post {
    /// Main image URL for the post.
    async fn index_image(&self, ctx: &Context<'_>) -> Result<String> {
        let config = ctx.data_unchecked::<Config>();

        let Some(ref image) = *self.index_image else {
            return Err(Error::new("No index image found"));
        };

        Ok(format!("{}/posts_images/{image}", config.storage_base_url))
    }

    /// Additional image URLs associated with the post.
    async fn images(&self, ctx: &Context<'_>) -> Result<Vec<String>> {
        let config = ctx.data_unchecked::<Config>();

        match &*self.images {
            Some(Json::Array(arr)) => Ok(arr
                .iter()
                .filter(|elem| elem.is_string())
                .map(|elem| {
                    format!(
                        "{}/posts_images/{}",
                        config.storage_base_url,
                        elem.as_str().unwrap()
                    )
                })
                .collect()),
            Some(Json::Object(map)) => Ok(map
                .values()
                .filter(|elem| elem.is_string())
                .map(|elem| {
                    format!(
                        "{}/posts_images/{}",
                        config.storage_base_url,
                        elem.as_str().unwrap()
                    )
                })
                .collect()),
            _ => Err(Error::new("invalid data in database")),
        }
    }

    /// The author of this post.
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

    /// Labels/categories associated with this post.
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
    /// Retrieve a paginated list of published posts.
    ///
    /// Use `featured: true` to filter only featured posts.
    /// Supports cursor-based pagination with `after`, `before`, `first`, and `last` arguments.
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

    /// Search posts by title, description, or content.
    ///
    /// Returns a paginated list of posts matching the search term.
    async fn search(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The search term to match against post title, description, and content.")]
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

    /// Retrieve a single post by ID.
    ///
    /// For published posts, only the `id` is required.
    /// For unpublished posts, a valid `token` (preview token) must be provided.
    async fn post(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The post ID.")] id: u32,
        #[graphql(desc = "Preview token for accessing unpublished posts.")] token: Option<String>,
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
