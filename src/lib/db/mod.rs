use std::{
    path::{Path, PathBuf},
    result,
};

use rusqlite::Connection;

/// Database to keep track of build traces
pub struct Db {
    /// The connection to the database file
    conn: Connection,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to open database at {path}")]
    ConnectionOpenError {
        path: PathBuf,
        #[source]
        error: rusqlite::Error,
    },
}

pub type Result<T> = result::Result<T, Error>;

impl Db {
    pub fn new<P: AsRef<Path>>(db_file: P) -> Result<Self> {
        let conn = Connection::open(db_file.as_ref()).map_err(|error| {
            Error::ConnectionOpenError {
                path: db_file.as_ref().to_owned(),
                error,
            }
        })?;

        Ok(Self { conn })
    }
}
