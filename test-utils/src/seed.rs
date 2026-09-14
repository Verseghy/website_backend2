//! The standard data set most integration tests run against.
//!
//! It is small enough to read in one sitting, yet covers the cases the
//! resolvers treat differently: published, featured, unpublished and backdated
//! posts; authors and labels with and without posts; a nested menu linking to
//! pages; soft-deleted pages and menu items; canteen days on both sides of an
//! ISO year boundary; events around a month's calendar grid; and colleague
//! names that only sort correctly under Hungarian collation.
//!
//! Every value is fixed, so responses are deterministic and suitable for
//! snapshots. Editing this data changes many snapshots at once; a test that
//! needs an unusual row should insert it itself through
//! [`TestApp::db`](crate::TestApp::db). The constants below name the rows tests
//! refer to.

use chrono::{NaiveDate, NaiveDateTime};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityName, EntityTrait};
use serde_json::json;
use website_backend2::entity::{
    canteen_data, canteen_menus, canteen_pivot_menus_data, colleagues_data, events_data,
    menu_items, pages, posts_authors, posts_data, posts_labels, posts_pivot_labels_data,
};

/// Published posts in the order listings return them, newest first.
pub const PUBLISHED_POSTS_NEWEST_FIRST: [u32; 5] = [1, 2, 3, 6, 5];
/// Published and featured posts, newest first.
pub const FEATURED_POSTS_NEWEST_FIRST: [u32; 2] = [1, 5];
/// The only unpublished post. It has the newest date of all posts, so a list
/// that leaked it would show it first.
pub const UNPUBLISHED_POST: u32 = 4;
/// Preview token that unlocks [`UNPUBLISHED_POST`].
pub const PREVIEW_TOKEN: &str = "preview-token-4";
/// A post with a higher id than posts 1 to 5 but an older date than posts 1 to 3.
pub const BACKDATED_POST: u32 = 6;
/// Author with a description, an image, and posts 1, 3 and the unpublished 4.
pub const AUTHOR_WITH_IMAGE: u32 = 1;
/// Author without description or image, with posts 2 and 6.
pub const AUTHOR_WITHOUT_IMAGE: u32 = 2;
/// Author without posts.
pub const AUTHOR_WITHOUT_POSTS: u32 = 3;
/// Label on posts 1, 6 and the unpublished 4.
pub const LABEL_NEWS: u32 = 1;
/// Label on post 2 only.
pub const LABEL_SPORT: u32 = 3;
/// Label without posts.
pub const LABEL_WITHOUT_POSTS: u32 = 4;
/// Slug of a live page.
pub const PAGE_SLUG: &str = "rolunk";
/// Slug of a soft-deleted page.
pub const DELETED_PAGE_SLUG: &str = "regi-oldal";
/// Name of a soft-deleted top-level menu item.
pub const DELETED_MENU_ITEM: &str = "Törölt menüpont";
/// Event on Sunday 4 October 2026, the last day of September 2026's calendar grid.
pub const EVENT_ON_LAST_GRID_DAY: u32 = 5;

/// Inserts the standard data set.
///
/// * `db` - connection to a test database whose tables exist and are empty.
///
/// # Panics
///
/// Panics if an insert fails, which means the entities and this data set
/// disagree.
pub async fn insert(db: &DatabaseConnection) {
    insert_posts(db).await;
    insert_pages_and_menu(db).await;
    insert_canteen(db).await;
    insert_colleagues(db).await;
    insert_events(db).await;
}

async fn insert_rows<A>(db: &DatabaseConnection, rows: Vec<A>)
where
    A: ActiveModelTrait,
    <A::Entity as EntityTrait>::Model: sea_orm::IntoActiveModel<A>,
{
    let table = <A::Entity as Default>::default().table_name().to_owned();

    <A::Entity as EntityTrait>::insert_many(rows)
        .exec_without_returning(db)
        .await
        .unwrap_or_else(|error| panic!("could not seed {table}: {error}"));
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("seed dates are valid")
}

fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> NaiveDateTime {
    date(year, month, day)
        .and_hms_opt(hour, minute, 0)
        .expect("seed times are valid")
}

/// `created_at` and `updated_at` of every row; the API never exposes them.
fn created() -> NaiveDateTime {
    at(2026, 1, 1, 0, 0)
}

fn text(value: &str) -> String {
    value.to_owned()
}

fn some(value: &str) -> Option<String> {
    Some(value.to_owned())
}

async fn insert_posts(db: &DatabaseConnection) {
    insert_rows(
        db,
        vec![
            posts_authors::ActiveModel {
                id: Set(1),
                name: Set(text("Kovács Anna")),
                description: Set(some("Magyar és történelem szakos tanár.")),
                image: Set(some("kovacs-anna.jpg")),
            },
            posts_authors::ActiveModel {
                id: Set(2),
                name: Set(text("Nagy Béla")),
                description: Set(None),
                image: Set(None),
            },
            posts_authors::ActiveModel {
                id: Set(3),
                name: Set(text("Szabó Csilla")),
                description: Set(some("Könyvtáros.")),
                image: Set(some("szabo-csilla.jpg")),
            },
        ],
    )
    .await;

    insert_rows(
        db,
        vec![
            label(1, "Hírek", "#1e88e5"),
            label(2, "Ünnepségek", "#8e24aa"),
            label(3, "Sport", "#43a047"),
            label(4, "Archívum", "#757575"),
        ],
    )
    .await;

    insert_rows(
        db,
        vec![
            posts_data::ActiveModel {
                id: Set(1),
                title: Set(text("Tanévnyitó ünnepség")),
                color: Set(text("#1e88e5")),
                description: Set(some("Ünnepélyesen megnyitottuk a 2026/2027-es tanévet.")),
                content: Set(some("<p>Az ünnepségen a diákok műsorral készültek.</p>")),
                index_image: Set(some("tanevnyito.jpg")),
                author_id: Set(Some(1)),
                images: Set(json!(["tanevnyito-1.jpg", "tanevnyito-2.jpg"])),
                date: Set(Some(date(2026, 9, 1))),
                created_at: Set(created()),
                updated_at: Set(created()),
                featured: Set(1),
                published: Set(1),
                preview_token: Set(None),
            },
            posts_data::ActiveModel {
                id: Set(2),
                title: Set(text("Őszi sportnap")),
                color: Set(text("#43a047")),
                description: Set(some("Futóverseny és focibajnokság az udvaron.")),
                content: Set(some("<p>Minden évfolyam csapatot indított.</p>")),
                index_image: Set(some("sportnap.jpg")),
                author_id: Set(Some(2)),
                // The images column is also stored as an object in production.
                images: Set(json!({ "elso": "sportnap-1.jpg", "masodik": "sportnap-2.jpg" })),
                date: Set(Some(date(2026, 8, 20))),
                created_at: Set(created()),
                updated_at: Set(created()),
                featured: Set(0),
                published: Set(1),
                preview_token: Set(None),
            },
            posts_data::ActiveModel {
                id: Set(3),
                title: Set(text("Nyári tábor beszámoló")),
                color: Set(text("#fb8c00")),
                description: Set(None),
                content: Set(some("<p>Egy hét a Balatonnál.</p>")),
                index_image: Set(some("tabor.jpg")),
                author_id: Set(Some(1)),
                images: Set(json!([])),
                date: Set(Some(date(2026, 7, 10))),
                created_at: Set(created()),
                updated_at: Set(created()),
                featured: Set(0),
                published: Set(1),
                preview_token: Set(None),
            },
            posts_data::ActiveModel {
                id: Set(4),
                title: Set(text("Szalagavató előzetes")),
                color: Set(text("#8e24aa")),
                description: Set(some("Hamarosan részletek.")),
                content: Set(some("<p>A szalagavató ünnepség programja.</p>")),
                index_image: Set(some("szalagavato.jpg")),
                author_id: Set(Some(1)),
                images: Set(json!([])),
                date: Set(Some(date(2026, 9, 10))),
                created_at: Set(created()),
                updated_at: Set(created()),
                featured: Set(1),
                published: Set(0),
                preview_token: Set(some(PREVIEW_TOKEN)),
            },
            posts_data::ActiveModel {
                id: Set(5),
                title: Set(text("Karácsonyi koncert")),
                color: Set(text("#e53935")),
                description: Set(some("Az iskolakórus ünnepi hangversenye.")),
                content: Set(some("<p>Telt ház volt a díszteremben.</p>")),
                index_image: Set(some("koncert.jpg")),
                author_id: Set(None),
                images: Set(json!(["koncert-1.jpg"])),
                date: Set(Some(date(2025, 12, 19))),
                created_at: Set(created()),
                updated_at: Set(created()),
                featured: Set(1),
                published: Set(1),
                preview_token: Set(None),
            },
            posts_data::ActiveModel {
                id: Set(6),
                title: Set(text("Farsangi bál")),
                color: Set(text("#fdd835")),
                description: Set(some("Utólag rögzített beszámoló.")),
                content: Set(some("<p>Jelmezverseny és tombola.</p>")),
                index_image: Set(some("farsang.jpg")),
                author_id: Set(Some(2)),
                images: Set(json!([])),
                date: Set(Some(date(2026, 2, 13))),
                created_at: Set(created()),
                updated_at: Set(created()),
                featured: Set(0),
                published: Set(1),
                preview_token: Set(None),
            },
        ],
    )
    .await;

    insert_rows(
        db,
        [(1, 1), (2, 1), (3, 2), (1, 4), (2, 5), (1, 6)]
            .into_iter()
            .map(
                |(labels_id, posts_id)| posts_pivot_labels_data::ActiveModel {
                    labels_id: Set(labels_id),
                    posts_id: Set(posts_id),
                },
            )
            .collect(),
    )
    .await;
}

