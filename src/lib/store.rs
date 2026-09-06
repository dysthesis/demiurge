use std::{
    fs, io,
    marker::PhantomData,
    path::{Component, Path},
};

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
pub struct Store<'a, K: Identity> {
    /// Where the store's physical backing is located in the filesystem.
    /// We store a reference instead of an owned value in case of an error; since
    /// a sufficiently descriptive error message involves printing the path, the
    /// path would need to live longer than even the store itself sometimes.
    /// Rather than copying, it is more efficient to keep the owned value
    /// somewhere else.
    path: &'a Path,
    _key: PhantomData<K>, // HACK: so that rustc won't complain.
}

#[derive(Debug, thiserror::Error)]
pub enum Error<'a> {
    #[error("The given path has an invalid form: {path}.")]
    InvalidPathFormat { path: &'a Path },

    #[error("Failed to create directory at {path}.")]
    CannotCreateDir {
        path: &'a Path,
        #[source]
        error: io::Error,
    },

    #[error("Cannot inspect store at {path}.")]
    CannotInspectStore {
        path: &'a Path,
        #[source]
        error: io::Error,
    },
}

pub type Result<'a, T> = std::result::Result<T, Error<'a>>;

impl<'a, K: Identity> Store<'a, K> {
    /// Construct a new instance of [`Store`] given a path to a directory that
    /// is/can be used as the physical backing of the data
    pub fn new(path: &'a Path) -> Result<'a, Self> {
        // TODO: path validation
        if !Self::is_valid_path(&path) {
            return Err(Error::InvalidPathFormat { path: &path });
        }

        if !(Self::is_existing_store(&path)?) {
            todo!("Initialise store")
        }

        Ok(Self {
            path,
            _key: PhantomData::<K>,
        })
    }

    #[inline]
    fn is_valid_path(path: &Path) -> bool {
        !path.as_os_str().is_empty()
            && !path
                .components()
                .any(|component| matches!(component, Component::ParentDir))
    }

    #[inline]
    fn is_existing_store<'p>(path: &'p Path) -> Result<'p, bool> {
        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,

            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(false);
            }

            Err(error) => {
                return Err(Error::CannotInspectStore { path, error });
            }
        };

        if !metadata.is_dir() {
            return Ok(false);
        }

        let version = match fs::read(path.join("version")) {
            Ok(version) => version,

            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(false);
            }

            Err(error) => {
                return Err(Error::CannotInspectStore { path, error });
            }
        };

        Ok(version == b"1\n")
    }

    #[inline]
    fn initialise_dir(path: &Path) -> Result<()> {
        // Change if the store layout ever changes.
        fs::create_dir_all(path).map_err(|error| Error::CannotCreateDir { path, error })
    }

    #[inline]
    pub fn contains_bytes(&self, bytes: &[u8]) -> bool {
        let key = K::of(bytes);
        self.contains_key(key)
    }

    #[inline]
    pub fn contains_key(&self, key: K) -> bool {
        todo!("Helper function to check if the given key.")
    }

    pub fn put(&self, bytes: &[u8]) -> Result<K> {
        todo!("Implement the method to put some bytes into the store")
    }

    pub fn get(&self, key: &K) -> Result<Vec<u8>> {
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
            let store = Store::<Key>::new(dir.path()).unwrap();

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
            let store = Store::<Key>::new(dir.path()).unwrap();

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
            let store = Store::<Key>::new(dir.path()).unwrap();

            let key = store.put(&bytes).unwrap();

            prop_assert_eq!(store.get(&key).unwrap(), bytes);
        }

        #[test]
        fn identical_content_has_identical_key(
            bytes in prop::collection::vec(any::<u8>(), 0..MAX_DATA_LEN),
            reps in 1usize..MAX_REPS,
        ) {
            let dir = tempfile::tempdir().unwrap();
            let store = Store::<Key>::new(dir.path()).unwrap();

            let first = store.put(&bytes).unwrap();

            for _ in 1..reps {
                prop_assert_eq!(store.put(&bytes).unwrap(), first.clone());
            }
        }
    }
}
