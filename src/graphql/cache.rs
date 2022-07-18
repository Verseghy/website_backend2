use async_graphql::{async_trait::async_trait, extensions::apollo_persisted_queries::CacheStorage};
use async_graphql_parser::types::ExecutableDocument;
use redis::{aio::ConnectionManager, AsyncCommands, RedisResult};

#[derive(Clone)]
pub struct RedisCache {
    manager: ConnectionManager,
}

impl RedisCache {
    pub async fn new() -> RedisResult<Self> {
        let url =
            std::env::var("REDIS_URL").expect("Could not find REDIS_URL environment variable");
        let client = redis::Client::open(url)?;

        Ok(Self {
            manager: ConnectionManager::new(client).await?,
        })
    }
}

#[async_trait]
impl CacheStorage for RedisCache {
    async fn get(&self, key: String) -> Option<ExecutableDocument> {
        let mut conn = self.manager.clone();

        let res: Vec<u8> = conn.get(&key).await.ok()?;
        match bincode::deserialize(&res) {
            Ok(document) => document,
            Err(_) => {
                let _: RedisResult<()> = conn.del(&key).await;
                None
            }
        }
    }

    async fn set(&self, key: String, document: ExecutableDocument) {
        let mut conn = self.manager.clone();

        if let Ok(data) = bincode::serialize(&document) {
            let _: RedisResult<()> = conn.set(key, &data).await;
        } else {
            tracing::warn!("cache: failed to serialize ExecutableDocument")
        }
    }
}
