use std::{fs, sync::Arc};

use demiurge::task::{self, fetch::Fetch, parse::Parse, render::Render, Output, Task};
use tempfile::tempdir;

fn evaluate(task: &Arc<dyn Task>) -> task::Result<Output> {
    let dependencies = task
        .dependencies()
        .iter()
        .map(evaluate)
        .collect::<Result<Vec<_>, _>>()?;

    task.run(&dependencies)
}

#[test]
fn fetch_parse_and_render() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("page.md");

    fs::write(&path, "# Hello").unwrap();

    let fetch = Arc::new(Fetch::new(path));
    let parse = Arc::new(Parse::new(fetch));
    let task: Arc<dyn Task> = Arc::new(Render::new(parse));

    let result = evaluate(&task).unwrap();

    let html = result
        .downcast_ref::<String>()
        .expect("Render returned the wrong output type");

    assert_eq!(html, "<h1>Hello</h1>\n");
}
