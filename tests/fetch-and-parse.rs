use std::fs;

use demiurge::{
    context::Ctx,
    store::{Key, Store},
    task::{fetch::Fetch, parse::Parse, render::Render},
};
use tempfile::tempdir;

#[test]
fn fetch_parse_and_render() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("page.md");

    fs::write(&path, "# Hello").unwrap();

    let store = Store::<Key>::new(directory.path().join("store")).unwrap();
    let mut ctx = Ctx::new(store);
    let fetch = ctx.add_task(Fetch::new(path)).unwrap();
    let parse = ctx.add_task(Parse::new(fetch)).unwrap();
    let render = ctx.add_task(Render::new(parse)).unwrap();

    ctx.run_task(fetch).unwrap();
    ctx.run_task(parse).unwrap();
    ctx.run_task(render).unwrap();

    let key = ctx.task_result(render).unwrap().unwrap();
    let html = ctx.get_object(&key).unwrap();

    assert_eq!(html, b"<h1>Hello</h1>\n");
}
