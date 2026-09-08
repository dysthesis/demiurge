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

#[test]
fn interleaved_chains_keep_sources_and_outputs_separate() {
    let directory = tempdir().unwrap();
    let path_a = directory.path().join("a.md");
    let path_b = directory.path().join("b.md");
    let source_a = b"# Alpha";
    let source_b = b"Beta **two**";
    fs::write(&path_a, source_a).unwrap();
    fs::write(&path_b, source_b).unwrap();

    let store = Store::<Key>::new(directory.path().join("store")).unwrap();
    let mut ctx = Ctx::new(store);
    let fetch_a = ctx.add_task(Fetch::new(path_a)).unwrap();
    let fetch_b = ctx.add_task(Fetch::new(path_b)).unwrap();
    let parse_b = ctx.add_task(Parse::new(fetch_b)).unwrap();
    let parse_a = ctx.add_task(Parse::new(fetch_a)).unwrap();
    let render_a = ctx.add_task(Render::new(parse_a)).unwrap();
    let render_b = ctx.add_task(Render::new(parse_b)).unwrap();

    ctx.run_task(fetch_b).unwrap();
    assert!(matches!(
        ctx.run_task(parse_a),
        Err(task::Error::DependencyOutputUnsatisfied)
    ));
    assert_eq!(ctx.task_result(parse_a).unwrap(), None);
    assert!(matches!(
        ctx.run_task(render_b),
        Err(task::Error::DependencyOutputUnsatisfied)
    ));
    assert_eq!(ctx.task_result(render_b).unwrap(), None);

    ctx.run_task(parse_b).unwrap();
    ctx.run_task(render_b).unwrap();
    ctx.run_task(fetch_a).unwrap();
    ctx.run_task(parse_a).unwrap();
    ctx.run_task(render_a).unwrap();

    let fetch_key_a = ctx.task_result(fetch_a).unwrap().unwrap();
    let fetch_key_b = ctx.task_result(fetch_b).unwrap().unwrap();
    assert_ne!(fetch_key_a, fetch_key_b);
    assert_eq!(ctx.get_object(&fetch_key_a).unwrap(), source_a);
    assert_eq!(ctx.get_object(&fetch_key_b).unwrap(), source_b);

    let render_key_a = ctx.task_result(render_a).unwrap().unwrap();
    let render_key_b = ctx.task_result(render_b).unwrap().unwrap();
    assert_ne!(render_key_a, render_key_b);
    assert_eq!(ctx.get_object(&render_key_a).unwrap(), b"<h1>Alpha</h1>\n");
    assert_eq!(
        ctx.get_object(&render_key_b).unwrap(),
        b"<p>Beta <strong>two</strong></p>\n"
    );
}

#[test]
fn fresh_context_observes_changes_without_altering_old_objects() {
    let directory = tempdir().unwrap();
    let root = directory.path().join("store");
    let path = directory.path().join("changing.md");
    let old_source = b"# Before";
    fs::write(&path, old_source).unwrap();

    let mut old_ctx = Ctx::new(Store::<Key>::new(root.clone()).unwrap());
    let old_fetch = old_ctx.add_task(Fetch::new(path.clone())).unwrap();
    let old_parse = old_ctx.add_task(Parse::new(old_fetch)).unwrap();
    let old_render = old_ctx.add_task(Render::new(old_parse)).unwrap();
    old_ctx.run_task(old_fetch).unwrap();
    old_ctx.run_task(old_parse).unwrap();
    old_ctx.run_task(old_render).unwrap();
    let old_fetch_key = old_ctx.task_result(old_fetch).unwrap().unwrap();
    let old_render_key = old_ctx.task_result(old_render).unwrap().unwrap();

    let new_source = b"# After & beyond";
    fs::write(&path, new_source).unwrap();
    old_ctx.run_task(old_fetch).unwrap();
    old_ctx.run_task(old_parse).unwrap();
    old_ctx.run_task(old_render).unwrap();
    assert_eq!(
        old_ctx.task_result(old_fetch).unwrap(),
        Some(old_fetch_key.clone())
    );
    assert_eq!(
        old_ctx.task_result(old_render).unwrap(),
        Some(old_render_key.clone())
    );

    let mut fresh_ctx = Ctx::new(Store::<Key>::new(root).unwrap());
    assert_eq!(fresh_ctx.get_object(&old_fetch_key).unwrap(), old_source);
    assert_eq!(
        fresh_ctx.get_object(&old_render_key).unwrap(),
        b"<h1>Before</h1>\n"
    );
    let fresh_fetch = fresh_ctx.add_task(Fetch::new(path)).unwrap();
    let fresh_parse = fresh_ctx.add_task(Parse::new(fresh_fetch)).unwrap();
    let fresh_render = fresh_ctx.add_task(Render::new(fresh_parse)).unwrap();
    fresh_ctx.run_task(fresh_fetch).unwrap();
    fresh_ctx.run_task(fresh_parse).unwrap();
    fresh_ctx.run_task(fresh_render).unwrap();

    let fresh_fetch_key = fresh_ctx.task_result(fresh_fetch).unwrap().unwrap();
    let fresh_render_key =
        fresh_ctx.task_result(fresh_render).unwrap().unwrap();
    assert_ne!(fresh_fetch_key, old_fetch_key);
    assert_ne!(fresh_render_key, old_render_key);
    assert_eq!(fresh_ctx.get_object(&fresh_fetch_key).unwrap(), new_source);
    assert_eq!(
        fresh_ctx.get_object(&fresh_render_key).unwrap(),
        b"<h1>After &amp; beyond</h1>\n"
    );
    assert_eq!(fresh_ctx.get_object(&old_fetch_key).unwrap(), old_source);
    assert_eq!(
        fresh_ctx.get_object(&old_render_key).unwrap(),
        b"<h1>Before</h1>\n"
    );
}
