use std::{collections::HashMap, fmt::Display, marker::PhantomData, path::PathBuf};

/// A stable identity for a sequence of bytes.
pub trait Identity: Clone + PartialEq + Eq {
    /// Derive the key of a chunk of bytes.
    fn of(bytes: &[u8]) -> Self;
}

/// A standard key for an object in the store. We use blake3 as it is faster than
/// SHA-256, while providing equivalent collision-resistance.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Key([u8; 32]);

impl Identity for Key {
    fn of(bytes: &[u8]) -> Self {
        Self(*blake3::hash(bytes).as_bytes())
    }
}

/// A content-addressable storage, used to store results from tasks.
pub struct Store<K: Identity> {
    /// Where the store's physical backing is located in the filesystem.
    path: PathBuf,
    _key: PhantomData<K>, // HACK: so that rustc won't complain.
}

#[derive(Debug, thiserror::Error)]
pub enum Error {}

pub type Result<T> = std::result::Result<T, Error>;

impl<K: Identity> Store<K> {
    /// Construct a new instance of [`Store`] given a path to a directory that
    /// is/can be used as the physical backing of the data
    pub fn new(path: PathBuf) -> Self {
        // TODO: path validation
        Self {
            path,
            _key: PhantomData::<K>,
        }
    }

    pub fn put(&self, bytes: &[u8]) -> Result<K> {
        todo!("Implement the method to put some bytes into the store")
    }

    pub fn get<'a>(&self, key: &K) -> Result<Vec<u8>> {
        todo!("Implement the method to get some bytes associated with the given key")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const MAX_DATA_LEN: usize = 8192;
    const MAX_REPS: usize = 10000;

    proptest! {
        #[test]
        fn put_idempotency(
            bytes in prop::collection::vec(any::<u8>(), 0..MAX_DATA_LEN),
            reps in 1usize..MAX_REPS,
        ) {
            let dir = tempfile::tempdir().unwrap();
            let store = Store::<Key>::new(dir.path().into());

            let expected = store.put(&bytes).unwrap();
            for _ in 1..reps {
                let key = store.put(&bytes).unwrap();
                prop_assert_eq!(key, expected.clone());
            }
            prop_assert_eq!(store.get(&expected).unwrap(), bytes);
        }

        #[test]
        fn get_idempotency(
            bytes in prop::collection::vec(any::<u8>(), 0..MAX_DATA_LEN),
            reps in 1usize..MAX_REPS,
        ) {
            let dir = tempfile::tempdir().unwrap();
            let store = Store::<Key>::new(dir.path().into());

            let key = store.put(&bytes).unwrap();

            for _ in 0..reps {
                prop_assert_eq!(store.get(&key).unwrap(), bytes.clone());
            }
        }

        #[test]
        fn put_then_get_roundtrips(
            bytes in prop::collection::vec(any::<u8>(), 0..MAX_DATA_LEN)
        ) {
            let dir = tempfile::tempdir().unwrap();
            let store = Store::<Key>::new(dir.path().into());

            let key = store.put(&bytes).unwrap();

            prop_assert_eq!(store.get(&key).unwrap(), bytes);
        }

        #[test]
        fn identical_content_has_identical_key(
            bytes in prop::collection::vec(any::<u8>(), 0..MAX_DATA_LEN),
            reps in 1usize..MAX_REPS,
        ) {
            let dir = tempfile::tempdir().unwrap();
            let store = Store::<Key>::new(dir.path().into());

            let first = store.put(&bytes).unwrap();

            for _ in 1..reps {
                prop_assert_eq!(store.put(&bytes).unwrap(), first.clone());
            }
        }
    }
}
