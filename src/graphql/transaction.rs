use async_graphql::{
    async_trait::async_trait,
    extensions::{Extension, ExtensionContext, ExtensionFactory, NextRequest},
    Response,
};
use sea_orm::{ConnectionTrait, DatabaseConnection};
use std::sync::Arc;

pub struct Transaction;

impl ExtensionFactory for Transaction {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(TransactionExtension)
    }
}

struct TransactionExtension;

#[async_trait]
impl Extension for TransactionExtension {
    async fn request(&self, ctx: &ExtensionContext<'_>, next: NextRequest<'_>) -> Response {
        let db: &DatabaseConnection = ctx.data().unwrap();

        let transaction = db.begin().await.unwrap();
        let res = next.run(ctx).await;
        transaction.commit().await.unwrap();

        res
    }
}
