use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use demiurge::{
    context::{self, Ctx},
    store::{self, Identity, Key, Store},
    task::{self, Output, Task},
};
use tempfile::{TempDir, tempdir};

fn context() -> (TempDir, PathBuf, Ctx) {
    let directory = tempdir().unwrap();
    let root = directory.path().join("store");
    let store = Store::<Key>::new(root.clone()).unwrap();
    (directory, root, Ctx::new(store))
}

fn object_path(root: &Path, key: &Key) -> PathBuf {
    let key = key.to_string();
    root.join("objects").join(&key[..2]).join(&key[2..])
}

#[test]
fn successful_fn_once_is_idempotent() {
    let (_directory, _root, mut ctx) = context();
    let invocations = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&invocations);
    let owned = String::from("consumed output");
    let task = ctx
        .add_task(Task::new(
            move |_: &[Output]| {
                observed.fetch_add(1, Ordering::SeqCst);
                Ok(owned.into_bytes())
            },
            vec![],
            1,
        ))
        .unwrap();

    ctx.run_task(task).unwrap();
    let first_key = ctx.task_result(task).unwrap().unwrap();
    let first_bytes = ctx.get_object(&first_key).unwrap();

    ctx.run_task(task).unwrap();
    let second_key = ctx.task_result(task).unwrap().unwrap();
    let second_bytes = ctx.get_object(&second_key).unwrap();

    assert_eq!(first_key, second_key);
    assert_eq!(first_bytes, b"consumed output");
    assert_eq!(second_bytes, first_bytes);
    assert_eq!(invocations.load(Ordering::SeqCst), 1);
}

#[test]
fn done_does_not_revalidate_or_recreate_objects() {
    let (_directory, root, mut ctx) = context();
    let dependency = ctx
        .add_task(Task::new(
            |_: &[Output]| Ok(b"dependency".to_vec()),
            vec![],
            1,
        ))
        .unwrap();
    let invocations = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&invocations);
    let task = ctx
        .add_task(Task::new(
            move |dependencies: &[Output]| {
                observed.fetch_add(1, Ordering::SeqCst);
                Ok([dependencies[0].as_slice(), b" result"].concat())
            },
            vec![dependency],
            1,
        ))
        .unwrap();

    ctx.run_task(dependency).unwrap();
    ctx.run_task(task).unwrap();
    let dependency_key = ctx.task_result(dependency).unwrap().unwrap();
    let result_key = ctx.task_result(task).unwrap().unwrap();
    assert_eq!(ctx.get_object(&result_key).unwrap(), b"dependency result");

    fs::remove_file(object_path(&root, &dependency_key)).unwrap();
    fs::remove_file(object_path(&root, &result_key)).unwrap();
    ctx.run_task(task).unwrap();

    assert_eq!(ctx.task_result(task).unwrap(), Some(result_key.clone()));
    assert_eq!(invocations.load(Ordering::SeqCst), 1);
    assert!(!object_path(&root, &dependency_key).exists());
    assert!(!object_path(&root, &result_key).exists());
}

#[test]
fn specification_failure_is_terminal_before_dependency_reread() {
    let (_directory, root, mut ctx) = context();
    let dependency = ctx
        .add_task(Task::new(
            |_: &[Output]| Ok(b"dependency".to_vec()),
            vec![],
            1,
        ))
        .unwrap();
    let invocations = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&invocations);
    let failed_path = PathBuf::from("distinguishable-input");
    let task = ctx
        .add_task(Task::new(
            {
                let failed_path = failed_path.clone();
                move |_: &[Output]| {
                    observed.fetch_add(1, Ordering::SeqCst);
                    Err(task::Error::Read {
                        path: failed_path,
                        source: io::Error::from(
                            io::ErrorKind::PermissionDenied,
                        ),
                    })
                }
            },
            vec![dependency],
            1,
        ))
        .unwrap();

    ctx.run_task(dependency).unwrap();
    let dependency_key = ctx.task_result(dependency).unwrap().unwrap();
    match ctx.run_task(task).unwrap_err() {
        task::Error::Read { path, source } => {
            assert_eq!(path, failed_path);
            assert_eq!(source.kind(), io::ErrorKind::PermissionDenied);
        }
        error => panic!("unexpected first error: {error:?}"),
    }
    assert_eq!(ctx.task_result(task).unwrap(), None);
    assert_eq!(invocations.load(Ordering::SeqCst), 1);

    fs::remove_file(object_path(&root, &dependency_key)).unwrap();
    assert!(matches!(ctx.run_task(task), Err(task::Error::TaskConsumed)));
    assert_eq!(ctx.task_result(task).unwrap(), None);
    assert_eq!(invocations.load(Ordering::SeqCst), 1);
}

