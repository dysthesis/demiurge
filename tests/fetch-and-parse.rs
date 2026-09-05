use std::{fs, sync::Arc};

use demiurge::task::{self, fetch::Fetch, parse::Parse, Output, Task};
use pulldown_cmark::{Event, HeadingLevel, Tag, TagEnd};
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
fn fetch_and_parse() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("page.md");

    fs::write(&path, "# Hello").unwrap();

    let task: Arc<dyn Task> = Arc::new(Parse::new(Arc::new(Fetch::new(path))));

    let result = evaluate(&task).unwrap();

    let events = result
        .downcast_ref::<Vec<Event<'static>>>()
        .expect("Parse returned the wrong output type");

    assert_eq!(
        events,
        &[
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                id: None,
                classes: vec![],
                attrs: vec![],
            }),
            Event::Text("Hello".into()),
            Event::End(TagEnd::Heading(HeadingLevel::H1)),
        ],
    );
}
