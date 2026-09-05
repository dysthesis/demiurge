pub mod fetch;
pub mod parse;
use std::{any::Any, io, path::PathBuf, result, str::Utf8Error, sync::Arc};

use thiserror::Error;

// NOTE: This is a placeholder output type until we get Store implemented.
pub type Output = Arc<dyn Any + Send + Sync>;
pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("expected {expected} dependency outputs, got {actual}")]
    DependencyCount { expected: usize, actual: usize },
    #[error("dependency output at index {index} has an unexpected type")]
    DependencyType { index: usize },
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Markdown source is not valid UTF-8: {0}")]
    InvalidUtf8(#[from] Utf8Error),
}

/// A unit of work in our build system.
pub trait Task: Send + Sync {
    /// List of other tasks whose output it needs to build.
    fn dependencies(&self) -> Vec<Arc<dyn Task>>;
    /// Take in the dependencies' output and builds its own output. This assumes
    /// that `dependencies` are up-to-date.
    fn run(&self, dependencies: &[Output]) -> Result<Output>;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Contrived, basic task for testing.
    struct Constant(i32);

    impl Task for Constant {
        fn dependencies(&self) -> Vec<Arc<dyn Task>> {
            vec![]
        }

        fn run(&self, _dependencies: &[Output]) -> Result<Output> {
            Ok(Arc::new(self.0))
        }
    }

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn task_is_dyn_compatible() {
        let _: Arc<dyn Task> = Arc::new(Constant(42));
    }

    #[test]
    fn task_references_are_send_and_sync() {
        assert_send_sync::<Arc<dyn Task>>();
        assert_send_sync::<Output>();
        assert_send_sync::<Error>();
    }

    #[test]
    fn task_can_produce_an_erased_output() {
        let task: Arc<dyn Task> = Arc::new(Constant(42));

        let output = task.run(&[]).unwrap();

        assert_eq!(output.downcast_ref::<i32>(), Some(&42));
    }
}
