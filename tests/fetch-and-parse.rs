use std::{
    fs, io,
    path::{Path, PathBuf},
};

use demiurge::{
    context::{self, Ctx},
    store::{self, Key, Store},
    task::{self, fetch::Fetch, parse::Parse, render::Render},
};
use tempfile::tempdir;

fn object_path(root: &Path, key: &Key) -> PathBuf {
    let key = key.to_string();
    root.join("objects").join(&key[..2]).join(&key[2..])
}

#[test]
fn pipeline_waits_for_each_stage_and_renders_exact_inputs() {
    let fixtures: &[(&str, &[u8], &[u8])] = &[
        ("happy", b"# Hello", b"<h1>Hello</h1>\n"),
        ("empty", b"", b""),
        (
            "unicode",
            "# Caf\u{e9} & tea\n\n**Bold** and `a < b & c`.\n".as_bytes(),
            "<h1>Caf\u{e9} &amp; tea</h1>\n<p><strong>Bold</strong> and <code>a &lt; b &amp; c</code>.</p>\n"
                .as_bytes(),
        ),
    ];

    for &(name, source, expected_html) in fixtures {
        let directory = tempdir().unwrap();
        let path = directory.path().join(format!("{name}.md"));
        fs::write(&path, source).unwrap();

        let store = Store::<Key>::new(directory.path().join("store")).unwrap();
        let mut ctx = Ctx::new(store);
        let fetch = ctx.add_task(Fetch::new(path)).unwrap();
        let parse = ctx.add_task(Parse::new(fetch)).unwrap();
        let render = ctx.add_task(Render::new(parse)).unwrap();

        assert!(matches!(
            ctx.run_task(parse),
            Err(task::Error::DependencyOutputUnsatisfied)
        ));
        assert_eq!(ctx.task_result(parse).unwrap(), None);
        assert!(matches!(
            ctx.run_task(render),
            Err(task::Error::DependencyOutputUnsatisfied)
        ));
        assert_eq!(ctx.task_result(render).unwrap(), None);

        ctx.run_task(fetch).unwrap();
        let fetch_key = ctx.task_result(fetch).unwrap().unwrap();
        assert_eq!(ctx.get_object(&fetch_key).unwrap(), source);

        assert!(matches!(
            ctx.run_task(render),
            Err(task::Error::DependencyOutputUnsatisfied)
        ));
        assert_eq!(ctx.task_result(render).unwrap(), None);

        ctx.run_task(parse).unwrap();
        ctx.run_task(render).unwrap();
        let render_key = ctx.task_result(render).unwrap().unwrap();
        assert_eq!(ctx.get_object(&render_key).unwrap(), expected_html);
    }
}

#[test]
fn invalid_utf8_stops_after_fetch() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("binary.md");
    let source = b"valid prefix\xffinvalid suffix";
    fs::write(&path, source).unwrap();

    let store = Store::<Key>::new(directory.path().join("store")).unwrap();
    let mut ctx = Ctx::new(store);
    let fetch = ctx.add_task(Fetch::new(path)).unwrap();
    let parse = ctx.add_task(Parse::new(fetch)).unwrap();
    let render = ctx.add_task(Render::new(parse)).unwrap();

    ctx.run_task(fetch).unwrap();
    let fetch_key = ctx.task_result(fetch).unwrap().unwrap();
    assert_eq!(ctx.get_object(&fetch_key).unwrap(), source);

    assert!(matches!(
        ctx.run_task(parse),
        Err(task::Error::InvalidUtf8(_))
    ));
    assert_eq!(ctx.task_result(parse).unwrap(), None);
    assert!(matches!(
        ctx.run_task(parse),
        Err(task::Error::TaskConsumed)
    ));
    assert_eq!(ctx.task_result(parse).unwrap(), None);
    assert!(matches!(
        ctx.run_task(render),
        Err(task::Error::DependencyOutputUnsatisfied)
    ));
    assert_eq!(ctx.task_result(render).unwrap(), None);
}

#[test]
fn missing_source_is_terminal_but_a_fresh_chain_recovers() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("created-later.md");
    let store = Store::<Key>::new(directory.path().join("store")).unwrap();
    let mut ctx = Ctx::new(store);
    let fetch = ctx.add_task(Fetch::new(path.clone())).unwrap();
    let parse = ctx.add_task(Parse::new(fetch)).unwrap();
    let render = ctx.add_task(Render::new(parse)).unwrap();

    match ctx.run_task(fetch).unwrap_err() {
        task::Error::Read {
            path: actual,
            source,
        } => {
            assert_eq!(actual, path);
            assert_eq!(source.kind(), io::ErrorKind::NotFound);
        }
        error => panic!("unexpected fetch error: {error:?}"),
    }
    assert_eq!(ctx.task_result(fetch).unwrap(), None);
    for downstream in [parse, render] {
        assert!(matches!(
            ctx.run_task(downstream),
            Err(task::Error::DependencyOutputUnsatisfied)
        ));
        assert_eq!(ctx.task_result(downstream).unwrap(), None);
    }

    fs::write(&path, b"# Recovered").unwrap();
    assert!(matches!(
        ctx.run_task(fetch),
        Err(task::Error::TaskConsumed)
    ));
    assert_eq!(ctx.task_result(fetch).unwrap(), None);

    let fresh_fetch = ctx.add_task(Fetch::new(path)).unwrap();
    let fresh_parse = ctx.add_task(Parse::new(fresh_fetch)).unwrap();
    let fresh_render = ctx.add_task(Render::new(fresh_parse)).unwrap();
    ctx.run_task(fresh_fetch).unwrap();
    ctx.run_task(fresh_parse).unwrap();
    ctx.run_task(fresh_render).unwrap();
    let result = ctx.task_result(fresh_render).unwrap().unwrap();
    assert_eq!(ctx.get_object(&result).unwrap(), b"<h1>Recovered</h1>\n");
}

#[test]
fn corrupted_fetch_object_blocks_parse_until_restored() {
    let directory = tempdir().unwrap();
    let root = directory.path().join("store");
    let path = directory.path().join("source.md");
    let source = b"# Restorable";
    fs::write(&path, source).unwrap();

    let store = Store::<Key>::new(root.clone()).unwrap();
    let mut ctx = Ctx::new(store);
    let fetch = ctx.add_task(Fetch::new(path)).unwrap();
    let parse = ctx.add_task(Parse::new(fetch)).unwrap();
    let render = ctx.add_task(Render::new(parse)).unwrap();
    ctx.run_task(fetch).unwrap();
    let fetch_key = ctx.task_result(fetch).unwrap().unwrap();
    let fetch_object = object_path(&root, &fetch_key);
    fs::write(&fetch_object, b"corrupt").unwrap();

    match ctx.run_task(parse).unwrap_err() {
        task::Error::Context(context::Error::StoreGetFailed {
            error: store::Error::CorruptObject { path },
        }) => assert_eq!(path, fetch_object),
        error => panic!("unexpected parse dependency error: {error:?}"),
    }
    assert_eq!(ctx.task_result(parse).unwrap(), None);
    assert!(matches!(
        ctx.run_task(render),
        Err(task::Error::DependencyOutputUnsatisfied)
    ));
    assert_eq!(ctx.task_result(render).unwrap(), None);

    fs::write(&fetch_object, source).unwrap();
    ctx.run_task(parse).unwrap();
    ctx.run_task(render).unwrap();
    let result = ctx.task_result(render).unwrap().unwrap();
    assert_eq!(ctx.get_object(&result).unwrap(), b"<h1>Restorable</h1>\n");
}
