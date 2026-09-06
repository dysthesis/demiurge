use std::path::PathBuf;

use crate::store::{Key, Store};

/// The whole build system.
pub struct Build<'a> {
    /// Owned value for where the store is located in the filesystem. See the
    /// documentation comment for [`Store`] for rationale.
    store_path: PathBuf,
    /// Logical representation for the object storage.
    store: Store<'a, Key>,
}
