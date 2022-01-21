mod resolvers;
mod types;

use async_graphql::{
    extensions::apollo_persisted_queries::{ApolloPersistedQueries, LruCacheStorage},
    EmptyMutation, EmptySubscription, MergedObject,
};
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
    let schema = Schema::build(Query::default(), EmptyMutation, EmptySubscription)
        .extension(ApolloPersistedQueries::new(LruCacheStorage::new(64)))
        .finish();

    tracing::info!("Schema created");
    schema
}
