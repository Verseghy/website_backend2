//! Snapshot assertions for GraphQL responses.

/// Asserts that a GraphQL response matches its stored insta snapshot.
///
/// Works like `insta::assert_json_snapshot!`, except that object keys are
/// sorted before the comparison. The order of sibling fields in responses is
/// not stable (a CI run serialized aliased root fields in a different order than
/// local runs), and key order carries no meaning in JSON. Array order, which
/// does, is kept.
///
/// * `$response` - the response body, typically returned by
///   [`TestApp::graphql`](crate::TestApp::graphql).
///
/// As with insta's own macro, the snapshot is named after the calling test.
#[macro_export]
macro_rules! assert_response_snapshot {
    ($response:expr $(,)?) => {
        $crate::macro_support::insta::with_settings!({ sort_maps => true }, {
            $crate::macro_support::insta::assert_json_snapshot!($response);
        })
    };
}
