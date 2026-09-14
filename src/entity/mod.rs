//! SeaORM entities for the tables the API reads.
//!
//! The tables were created by the earlier Laravel backend and are migrated with
//! the SQL files in `migrations/`. The entities describe their production shape:
//! unsigned integer ids, string lengths, nullability and column types. The
//! integration tests generate their schema from these definitions (see
//! `test-utils/src/database.rs`), so a schema change in production has to be
//! mirrored here, and a new entity also has to be added to the table list there.
//!
//! The unsigned types matter beyond the tests: sqlx only decodes `u32`/`u16`
//! from `UNSIGNED` columns, and the resolvers read ids as `u32`. Where
//! production uses `LONGTEXT`, the entities declare `TEXT`; both decode to
//! `String`.

pub mod prelude;

pub mod canteen_data;
pub mod canteen_menus;
pub mod canteen_pivot_menus_data;
pub mod colleagues_data;
pub mod events_data;
pub mod menu_items;
pub mod pages;
pub mod posts_authors;
pub mod posts_data;
pub mod posts_labels;
pub mod posts_pivot_labels_data;
