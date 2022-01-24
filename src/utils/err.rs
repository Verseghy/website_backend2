use std::error::Error;

pub fn db_error(err: impl Error) -> async_graphql::Error {
    async_graphql::Error::new(format!("Database error: {}", err))
}
