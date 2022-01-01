mod utils;

mod canteen;

use self::canteen::CanteenQuery;
use crate::database;
use async_graphql::{EmptyMutation, EmptySubscription, MergedObject};

#[derive(MergedObject, Default)]
pub struct Query(CanteenQuery);

pub type Schema = async_graphql::Schema<Query, EmptyMutation, EmptySubscription>;

pub async fn create_schema() -> Schema {
    let db = database::connect().await;
    Schema::build(Query::default(), EmptyMutation, EmptySubscription)
        .data(db)
        .finish()
}
