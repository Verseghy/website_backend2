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
    type Error = String;
    fn decode_cursor(s: &str) -> Result<Self, Self::Error> {
        let mut iter = s.split('#');
        if let (Some(date), Some(id)) = (iter.next(), iter.next()) {
            Ok(Self {
                date: Date(
                    NaiveDate::parse_from_str(date, DATE_FORMAT)
                        .map_err(|_| "Wrong date format in cursor".to_string())?,
                ),
                id: id.parse().map_err(|_| "Could not parse id".to_string())?,
            })
        } else {
            Err("Wrong post cursor format".to_string())
        }
    }

    fn encode_cursor(&self) -> String {
        format!("{}#{}", self.date.0.format(DATE_FORMAT), self.id)
    }
}
