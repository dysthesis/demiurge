use pulldown_cmark::Parser;

use crate::{
    context::TaskId,
    task::{Error, Output, Spec, Task},
};

/// A task specification which parses UTF-8 Markdown.
pub struct Parse {
    input: TaskId,
}
impl Parse {
    #[inline]
    pub fn new(input: TaskId) -> Self {
        Self { input }
    }

    #[inline]
    fn spec() -> impl Spec {
        |dependencies: &[Output]| {
            let [source] = dependencies else {
                return Err(Error::DependencyCount {
                    expected: 1,
                    actual: dependencies.len(),
                });
            };

            let source = std::str::from_utf8(source)?;
            // simplification: Store accepts bytes, so Render reparses until a
            // stable event encoding is introduced.
            Parser::new(source).for_each(drop);
            Ok(source.as_bytes().to_vec())
        }
    }
}

impl From<Parse> for Task {
    fn from(parse: Parse) -> Self {
        Task::new(Parse::spec(), vec![parse.input])
    }
}

#[cfg(test)]
mod tests {
    use crate::task::Result;

    use super::*;
    use proptest::prelude::*;

    fn run_parse(source: impl Into<Vec<u8>>) -> Result<Output> {
        Parse::spec()(std::slice::from_ref(&source.into()))
    }

    #[test]
    fn missing_dependency_returns_error() {
        assert!(matches!(
            Parse::spec()(&[]),
            Err(Error::DependencyCount {
                expected: 1,
                actual: 0,
            }),
        ));
    }

    #[test]
    fn extra_dependency_returns_error() {
        let dependency = Vec::new();
        let dependencies = [dependency.clone(), dependency];

        assert!(matches!(
            Parse::spec()(&dependencies),
            Err(Error::DependencyCount {
                expected: 1,
                actual: 2,
            }),
        ));
    }

    #[test]
    fn invalid_utf8_returns_error() {
        assert!(matches!(
            run_parse([0xff]),
            Err(crate::task::Error::InvalidUtf8(_)),
        ));
    }

    #[test]
    fn preserves_valid_markdown_for_rendering() {
        assert_eq!(run_parse("# Hello").unwrap(), b"# Hello");
    }

    proptest! {
        #[test]
        fn arbitrary_input_does_not_panic(
            contents in prop::collection::vec(any::<u8>(), 0..65536)
        ) {
            let _ = run_parse(contents);
        }
    }
}
