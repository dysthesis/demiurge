use super::*;

#[test]
fn empty_path_is_rejected_and_preserved() {
    let path = PathBuf::new();

    match Store::<Key>::new(path.clone()) {
        Err(Error::InvalidPathFormat { path: error_path }) => {
            assert_eq!(error_path, path)
        }
        _ => panic!("empty path was not rejected as InvalidPathFormat"),
    }
}

#[test]
fn explicit_parent_component_is_rejected_and_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("child").join("..").join("store");

    match Store::<Key>::new(path.clone()) {
        Err(Error::InvalidPathFormat { path: error_path }) => {
            assert_eq!(error_path, path)
        }
        _ => panic!("parent component was not rejected as InvalidPathFormat"),
    }
}

#[test]
fn constructor_immediately_creates_an_absent_nested_root() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("one").join("two").join("store");
    assert!(!path.exists());

    let _store = Store::<Key>::new(path.clone()).unwrap();

    assert!(path.is_dir());
}

#[test]
fn regular_file_cannot_be_used_as_store_root_and_is_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("store");
    let contents = b"existing file";
    fs::write(&path, contents).unwrap();

    match Store::<Key>::new(path.clone()) {
        Err(Error::CannotCreateDir {
            path: error_path,
            error,
        }) => {
            assert_eq!(error_path, path);
            assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        }
        _ => panic!("regular-file root did not report CannotCreateDir"),
    }
    assert_eq!(fs::read(path).unwrap(), contents);
}

#[test]
fn directory_at_version_path_prevents_store_inspection() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("store");
    fs::create_dir_all(path.join("version")).unwrap();

    match Store::<Key>::new(path.clone()) {
        Err(Error::CannotInspectStore {
            path: error_path,
            error,
        }) => {
            assert_eq!(error_path, path);
            assert_ne!(error.kind(), io::ErrorKind::NotFound);
        }
        _ => panic!("directory version did not report CannotInspectStore"),
    }
    assert!(path.is_dir());
}

#[cfg(unix)]
#[test]
fn regular_file_ancestor_prevents_store_inspection() {
    let dir = tempfile::tempdir().unwrap();
    let ancestor = dir.path().join("ancestor");
    fs::write(&ancestor, b"ancestor file").unwrap();
    let path = ancestor.join("store");

    match Store::<Key>::new(path.clone()) {
        Err(Error::CannotInspectStore {
            path: error_path,
            error,
        }) => {
            assert_eq!(error_path, path);
            assert_eq!(error.kind(), io::ErrorKind::NotADirectory);
        }
        _ => panic!("file ancestor did not report CannotInspectStore"),
    }
    assert_eq!(fs::read(ancestor).unwrap(), b"ancestor file");
}
