use std::{fs, path::PathBuf};

use crate::task::{Error, Output, Spec, Task};

/// A task specification which reads the content of a path.
pub struct Fetch(PathBuf);
impl Fetch {
    #[inline]
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    #[inline]
    fn spec(self) -> impl Spec {
        let Self(path) = self;
        move |dependencies: &[Output]| {
            if !dependencies.is_empty() {
                return Err(Error::DependencyCount {
                    expected: 0,
                    actual: dependencies.len(),
                });
            }

            fs::read(&path).map_err(|source| Error::Read { path, source })
        }
    }
}

const FETCH_TASK_VERSION: usize = 1;

impl From<Fetch> for Task {
    fn from(fetch: Fetch) -> Self {
        Task::new(fetch.spec(), vec![], FETCH_TASK_VERSION)
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn rejects_dependencies() {
        let result =
            Fetch::new(PathBuf::from("anything")).spec()(&[Vec::new()]);

        assert!(matches!(
            result,
            Err(Error::DependencyCount {
                expected: 0,
                actual: 1,
            })
        ));
    }

    #[test]
    fn missing_file_returns_error() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("missing");

        let error = match Fetch::new(path.clone()).spec()(&[]) {
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

        let first = Fetch::new(path.clone()).spec()(&[]).unwrap();

        assert_eq!(first, b"before");

        fs::write(&path, b"after").unwrap();

        let second = Fetch::new(path).spec()(&[]).unwrap();

        assert_eq!(second, b"after");
    }

    proptest! {
        #[test]
        fn returns_exact_file_contents(
            contents in prop::collection::vec(any::<u8>(), 0..65536)
        ) {
            let directory = tempdir().unwrap();
            let path = directory.path().join("file");

            fs::write(&path, &contents).unwrap();

            let result = Fetch::new(path).spec()(&[]).unwrap();

            prop_assert_eq!(result, contents);
        }
    }
}
