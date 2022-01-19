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
use chrono::NaiveDate;
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
            .map_err(|err| Error::new(format!("database error: {:?}", err)))
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
enum QueryMinMax {
    Id,
}

async fn get_published_posts_min_max_id(db: &DatabaseTransaction) -> Result<(u32, u32)> {
    let order_by = |order: Order| async move {
        PostsData::find()
            .select_only()
            .column(posts_data::Column::Id)
            .filter(posts_data::Column::Published.eq(true))
            .order_by(posts_data::Column::Id, order)
            .into_values::<(u32,), QueryMinMax>()
            .one(db)
            .await
            .map_err(|err| Error::new(format!("database error: {:?}", err)))
    };
    let min = order_by(Order::Asc).await?;
    let max = order_by(Order::Desc).await?;

    let ((min,), (max,)) = (min.unwrap_or((0,)), max.unwrap_or((0,)));

    Ok((min, max))
}

#[derive(Default)]
pub struct PostsQuery;

fn build_paginated_posts(
    after: Option<i64>,
    before: Option<i64>,
    first: Option<usize>,
    last: Option<usize>,
) -> Select<PostsData> {
    let mut query = PostsData::find()
        .select_only()
        .column(posts_data::Column::Id);

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

    query
}

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
                let mut query = build_paginated_posts(after, before, first, last);

                select_columns_connection!(ctx, query, posts_data::Column);
                select_columns_connection!(ctx, query, "author" => posts_data::Column::AuthorId);

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

    async fn search(
        &self,
        ctx: &Context<'_>,
        term: String,
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
                let mut query = build_paginated_posts(after, before, first, last);

                select_columns_connection!(ctx, query, posts_data::Column);
                select_columns_connection!(ctx, query, "author" => posts_data::Column::AuthorId);

                query = query.filter(
                    Condition::any()
                        .add(posts_data::Column::Content.like(format!("%{}%", term).as_str()))
                        .add(posts_data::Column::Description.like(format!("%{}%", term).as_str()))
                        .add(posts_data::Column::Title.like(format!("%{}%", term).as_str())),
                );

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

        query
            .filter(posts_data::Column::Id.eq(id))
            .order_by(posts_data::Column::Id, Order::Desc)
            .into_model::<Post>()
            .one(db.deref())
            .await
            .map_err(|err| Error::new(format!("database error: {:?}", err)))
    }

    async fn archive(&self, ctx: &Context<'_>, year: i32, month: u32) -> Result<Vec<Post>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = PostsData::find().select_only();

        select_columns!(ctx, query, posts_data::Column);
        select_columns!(ctx, query, "author" => posts_data::Column::AuthorId);

        let start = NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| Error::new("invalid date"))?
            .and_hms(0, 0, 0);

        let end = if month < 12 {
            NaiveDate::from_ymd_opt(year, month + 1, 1).ok_or_else(|| Error::new("invalid date"))?
        } else {
            NaiveDate::from_ymd_opt(year + 1, 1, 1).ok_or_else(|| Error::new("invalid date"))?
        }
        .and_hms(0, 0, 0);

        query
            .filter(posts_data::Column::Date.gte(start))
            .filter(posts_data::Column::Date.lt(end))
            .order_by(posts_data::Column::Date, Order::Desc)
            .into_model::<Post>()
            .all(db.deref())
            .await
            .map_err(|err| Error::new(format!("database error: {:?}", err)))
    }

    async fn preview(&self, ctx: &Context<'_>, id: u32, token: String) -> Result<Option<Post>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = PostsData::find().select_only();

        select_columns!(ctx, query, posts_data::Column);
        select_columns!(ctx, query, "author" => posts_data::Column::AuthorId);

        query
            .filter(posts_data::Column::Id.eq(id))
            .filter(posts_data::Column::Published.eq(false))
            .filter(posts_data::Column::PreviewToken.eq(token))
            .into_model::<Post>()
            .one(db.deref())
            .await
            .map_err(|err| Error::new(format!("database error: {:?}", err)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::QueryTrait;
    use test_case::test_case;

    #[test_case(
        None, None, Some(10), None
        => "SELECT `posts_data`.`id` FROM `posts_data` ORDER BY `posts_data`.`id` ASC LIMIT 10";
        "only first"
    )]
    #[test_case(
        None, None, None, Some(10)
        => "SELECT `posts_data`.`id` FROM `posts_data` ORDER BY `posts_data`.`id` DESC LIMIT 10";
        "only last"
    )]
    #[test_case(
        Some(69), None, Some(10), None
        => "SELECT `posts_data`.`id` FROM `posts_data` WHERE `posts_data`.`id` > 69 ORDER BY `posts_data`.`id` ASC LIMIT 10";
        "first with after"
    )]
    #[test_case(
        None, Some(69), Some(10), None
        => "SELECT `posts_data`.`id` FROM `posts_data` WHERE `posts_data`.`id` < 69 ORDER BY `posts_data`.`id` ASC LIMIT 10";
        "first with before"
    )]
    #[test_case(
        Some(69), None, None, Some(10)
        => "SELECT `posts_data`.`id` FROM `posts_data` WHERE `posts_data`.`id` > 69 ORDER BY `posts_data`.`id` DESC LIMIT 10";
        "last with after"
    )]
    #[test_case(
        None, Some(69), None, Some(10)
        => "SELECT `posts_data`.`id` FROM `posts_data` WHERE `posts_data`.`id` < 69 ORDER BY `posts_data`.`id` DESC LIMIT 10";
        "last with before"
    )]
    fn test_build_paginated_posts(
        after: Option<i64>,
        before: Option<i64>,
        first: Option<usize>,
        last: Option<usize>,
    ) -> String {
        build_paginated_posts(after, before, first, last)
            .build(sea_orm::DatabaseBackend::MySql)
            .to_string()
    }
}
