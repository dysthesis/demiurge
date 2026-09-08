use std::fs;

use demiurge::{
    context::Ctx,
    store::{Key, Store},
    task::{self, fetch::Fetch, parse::Parse, render::Render},
};
use tempfile::tempdir;

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
