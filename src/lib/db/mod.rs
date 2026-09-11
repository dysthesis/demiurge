use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    result,
};

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeKey {
    pub kind: String,
    pub key: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OutputHash(pub [u8; 32]);
impl TryFrom<Vec<u8>> for OutputHash {
    type Error = Error;

    fn try_from(value: Vec<u8>) -> Result<Self> {
        let len = value.len();

        let value = value
            .try_into()
            .map_err(|_| Error::InvalidHashLength { len })?;

        Ok(Self(value))
    }
}
impl OutputHash {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dependency {
    pub node: NodeKey,

    /// Output hash observed by the parent when it last ran.
    pub expected: OutputHash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trace {
    pub node: NodeKey,
    pub output_hash: OutputHash,
    pub dependencies: Vec<Dependency>,
}

/// Database to keep track of build traces
pub struct Db {
    /// The connection to the database file
    conn: Connection,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Hash length is invalid: {len}")]
    InvalidHashLength { len: usize },
    #[error("Database query failed.")]
    Query {
        #[source]
        error: rusqlite::Error,
    },
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
    #[error("dependency {node:?} has no cached trace")]
    MissingDependency { node: NodeKey },

    #[error("dependency {node:?} changed while storing its parent trace")]
    StaleDependency { node: NodeKey },

    #[error("trace for {node:?} depends on itself")]
    SelfDependency { node: NodeKey },

    #[error("trace contains dependency {node:?} more than once")]
    DuplicateDependency { node: NodeKey },
}

pub type Result<T> = result::Result<T, Error>;

const DB_MIGRATION: &str = include_str!("./queries/migration.sql");

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
    pub fn trace(&self, node: &NodeKey) -> Result<Option<Trace>> {
        let stored = self
            .conn
            .query_row(
                "
                SELECT id, output_hash
                FROM node
                WHERE kind = ?1
                  AND key = ?2
                ",
                params![&node.kind, &node.key],
                |row| {
                    let id: i64 = row.get(0)?;
                    let output_hash: Vec<u8> = row.get(1)?;

                    Ok((id, output_hash))
                },
            )
            .optional()
            .map_err(|error| Error::Query { error })?;

        let Some((parent_id, output_hash)) = stored else {
            return Ok(None);
        };

        let output_hash = decode_hash(output_hash)?;

        let mut statement = self
            .conn
            .prepare(
                "
                SELECT
                    node.kind,
                    node.key,
                    dependency.expected
                FROM dependency
                INNER JOIN node
                    ON node.id = dependency.dep
                WHERE dependency.parent = ?1
                ORDER BY node.kind, node.key
                ",
            )
            .map_err(|error| Error::Query { error })?;

        let rows = statement
            .query_map(params![parent_id], |row| {
                let kind: String = row.get(0)?;
                let key: String = row.get(1)?;
                let expected: Vec<u8> = row.get(2)?;

                Ok((kind, key, expected))
            })
            .map_err(|error| Error::Query { error })?;

        let rows = rows
            .collect::<std::result::Result<Vec<_>, rusqlite::Error>>()
            .map_err(|error| Error::Query { error })?;

        let dependencies = rows
            .into_iter()
            .map(|(kind, key, expected)| {
                Ok(Dependency {
                    node: NodeKey { kind, key },
                    expected: decode_hash(expected)?,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Some(Trace {
            node: node.clone(),
            output_hash,
            dependencies,
        }))
    }
    pub fn replace_trace(&mut self, trace: &Trace) -> Result<()> {
        let mut seen = HashSet::with_capacity(trace.dependencies.len());

        for dependency in &trace.dependencies {
            if dependency.node == trace.node {
                return Err(Error::SelfDependency {
                    node: trace.node.clone(),
                });
            }

            if !seen.insert(&dependency.node) {
                return Err(Error::DuplicateDependency {
                    node: dependency.node.clone(),
                });
            }
        }

        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| Error::Query { error })?;

        let mut dependencies = Vec::with_capacity(trace.dependencies.len());

        for dependency in &trace.dependencies {
            let stored = tx
                .query_row(
                    "
                    SELECT id, output_hash
                    FROM node
                    WHERE kind = ?1
                      AND key = ?2
                    ",
                    params![&dependency.node.kind, &dependency.node.key,],
                    |row| {
                        let id: i64 = row.get(0)?;
                        let output_hash: Vec<u8> = row.get(1)?;

                        Ok((id, output_hash))
                    },
                )
                .optional()
                .map_err(|error| Error::Query { error })?;

            let Some((dep_id, output_hash)) = stored else {
                return Err(Error::MissingDependency {
                    node: dependency.node.clone(),
                });
            };

            let actual = decode_hash(output_hash)?;

            if actual != dependency.expected {
                return Err(Error::StaleDependency {
                    node: dependency.node.clone(),
                });
            }

            dependencies.push((dep_id, dependency.expected));
        }

        tx.execute(
            "
            INSERT INTO node (
                kind,
                key,
                output_hash
            )
            VALUES (?1, ?2, ?3)

            ON CONFLICT (kind, key)
            DO UPDATE SET
                output_hash = excluded.output_hash
            ",
            params![
                &trace.node.kind,
                &trace.node.key,
                trace.output_hash.as_bytes(),
            ],
        )
        .map_err(|error| Error::Query { error })?;

        let parent_id: i64 = tx
            .query_row(
                "
                SELECT id
                FROM node
                WHERE kind = ?1
                  AND key = ?2
                ",
                params![&trace.node.kind, &trace.node.key,],
                |row| row.get(0),
            )
            .map_err(|error| Error::Query { error })?;

        tx.execute(
            "
            DELETE FROM dependency
            WHERE parent = ?1
            ",
            params![parent_id],
        )
        .map_err(|error| Error::Query { error })?;

        for (dep_id, expected) in dependencies {
            tx.execute(
                "
                INSERT INTO dependency (
                    parent,
                    dep,
                    expected
                )
                VALUES (?1, ?2, ?3)
                ",
                params![parent_id, dep_id, expected.as_bytes(),],
            )
            .map_err(|error| Error::Query { error })?;
        }

        tx.commit().map_err(|error| Error::Query { error })?;

        Ok(())
    }
}

fn decode_hash(bytes: Vec<u8>) -> Result<OutputHash> {
    let actual = bytes.len();

    let bytes: [u8; 32] = bytes
        .try_into()
        .map_err(|_| Error::InvalidHashLength { len: actual })?;

    Ok(OutputHash(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn node(key: &str) -> NodeKey {
        NodeKey {
            kind: "test".to_owned(),
            key: key.to_owned(),
        }
    }

    fn hash(byte: u8) -> OutputHash {
        OutputHash::new([byte; 32])
    }

    fn leaf(key: &str, output: u8) -> Trace {
        Trace {
            node: node(key),
            output_hash: hash(output),
            dependencies: Vec::new(),
        }
    }

    fn dependency(trace: &Trace) -> Dependency {
        Dependency {
            node: trace.node.clone(),
            expected: trace.output_hash,
        }
    }

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
    fn output_hash_accepts_exactly_32_bytes() {
        let result = OutputHash::try_from(vec![7_u8; 32]).unwrap();

        assert_eq!(result, hash(7));
    }

    #[test]
    fn output_hash_rejects_invalid_length() {
        let error = OutputHash::try_from(vec![0_u8; 31]).unwrap_err();

        assert!(matches!(error, Error::InvalidHashLength { len: 31 }));
    }

    #[test]
    fn trace_returns_none_for_unknown_node() {
        let db = Db::new(":memory:").unwrap();

        let result = db.trace(&node("unknown")).unwrap();

        assert_eq!(result, None);
    }

    #[test]
    fn replace_trace_round_trips_leaf() {
        let mut db = Db::new(":memory:").unwrap();

        let trace = leaf("leaf", 1);

        db.replace_trace(&trace).unwrap();

        assert_eq!(db.trace(&trace.node).unwrap(), Some(trace),);
    }

    #[test]
    fn replace_trace_round_trips_dependencies() {
        let mut db = Db::new(":memory:").unwrap();

        let a = leaf("a", 1);
        let b = leaf("b", 2);

        db.replace_trace(&a).unwrap();
        db.replace_trace(&b).unwrap();

        let parent = Trace {
            node: node("parent"),
            output_hash: hash(3),
            dependencies: vec![dependency(&a), dependency(&b)],
        };

        db.replace_trace(&parent).unwrap();

        assert_eq!(db.trace(&parent.node).unwrap(), Some(parent),);
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
                INSERT INTO node (kind, key, output_hash)
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
