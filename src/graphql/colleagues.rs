use crate::{
    entity::colleagues_data::{Column, Entity as ColleaguesData},
    select_columns,
    utils::Maybe,
};
use async_graphql::{Context, Error, Object, Result, SimpleObject};
use sea_orm::{prelude::*, query::QuerySelect, DatabaseConnection, FromQueryResult};
use std::cmp::Ordering;

#[derive(SimpleObject, Debug, FromQueryResult)]
pub struct Colleague {
    pub id: Maybe<u32>,
    pub name: Maybe<String>,
    pub jobs: Maybe<String>,
    pub subjects: Maybe<String>,
    pub roles: Maybe<String>,
    pub awards: Maybe<String>,
    pub image: Maybe<String>,
    pub category: Maybe<i16>,
}

#[derive(Default)]
pub struct ColleaguesQuery;

#[Object]
impl ColleaguesQuery {
    async fn colleagues(&self, ctx: &Context<'_>) -> Result<Vec<Colleague>> {
        let db: &DatabaseConnection = ctx.data().unwrap();
        let mut query = ColleaguesData::find().select_only();

        select_columns!(ctx, query,
            "id" => Column::Id,
            "name" => Column::Name,
            "jobs" => Column::Jobs,
            "subjects" => Column::Subjects,
            "roles" => Column::Roles,
            "awards" => Column::Awards,
            "image" => Column::Image,
            "category" => Column::Category);

        let mut res = query
            .into_model::<Colleague>()
            .all(db)
            .await
            .map_err(|_| Error::new("database error"))?;

        if ctx.look_ahead().field("name").exists() {
            res.sort_by(|a, b| {
                let a = a.name.as_ref().unwrap();
                let b = b.name.as_ref().unwrap();

                let a_name = if a.starts_with("Dr. ") {
                    &a[4..]
                } else {
                    &a[..]
                };

                let b_name = if b.starts_with("Dr. ") {
                    &b[4..]
                } else {
                    &b[..]
                };

                a_name.cmp(b_name)
            });
        }

        Ok(res)
    }
}
