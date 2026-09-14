# Testing

This backend serves the public school website, so a change should be provably
safe before it is merged. The tests are organised in layers, cheapest first.

## Running the tests

```sh
cargo nextest run   # or: cargo test
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
API change. Confirm the frontend still works before accepting it, then update
the snapshot and commit it together with the code change:

```sh
INSTA_UPDATE=always cargo test   # or: cargo insta review
git diff src/graphql/snapshots/
```

In CI, insta never writes snapshots; any mismatch fails the build.

## Known bugs

A test describing correct behaviour that the code does not have yet is marked
`#[ignore = "bug: <description>"]`. It documents the bug without failing CI, and
the pull request that fixes the bug removes the `#[ignore]`.

```sh
cargo nextest list --run-ignored only   # list known bugs
cargo nextest run --run-ignored only    # run them (they are expected to fail)
```
