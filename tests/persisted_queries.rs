//! Apollo automatic persisted queries and the Redis cache behind them.

use async_graphql::extensions::apollo_persisted_queries::CacheStorage;
use envconfig::Envconfig;
use redis::AsyncCommands;
use sha2::{Digest, Sha256};
use test_utils::{TestConfig, prelude::*};
use website_backend2::graphql::RedisCache;

/// The `extensions` object of a persisted-query request.
fn persisted_query(hash: &str) -> Value {
    json!({ "persistedQuery": { "version": 1, "sha256Hash": hash } })
}

fn sha256_hex(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

/// A query whose text, and therefore hash, no earlier run has sent, so Redis
/// cannot already hold it.
fn fresh_query() -> String {
    format!(
        "{{ {}: colleagues {{ name }} }}",
        unique_name("colleagues_")
    )
}

fn test_config() -> TestConfig {
    TestConfig::init_from_env().expect("test configuration")
}

async fn redis_connection(config: &TestConfig) -> redis::aio::MultiplexedConnection {
    redis::Client::open(config.redis_url.as_str())
        .expect("Redis URL")
        .get_multiplexed_async_connection()
        .await
        .expect("Redis connection")
}

#[tokio::test]
async fn unknown_hash_asks_the_client_for_the_query() {
    let app = TestApp::seeded().await;

    let response = app
        .graphql_request(json!({ "extensions": persisted_query(&sha256_hex(&fresh_query())) }))
        .await;

    assert_eq!(error_messages(&response), ["PersistedQueryNotFound"]);
}

#[tokio::test]
async fn registered_query_can_be_run_by_its_hash() {
    let app = TestApp::seeded().await;
    let query = fresh_query();
    let hash = sha256_hex(&query);

    let registered = app
        .graphql_request(json!({ "query": query, "extensions": persisted_query(&hash) }))
        .await;
    let by_hash = app
        .graphql_request(json!({ "extensions": persisted_query(&hash) }))
        .await;

    expect_data(&registered);
    assert_eq!(by_hash, registered);
}

#[tokio::test]
async fn query_with_a_mismatched_hash_is_rejected() {
    let app = TestApp::seeded().await;

    let response = app
        .graphql_request(json!({
            "query": fresh_query(),
            "extensions": persisted_query(&sha256_hex("a different query")),
        }))
        .await;

    assert_eq!(
        error_messages(&response),
        ["provided sha does not match query"]
    );
}

#[tokio::test]
async fn cache_returns_the_documents_it_stored() {
    let config = test_config();
    let cache = RedisCache::new(&config.redis_url).await.expect("cache");
    let key = unique_name("website_backend2_test:");
    let document =
        async_graphql_parser::parse_query("{ colleagues { name } }").expect("valid query");

    cache.set(key.clone(), document.clone()).await;
    let cached = cache.get(key.clone()).await.expect("document is cached");

    assert_eq!(
        serde_json::to_value(&cached).expect("serializable"),
        serde_json::to_value(&document).expect("serializable")
    );
    let () = redis_connection(&config)
        .await
        .del(&key)
        .await
        .expect("delete test key");
}

#[tokio::test]
async fn cache_evicts_entries_it_cannot_decode() {
    let config = test_config();
    let cache = RedisCache::new(&config.redis_url).await.expect("cache");
    let mut redis = redis_connection(&config).await;
    let key = unique_name("website_backend2_test:");
    let () = redis
        .set(&key, b"not a document".as_slice())
        .await
        .expect("store corrupt entry");

    assert!(cache.get(key.clone()).await.is_none());

    let exists: bool = redis.exists(&key).await.expect("check key");
    assert!(!exists, "the corrupt entry should have been deleted");
}
