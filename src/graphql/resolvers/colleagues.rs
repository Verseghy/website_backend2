use crate::{
    Config,
    entity::colleagues_data::{Column, Entity as ColleaguesData},
    select_columns,
    utils::{Maybe, db_error},
};
use async_graphql::{ComplexObject, Context, Object, Result, SimpleObject};
use icu_collator::{Collator, CollatorBorrowed, options::CollatorOptions};
use icu_locale_core::locale;
use prometheus::{IntCounterVec, labels};
use sea_orm::{
    DatabaseTransaction, FromQueryResult,
    prelude::*,
    query::{QueryOrder, QuerySelect},
};
use std::{
    cmp::Ordering,
    ops::Deref,
    sync::{Arc, LazyLock},
};

static COLLATOR: LazyLock<CollatorBorrowed<'static>> =
    LazyLock::new(|| Collator::try_new(locale!("hu").into(), CollatorOptions::default()).unwrap());

/// Title ignored at the start of a name when sorting.
const TITLE_PREFIX: &str = "Dr. ";

/// A staff member or colleague.
#[derive(SimpleObject, Debug, FromQueryResult)]
#[graphql(complex)]
pub struct Colleague {
    /// Unique identifier.
    pub id: Maybe<u32>,
    /// Full name (may include title like "Dr.").
    pub name: Maybe<String>,
    /// Job titles or positions.
    pub jobs: Maybe<Option<String>>,
    /// Teaching subjects.
    pub subjects: Maybe<Option<String>>,
    /// Additional roles or responsibilities.
    pub roles: Maybe<Option<String>>,
    /// Awards and recognitions.
    pub awards: Maybe<Option<String>>,
    #[graphql(skip)]
    pub image: Maybe<Option<String>>,
    /// Category identifier for grouping colleagues.
    pub category: Maybe<u16>,
}

#[ComplexObject]
impl Colleague {
    /// Profile image URL.
    async fn image(&self, ctx: &Context<'_>) -> Result<Option<String>> {
        let config = ctx.data_unchecked::<Config>();

        let Some(Some(ref image)) = *self.image else {
            return Ok(None);
        };

        Ok(Some(format!(
            "{}/colleagues_images/{}",
            config.storage_base_url, image
        )))
    }
}

/// Orders colleague names the way the staff page lists them.
///
/// Names are compared with Hungarian collation rules: the digraphs "cs", "gy",
/// "sz", "zs" and friends sort as letters of their own, and long vowels sort
/// together with their short pair (a/á, o/ó, ö/ő, ...). A leading "Dr. " title
/// is ignored.
///
/// * `a` - first name to compare, as stored in the database.
/// * `b` - second name to compare, as stored in the database.
fn compare_names(a: &str, b: &str) -> Ordering {
    let a = a.strip_prefix(TITLE_PREFIX).unwrap_or(a);
    let b = b.strip_prefix(TITLE_PREFIX).unwrap_or(b);

    COLLATOR.compare(a, b)
}

#[derive(Default)]
pub struct ColleaguesQuery;

#[Object]
impl ColleaguesQuery {
    /// Retrieve all colleagues, sorted alphabetically by name.
    ///
    /// Names with "Dr." prefix are sorted by the name portion (ignoring the title).
    async fn colleagues(&self, ctx: &Context<'_>) -> Result<Vec<Colleague>> {
        ctx.data_unchecked::<IntCounterVec>()
            .with(&labels! {"resource" => "colleagues"})
            .inc();

        let db = ctx.data::<Arc<DatabaseTransaction>>().unwrap();
        let mut query = ColleaguesData::find().select_only();

        select_columns!(ctx, query, Column);

        let mut res = query
            .order_by_asc(Column::Name)
            .into_model::<Colleague>()
            .all(db.deref())
            .await
            .map_err(db_error)?;

        if ctx.look_ahead().field("name").exists() {
            res.sort_by(|a, b| compare_names(a.name.as_ref().unwrap(), b.name.as_ref().unwrap()));
        }

        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn sorted<'a>(names: &[&'a str]) -> Vec<&'a str> {
        let mut names = names.to_vec();
        names.sort_by(|a, b| compare_names(a, b));
        names
    }

    #[test]
    fn title_is_ignored() {
        // Byte order would put "Csaba" first because "C" < "D".
        assert_eq!(
            sorted(&["Csaba Kiss", "Dr. Béla Tóth"]),
            ["Dr. Béla Tóth", "Csaba Kiss"]
        );
    }

    #[test]
    fn digraphs_sort_after_every_word_on_their_first_letter() {
        assert_eq!(
            sorted(&["Csaba", "Cukor", "Cirkó"]),
            ["Cirkó", "Cukor", "Csaba"]
        );
        assert_eq!(
            sorted(&["Gyula", "Guszti", "Gábor"]),
            ["Gábor", "Guszti", "Gyula"]
        );
        assert_eq!(
            sorted(&["Sztojka", "Sütő", "Szabó"]),
            ["Sütő", "Szabó", "Sztojka"]
        );
        assert_eq!(sorted(&["Zsolt", "Zuzana"]), ["Zuzana", "Zsolt"]);
    }

    #[test]
    fn long_vowels_sort_with_their_short_pair() {
        assert_eq!(sorted(&["Adorján", "Ádám"]), ["Ádám", "Adorján"]);
        assert_eq!(sorted(&["Örs", "Őri"]), ["Őri", "Örs"]);
    }

    #[test]
    fn o_with_diaeresis_is_a_separate_letter_after_o() {
        assert_eq!(
            sorted(&["Őry", "Ozsvár", "Óvári", "Oláh"]),
            ["Oláh", "Óvári", "Ozsvár", "Őry"]
        );
    }

    proptest! {
        #[test]
        fn comparison_is_antisymmetric(a in any::<String>(), b in any::<String>()) {
            prop_assert_eq!(compare_names(&a, &b), compare_names(&b, &a).reverse());
        }
    }
}
