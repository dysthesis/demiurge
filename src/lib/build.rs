use std::path::PathBuf;

use crate::store::{Key, Store};

/// The whole build system.
pub struct Build {
    /// Logical representation for the object storage.
    store: Store<Key>,
}
