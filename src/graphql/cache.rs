use async_graphql::{async_trait::async_trait, extensions::apollo_persisted_queries::CacheStorage};
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
    async fn get(&self, key: String) -> Option<String> {
        let mut conn = self.manager.clone();
        conn.get(key).await.ok()
    }

    async fn set(&self, key: String, query: String) {
        let mut conn = self.manager.clone();
        let _: RedisResult<()> = conn.set(key, query).await;
    }
}
