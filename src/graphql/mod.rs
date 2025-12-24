mod cache;
pub mod resolvers;
pub mod types;

use async_graphql::{
    EmptyMutation, EmptySubscription, MergedObject,
    extensions::{Analyzer, apollo_persisted_queries::ApolloPersistedQueries},
};
use cache::RedisCache;
use resolvers::{
    ArchiveQuery, AuthorsQuery, CanteenQuery, ColleaguesQuery, EventsQuery, LabelQuery, MenuQuery,
    PagesQuery, PostsQuery,
};

#[derive(MergedObject, Default)]
pub struct Query(
    CanteenQuery,
    ColleaguesQuery,
    EventsQuery,
    PagesQuery,
    AuthorsQuery,
    PostsQuery,
    LabelQuery,
    MenuQuery,
    ArchiveQuery,
);

pub type Schema = async_graphql::Schema<Query, EmptyMutation, EmptySubscription>;

pub async fn create_schema() -> Schema {
    let cache = RedisCache::new()
        .await
        .expect("Could not create redis cache");

    let schema = Schema::build(Query::default(), EmptyMutation, EmptySubscription)
        .extension(Analyzer)
        .extension(ApolloPersistedQueries::new(cache))
        .limit_complexity(64)
        .finish();

    tracing::info!("Schema created");
    schema
}
