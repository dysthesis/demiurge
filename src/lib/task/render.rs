use std::sync::Arc;

use pulldown_cmark::Event;

use crate::task::{parse::Parse, Error, Task};

pub struct Render(Arc<dyn Task>);
impl Render {
    #[inline]
    pub fn new(input: Arc<dyn Task>) -> Self {
        Self(input)
    }
}

impl Task for Render {
    fn dependencies(&self) -> Vec<Arc<dyn Task>> {
        vec![self.0.clone()]
    }

    fn run(&self, dependencies: &[super::Output]) -> super::Result<super::Output> {
        let [parsed] = dependencies else {
            return Err(Error::DependencyCount {
                expected: 1,
                actual: dependencies.len(),
            });
        };

        let mut rendered = String::new();
        pulldown_cmark::html::push_html(
            &mut rendered,
            parsed
                .downcast_ref::<Vec<Event>>()
                .expect("Parse returned the wrong output type")
                .into_iter()
                .cloned(),
        );
        Ok(Arc::new(rendered))
    }
}
#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use pulldown_cmark::{HeadingLevel, Tag, TagEnd};

    use crate::task::{fetch::Fetch, Output};

    use super::*;

    fn render() -> Render {
        let fetch = Arc::new(Fetch::new(PathBuf::new()));
        let parse = Arc::new(Parse::new(fetch));

        Render::new(parse)
    }

    #[test]
    fn has_one_dependency() {
        let render = render();

        assert_eq!(render.dependencies().len(), 1);
    }

    #[test]
    fn rejects_no_dependencies() {
        let result = render().run(&[]);

        assert!(matches!(
            result,
            Err(Error::DependencyCount {
                expected: 1,
                actual: 0,
            })
        ));
    }

    #[test]
    fn rejects_multiple_dependencies() {
        let output: Output = Arc::new(Vec::<Event<'static>>::new());

        let result = render().run(&[output.clone(), output]);

        assert!(matches!(
            result,
            Err(Error::DependencyCount {
                expected: 1,
                actual: 2,
            })
        ));
    }

    #[test]
    fn renders_events_as_html() {
        let parsed: Output = Arc::new(vec![
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                id: None,
                classes: vec![],
                attrs: vec![],
            }),
            Event::Text("Hello".into()),
            Event::End(TagEnd::Heading(HeadingLevel::H1)),
        ]);

        let result = render().run(&[parsed]).unwrap();

        let html = result
            .downcast_ref::<String>()
            .expect("Render returned the wrong output type");

        assert_eq!(html, "<h1>Hello</h1>\n");
    }
    #[test]
    #[should_panic(expected = "Parse returned the wrong output type")]
    fn rejects_wrong_dependency_type() {
        let dependency: Output = Arc::new(String::from("not parsed events"));

        let _ = render().run(&[dependency]);
    }
}
