//! Pages, the navigation menu, colleagues, authors and labels.

use test_utils::prelude::*;

#[tokio::test]
async fn colleagues_are_sorted_by_name_in_hungarian_order() {
    let app = TestApp::seeded().await;

    let response = app
        .graphql("{ colleagues { id name jobs subjects roles awards image category } }")
        .await;

    assert_response_snapshot!(response);
}

#[tokio::test]
async fn page_is_found_by_slug() {
    let app = TestApp::seeded().await;

    let response = app
        .graphql_with(
            "query ($slug: String!) {
                page(slug: $slug) { id template name title content extras }
                missing: page(slug: \"nincs-ilyen\") { id }
            }",
            json!({ "slug": seed::PAGE_SLUG }),
        )
        .await;

    assert_response_snapshot!(response);
}

#[tokio::test]
#[ignore = "bug: soft-deleted pages are still served"]
async fn soft_deleted_page_is_not_served() {
    let app = TestApp::seeded().await;

    let response = app
        .graphql_with(
            "query ($slug: String!) { page(slug: $slug) { id } }",
            json!({ "slug": seed::DELETED_PAGE_SLUG }),
        )
        .await;

    assert_eq!(expect_data(&response)["page"], Value::Null);
}

#[tokio::test]
async fn menu_is_a_tree_in_position_order() {
    let app = TestApp::seeded().await;

    let response = app
        .graphql(
            "{ menu { name type link slug children { name type link slug children { name } } } }",
        )
        .await;

    assert_response_snapshot!(response);
}

#[tokio::test]
#[ignore = "bug: a menu item's slug is null unless its link is selected too"]
async fn menu_slug_does_not_depend_on_selecting_link() {
    // "Kapcsolat" links to the page with the slug "kapcsolat".
    let app = TestApp::seeded().await;

    let response = app.graphql("{ menu { name slug } }").await;

    let contact = expect_data(&response)["menu"]
        .as_array()
        .expect("menu")
        .iter()
        .find(|item| item["name"] == "Kapcsolat")
        .expect("seeded menu item");
    assert_eq!(contact["slug"], "kapcsolat");
}

#[tokio::test]
async fn soft_deleted_menu_items_are_not_listed() {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, sea_query::Expr};
    use website_backend2::entity::menu_items;

    let app = TestApp::seeded().await;
    // The seed data only deletes a top-level item. Also delete "Régi oldal"
    // (id 3), a child of "Iskolánk", to cover `children`.
    menu_items::Entity::update_many()
        .col_expr(
            menu_items::Column::DeletedAt,
            Expr::current_timestamp().into(),
        )
        .filter(menu_items::Column::Id.eq(3))
        .exec(app.db())
        .await
        .expect("delete child menu item");

    let response = app.graphql("{ menu { name children { name } } }").await;

    let names = |items: &Value| -> Vec<String> {
        items
            .as_array()
            .expect("menu items")
            .iter()
            .filter_map(|item| item["name"].as_str().map(str::to_owned))
            .collect()
    };
    let menu = &expect_data(&response)["menu"];
    assert!(
        !names(menu).contains(&seed::DELETED_MENU_ITEM.to_owned()),
        "{menu:#}"
    );
    assert_eq!(names(&menu[0]["children"]), ["Rólunk"], "{menu:#}");
}

#[tokio::test]
async fn authors_are_found_by_id() {
    let app = TestApp::seeded().await;

    let response = app
        .graphql(
            "{
                with_image: author(id: 1) { id name description image }
                without_image: author(id: 2) { id name description image }
                missing: author(id: 99) { id }
            }",
        )
        .await;

    assert_response_snapshot!(response);
}

#[tokio::test]
async fn labels_are_found_by_id() {
    let app = TestApp::seeded().await;

    let response = app
        .graphql("{ label(id: 1) { id name color } missing: label(id: 99) { id } }")
        .await;

    assert_response_snapshot!(response);
}
