use async_graphql::{async_trait::async_trait, extensions::apollo_persisted_queries::CacheStorage};
use redis::{Commands, Connection, RedisResult};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct RedisCache {
    conn: Arc<Mutex<Connection>>,
}

impl RedisCache {
    pub fn new(uri: &str) -> RedisResult<Self> {
        let client = redis::Client::open(uri)?;
        let conn = client.get_connection()?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[async_trait]
impl CacheStorage for RedisCache {
    async fn get(&self, key: String) -> Option<String> {
        let mut lock = self.conn.lock().unwrap();
        lock.get(key).ok()
    }

    async fn set(&self, key: String, query: String) {
        let mut lock = self.conn.lock().unwrap();
        let _: RedisResult<()> = lock.set(key, query);
    }
}
