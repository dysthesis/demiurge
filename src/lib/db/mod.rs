use std::{path::Path, result};

use rusqlite::Connection;

/// Database to keep track of build traces
pub struct Db {
    /// The connection to the database file
    conn: Connection,
}

#[derive(Debug, thiserror::Error)]
pub enum Error<'a> {
    #[error("Failed to open database at {path}")]
    ConnectionOpenError {
        path: &'a Path,
        #[source]
        error: rusqlite::Error,
    },
}

pub type Result<'a, T> = result::Result<T, Error<'a>>;

impl Db {
    pub fn new<P: AsRef<Path>>(db_file: P) -> Result<'_, Self> {
        let conn = Connection::open(db_file).map_err(|error| {
            Error::ConnectionOpenError {
                path: db_file.into(),
                error,
            }
        })?;

        Ok(Self { conn })
    }
}
