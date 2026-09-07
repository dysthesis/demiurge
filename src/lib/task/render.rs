use pulldown_cmark::Parser;

use crate::{
    context::TaskId,
    task::{Error, Output, Spec, Task},
};

/// A task specification which renders stored Markdown as HTML.
pub struct Render {
    input: TaskId,
}
impl Render {
    #[inline]
    pub fn new(input: TaskId) -> Self {
        Self { input }
    }

    #[inline]
    fn spec() -> impl Spec {
        |dependencies: &[Output]| {
            let [parsed] = dependencies else {
                return Err(Error::DependencyCount {
                    expected: 1,
                    actual: dependencies.len(),
                });
            };

            let source = std::str::from_utf8(parsed)?;

            let mut rendered = String::new();
            pulldown_cmark::html::push_html(&mut rendered, Parser::new(source));
            Ok(rendered.into_bytes())
        }
    }
}

impl From<Render> for Task {
    fn from(render: Render) -> Self {
        Task::new(Render::spec(), vec![render.input])
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(source: &str) -> Output {
        source.as_bytes().to_vec()
    }

    #[test]
    fn rejects_no_dependencies() {
        let result = Render::spec()(&[]);

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
        let output = parsed("");

        let result = Render::spec()(&[output.clone(), output]);

        assert!(matches!(
            result,
            Err(Error::DependencyCount {
                expected: 1,
                actual: 2,
            })
        ));
    }

    #[test]
    fn renders_markdown_as_html() {
        let result = Render::spec()(&[parsed("# Hello")]).unwrap();

        assert_eq!(result, b"<h1>Hello</h1>\n");
    }

    #[test]
    fn rejects_invalid_utf8() {
        let dependency = vec![0xff];

        assert!(matches!(
            Render::spec()(&[dependency]),
            Err(Error::InvalidUtf8(_))
        ));
    }
}
