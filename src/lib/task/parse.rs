use pulldown_cmark::{Event, Parser};

use crate::task::{Output, Result, Task};
use std::sync::Arc;

/// A [`Task`] which parses the contents of the given [`Task`]'s output as
/// Markdown.
pub struct Parse(Arc<dyn Task>);
impl Parse {
    #[inline]
    pub fn new(input: Arc<dyn Task>) -> Self {
        Self(input)
    }
}

impl Task for Parse {
    fn dependencies(&self) -> Vec<Arc<dyn Task>> {
        vec![self.0.clone()]
    }
    fn run(&self, dependencies: &[Output]) -> Result<Output> {
        let source = dependencies[0]
            .downcast_ref::<Vec<u8>>()
            .expect("Parse dependency returned the wrong output type");

        let source = std::str::from_utf8(source)?;

        let events: Vec<Event<'static>> = Parser::new(source).map(Event::into_static).collect();

        Ok(Arc::new(events))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    struct DummySource;

    impl Task for DummySource {
        fn dependencies(&self) -> Vec<Arc<dyn Task>> {
            vec![]
        }

        fn run(&self, _dependencies: &[Output]) -> Result<Output> {
            unreachable!("DummySource should not be evaluated in Parse unit tests")
        }
    }

    fn source_task() -> Arc<dyn Task> {
        Arc::new(DummySource)
    }
    fn run_parse(source: impl Into<Vec<u8>>) -> Result<Output> {
        let dependency: Output = Arc::new(source.into());
        Parse::new(source_task()).run(std::slice::from_ref(&dependency))
    }

    #[test]
    fn invalid_utf8_returns_error() {
        assert!(matches!(
            run_parse([0xff]),
            Err(crate::task::Error::InvalidUtf8(_)),
        ));
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
