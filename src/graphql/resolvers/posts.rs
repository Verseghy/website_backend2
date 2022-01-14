use super::{Author, Label};
use crate::{
    entity::{
        posts_authors::{self, Entity as PostsAuthors},
        posts_data::{self, Entity as PostsData},
        posts_labels::{self, Entity as PostsLabels},
        posts_pivot_labels_data,
    },
    graphql::types::DateTime,
    select_columns, select_columns_connection,
    utils::Maybe,
};
use async_graphql::{
    connection::{query, Connection, Edge, EmptyFields},
    ComplexObject, Context, Error, Object, Result, SimpleObject,
};
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

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
enum QueryMinMax {
    Id,
}

async fn get_published_posts_min_max_id(db: &DatabaseTransaction) -> Result<(u32, u32)> {
    let min = PostsData::find()
        .select_only()
        .column(posts_data::Column::Id)
        .filter(posts_data::Column::Published.eq(true))
        .order_by(posts_data::Column::Id, Order::Asc)
        .into_values::<(u32,), QueryMinMax>()
        .one(db)
        .await
        .map_err(|err| Error::new(format!("database error: {:?}", err)))?;

    let max = PostsData::find()
        .select_only()
        .column(posts_data::Column::Id)
        .filter(posts_data::Column::Published.eq(true))
        .order_by(posts_data::Column::Id, Order::Desc)
        .into_values::<(u32,), QueryMinMax>()
        .one(db)
        .await
        .map_err(|err| Error::new(format!("database error: {:?}", err)))?;

    let ((min,), (max,)) = (min.unwrap_or((0,)), max.unwrap_or((0,)));

    Ok((min, max))
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
    ) -> Result<Connection<i64, Post, EmptyFields, EmptyFields>> {
        query(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
                let mut query = PostsData::find()
                    .select_only()
                    .column(posts_data::Column::Id);

                select_columns_connection!(ctx, query, posts_data::Column);
                select_columns_connection!(ctx, query, "author" => posts_data::Column::AuthorId);

                if let Some(before) = before {
                    query = query.filter(posts_data::Column::Id.lt(before));
                }

                if let Some(after) = after {
                    query = query.filter(posts_data::Column::Id.gt(after));
                }

                if let Some(first) = first {
                    query = query
                        .limit(first as u64)
                        .order_by(posts_data::Column::Id, Order::Asc);
                }

                if let Some(last) = last {
                    query = query
                        .limit(last as u64)
                        .order_by(posts_data::Column::Id, Order::Desc);
                }

                if featured {
                    query = query.filter(posts_data::Column::Featured.eq(true));
                }

                let mut res = query
                    .into_model::<Post>()
                    .all(db.deref())
                    .await
                    .map_err(|err| Error::new(format!("database error: {:?}", err)))?;

                res.sort_by(|a, b| b.id.cmp(&a.id));

                let (min, max) = get_published_posts_min_max_id(db.deref()).await?;

                let mut connection = Connection::new(
                    min < res.last().map(|x| x.id.unwrap_or(0)).unwrap_or(0),
                    max > res.last().map(|x| x.id.unwrap_or(0)).unwrap_or(0),
                );

                connection.append(
                    res.into_iter()
                        .map(|post| Edge::new(post.id.unwrap() as i64, post)),
                );

                Ok::<_, Error>(connection)
            },
        )
        .await
    }

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
