//! Listing, searching, looking up and paginating posts.

use test_utils::prelude::*;

/// Node ids of the connection at `data.<field>`.
fn ids_of(response: &Value, field: &str) -> Vec<u32> {
    node_ids(&expect_data(response)[field])
}

#[tokio::test]
async fn published_posts_are_listed_newest_first() {
    let app = TestApp::seeded().await;

    let response = app
        .graphql("{ posts(first: 50) { edges { node { id } } } }")
        .await;

    assert_eq!(
        ids_of(&response, "posts"),
        seed::PUBLISHED_POSTS_NEWEST_FIRST
    );
}

#[tokio::test]
async fn last_also_returns_the_newest_posts() {
    // Unlike Relay connections, `last` does not count from the oldest post:
    // website_frontend2's search asks for `last: 20` and shows the newest.
    let app = TestApp::seeded().await;

    let response = app
        .graphql("{ posts(last: 2) { edges { node { id } } } }")
        .await;

    assert_eq!(ids_of(&response, "posts"), [1, 2]);
}

#[tokio::test]
async fn featured_filter_lists_only_featured_posts() {
    let app = TestApp::seeded().await;

    let response = app
        .graphql("{ posts(featured: true, first: 50) { edges { node { id } } } }")
        .await;

    assert_eq!(
        ids_of(&response, "posts"),
        seed::FEATURED_POSTS_NEWEST_FIRST
    );
}

#[tokio::test]
async fn search_matches_title_description_and_content() {
    // "ünnep" appears in post 1's title and content, in post 5's description
    // and in the content of the unpublished post 4.
    let app = TestApp::seeded().await;

    let response = app
        .graphql(r#"{ search(term: "ünnep", first: 50) { edges { node { id } } } }"#)
        .await;

    assert_eq!(ids_of(&response, "search"), [1, 5]);
}

#[tokio::test]
async fn search_ignores_case_and_accents() {
    // Follows from the utf8mb4_unicode_ci collation of the production tables.
    let app = TestApp::seeded().await;

    let response = app
        .graphql(r#"{ search(term: "UNNEP", first: 50) { edges { node { id } } } }"#)
        .await;

    assert_eq!(ids_of(&response, "search"), [1, 5]);
}

#[tokio::test]
async fn unpublished_posts_are_not_listed_anywhere() {
    let app = TestApp::seeded().await;

    let response = app
        .graphql(
            r#"{
                posts(first: 50) { edges { node { id } } }
                featured: posts(featured: true, first: 50) { edges { node { id } } }
                search(term: "szalagavató", first: 50) { edges { node { id } } }
                label(id: 1) { posts(first: 50) { edges { node { id } } } }
                author(id: 1) { posts(first: 50) { edges { node { id } } } }
                archive { posts(year: 2026, month: 9) { id } }
            }"#,
        )
        .await;

    let data = expect_data(&response);
    for connection in [
        &data["posts"],
        &data["featured"],
        &data["search"],
        &data["label"]["posts"],
        &data["author"]["posts"],
    ] {
        assert!(
            !node_ids(connection).contains(&seed::UNPUBLISHED_POST),
            "{connection:#}"
        );
    }
    assert_eq!(data["archive"]["posts"], json!([{ "id": 1 }]));
}

#[tokio::test]
async fn archive_counts_published_posts_per_month_newest_first() {
    let app = TestApp::seeded().await;

    let response = app
        .graphql("{ archive { info { year month count } } }")
        .await;

    assert_response_snapshot!(response);
}

#[tokio::test]
async fn archive_lists_the_published_posts_of_a_month() {
    let app = TestApp::seeded().await;

    let response = app
        .graphql(
            "{ archive {
                december: posts(year: 2025, month: 12) { id title date }
                september: posts(year: 2026, month: 9) { id title date }
                empty: posts(year: 2026, month: 4) { id }
            } }",
        )
        .await;

    assert_response_snapshot!(response);
}

#[tokio::test]
async fn published_post_is_found_by_id() {
    let app = TestApp::seeded().await;

    let response = app
        .graphql("{ post(id: 1) { id title } missing: post(id: 99) { id } }")
        .await;

    assert_eq!(
        expect_data(&response),
        &json!({ "post": { "id": 1, "title": "Tanévnyitó ünnepség" }, "missing": null })
    );
}

