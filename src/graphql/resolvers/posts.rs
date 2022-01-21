use super::{Author, Label};
use crate::{
    entity::{
        posts_authors::{self, Entity as PostsAuthors},
        posts_data::{self, Entity as PostsData},
        posts_labels::{self, Entity as PostsLabels},
        posts_pivot_labels_data,
    },
    graphql::types::{Date, PostCursor},
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
    #[graphql(skip)]
    pub index_image: Maybe<String>,
    #[graphql(skip)]
    pub author_id: Maybe<u32>,
    #[graphql(skip)]
    pub images: Maybe<Json>,
    pub date: Maybe<Date>,
}

#[ComplexObject]
impl Post {
    async fn index_image(&self) -> Result<String> {
        if let Some(ref image) = *self.index_image {
            Ok(format!(
                "https://backend.verseghy-gimnazium.net/storage/posts_images/{}",
                image
            ))
        } else {
            Err(Error::new("No index image found"))
        }
    }

    async fn images(&self) -> Result<Vec<String>> {
        if let Some(Json::Array(ref arr)) = &*self.images {
            Ok(arr
                .iter()
                .filter(|elem| elem.is_string())
                .map(|elem| {
                    format!(
                        "https://backend.verseghy-gimnazium.net/storage/posts_images/{}",
                        elem.as_str().unwrap()
                    )
                })
                .collect())
        } else {
            Err(Error::new("invalid data in database"))
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
            .map_err(|err| Error::new(format!("database error: {:?}", err)))
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
            .map_err(|err| Error::new(format!("database error: {:?}", err)))
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveColumn)]
enum QueryMinMax {
    Date,
    Id,
}

async fn get_published_posts_min_max_id(
    db: &DatabaseTransaction,
) -> Result<((NaiveDate, u32), (NaiveDate, u32))> {
    let order_by = |order: Order| async move {
        PostsData::find()
            .select_only()
            .column(posts_data::Column::Date)
            .column(posts_data::Column::Id)
            .filter(posts_data::Column::Published.eq(true))
            .order_by(posts_data::Column::Date, order)
            .order_by(posts_data::Column::Id, order)
            .into_values::<(NaiveDate, u32), QueryMinMax>()
            .one(db)
            .await
            .map_err(|err| Error::new(format!("database error: {:?}", err)))
    };
    let min = order_by(Order::Asc)
        .await?
        .ok_or_else(|| Error::new("Could not get min value"))?;
    let max = order_by(Order::Desc)
        .await?
        .ok_or_else(|| Error::new("Could not get max value"))?;

    Ok((min, max))
}

fn get_connection(
    vec: &[Post],
    min: (NaiveDate, u32),
    max: (NaiveDate, u32),
) -> Result<Connection<PostCursor, Post, EmptyFields, EmptyFields>> {
    if vec.is_empty() {
        Ok(Connection::new(false, false))
    } else {
        let first = vec.first().unwrap();
        let last = vec.last().unwrap();

        let prev = min.0 <= last.date.ok_or_else(|| Error::new("No date found"))?.0
            && min.1 != last.id.ok_or_else(|| Error::new("No id found"))?;
        let next = max.0 >= first.date.ok_or_else(|| Error::new("No date found"))?.0
            && max.1 != first.id.ok_or_else(|| Error::new("No id found"))?;

        Ok(Connection::new(prev, next))
    }
}

#[derive(Default)]
pub struct PostsQuery;

fn build_paginated_posts(
    after: Option<PostCursor>,
    before: Option<PostCursor>,
    first: Option<usize>,
    last: Option<usize>,
) -> Select<PostsData> {
    let mut query = PostsData::find()
        .select_only()
        .column(posts_data::Column::Id)
        .column(posts_data::Column::Date);

    if let Some(before) = before {
        query = query
            .filter(posts_data::Column::Date.lte(before.date()))
            .filter(posts_data::Column::Id.lt(before.id()));
    }

    if let Some(after) = after {
        query = query
            .filter(posts_data::Column::Date.gte(after.date()))
            .filter(posts_data::Column::Id.gt(after.id()));
    }

    if let Some(first) = first {
        query = query
            .limit(first as u64)
            .order_by(posts_data::Column::Date, Order::Asc)
            .order_by(posts_data::Column::Id, Order::Asc);
    }

    if let Some(last) = last {
        query = query
            .limit(last as u64)
            .order_by(posts_data::Column::Date, Order::Desc)
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
    ) -> Result<Connection<PostCursor, Post, EmptyFields, EmptyFields>> {
        query(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
                let mut query = build_paginated_posts(after, before, first, last);

                select_columns_connection!(ctx, query, posts_data::Column);
                select_columns_connection!(ctx, query,
                    "author" => posts_data::Column::AuthorId,
                    "labels" => posts_data::Column::Id);

                if featured {
                    query = query.filter(posts_data::Column::Featured.eq(true));
                }

                let mut res = query
                    .filter(posts_data::Column::Published.eq(true))
                    .into_model::<Post>()
                    .all(db.deref())
                    .await
                    .map_err(|err| Error::new(format!("database error: {:?}", err)))?;

                res.sort_by(|a, b| b.date.cmp(&a.date));

                let (min, max) = get_published_posts_min_max_id(db.deref()).await?;

                let mut connection = get_connection(&res, min, max)?;

                connection.append(res.into_iter().map(|post| {
                    let cursor = PostCursor::new(post.date.unwrap(), post.id.unwrap());
                    Edge::new(cursor, post)
                }));

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
    ) -> Result<Connection<PostCursor, Post, EmptyFields, EmptyFields>> {
        query(
            after,
            before,
            first,
            last,
            |after, before, first, last| async move {
                let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
                let mut query = build_paginated_posts(after, before, first, last);

                select_columns_connection!(ctx, query, posts_data::Column);
                select_columns_connection!(ctx, query,
                    "author" => posts_data::Column::AuthorId,
                    "labels" => posts_data::Column::Id);

                let mut res = query
                    .filter(
                        Condition::any()
                            .add(posts_data::Column::Content.like(format!("%{}%", term).as_str()))
                            .add(
                                posts_data::Column::Description
                                    .like(format!("%{}%", term).as_str()),
                            )
                            .add(posts_data::Column::Title.like(format!("%{}%", term).as_str())),
                    )
                    .filter(posts_data::Column::Published.eq(true))
                    .into_model::<Post>()
                    .all(db.deref())
                    .await
                    .map_err(|err| Error::new(format!("database error: {:?}", err)))?;

                res.sort_by(|a, b| b.id.cmp(&a.id));

                let (min, max) = get_published_posts_min_max_id(db.deref()).await?;

                let mut connection = get_connection(&res, min, max)?;

                connection.append(res.into_iter().map(|post| {
                    let cursor = PostCursor::new(post.date.unwrap(), post.id.unwrap());
                    Edge::new(cursor, post)
                }));

                Ok::<_, Error>(connection)
            },
        )
        .await
    }

    async fn post(
        &self,
        ctx: &Context<'_>,
        id: u32,
        token: Option<String>,
    ) -> Result<Option<Post>> {
        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = PostsData::find().select_only();

        select_columns!(ctx, query, posts_data::Column);
        select_columns!(ctx, query, "author" => posts_data::Column::AuthorId);

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
