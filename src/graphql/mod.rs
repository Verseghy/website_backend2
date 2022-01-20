mod resolvers;
mod types;

use async_graphql::{extensions::ApolloTracing, EmptyMutation, EmptySubscription, MergedObject};
use resolvers::{
    AuthorsQuery, CanteenQuery, ColleaguesQuery, EventsQuery, LabelQuery, MenuQuery, PagesQuery,
    PostsQuery,
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
);

pub type Schema = async_graphql::Schema<Query, EmptyMutation, EmptySubscription>;

pub async fn create_schema() -> Schema {
    let schema = Schema::build(Query::default(), EmptyMutation, EmptySubscription).finish();

    tracing::info!("Schema created");
    schema
}
