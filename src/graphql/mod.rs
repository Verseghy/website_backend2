//! The GraphQL API: schema construction, resolvers and custom scalar types.

mod cache;
pub mod resolvers;
pub mod types;

pub use cache::RedisCache;

use async_graphql::{
    EmptyMutation, EmptySubscription, MergedObject,
    extensions::{
        Analyzer,
        apollo_persisted_queries::{ApolloPersistedQueries, CacheStorage},
    },
};
use resolvers::{
    ArchiveQuery, AuthorsQuery, CanteenQuery, ColleaguesQuery, EventsQuery, LabelQuery, MenuQuery,
    PagesQuery, PostsQuery,
};

/// Root query type, merged from the per-resource query objects.
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

/// The schema served at `/graphql`. The API is read-only, so it has no
/// mutations or subscriptions.
pub type Schema = async_graphql::Schema<Query, EmptyMutation, EmptySubscription>;

/// Highest complexity a single query may have, bounding the work one request
/// can trigger.
const COMPLEXITY_LIMIT: usize = 256;

/// Builds the GraphQL schema.
///
/// * `persisted_query_cache` - storage for Apollo automatic persisted queries.
///   Production passes a [`RedisCache`] so every replica shares the cache;
///   code that has no Redis available (such as unit tests) can pass
///   [`LruCacheStorage`](async_graphql::extensions::apollo_persisted_queries::LruCacheStorage).
pub fn build_schema(persisted_query_cache: impl CacheStorage) -> Schema {
    let schema = Schema::build(Query::default(), EmptyMutation, EmptySubscription)
        .extension(Analyzer)
        .extension(ApolloPersistedQueries::new(persisted_query_cache))
        .limit_complexity(COMPLEXITY_LIMIT)
        .finish();

    tracing::info!("Schema created");
    schema
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::extensions::apollo_persisted_queries::LruCacheStorage;

    /// Guards the public API contract. website_frontend2 is written against this
    /// schema, so a changed snapshot is a client-visible change: make sure the
    /// frontend still works before accepting it (see `docs/testing.md`).
    #[test]
    fn schema_sdl() {
        let schema = build_schema(LruCacheStorage::new(1));

        insta::assert_snapshot!(schema.sdl());
    }
}
