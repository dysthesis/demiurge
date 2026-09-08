use super::*;
use proptest::prelude::*;

fn exercise_lifecycle(first: &[u8], second: &[u8]) {
    assert_ne!(first, second);
    let dir = tempfile::tempdir().unwrap();
    let store = Store::<Key>::new(dir.path().join("store")).unwrap();
    let first_key = Key::of(first);
    let second_key = Key::of(second);
    assert_ne!(first_key, second_key);
    for (key, bytes) in [(&first_key, first), (&second_key, second)] {
        assert!(!store.contains_key(key));
        assert!(!store.contains_bytes(bytes));
        assert!(matches!(store.get(key), Err(Error::ObjectNotFound { .. })));
    }

    assert_eq!(store.put(first).unwrap(), first_key);
    assert!(!store.contains_key(&second_key));
    assert_eq!(store.put(second).unwrap(), second_key);
    for (key, bytes) in [(&first_key, first), (&second_key, second)] {
        assert!(store.contains_key(key));
        assert!(store.contains_bytes(bytes));
        assert_eq!(store.put(bytes).unwrap(), *key);
        for _ in 0..3 {
            assert_eq!(store.get(key).unwrap(), bytes);
        }
    }

    drop(store);
    let reopened = Store::<Key>::new(dir.path().join("store")).unwrap();
    assert_eq!(reopened.get(&first_key).unwrap(), first);
    assert_eq!(reopened.get(&second_key).unwrap(), second);
    fs::remove_file(reopened.object_path(&first_key)).unwrap();
    assert!(!reopened.contains_key(&first_key));
    assert!(!reopened.contains_bytes(first));
    assert!(matches!(
        reopened.get(&first_key),
        Err(Error::ObjectNotFound { .. })
    ));
    assert!(reopened.contains_key(&second_key));
    assert_eq!(reopened.get(&second_key).unwrap(), second);
    assert_eq!(reopened.put(first).unwrap(), first_key);
    assert_eq!(reopened.get(&first_key).unwrap(), first);
}

proptest! {
    #[test]
    fn arbitrary_binary_object_transitions(
        first in prop::collection::vec(any::<u8>(), 0..8192),
        mut second in prop::collection::vec(any::<u8>(), 0..8192),
    ) {
        if first == second {
            second.push(0);
        }
        exercise_lifecycle(&first, &second);
    }
}

#[test]
fn empty_object_and_binary_object_are_independent() {
    exercise_lifecycle(b"", &[0, 255, 128, 0, 10]);
}

#[test]
fn upstream_blake3_vectors_define_bytes_hex_and_disk_layout() {
    // BLAKE3 upstream test_vectors/test_vectors.json, hash mode, first 32
    // bytes. Inputs repeat 0..=250. The six-byte vector starts with 0x06,
    // independently pinning the leading zero in both hex and the bucket name.
    // https://github.com/BLAKE3-team/BLAKE3/blob/master/test_vectors/test_vectors.json
    let vectors: &[(usize, [u8; 32], &str)] = &[
        (
            0,
            [
                0xaf, 0x13, 0x49, 0xb9, 0xf5, 0xf9, 0xa1, 0xa6, 0xa0, 0x40,
                0x4d, 0xea, 0x36, 0xdc, 0xc9, 0x49, 0x9b, 0xcb, 0x25, 0xc9,
                0xad, 0xc1, 0x12, 0xb7, 0xcc, 0x9a, 0x93, 0xca, 0xe4, 0x1f,
                0x32, 0x62,
            ],
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262",
        ),
        (
            6,
            [
                0x06, 0xc4, 0xe8, 0xff, 0xb6, 0x87, 0x2f, 0xad, 0x96, 0xf9,
                0xaa, 0xca, 0x5e, 0xee, 0x15, 0x53, 0xeb, 0x62, 0xae, 0xd0,
                0xad, 0x71, 0x98, 0xce, 0xf4, 0x2e, 0x87, 0xf6, 0xa6, 0x16,
                0xc8, 0x44,
            ],
            "06c4e8ffb6872fad96f9aaca5eee1553eb62aed0ad7198cef42e87f6a616c844",
        ),
    ];
    for &(length, raw, hex) in vectors {
        let bytes: Vec<_> =
            (0..length).map(|index| (index % 251) as u8).collect();
        let key = Key::of(&bytes);
        assert_eq!(key.as_bytes(), raw.as_slice());
        assert_eq!(key.to_string(), hex);
        let dir = tempfile::tempdir().unwrap();
        let store = Store::<Key>::new(dir.path().into()).unwrap();
        assert_eq!(store.put(&bytes).unwrap(), key);
        // Do not use object_path or key Display to construct the oracle path.
        let path = dir.path().join("objects").join(&hex[..2]).join(&hex[2..]);
        assert_eq!(fs::read(&path).unwrap(), bytes);
        drop(store);
        let reopened = Store::<Key>::new(dir.path().into()).unwrap();
        assert_eq!(reopened.get(&key).unwrap(), bytes);

        // Also read a pre-existing canonical object that Store did not write.
        let fixture = tempfile::tempdir().unwrap();
        let path = fixture
            .path()
            .join("objects")
            .join(&hex[..2])
            .join(&hex[2..]);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, &bytes).unwrap();
        let compatible = Store::<Key>::new(fixture.path().into()).unwrap();
        assert_eq!(compatible.get(&Key(raw)).unwrap(), bytes);
    }
}
