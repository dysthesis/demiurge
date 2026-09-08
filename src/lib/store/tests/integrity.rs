use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
struct CollidingIdentity;

impl Identity for CollidingIdentity {
    fn of(_bytes: &[u8]) -> Self {
        Self
    }

    fn as_bytes(&self) -> &[u8] {
        b"fixed"
    }
}

impl Display for CollidingIdentity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("aabb")
    }
}

#[test]
fn corrupted_object_is_rejected_without_overwriting_it() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::<Key>::new(dir.path().join("store")).unwrap();
    let original = b"original object";
    let corrupted = b"different bytes";
    let key = store.put(original).unwrap();
    let path = store.object_path(&key);
    assert_ne!(Key::of(original), Key::of(corrupted));
    fs::write(&path, corrupted).unwrap();
    assert!(store.contains_key(&key));

    match store.get(&key) {
        Err(Error::CorruptObject { path: error_path }) => {
            assert_eq!(error_path, path)
        }
        _ => panic!("get did not report the corrupted object"),
    }
    assert_eq!(fs::read(&path).unwrap(), corrupted);

    match store.put(original) {
        Err(Error::CorruptObject { path: error_path }) => {
            assert_eq!(error_path, path)
        }
        _ => panic!("put did not report the corrupted object"),
    }
    assert_eq!(fs::read(path).unwrap(), corrupted);
}

#[test]
fn colliding_identity_preserves_the_first_object() {
    let dir = tempfile::tempdir().unwrap();
    let store =
        Store::<CollidingIdentity>::new(dir.path().join("store")).unwrap();
    let first = b"first";
    let second = b"second";
    let key = store.put(first).unwrap();
    let path = store.object_path(&key);

    match store.put(second) {
        Err(Error::IdentityCollision { path: error_path }) => {
            assert_eq!(error_path, path)
        }
        _ => panic!("different bytes with one identity did not collide"),
    }
    assert_eq!(fs::read(&path).unwrap(), first);
    assert_eq!(store.get(&key).unwrap(), first);
    assert_eq!(store.put(first).unwrap(), key);
    assert_eq!(fs::read(path).unwrap(), first);
}

#[test]
fn missing_object_has_a_distinct_error_and_exact_path() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::<Key>::new(dir.path().join("store")).unwrap();
    let key = Key::of(b"never stored");
    let path = store.object_path(&key);

    match store.get(&key) {
        Err(Error::ObjectNotFound { path: error_path }) => {
            assert_eq!(error_path, path)
        }
        _ => panic!("missing object did not report ObjectNotFound"),
    }
}

#[cfg(unix)]
#[test]
fn directory_at_object_path_is_unreadable_not_missing() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::<Key>::new(dir.path().join("store")).unwrap();
    let key = Key::of(b"directory object");
    let path = store.object_path(&key);
    fs::create_dir_all(&path).unwrap();

    match store.get(&key) {
        Err(Error::CannotReadObject {
            path: error_path,
            error,
        }) => {
            assert_eq!(error_path, path);
            assert_ne!(error.kind(), io::ErrorKind::NotFound);
        }
        _ => panic!("unreadable object did not report CannotReadObject"),
    }
}
