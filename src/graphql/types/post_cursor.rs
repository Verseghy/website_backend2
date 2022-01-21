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
        let split = s.split_once('#');

        if let Some((date, id)) = split {
            Ok(Self {
                date: Date(NaiveDate::parse_from_str(date, DATE_FORMAT)?),
                id: id.parse()?,
            })
        } else {
            Err(PostCursorError::WrongFormat)
        }
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