fn label(id: u32, name: &str, color: &str) -> posts_labels::ActiveModel {
    posts_labels::ActiveModel {
        id: Set(id),
        name: Set(text(name)),
        color: Set(text(color)),
    }
}

async fn insert_pages_and_menu(db: &DatabaseConnection) {
    insert_rows(
        db,
        vec![
            pages::ActiveModel {
                id: Set(1),
                template: Set(text("default")),
                name: Set(text("rolunk")),
                title: Set(text("Rólunk")),
                slug: Set(text(PAGE_SLUG)),
                content: Set(text("<p>Iskolánk bemutatkozása.</p>")),
                extras: Set(json!({ "meta_description": "Az iskola bemutatása" })),
                created_at: Set(created()),
                updated_at: Set(created()),
                deleted_at: Set(None),
            },
            pages::ActiveModel {
                id: Set(2),
                template: Set(text("contact")),
                name: Set(text("kapcsolat")),
                title: Set(text("Kapcsolat")),
                slug: Set(text("kapcsolat")),
                content: Set(text("<p>Elérhetőségeink.</p>")),
                extras: Set(json!({ "address": "1000 Budapest, Példa utca 1.", "email": "info@example.test" })),
                created_at: Set(created()),
                updated_at: Set(created()),
                deleted_at: Set(None),
            },
            pages::ActiveModel {
                id: Set(3),
                template: Set(text("default")),
                name: Set(text("regi-oldal")),
                title: Set(text("Régi oldal")),
                slug: Set(text(DELETED_PAGE_SLUG)),
                content: Set(text("<p>Ez az oldal törölve lett.</p>")),
                extras: Set(json!({})),
                created_at: Set(created()),
                updated_at: Set(created()),
                deleted_at: Set(Some(at(2026, 3, 1, 12, 0))),
            },
        ],
    )
    .await;

    // Nested set: "Iskolánk" (1-6) contains "Rólunk" (2-3) and "Régi oldal" (4-5).
    insert_rows(
        db,
        vec![
            menu_item(
                1,
                "Iskolánk",
                "internal_link",
                some("/informaciok"),
                None,
                None,
                (1, 6, 1),
                None,
            ),
            menu_item(
                2,
                "Rólunk",
                "page_link",
                None,
                Some(1),
                Some(1),
                (2, 3, 2),
                None,
            ),
            menu_item(
                3,
                "Régi oldal",
                "page_link",
                None,
                Some(3),
                Some(1),
                (4, 5, 2),
                None,
            ),
            menu_item(
                4,
                "Kapcsolat",
                "page_link",
                None,
                Some(2),
                None,
                (7, 8, 1),
                None,
            ),
            menu_item(
                5,
                "E-napló",
                "external_link",
                some("https://example.test/e-naplo"),
                None,
                None,
                (9, 10, 1),
                None,
            ),
            menu_item(
                6,
                DELETED_MENU_ITEM,
                "external_link",
                some("https://example.test/torolt"),
                None,
                None,
                (11, 12, 1),
                Some(at(2026, 3, 1, 12, 0)),
            ),
        ],
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
fn menu_item(
    id: u32,
    name: &str,
    kind: &str,
    link: Option<String>,
    page_id: Option<u32>,
    parent_id: Option<u32>,
    (lft, rgt, depth): (u32, u32, u32),
    deleted_at: Option<NaiveDateTime>,
) -> menu_items::ActiveModel {
    menu_items::ActiveModel {
        id: Set(id),
        name: Set(text(name)),
        r#type: Set(text(kind)),
        link: Set(link),
        page_id: Set(page_id),
        parent_id: Set(parent_id),
        lft: Set(lft),
        rgt: Set(rgt),
        depth: Set(depth),
        created_at: Set(created()),
        updated_at: Set(created()),
        deleted_at: Set(deleted_at),
    }
}

async fn insert_canteen(db: &DatabaseConnection) {
    insert_rows(
        db,
        [
            (1, "Gulyásleves", 0),
            (2, "Rántott csirke rizzsel", 1),
            (3, "Túrós palacsinta", 2),
            (4, "Zöldségleves", 0),
            (5, "Paprikás krumpli", 1),
        ]
        .into_iter()
        .map(|(id, menu, kind)| canteen_menus::ActiveModel {
            id: Set(id),
            menu: Set(text(menu)),
            r#type: Set(kind),
            created_at: Set(created()),
            updated_at: Set(created()),
        })
        .collect(),
    )
    .await;

    // Monday of ISO week 2025-W52, Monday and Friday of 2026-W01 (which starts
    // in December 2025), and Monday of 2026-W02.
    let days = [
        (1, date(2025, 12, 22), vec![1, 2]),
        (2, date(2025, 12, 29), vec![4, 5, 3]),
        (3, date(2026, 1, 2), vec![1, 5]),
        (4, date(2026, 1, 5), vec![4, 2]),
    ];

    insert_rows(
        db,
        days.iter()
            .map(|(id, day, _)| canteen_data::ActiveModel {
                id: Set(*id),
                date: Set(*day),
                created_at: Set(created()),
                updated_at: Set(created()),
            })
            .collect(),
    )
    .await;

    insert_rows(
        db,
        days.iter()
            .flat_map(|(data_id, _, menus)| {
                menus
                    .iter()
                    .map(|menu_id| canteen_pivot_menus_data::ActiveModel {
                        menu_id: Set(*menu_id),
                        data_id: Set(*data_id),
                    })
            })
            .collect(),
    )
    .await;
}

async fn insert_colleagues(db: &DatabaseConnection) {
    // Byte order would sort these differently from Hungarian collation: "Á"
    // after "Z", "Cs" before "Ci", "Ő" after "Z".
    insert_rows(
        db,
        vec![
            colleague(
                1,
                "Oláh Zsófia",
                some("tanár"),
                some("angol nyelv"),
                None,
                None,
                None,
                Some(1),
            ),
            colleague(
                2,
                "Dr. Cirkó Béla",
                some("tanár"),
                some("fizika"),
                some("munkaközösség-vezető"),
                some("Az év tanára (2024)"),
                some("cirko-bela.jpg"),
                Some(1),
            ),
            colleague(
                3,
                "Csizmadia Ágnes",
                some("igazgatóhelyettes"),
                some("matematika"),
                None,
                None,
                some("csizmadia-agnes.jpg"),
                Some(0),
            ),
            colleague(
                4,
                "Ádám Éva",
                some("óraadó tanár"),
                some("rajz"),
                None,
                None,
                None,
                Some(2),
            ),
            colleague(
                5,
                "Adorján Péter",
                some("rendszergazda"),
                None,
                None,
                None,
                None,
                Some(3),
            ),
            colleague(
                6,
                "Őri Gábor",
                some("karbantartó"),
                None,
                None,
                None,
                None,
                Some(4),
            ),
            colleague(
                7,
                "Zsigmond Lili",
                some("tanár"),
                some("biológia"),
                None,
                None,
                None,
                Some(1),
            ),
            colleague(
                8,
                "Zakariás Ottó",
                some("tanár"),
                some("testnevelés"),
                None,
                None,
                None,
                None,
            ),
        ],
    )
    .await;
}

#[allow(clippy::too_many_arguments)]
fn colleague(
    id: u32,
    name: &str,
    jobs: Option<String>,
    subjects: Option<String>,
    roles: Option<String>,
    awards: Option<String>,
    image: Option<String>,
    category: Option<u16>,
) -> colleagues_data::ActiveModel {
    colleagues_data::ActiveModel {
        id: Set(id),
        name: Set(some(name)),
        jobs: Set(jobs),
        subjects: Set(subjects),
        roles: Set(roles),
        awards: Set(awards),
        image: Set(image),
        category: Set(category),
    }
}

async fn insert_events(db: &DatabaseConnection) {
    // September 2026's calendar grid runs from Monday 31 August to Sunday 4 October.
    insert_rows(
        db,
        vec![
            event(
                1,
                "Nyári szünet utolsó napja",
                at(2026, 8, 30, 10, 0),
                at(2026, 8, 30, 11, 0),
                None,
                "#757575",
            ),
            event(
                2,
                "Szülői értekezlet",
                at(2026, 8, 31, 17, 0),
                at(2026, 8, 31, 19, 0),
                some("Minden évfolyam"),
                "#fb8c00",
            ),
            event(
                3,
                "Tanévnyitó",
                at(2026, 9, 1, 8, 0),
                at(2026, 9, 1, 10, 0),
                some("Díszterem"),
                "#1e88e5",
            ),
            event(
                4,
                "Témahét",
                at(2026, 9, 28, 8, 0),
                at(2026, 10, 2, 14, 0),
                None,
                "#43a047",
            ),
            event(
                EVENT_ON_LAST_GRID_DAY,
                "Nyílt nap",
                at(2026, 10, 4, 9, 0),
                at(2026, 10, 4, 12, 0),
                some("Leendő kilencedikeseknek"),
                "#8e24aa",
            ),
            event(
                6,
                "Pályaorientációs nap",
                at(2026, 10, 15, 8, 0),
                at(2026, 10, 15, 14, 0),
                None,
                "#e53935",
            ),
        ],
    )
    .await;
}

fn event(
    id: u32,
    title: &str,
    date_from: NaiveDateTime,
    date_to: NaiveDateTime,
    description: Option<String>,
    color: &str,
) -> events_data::ActiveModel {
    events_data::ActiveModel {
        id: Set(id),
        date_from: Set(date_from),
        date_to: Set(date_to),
        title: Set(text(title)),
        description: Set(description),
        color: Set(text(color)),
        created_at: Set(created()),
        updated_at: Set(created()),
    }
}
