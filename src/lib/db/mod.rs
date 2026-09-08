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
    #[error("failed to migrate database at {path}")]
    Migration {
        path: PathBuf,
        #[source]
        error: rusqlite::Error,
    },
}

pub type Result<T> = result::Result<T, Error>;

const DB_MIGRATION: &'static str = include_str!("./queries/migration.sql");

impl Db {
    pub fn new<P: AsRef<Path>>(db_file: P) -> Result<Self> {
        let path = db_file.as_ref();
        let mut conn = Connection::open(path).map_err(|error| {
            Error::ConnectionOpenError {
                path: db_file.as_ref().to_owned(),
                error,
            }
        })?;

        conn.pragma_update(None, "foreign_keys", true)
            .map_err(|error| Error::Migration {
                path: path.to_owned(),
                error,
            })?;

        let version: u32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|error| Error::Migration {
                path: path.to_owned(),
                error,
            })?;

        match version {
            0 => {
                let tx =
                    conn.transaction().map_err(|error| Error::Migration {
                        path: path.to_owned(),
                        error,
                    })?;

                tx.execute_batch(DB_MIGRATION).map_err(|error| {
                    Error::Migration {
                        path: path.to_owned(),
                        error,
                    }
                })?;

                tx.pragma_update(None, "user_version", 1).map_err(|error| {
                    Error::Migration {
                        path: path.to_owned(),
                        error,
                    }
                })?;

                tx.commit().map_err(|error| Error::Migration {
                    path: path.to_owned(),
                    error,
                })?;
            }
            1 => {
                // Already current.
            }
            version => {
                todo!("unsupported database version {version}");
            }
        }

        Ok(Self { conn })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn new_applies_migrations() {
        let db = Db::new(":memory:").unwrap();

        let node_exists: bool = db
            .conn
            .query_row(
                "
                SELECT EXISTS (
                    SELECT 1
                    FROM sqlite_schema
                    WHERE type = 'table'
                      AND name = 'node'
                )
                ",
                [],
                |row| row.get(0),
            )
            .unwrap();

        let dependency_exists: bool = db
            .conn
            .query_row(
                "
                SELECT EXISTS (
                    SELECT 1
                    FROM sqlite_schema
                    WHERE type = 'table'
                      AND name = 'dependency'
                )
                ",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(node_exists);
        assert!(dependency_exists);
    }
    #[test]
    fn migration_enforces_hash_length() {
        let db = Db::new(":memory:").unwrap();

        let result = db.conn.execute(
            "
        INSERT INTO node (kind, key, hash)
        VALUES (?1, ?2, ?3)
        ",
            rusqlite::params!["file", "foo.md", vec![0_u8; 31],],
        );

        assert!(result.is_err());
    }
    #[test]
    fn migration_accepts_valid_node() {
        let db = Db::new(":memory:").unwrap();

        db.conn
            .execute(
                "
            INSERT INTO node (kind, key, hash)
            VALUES (?1, ?2, ?3)
            ",
                rusqlite::params!["file", "foo.md", vec![0_u8; 32],],
            )
            .unwrap();
    }

    proptest! {
        #[test]
        fn reopening_database_is_idempotent(reopens in 1usize..100) {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("build.sqlite");

            {
                let db = Db::new(&path).unwrap();

                db.conn.execute(
                    "
                INSERT INTO node (kind, key, hash)
                VALUES (?1, ?2, ?3)
                ",
                    rusqlite::params![
                        "file",
                        "foo.md",
                        vec![42_u8; 32],
                    ],
                ).unwrap();
            }

            for _ in 0..reopens {
                let _db = Db::new(&path).unwrap();
            }

            let db = Db::new(&path).unwrap();

            let version: u32 = db
                .conn
                .pragma_query_value(
                    None,
                    "user_version",
                    |row| row.get(0),
                )
                .unwrap();

            prop_assert_eq!(version, 1);

            let count: u32 = db
                .conn
                .query_row(
                    "
                SELECT COUNT(*)
                FROM node
                WHERE kind = ?1
                  AND key = ?2
                ",
                    ["file", "foo.md"],
                    |row| row.get(0),
                )
                .unwrap();

            prop_assert_eq!(count, 1);
        }
    }
}
