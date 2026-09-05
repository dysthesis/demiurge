use std::{fs, path::PathBuf, sync::Arc};

use crate::task::{Error, Output, Result, Task};

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

    fn run(&self, _dependencies: &[Output]) -> Result<Output> {
        let contents = fs::read(&self.0).map_err(|source| Error::Read {
            path: self.0.clone(),
            source,
        })?;

        Ok(Arc::new(contents))
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use tempfile::tempdir;

    use super::*;

    fn output(output: &Output) -> &Vec<u8> {
        output
            .downcast_ref::<Vec<u8>>()
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

        let error = match Fetch(path.clone()).run(&[]) {
            Err(error) => error,
            Ok(_) => panic!("missing file should fail"),
        };

        match error {
            Error::Read {
                path: error_path,
                source,
            } => {
                assert_eq!(error_path, path);
                assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
            }
            error => panic!("unexpected error: {error}"),
        }
    }

    #[test]
    fn observes_changes_to_file() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("file");

        fs::write(&path, b"before").unwrap();

        let first = Fetch(path.clone()).run(&[]).unwrap();

        assert_eq!(output(&first), b"before");

        fs::write(&path, b"after").unwrap();

        let second = Fetch(path).run(&[]).unwrap();

        assert_eq!(output(&second), b"after");
    }

    proptest! {
        #[test]
        fn returns_exact_file_contents(
            contents in prop::collection::vec(any::<u8>(), 0..65536)
        ) {
            let directory = tempdir().unwrap();
            let path = directory.path().join("file");

            fs::write(&path, &contents).unwrap();

            let result = Fetch(path).run(&[]).unwrap();

            prop_assert_eq!(
                output(&result),
                &contents,
            );
        }
    }
}
