mod fetch;
mod parse;
use std::{any::Any, sync::Arc};

// NOTE: This is a placeholder output type until we get Store implemented.
pub(crate) type Output = Arc<dyn Any + Send + Sync>;

/// A unit of work in our build system.
pub trait Task: Send + Sync {
    /// List of other tasks whose output it needs to build.
    fn dependencies(&self) -> Vec<Arc<dyn Task>>;
    /// Take in the dependencies' output and builds its own output. This assumes
    /// that `dependencies` are up-to-date.
    fn run(&self, dependencies: &[Output]) -> Output;
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

        fn run(&self, _dependencies: &[Output]) -> Output {
            Arc::new(self.0)
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
    }

    #[test]
    fn task_can_produce_an_erased_output() {
        let task: Arc<dyn Task> = Arc::new(Constant(42));

        let output = task.run(&[]);

        assert_eq!(output.downcast_ref::<i32>(), Some(&42));
    }
}
