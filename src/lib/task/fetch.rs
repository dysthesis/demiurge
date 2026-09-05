use std::{fs, path::PathBuf, sync::Arc};

use crate::task::{Output, Task};

/// A [`Task`] which fetches the content of the given path. It is a leaf Task,
/// which means that it does not depend on anything else.
pub struct Fetch(PathBuf);
impl Fetch {
    #[inline]
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }
}

impl Task for Fetch {
    fn dependencies(&self) -> Vec<std::sync::Arc<dyn Task>> {
        vec![]
    }

    fn run(&self, _dependencies: &[Output]) -> Output {
        Arc::new(fs::read(&self.0))
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use proptest::prelude::*;
    use tempfile::tempdir;

    use super::*;

    fn output(output: &Output) -> &io::Result<Vec<u8>> {
        output
            .downcast_ref::<io::Result<Vec<u8>>>()
            .expect("Fetch returned the wrong output type")
    }

    #[test]
    fn has_no_dependencies() {
        let fetch = Fetch(PathBuf::from("anything"));

        assert!(fetch.dependencies().is_empty());
    }

    #[test]
    fn missing_file_returns_error() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("missing");

        let result = Fetch(path).run(&[]);

        assert_eq!(
            output(&result).as_ref().unwrap_err().kind(),
            io::ErrorKind::NotFound,
        );
    }

    #[test]
    fn observes_changes_to_file() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("file");

        fs::write(&path, b"before").unwrap();

        let first = Fetch(path.clone()).run(&[]);

        assert_eq!(output(&first).as_ref().unwrap(), b"before",);

        fs::write(&path, b"after").unwrap();

        let second = Fetch(path).run(&[]);

        assert_eq!(output(&second).as_ref().unwrap(), b"after",);
    }

    proptest! {
        #[test]
        fn returns_exact_file_contents(
            contents in prop::collection::vec(any::<u8>(), 0..65536)
        ) {
            let directory = tempdir().unwrap();
            let path = directory.path().join("file");

            fs::write(&path, &contents).unwrap();

            let result = Fetch(path).run(&[]);

            prop_assert_eq!(
                output(&result).as_ref().unwrap(),
                &contents,
            );
        }
    }
}
