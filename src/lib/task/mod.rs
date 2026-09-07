pub mod fetch;
pub mod parse;
pub mod render;
use std::{io, path::PathBuf, result, str::Utf8Error};

use thiserror::Error;

use crate::{
    context::{self, Ctx, TaskId},
    store::Key,
};

/// The serialisable value passed between task specifications.
pub type Output = Vec<u8>;
pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Context(#[from] context::Error),

    #[error("expected {expected} dependency outputs, got {actual}")]
    DependencyCount { expected: usize, actual: usize },
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Markdown source is not valid UTF-8: {0}")]
    InvalidUtf8(#[from] Utf8Error),

    #[error("one or more task dependencies have not completed")]
    DependencyOutputUnsatisfied,
    #[error("task specification has already been consumed")]
    TaskConsumed,
}

/// The actual function that builds the desired output, given its list of
/// dependencies. A specification is one-shot because its task runs at most once
/// in a build.
pub trait Spec: Send + FnOnce(&[Output]) -> Result<Output> {}

impl<T> Spec for T where T: Send + FnOnce(&[Output]) -> Result<Output> {}

/// The execution state of a task.
#[derive(Debug)]
pub enum State {
    Pending,
    Done(Key),
    Failed,
}

/// A unit of work in our build systems. Produces an object that is stored in
/// [`crate::store::Store`], providing us with a key to it, and optionally
/// requires the outputs of other tasks as dependencies.
pub struct Task {
    deps: Vec<TaskId>,
    spec: Option<Box<dyn Spec>>,
    state: State,
}

impl Task {
    pub fn register(
        ctx: &mut Ctx,
        spec: Box<dyn Spec>,
        dependencies: Vec<TaskId>,
    ) -> Result<TaskId> {
        let this = Task {
            deps: dependencies,
            spec: Some(spec),
            state: State::Pending,
        };

        Ok(ctx.add_task(this)?)
    }

    #[inline]
    fn get_deps_results(&self, ctx: &Ctx) -> Result<Vec<Output>> {
        self.deps
            .iter()
            .map(|&id| -> Result<Output> {
                let key = ctx
                    .task_result(id)?
                    .ok_or(Error::DependencyOutputUnsatisfied)?;
                Ok(ctx.get_object(&key)?)
            })
            .collect()
    }

    pub fn run(&mut self, ctx: &Ctx) -> Result<()> {
        match &self.state {
            State::Done(_) => return Ok(()),
            State::Failed => return Err(Error::TaskConsumed),
            State::Pending => {}
        }

        let deps = self.get_deps_results(ctx)?;
        let spec = self.spec.take().ok_or(Error::TaskConsumed)?;

        match spec(&deps)
            .and_then(|output| ctx.publish_object(&output).map_err(Error::from))
        {
            Ok(key) => {
                self.state = State::Done(key);
                Ok(())
            }
            Err(error) => {
                self.state = State::Failed;
                Err(error)
            }
        }
    }

    pub fn result(&self) -> Option<Key> {
        match &self.state {
            State::Pending | State::Failed => None,
            State::Done(res) => Some(res.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::store::{Key, Store};

    use super::*;

    fn context() -> (tempfile::TempDir, Ctx) {
        let directory = tempfile::tempdir().unwrap();
        let store = Store::<Key>::new(directory.path().join("store")).unwrap();
        (directory, Ctx::new(store))
    }

    fn assert_send<T: Send>() {}

    #[test]
    fn spec_is_dyn_compatible() {
        let _: Box<dyn Spec> =
            Box::new(|_: &[Output]| Ok(b"constant".to_vec()));
    }

    #[test]
    fn tasks_and_errors_are_send() {
        assert_send::<Box<dyn Spec>>();
        assert_send::<Task>();
        assert_send::<Error>();
    }

    #[test]
    fn task_publishes_its_output() {
        let (_directory, mut ctx) = context();
        let task = Task::register(
            &mut ctx,
            Box::new(|_: &[Output]| Ok(b"constant".to_vec())),
            vec![],
        )
        .unwrap();

        ctx.run_task(task).unwrap();

        let key = ctx.task_result(task).unwrap().unwrap();
        assert_eq!(ctx.get_object(&key).unwrap(), b"constant");
    }

    #[test]
    fn task_waits_for_its_dependencies() {
        let (_directory, mut ctx) = context();
        let dependency = Task::register(
            &mut ctx,
            Box::new(|_: &[Output]| Ok(b"dependency".to_vec())),
            vec![],
        )
        .unwrap();
        let task = Task::register(
            &mut ctx,
            Box::new(|dependencies: &[Output]| {
                let [dependency] = dependencies else {
                    unreachable!("Task checked its dependency count")
                };
                Ok([dependency.as_slice(), b" output"].concat())
            }),
            vec![dependency],
        )
        .unwrap();

        assert!(matches!(
            ctx.run_task(task),
            Err(Error::DependencyOutputUnsatisfied)
        ));

        ctx.run_task(dependency).unwrap();
        ctx.run_task(task).unwrap();

        let key = ctx.task_result(task).unwrap().unwrap();
        assert_eq!(ctx.get_object(&key).unwrap(), b"dependency output");
    }
}
