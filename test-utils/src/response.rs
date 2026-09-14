//! Helpers for inspecting GraphQL responses.

use serde_json::Value;
use uuid::Uuid;

/// Returns the `data` of a GraphQL response.
///
/// * `response` - a body returned by [`TestApp::graphql`](crate::TestApp::graphql)
///   or its variants.
///
/// # Panics
///
/// Panics and prints the response if it contains errors.
pub fn expect_data(response: &Value) -> &Value {
    assert!(
        response.get("errors").is_none(),
        "unexpected GraphQL errors: {response:#}"
    );

    &response["data"]
}

/// Returns the messages of a GraphQL response's errors, in order.
///
/// * `response` - a body returned by [`TestApp::graphql`](crate::TestApp::graphql)
///   or its variants.
///
/// The result is empty if the response has no errors.
pub fn error_messages(response: &Value) -> Vec<&str> {
    response["errors"]
        .as_array()
        .map(|errors| {
            errors
                .iter()
                .filter_map(|error| error["message"].as_str())
                .collect()
        })
        .unwrap_or_default()
}

/// Returns the node ids of a connection, in order.
///
/// * `connection` - a connection object selected with at least
///   `edges { node { id } }`.
///
/// # Panics
///
/// Panics if `connection` has no `edges` array or a node has no numeric id.
pub fn node_ids(connection: &Value) -> Vec<u32> {
    connection["edges"]
        .as_array()
        .unwrap_or_else(|| panic!("not a connection with edges: {connection:#}"))
        .iter()
        .map(|edge| {
            edge["node"]["id"]
                .as_u64()
                .and_then(|id| u32::try_from(id).ok())
                .unwrap_or_else(|| panic!("edge without a numeric node id: {edge:#}"))
        })
        .collect()
}

/// Returns `prefix` followed by a random suffix no other test run uses.
///
/// * `prefix` - readable start of the name; it must itself be valid wherever
///   the name is used (a GraphQL alias, a Redis key, ...).
///
/// Use it for values that end up in services shared between tests, such as
/// Redis.
pub fn unique_name(prefix: &str) -> String {
    format!("{prefix}{}", Uuid::new_v4().simple())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn data_of_a_successful_response() {
        let response = json!({ "data": { "posts": [] } });

        assert_eq!(expect_data(&response), &json!({ "posts": [] }));
    }

    #[test]
    #[should_panic(expected = "unexpected GraphQL errors")]
    fn data_of_a_failed_response_panics() {
        expect_data(&json!({ "data": null, "errors": [{ "message": "boom" }] }));
    }

    #[test]
    fn error_messages_are_listed_in_order() {
        let response = json!({ "errors": [{ "message": "first" }, { "message": "second" }] });

        assert_eq!(error_messages(&response), ["first", "second"]);
        assert!(error_messages(&json!({ "data": {} })).is_empty());
    }

    #[test]
    fn node_ids_are_read_in_order() {
        let connection = json!({ "edges": [{ "node": { "id": 3 } }, { "node": { "id": 1 } }] });

        assert_eq!(node_ids(&connection), [3, 1]);
    }

    #[test]
    #[should_panic(expected = "not a connection")]
    fn node_ids_of_something_else_panics() {
        node_ids(&json!({ "id": 1 }));
    }

    #[test]
    fn unique_names_keep_their_prefix_and_differ() {
        let first = unique_name("alias_");
        let second = unique_name("alias_");

        assert!(first.starts_with("alias_"));
        assert_ne!(first, second);
    }
}
