mod resolvers;
mod transaction;
mod types;

use self::transaction::Transaction;
use crate::database;
use async_graphql::{extensions::ApolloTracing, EmptyMutation, EmptySubscription, MergedObject};
use resolvers::{CanteenQuery, ColleaguesQuery, EventsQuery, PagesQuery};

#[derive(MergedObject, Default)]
pub struct Query(CanteenQuery, ColleaguesQuery, EventsQuery, PagesQuery);

pub type Schema = async_graphql::Schema<Query, EmptyMutation, EmptySubscription>;

pub async fn create_schema() -> Schema {
    let db = database::connect().await;
    let schema = Schema::build(Query::default(), EmptyMutation, EmptySubscription)
        .data(db)
        .extension(Transaction)
        .extension(ApolloTracing)
        .finish();

    tracing::info!("Schema created");
    schema
}
