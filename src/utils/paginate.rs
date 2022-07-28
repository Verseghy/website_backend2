use crate::{
    entity::posts_data::{Column, Entity as PostsData},
    graphql::{resolvers::Post, types::PostCursor},
    select_columns_connection,
    utils::db_error,
};
use async_graphql::{
    connection::{query, Connection, Edge, EmptyFields},
    Context, Error, Result,
};
use chrono::NaiveDate;
use sea_orm::{
    entity::{EntityTrait, RelationDef},
    query::{Order, QueryFilter, QueryOrder, QuerySelect},
    ColumnTrait, DatabaseTransaction, DeriveColumn, EnumIter, IdenStatic, JoinType, Select,
};
use sea_query::query::IntoCondition;
use std::ops::Deref;

fn build_paginated_posts(
    after: Option<PostCursor>,
    before: Option<PostCursor>,
    first: Option<usize>,
    last: Option<usize>,
) -> Select<PostsData> {
    let mut query = PostsData::find()
        .select_only()
        .column(Column::Id)
        .column(Column::Date);

    if let Some(before) = before {
        query = query
            .filter(Column::Date.lte(before.date()))
            .filter(Column::Id.lt(before.id()));
    }

    if let Some(after) = after {
        query = query
            .filter(Column::Date.gte(after.date()))
            .filter(Column::Id.gt(after.id()));
    }

    if let Some(first) = first {
        query = query
            .limit(first as u64)
            .order_by(Column::Date, Order::Asc)
            .order_by(Column::Id, Order::Asc);
    }

    if let Some(last) = last {
        query = query
            .limit(last as u64)
            .order_by(Column::Date, Order::Desc)
            .order_by(Column::Id, Order::Desc);
    }

    query
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
            .column(Column::Date)
            .column(Column::Id)
            .filter(Column::Published.eq(true))
            .order_by(Column::Date, order.clone())
            .order_by(Column::Id, order)
            .into_values::<(NaiveDate, u32), QueryMinMax>()
            .one(db)
            .await
            .map_err(db_error)
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

#[allow(clippy::too_many_arguments)]
pub async fn create_paginated_posts<C>(
    after: Option<String>,
    before: Option<String>,
    first: Option<i32>,
    last: Option<i32>,
    ctx: &Context<'_>,
    db: &DatabaseTransaction,
    condition: C,
    join: Option<RelationDef>,
) -> Result<Connection<PostCursor, Post, EmptyFields, EmptyFields>>
where
    C: IntoCondition,
{
    query(
        after,
        before,
        first,
        last,
        |after, before, first, last| async move {
            let mut query = build_paginated_posts(after, before, first, last);

            select_columns_connection!(ctx, query, Column);
            select_columns_connection!(ctx, query, 
                "author" => Column::AuthorId,
                "labels" => Column::Id);

            if let Some(join) = join {
                query = query.join_rev(JoinType::Join, join);
            }

            let mut res = query
                .filter(condition)
                .filter(Column::Published.eq(true))
                .into_model::<Post>()
                .all(db.deref())
                .await
                .map_err(db_error)?;

            res.sort_by(|a, b| b.date.cmp(&a.date));

            let (min, max) = get_published_posts_min_max_id(db.deref()).await?;

            let mut connection = get_connection(&res, min, max)?;

            connection.edges.extend(res.into_iter().map(|post| {
                let cursor = PostCursor::new(post.date.unwrap(), post.id.unwrap());
                Edge::new(cursor, post)
            }));

            Ok::<_, Error>(connection)
        },
    )
    .await
}