#[tokio::test]
async fn unpublished_post_is_only_found_with_its_preview_token() {
    let app = TestApp::seeded().await;
    let query = "query ($token: String) { post(id: 4, token: $token) { id } }";

    for token in [Value::Null, json!("wrong-token")] {
        let response = app.graphql_with(query, json!({ "token": token })).await;

        assert_eq!(expect_data(&response)["post"], Value::Null, "token {token}");
    }

    let response = app
        .graphql_with(query, json!({ "token": seed::PREVIEW_TOKEN }))
        .await;

    assert_eq!(expect_data(&response)["post"]["id"], seed::UNPUBLISHED_POST);
}

#[tokio::test]
async fn preview_token_only_finds_unpublished_posts() {
    // A token switches the lookup to unpublished posts, so a published post
    // is not found with one, not even with a valid token.
    let app = TestApp::seeded().await;

    let response = app
        .graphql_with(
            "query ($token: String) { post(id: 1, token: $token) { id } }",
            json!({ "token": seed::PREVIEW_TOKEN }),
        )
        .await;

    assert_eq!(expect_data(&response)["post"], Value::Null);
}

#[tokio::test]
async fn author_and_label_post_lists_are_filtered() {
    let app = TestApp::seeded().await;

    let response = app
        .graphql(
            "{
                sport: label(id: 3) { posts(first: 50) { edges { node { id } } } }
                unused_label: label(id: 4) { posts(first: 50) { edges { node { id } } } }
                author: author(id: 2) { posts(first: 50) { edges { node { id } } } }
                author_without_posts: author(id: 3) { posts(first: 50) { edges { node { id } } } }
            }",
        )
        .await;

    let data = expect_data(&response);
    assert_eq!(node_ids(&data["sport"]["posts"]), [2]);
    assert_eq!(node_ids(&data["unused_label"]["posts"]), Vec::<u32>::new());
    assert_eq!(
        node_ids(&data["author"]["posts"]),
        [2, seed::BACKDATED_POST]
    );
    assert_eq!(
        node_ids(&data["author_without_posts"]["posts"]),
        Vec::<u32>::new()
    );
}

#[tokio::test]
#[ignore = "bug: cursor pagination skips posts with a higher id than the cursor's post but an older date"]
async fn paging_through_posts_visits_every_published_post_once() {
    let app = TestApp::seeded().await;
    let query = "query ($after: String) {
        posts(first: 2, after: $after) {
            edges { node { id } }
            pageInfo { endCursor hasPreviousPage }
        }
    }";

    let mut seen = Vec::new();
    let mut after = Value::Null;
    for _ in 0..seed::PUBLISHED_POSTS_NEWEST_FIRST.len() {
        let response = app.graphql_with(query, json!({ "after": after })).await;
        let posts = &expect_data(&response)["posts"];
        seen.extend(node_ids(posts));

        // In this API `hasPreviousPage` means "older posts exist".
        if posts["pageInfo"]["hasPreviousPage"] != true {
            break;
        }
        after = posts["pageInfo"]["endCursor"].clone();
    }

    assert_eq!(seen, seed::PUBLISHED_POSTS_NEWEST_FIRST);
}

#[tokio::test]
#[ignore = "bug: pageInfo of filtered post lists is computed from all published posts"]
async fn page_info_of_a_filtered_list_reflects_the_filter() {
    // Post 2 is the only post labelled "Sport", so there is nothing before or after it.
    let app = TestApp::seeded().await;

    let response = app
        .graphql(
            "{ label(id: 3) { posts(first: 50) { pageInfo { hasPreviousPage hasNextPage } } } }",
        )
        .await;

    assert_eq!(
        expect_data(&response)["label"]["posts"]["pageInfo"],
        json!({ "hasPreviousPage": false, "hasNextPage": false })
    );
}

#[tokio::test]
#[ignore = "bug: listing posts returns an error instead of an empty list when no post is published"]
async fn listing_posts_without_published_posts_returns_an_empty_list() {
    let app = TestApp::empty().await;

    let response = app
        .graphql("{ posts(first: 10) { edges { node { id } } } }")
        .await;

    assert_eq!(ids_of(&response, "posts"), Vec::<u32>::new());
}
