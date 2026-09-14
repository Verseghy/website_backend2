//! The GraphQL operations website_frontend2 sends, run against the seed data.
//!
//! The documents in `tests/frontend/` are copied from website_frontend2's
//! `src/data/*.data.ts` files with their fragments inlined, and the variables
//! below mirror what the site passes. When the frontend changes an operation,
//! update the copy so the backend stays tested against what the site requests.

use test_utils::prelude::*;

async fn run(document: &str, variables: Value) -> Value {
    let app = TestApp::seeded().await;

    app.graphql_with(document, variables).await
}

#[tokio::test]
async fn home_page() {
    let response = run(
        include_str!("frontend/home.graphql"),
        json!({ "first": 21, "numFeaturedPosts": 20 }),
    )
    .await;

    assert_response_snapshot!(response);
}

#[tokio::test]
async fn post_page() {
    let response = run(include_str!("frontend/post.graphql"), json!({ "id": 1 })).await;

    assert_response_snapshot!(response);
}

#[tokio::test]
async fn post_page_of_an_unpublished_post_finds_nothing() {
    // The frontend redirects to its 404 page when `post` is null.
    let response = run(
        include_str!("frontend/post.graphql"),
        json!({ "id": seed::UNPUBLISHED_POST }),
    )
    .await;

    assert_eq!(expect_data(&response)["post"], Value::Null);
}

#[tokio::test]
async fn search_page_by_term() {
    let response = run(
        include_str!("frontend/search_term.graphql"),
        json!({ "term": "ünnep" }),
    )
    .await;

    assert_response_snapshot!(response);
}

#[tokio::test]
async fn search_page_by_author() {
    let response = run(
        include_str!("frontend/search_author.graphql"),
        json!({ "authorID": seed::AUTHOR_WITH_IMAGE }),
    )
    .await;

    assert_response_snapshot!(response);
}

#[tokio::test]
async fn search_page_by_label() {
    let response = run(
        include_str!("frontend/search_label.graphql"),
        json!({ "labelID": seed::LABEL_NEWS }),
    )
    .await;

    assert_response_snapshot!(response);
}

#[tokio::test]
async fn canteen_page() {
    // The frontend asks for the current and the next ISO week.
    let response = run(
        include_str!("frontend/canteen.graphql"),
        json!({ "year1": 2026, "week1": 1, "year2": 2026, "week2": 2 }),
    )
    .await;

    assert_response_snapshot!(response);
}

#[tokio::test]
async fn colleagues_page() {
    let response = run(include_str!("frontend/colleagues.graphql"), json!({})).await;

    assert_response_snapshot!(response);
}

#[tokio::test]
async fn information_page_menu() {
    let response = run(include_str!("frontend/information_menu.graphql"), json!({})).await;

    assert_response_snapshot!(response);
}

#[tokio::test]
async fn page() {
    let response = run(
        include_str!("frontend/page.graphql"),
        json!({ "slug": seed::PAGE_SLUG }),
    )
    .await;

    assert_response_snapshot!(response);
}
