use async_graphql::{async_trait::async_trait, extensions::apollo_persisted_queries::CacheStorage};
use async_graphql_parser::types::ExecutableDocument;
use redis::{AsyncCommands, RedisResult, aio::ConnectionManager};

/// Apollo persisted-query cache stored in Redis.
///
/// Parsed query documents are stored as JSON under the hash the client sent,
/// so every replica of the service shares one cache. Entries that cannot be
/// deserialized are deleted and treated as a cache miss.
#[derive(Clone)]
pub struct RedisCache {
    manager: ConnectionManager,
}

impl RedisCache {
    /// Connects to Redis.
    ///
    /// * `url` - Redis connection URL, e.g. `redis://127.0.0.1`.
    ///
    /// The underlying connection manager reconnects on its own if the
    /// connection drops later.
    pub async fn new(url: &str) -> RedisResult<Self> {
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

        match serde_json::from_slice(&res) {
            Ok(document) => document,
            Err(_) => {
                let _: RedisResult<()> = conn.del(&key).await;
                None
            }
        }
    }

    async fn set(&self, key: String, document: ExecutableDocument) {
        let mut conn = self.manager.clone();

        let Ok(data) = serde_json::to_vec(&document) else {
            tracing::warn!("cache: failed to serialize ExecutableDocument");
            return;
        };

        let _: RedisResult<()> = conn.set(key, &data).await;
    }
}
