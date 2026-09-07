use std::{fs, path::PathBuf};

use crate::task::{Error, Output, Spec};

/// A task specification which reads the content of a path.
pub struct Fetch;
impl Fetch {
    #[inline]
    pub fn spec(path: PathBuf) -> impl Spec {
        move |dependencies: &[Output]| {
            if !dependencies.is_empty() {
                return Err(Error::DependencyCount {
                    expected: 0,
                    actual: dependencies.len(),
                });
            }

            fs::read(&path).map_err(|source| Error::Read {
                path: path.clone(),
                source,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn rejects_dependencies() {
        let result = Fetch::spec(PathBuf::from("anything"))(&[Vec::new()]);

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

        let error = match Fetch::spec(path.clone())(&[]) {
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

        let first = Fetch::spec(path.clone())(&[]).unwrap();

        assert_eq!(first, b"before");

        fs::write(&path, b"after").unwrap();

        let second = Fetch::spec(path)(&[]).unwrap();

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

            let result = Fetch::spec(path)(&[]).unwrap();

            prop_assert_eq!(result, contents);
        }
    }
}