#[cfg(unix)]
#[test]
fn publication_failure_consumes_task_but_not_fresh_task() {
    let (_directory, root, mut ctx) = context();
    let output = b"publication output";
    let expected_key = Key::of(output);
    let expected_parent = object_path(&root, &expected_key)
        .parent()
        .unwrap()
        .to_owned();
    fs::write(root.join("objects"), b"obstruction").unwrap();

    let invocations = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&invocations);
    let task = ctx
        .add_task(Task::new(
            move |_: &[Output]| {
                observed.fetch_add(1, Ordering::SeqCst);
                Ok(output.to_vec())
            },
            vec![],
            1,
        ))
        .unwrap();

    match ctx.run_task(task).unwrap_err() {
        task::Error::Context(context::Error::StorePutFailed {
            error: store::Error::CannotCreateDir { path, error },
        }) => {
            assert_eq!(path, expected_parent);
            assert_eq!(error.kind(), io::ErrorKind::NotADirectory);
        }
        error => panic!("unexpected publication error: {error:?}"),
    }
    assert_eq!(ctx.task_result(task).unwrap(), None);
    assert_eq!(invocations.load(Ordering::SeqCst), 1);

    fs::remove_file(root.join("objects")).unwrap();
    assert!(matches!(ctx.run_task(task), Err(task::Error::TaskConsumed)));
    assert_eq!(ctx.task_result(task).unwrap(), None);
    assert_eq!(invocations.load(Ordering::SeqCst), 1);

    let fresh = ctx
        .add_task(Task::new(|_: &[Output]| Ok(output.to_vec()), vec![], 1))
        .unwrap();
    ctx.run_task(fresh).unwrap();
    let fresh_key = ctx.task_result(fresh).unwrap().unwrap();
    assert_eq!(fresh_key, expected_key);
    assert_eq!(ctx.get_object(&fresh_key).unwrap(), output);
}

#[test]
fn dependency_read_failures_are_retryable_before_consumption() {
    for corrupt in [false, true] {
        let (_directory, root, mut ctx) = context();
        let dependency_bytes = b"restorable dependency";
        let dependency = ctx
            .add_task(Task::new(
                |_: &[Output]| Ok(dependency_bytes.to_vec()),
                vec![],
                1,
            ))
            .unwrap();
        ctx.run_task(dependency).unwrap();
        let dependency_key = ctx.task_result(dependency).unwrap().unwrap();
        let dependency_path = object_path(&root, &dependency_key);

        if corrupt {
            fs::write(&dependency_path, b"corrupt").unwrap();
        } else {
            fs::remove_file(&dependency_path).unwrap();
        }

        let invocations = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&invocations);
        let consumer = ctx
            .add_task(Task::new(
                move |dependencies: &[Output]| {
                    observed.fetch_add(1, Ordering::SeqCst);
                    Ok([b"consumer: ".as_slice(), dependencies[0].as_slice()]
                        .concat())
                },
                vec![dependency],
                1,
            ))
            .unwrap();

        match ctx.run_task(consumer).unwrap_err() {
            task::Error::Context(context::Error::StoreGetFailed {
                error: store::Error::CorruptObject { path },
            }) if corrupt => assert_eq!(path, dependency_path),
            task::Error::Context(context::Error::StoreGetFailed {
                error: store::Error::ObjectNotFound { path },
            }) if !corrupt => assert_eq!(path, dependency_path),
            error => panic!("unexpected dependency read error: {error:?}"),
        }
        assert_eq!(invocations.load(Ordering::SeqCst), 0);
        assert_eq!(ctx.task_result(consumer).unwrap(), None);

        fs::write(&dependency_path, dependency_bytes).unwrap();
        ctx.run_task(consumer).unwrap();
        let result = ctx.task_result(consumer).unwrap().unwrap();
        assert_eq!(
            ctx.get_object(&result).unwrap(),
            b"consumer: restorable dependency"
        );
        assert_eq!(invocations.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn batch_handles_map_to_their_exact_tasks() {
    let (_directory, _root, mut ctx) = context();
    let original = ctx
        .add_task(Task::new(
            |_: &[Output]| Ok(b"original".to_vec()),
            vec![],
            1,
        ))
        .unwrap();
    let handles = ctx
        .add_tasks([
            Task::new(|_: &[Output]| Ok(b"batch A".to_vec()), vec![], 1),
            Task::new(|_: &[Output]| Ok(b"batch B".to_vec()), vec![], 1),
            Task::new(|_: &[Output]| Ok(b"batch C".to_vec()), vec![], 1),
        ])
        .unwrap();

    for index in [2, 0, 1] {
        ctx.run_task(handles[index]).unwrap();
    }
    ctx.run_task(original).unwrap();

    for (handle, expected) in
        handles
            .into_iter()
            .zip([b"batch A".as_slice(), b"batch B", b"batch C"])
    {
        let key = ctx.task_result(handle).unwrap().unwrap();
        assert_eq!(ctx.get_object(&key).unwrap(), expected);
    }
    let original_key = ctx.task_result(original).unwrap().unwrap();
    assert_eq!(ctx.get_object(&original_key).unwrap(), b"original");
}

#[test]
fn duplicate_dependencies_preserve_order_across_partial_readiness() {
    let (_directory, _root, mut ctx) = context();
    let [a, b, c] = [b"A", b"B", b"C"].map(|output| {
        ctx.add_task(Task::new(
            move |_: &[Output]| Ok(output.to_vec()),
            vec![],
            1,
        ))
        .unwrap()
    });
    let invocations = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&invocations);
    let consumer = ctx
        .add_task(Task::new(
            move |dependencies: &[Output]| {
                observed.fetch_add(1, Ordering::SeqCst);
                Ok(dependencies.concat())
            },
            vec![b, a, b, c],
            1,
        ))
        .unwrap();

    ctx.run_task(a).unwrap();
    ctx.run_task(b).unwrap();
    assert!(matches!(
        ctx.run_task(consumer),
        Err(task::Error::DependencyOutputUnsatisfied)
    ));
    assert_eq!(ctx.task_result(consumer).unwrap(), None);
    assert_eq!(invocations.load(Ordering::SeqCst), 0);

    ctx.run_task(c).unwrap();
    ctx.run_task(consumer).unwrap();
    let result = ctx.task_result(consumer).unwrap().unwrap();
    assert_eq!(ctx.get_object(&result).unwrap(), b"BABC");
    assert_eq!(invocations.load(Ordering::SeqCst), 1);
}
