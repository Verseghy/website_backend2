use super::Date;
use async_graphql::types::connection::CursorType;
use chrono::NaiveDate;

const DATE_FORMAT: &str = "%Y-%m-%d";

pub struct PostCursor {
    date: Date,
    id: u32,
}

impl PostCursor {
    pub fn new(date: Date, id: u32) -> Self {
        Self { date, id }
    }

    pub fn date(&self) -> NaiveDate {
        self.date.0
    }

    pub fn id(&self) -> u32 {
        self.id
    }
}

impl CursorType for PostCursor {
    type Error = PostCursorError;
    fn decode_cursor(s: &str) -> Result<Self, Self::Error> {
        let Some((date, id)) = s.split_once('#') else {
            return Err(PostCursorError::WrongFormat);
        };

        Ok(Self {
            date: Date(NaiveDate::parse_from_str(date, DATE_FORMAT)?),
            id: id.parse()?,
        })
    }

    fn encode_cursor(&self) -> String {
        format!("{}#{}", self.date.0.format(DATE_FORMAT), self.id)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum PostCursorError {
    #[error("Wrong date format in cursor")]
    WrongDate(#[from] chrono::ParseError),
    #[error("Wrong cursor format")]
    WrongFormat,
    #[error("Invalid id in cursor")]
    InvalidId(#[from] std::num::ParseIntError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn cursor(year: i32, month: u32, day: u32, id: u32) -> PostCursor {
        PostCursor::new(Date(NaiveDate::from_ymd_opt(year, month, day).unwrap()), id)
    }

    #[test]
    fn encodes_date_and_id_separated_by_a_hash() {
        assert_eq!(cursor(2024, 1, 15, 42).encode_cursor(), "2024-01-15#42");
    }

    #[test]
    fn decodes_date_and_id() {
        let decoded = PostCursor::decode_cursor("2024-01-15#42").unwrap();

        assert_eq!(
            decoded.date(),
            NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()
        );
        assert_eq!(decoded.id(), 42);
    }

    #[test]
    fn rejects_a_cursor_without_separator() {
        assert!(matches!(
            PostCursor::decode_cursor("2024-01-15"),
            Err(PostCursorError::WrongFormat)
        ));
    }

    #[test]
    fn rejects_an_invalid_date() {
        for input in ["#1", "2024-13-01#1", "15.01.2024#1"] {
            assert!(
                matches!(
                    PostCursor::decode_cursor(input),
                    Err(PostCursorError::WrongDate(_))
                ),
                "{input:?}"
            );
        }
    }

    #[test]
    fn rejects_an_invalid_id() {
        for input in [
            "2024-01-15#",
            "2024-01-15#abc",
            "2024-01-15#-1",
            "2024-01-15#4294967296",
            "2024-01-15#1#2",
        ] {
            assert!(
                matches!(
                    PostCursor::decode_cursor(input),
                    Err(PostCursorError::InvalidId(_))
                ),
                "{input:?}"
            );
        }
    }

    proptest! {
        #[test]
        fn round_trips_through_its_encoding(days in 1..=3_652_059i32, id in any::<u32>()) {
            // Day 1 is 0001-01-01 and day 3 652 059 is 9999-12-31.
            let date = NaiveDate::from_num_days_from_ce_opt(days).unwrap();
            let encoded = PostCursor::new(Date(date), id).encode_cursor();

            let decoded = PostCursor::decode_cursor(&encoded).unwrap();

            prop_assert_eq!(decoded.date(), date);
            prop_assert_eq!(decoded.id(), id);
        }
    }
}
