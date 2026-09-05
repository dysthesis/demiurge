use crate::task::{Output, Task};
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
        vec![]
    }

    fn run(&self, dependencies: &[Output]) -> Output {
        todo!("Markdown parser")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    struct Source(Vec<u8>);

    impl Task for Source {
        fn dependencies(&self) -> Vec<Arc<dyn Task>> {
            vec![]
        }

        fn run(&self, _dependencies: &[Output]) -> Output {
            Arc::new(self.0.clone())
        }
    }

    fn source(contents: impl Into<Vec<u8>>) -> Arc<dyn Task> {
        Arc::new(Source(contents.into()))
    }
    struct DummySource;

    impl Task for DummySource {
        fn dependencies(&self) -> Vec<Arc<dyn Task>> {
            vec![]
        }

        fn run(&self, _dependencies: &[Output]) -> Output {
            unreachable!("DummySource should not be evaluated in Parse unit tests")
        }
    }

    fn source_task() -> Arc<dyn Task> {
        Arc::new(DummySource)
    }
    fn run_parse(source: impl Into<Vec<u8>>) -> Output {
        let dependency: Output = Arc::new(source.into());
        Parse::new(source_task()).run(std::slice::from_ref(&dependency))
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
