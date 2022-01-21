mod cache;
mod resolvers;
mod types;

use async_graphql::{
    extensions::apollo_persisted_queries::ApolloPersistedQueries, EmptyMutation, EmptySubscription,
    MergedObject,
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
    let redis_url =
        std::env::var("REDIS_URL").expect("Could not find REDIS_URL environment variable");
    let cache = RedisCache::new(redis_url.as_str()).expect("Could not create redis cache");

    let schema = Schema::build(Query::default(), EmptyMutation, EmptySubscription)
        .extension(ApolloPersistedQueries::new(cache))
        .finish();

    tracing::info!("Schema created");
    schema
}
