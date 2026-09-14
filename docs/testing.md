# Testing

This backend serves the public school website, so a change should be provably
safe before it is merged. The tests are organised in layers, cheapest first.

## Running the tests

Unit tests and the schema snapshot need nothing but Rust. The integration tests
also need MySQL and Redis, which `compose.yaml` provides:

```sh
docker compose up -d --wait         # or: podman compose up -d
cargo nextest run --workspace       # or: cargo test --workspace
```

CI runs the same suite (plus doctests) on every pull request. `Build`, `Format`,
`Clippy` and `Test` are required checks on `master`.

## Unit tests

Unit tests live next to the code in `#[cfg(test)] mod tests` blocks and need
neither MySQL nor Redis. They cover the logic that is easiest to get subtly
wrong:

| Area | Location |
| --- | --- |
| Month, calendar-grid and ISO-week bounds | `src/utils/date_range.rs` |
| `Date` and `DateTime` scalars, post cursors | `src/graphql/types/` |
| Page-info flags of post connections | `src/utils/paginate.rs` |
| Hungarian ordering of colleague names | `src/graphql/resolvers/colleagues.rs` |
| Post image URLs | `src/graphql/resolvers/posts.rs` |
| Configuration parsing | `src/lib.rs` |

Calendar and cursor logic also uses [proptest](https://docs.rs/proptest) to
check invariants, such as "an ISO week always runs from Monday to Sunday",
across many generated inputs.

When logic inside a resolver becomes non-trivial, move it into a pure function
and test that function directly, as `src/utils/date_range.rs` does.

## API schema snapshot

`src/graphql/mod.rs` snapshots the complete GraphQL schema (SDL) with
[insta](https://insta.rs). Adding, renaming or removing a field, or changing
its type or nullability, makes the test fail and prints the diff.

website_frontend2 is written against this schema, so a snapshot change is an
API change: confirm the frontend still works before accepting it.

## Integration tests

Integration tests live in `tests/` and use the harness in `test-utils/`. Each
test starts its own `TestApp`, which consists of:

- a new MySQL database named `test_<uuid>`, whose tables are generated from the
  sea-orm entities in `src/entity/`;
- optionally, the standard data set from `test-utils/src/seed.rs`;
- the real application router, middleware included, on a random local port.

The database is dropped when the test ends, so tests never see each other's
data and can run in parallel.

```rust
use test_utils::prelude::*;

#[tokio::test]
async fn colleagues_are_sorted_by_name() {
    let app = TestApp::seeded().await;

    let response = app.graphql("{ colleagues { name } }").await;

    assert_response_snapshot!(response);
}
```

`assert_response_snapshot!` is `insta::assert_json_snapshot!` with object keys
sorted: the order of sibling fields in responses is not stable between runs,
while array order, which tests rely on, is kept.

| File | Covers |
| --- | --- |
| `tests/frontend_queries.rs` | The exact operations website_frontend2 sends, copied to `tests/frontend/` |
| `tests/posts.rs` | Post lists, filters, search, preview tokens, archive, pagination |
| `tests/calendar.rs` | Canteen weeks and event months |
| `tests/content.rs` | Pages, menu, colleagues, authors, labels |
| `tests/http.rs` | Health checks, metrics, CORS, compression, limits, routing |
| `tests/persisted_queries.rs` | Apollo persisted queries and the Redis cache |

When website_frontend2 changes one of its operations, update the copy in
`tests/frontend/` in the same way.

### Schema from entities

The test schema is generated from the entities, so an entity that disagrees
with production makes the tests disagree with production too. When a migration
in `migrations/` changes a table, change its entity in the same pull request,
and add new entities to `create_tables` in `test-utils/src/database.rs`.

Test databases use the `utf8mb4_unicode_ci` collation of the production tables,
which is what makes searches ignore case and accents.

### Seed data

`test-utils/src/seed.rs` is a small, fixed data set covering the cases the
resolvers treat differently; its module documentation lists them, and its
constants name the rows tests refer to. Changing it changes many snapshots, so
a test that needs an unusual row should insert it through `app.db()` instead.

### Configuration

| Variable | Default (matches `compose.yaml`) |
| --- | --- |
| `TEST_MYSQL_URL` | `mysql://root:secret@127.0.0.1:33061` (a server URL without a database name) |
| `TEST_REDIS_URL` | `redis://127.0.0.1:6379` |

A test run that is killed leaves its `test_*` databases behind. They are
harmless, and recreating the container removes them:

```sh
docker compose up -d --wait --force-recreate database
```

## Updating snapshots

Snapshot files live in `snapshots/` directories next to the tests that own
them. When a change is intended, re-run the tests with `INSTA_UPDATE=always`
(or use `cargo insta review`), check the diff, and commit the updated `.snap`
files together with the code change:

```sh
INSTA_UPDATE=always cargo test --workspace
git diff -- '*.snap'
```

In CI, insta never writes snapshots; any mismatch fails the build.

## Known bugs

A test describing correct behaviour that the code does not have yet is marked
`#[ignore = "bug: <description>"]`. It documents the bug without failing CI, and
the pull request that fixes the bug removes the `#[ignore]`. Snapshots record
current behaviour, so that pull request also updates the affected snapshots.

```sh
cargo nextest list --workspace --run-ignored only   # list known bugs
cargo nextest run --workspace --run-ignored only    # run them (they are expected to fail)
```
