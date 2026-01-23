use super::Post;
use crate::{
    entity::{
        posts_data,
        posts_labels::{self, Entity as PostsLabels},
        posts_pivot_labels_data,
    },
    graphql::types::PostCursor,
    select_columns,
    utils::{Maybe, create_paginated_posts, db_error},
};
use async_graphql::{
    ComplexObject, Context, Object, Result, SimpleObject,
    connection::{Connection, EmptyFields},
};
use prometheus::{IntCounterVec, labels};
use sea_orm::{Condition, DatabaseTransaction, FromQueryResult, prelude::*, query::QuerySelect};
use std::{ops::Deref, sync::Arc};

/// A category label for posts.
#[derive(SimpleObject, Debug, FromQueryResult)]
#[graphql(complex)]
pub struct Label {
    /// Unique identifier.
    pub id: Maybe<u32>,
    /// Label name.
    pub name: Maybe<String>,
    /// Display color (hex or named color).
    pub color: Maybe<String>,
}

#[ComplexObject]
impl Label {
    /// Paginated list of posts with this label.
    ///
    /// Use `featured: true` to filter only featured posts.
    async fn posts(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = false, desc = "Filter to only featured posts.")] featured: bool,
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
                .add(posts_pivot_labels_data::Column::LabelsId.eq(self.id.deref().unwrap()))
        };

        create_paginated_posts(
            after,
            before,
            first,
            last,
            ctx,
            db,
            condition,
            Some(posts_pivot_labels_data::Relation::Posts.def()),
        )
        .await
    }
}

#[derive(Default)]
pub struct LabelQuery;

#[Object]
impl LabelQuery {
    /// Retrieve a label by its ID.
    pub async fn label(
        &self,
        ctx: &Context<'_>,
        #[graphql(desc = "The label's unique identifier.")] id: u32,
    ) -> Result<Option<Label>> {
        ctx.data_unchecked::<IntCounterVec>()
            .with(&labels! {"resource" => "label"})
            .inc();

        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = PostsLabels::find().select_only();

        select_columns!(ctx, query, posts_labels::Column);
        select_columns!(ctx, query, "posts" => posts_labels::Column::Id);

        query
            .filter(posts_labels::Column::Id.eq(id))
            .into_model::<Label>()
            .one(db.deref())
            .await
            .map_err(db_error)
    }
}
