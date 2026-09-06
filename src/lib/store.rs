use std::{
    fmt::Display,
    fs,
    io::{self, Write},
    marker::PhantomData,
    path::{Component, Path, PathBuf},
};

/// A stable identity for a sequence of bytes.
pub trait Identity: Clone + PartialEq + Eq + Display {
    /// Derive the key of a chunk of bytes.
    fn of(bytes: &[u8]) -> Self;

    /// Canonical byte representation of this identity.
    fn as_bytes(&self) -> &[u8];
}

/// A standard key for an object in the store. We use blake3 as it is faster than
/// SHA-256, while providing equivalent collision-resistance.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Key([u8; 32]);

impl Identity for Key {
    #[inline]
    fn of(bytes: &[u8]) -> Self {
        Self(*blake3::hash(bytes).as_bytes())
    }

    #[inline]
    fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }

        Ok(())
    }
}

/// A content-addressable storage, used to store results from tasks.
pub struct Store<K: Identity> {
    /// Where the store's physical backing is located in the filesystem.
    /// We store a reference instead of an owned value in case of an error; since
    /// a sufficiently descriptive error message involves printing the path, the
    /// path would need to live longer than even the store itself sometimes.
    /// Rather than copying, it is more efficient to keep the owned value
    /// somewhere else.
    path: PathBuf,
    _key: PhantomData<K>, // HACK: so that rustc won't complain.
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("The given path has an invalid form: {path}.")]
    InvalidPathFormat { path: PathBuf },

    #[error("Failed to create directory at {path}.")]
    CannotCreateDir {
        path: PathBuf,
        #[source]
        error: io::Error,
    },

    #[error("Cannot inspect store at {path}.")]
    CannotInspectStore {
        path: PathBuf,
        #[source]
        error: io::Error,
    },

    #[error("Object does not exist: {path}.")]
    ObjectNotFound { path: PathBuf },

    #[error("Cannot read object at {path}.")]
    CannotReadObject {
        path: PathBuf,
        #[source]
        error: io::Error,
    },

    #[error("Object at {path} does not match its content identity.")]
    CorruptObject { path: PathBuf },

    #[error("Two different objects have the same content identity at {path}.")]
    IdentityCollision { path: PathBuf },

    #[error("Cannot synchronise directory at {path}.")]
    CannotSyncDir {
        path: PathBuf,
        #[source]
        error: io::Error,
    },

    #[error("Cannot create temporary object in {path}.")]
    CannotCreateTemp {
        path: PathBuf,
        #[source]
        error: io::Error,
    },

    #[error("Cannot write object at {path}.")]
    CannotWriteObject {
        path: PathBuf,
        #[source]
        error: io::Error,
    },

    #[error("Cannot synchronise object at {path}.")]
    CannotSyncObject {
        path: PathBuf,
        #[source]
        error: io::Error,
    },

    #[error("Cannot publish object at {path}.")]
    CannotPublishObject {
        path: PathBuf,
        #[source]
        error: io::Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

impl<'a, K: Identity> Store<K> {
    /// Construct a new instance of [`Store`] given a path to a directory that
    /// is/can be used as the physical backing of the data
    pub fn new(path: PathBuf) -> Result<Self> {
        // TODO: path validation
        if !Self::is_valid_path(&path) {
            return Err(Error::InvalidPathFormat { path });
        }

        if !(Self::is_existing_store(&path)?) {
            Self::initialise_dir(&path)?;
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
    fn is_existing_store(path: &Path) -> Result<bool> {
        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,

            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(false);
            }

            Err(error) => {
                return Err(Error::CannotInspectStore {
                    path: path.into(),
                    error,
                });
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
                return Err(Error::CannotInspectStore {
                    path: path.into(),
                    error,
                });
            }
        };

        Ok(version == b"1\n")
    }

    #[inline]
    fn initialise_dir(path: &Path) -> Result<()> {
        // Change if the store layout ever changes.
        fs::create_dir_all(path).map_err(|error| Error::CannotCreateDir {
            path: path.into(),
            error,
        })
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

    #[inline]
    fn object_path(&self, key: &K) -> PathBuf {
        let key = key.to_string();

        self.path.join("objects").join(&key[..2]).join(&key[2..])
    }

    #[inline]
    fn verify_existing(
        &self,
        key: &K,
        requested: &[u8],
        existing: &[u8],
        path: &Path,
    ) -> Result<()> {
        if existing == requested {
            return Ok(());
        }

        if K::of(existing) == *key {
            // Existing bytes are different but genuinely have the same
            // identity.
            Err(Error::IdentityCollision {
                path: path.to_owned(),
            })
        } else {
            // The filename says this is `key`, but its contents hash to
            // something else.
            Err(Error::CorruptObject {
                path: path.to_owned(),
            })
        }
    }

    pub fn put(&self, bytes: &[u8]) -> Result<K> {
        let key = K::of(bytes);
        let path = self.object_path(&key);

        let parent = path.parent().expect("object path always has a parent");

        fs::create_dir_all(parent).map_err(|error| Error::CannotCreateDir {
            path: parent.to_owned(),
            error,
        })?;

        // This is only an optimisation. Correctness must not depend on this
        // check because another writer can appear/disappear after it.
        match fs::read(&path) {
            Ok(existing) => {
                self.verify_existing(&key, bytes, &existing, &path)?;
                return Ok(key);
            }

            Err(error) if error.kind() == io::ErrorKind::NotFound => {}

            Err(error) => {
                return Err(Error::CannotReadObject { path, error });
            }
        }

        // The temporary file lives in the same directory as the final object.
        // That matters both for publication semantics and because hard links
        // generally require source and destination to be on the same filesystem.
        let mut temporary = tempfile::Builder::new()
            .prefix(".tmp-")
            .tempfile_in(parent)
            .map_err(|error| Error::CannotCreateTemp {
                path: parent.to_owned(),
                error,
            })?;

        temporary
            .write_all(bytes)
            .map_err(|error| Error::CannotWriteObject {
                path: temporary.path().to_owned(),
                error,
            })?;

        // Ensure all object contents are durable before making the final
        // filename visible.
        temporary
            .as_file()
            .sync_all()
            .map_err(|error| Error::CannotSyncObject {
                path: temporary.path().to_owned(),
                error,
            })?;

        loop {
            match fs::hard_link(temporary.path(), &path) {
                // Race won, the final pathname now refers to the
                // already-complete inode.
                Ok(()) => {
                    sync_directory(parent)?;

                    // `temporary` dropping removes only its temporary pathname.
                    // The final hard link remains.
                    return Ok(key);
                }

                // Another writer published this identity first.
                //
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    match fs::read(&path) {
                        Ok(existing) => {
                            self.verify_existing(&key, bytes, &existing, &path)?;

                            return Ok(key);
                        }

                        /*
                         * Something removed the object between hard_link()
                         * reporting AlreadyExists and our read.
                         *
                         * A normal Store never does this during put(), but this
                         * also makes the operation behave sensibly alongside a
                         * future concurrent GC.
                         */
                        Err(error) if error.kind() == io::ErrorKind::NotFound => {
                            continue;
                        }

                        Err(error) => {
                            return Err(Error::CannotReadObject { path, error });
                        }
                    }
                }

                Err(error) => {
                    return Err(Error::CannotPublishObject { path, error });
                }
            }
        }
    }
    pub fn get(&self, key: &K) -> Result<Vec<u8>> {
        let path = self.object_path(key);

        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,

            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(Error::ObjectNotFound { path });
            }

            Err(error) => {
                return Err(Error::CannotReadObject { path, error });
            }
        };

        if K::of(&bytes) != *key {
            return Err(Error::CorruptObject { path });
        }

        Ok(bytes)
    }
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";

    let mut result = String::with_capacity(bytes.len() * 2);

    for &byte in bytes {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 0x0f) as usize] as char);
    }

    result
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<()> {
    fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| Error::CannotSyncDir {
            path: path.to_owned(),
            error,
        })
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<()> {
    todo!("I don't use W*ndows lmao.")
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
            let store = Store::<Key>::new(dir.path().into()).unwrap();

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
            let store = Store::<Key>::new(dir.path().into()).unwrap();

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
            let store = Store::<Key>::new(dir.path().into()).unwrap();

            let key = store.put(&bytes).unwrap();

            prop_assert_eq!(store.get(&key).unwrap(), bytes);
        }

        #[test]
        fn identical_content_has_identical_key(
            bytes in prop::collection::vec(any::<u8>(), 0..MAX_DATA_LEN),
            reps in 1usize..MAX_REPS,
        ) {
            let dir = tempfile::tempdir().unwrap();
            let store = Store::<Key>::new(dir.path().into()).unwrap();

            let first = store.put(&bytes).unwrap();

            for _ in 1..reps {
                prop_assert_eq!(store.put(&bytes).unwrap(), first.clone());
            }
        }
    }
}
