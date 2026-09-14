//! Date-based queries: canteen menus by ISO week and events by month.

use test_utils::prelude::*;

#[tokio::test]
async fn canteen_week_spanning_new_year() {
    // ISO week 1 of 2026 runs from Monday 29 December 2025 to Sunday 4 January 2026.
    let app = TestApp::seeded().await;

    let response = app
        .graphql("{ canteen(year: 2026, week: 1) { id date menus { id menu type } } }")
        .await;

    assert_response_snapshot!(response);
}

#[tokio::test]
async fn canteen_week_without_menus_is_empty() {
    let app = TestApp::seeded().await;

    let response = app
        .graphql("{ canteen(year: 2026, week: 30) { date } }")
        .await;

    assert_eq!(expect_data(&response)["canteen"], json!([]));
}

#[tokio::test]
#[ignore = "bug: a canteen week that does not exist panics and returns HTTP 500"]
async fn nonexistent_canteen_week_is_a_graphql_error() {
    let app = TestApp::seeded().await;

    let response = app
        .post("/graphql")
        .json(&json!({ "query": "{ canteen(year: 2026, week: 0) { date } }" }))
        .send()
        .await
        .expect("request");

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = response.json().await.expect("JSON body");
    assert!(!error_messages(&body).is_empty(), "{body:#}");
}

#[tokio::test]
async fn events_of_a_month_cover_its_calendar_grid() {
    // September 2026's grid runs from Monday 31 August to Sunday 4 October.
    let app = TestApp::seeded().await;

    let response = app
        .graphql("{ events(year: 2026, month: 9) { id title dateFrom dateTo description color } }")
        .await;

    assert_response_snapshot!(response);
}

#[tokio::test]
async fn events_include_the_last_day_of_the_calendar_grid() {
    let app = TestApp::seeded().await;

    let response = app.graphql("{ events(year: 2026, month: 9) { id } }").await;

    let ids: Vec<u64> = expect_data(&response)["events"]
        .as_array()
        .expect("events")
        .iter()
        .filter_map(|event| event["id"].as_u64())
        .collect();
    assert!(
        ids.contains(&u64::from(seed::EVENT_ON_LAST_GRID_DAY)),
        "{ids:?}"
    );
}

#[tokio::test]
async fn events_of_an_invalid_month_are_a_graphql_error() {
    let app = TestApp::seeded().await;

    let response = app
        .graphql("{ events(year: 2026, month: 13) { id } }")
        .await;

    assert_eq!(error_messages(&response), ["invalid date"]);
}
