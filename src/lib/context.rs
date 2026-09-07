use std::sync::{Mutex, MutexGuard};

use crate::{
    store::{self, Key, Store},
    task::{self, Task},
};

/// The whole build system.
pub struct Ctx {
    /// An append-only list of tasks relevant to the current build.
    tasks: Vec<Mutex<Task>>,
    /// Logical representation for the object storage.
    store: Store<Key>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to acquire task lock: mutex is poisoned")]
    TasksPoisoned,

    #[error("failed to add task because the task table is full")]
    TasksFull,

    #[error("no task with ID {id} found")]
    TaskNotFound { id: TaskId },

    #[error("failed to put object into the store")]
    StorePutFailed {
        #[source]
        error: store::Error,
    },

    #[error("failed to get object from the store")]
    StoreGetFailed {
        #[source]
        error: store::Error,
    },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TaskId(usize);
impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Ctx {
    pub fn new(store: Store<Key>) -> Self {
        Self {
            tasks: Vec::new(),
            store,
        }
    }

    /// Add one task into the map
    pub fn add_task(&mut self, task: Task) -> Result<TaskId> {
        self.tasks.try_reserve(1).map_err(|_| Error::TasksFull)?;

        let id = self.tasks.len();
        self.tasks.push(Mutex::new(task));
        Ok(TaskId(id))
    }

    /// Add multiple tasks into the map
    pub fn add_tasks<I: IntoIterator<Item = Task>>(
        &mut self,
        tasks: I,
    ) -> Result<Vec<TaskId>> {
        let mut ids = Vec::new();
        for task in tasks {
            ids.push(self.add_task(task)?);
        }

        Ok(ids)
    }

    pub fn get_task(&self, id: TaskId) -> Result<MutexGuard<'_, Task>> {
        self.tasks
            .get(id.0)
            .ok_or(Error::TaskNotFound { id })?
            .lock()
            .map_err(|_| Error::TasksPoisoned)
    }

    pub fn task_result(&self, id: TaskId) -> Result<Option<Key>> {
        Ok(self.get_task(id)?.result())
    }

    pub fn run_task(&self, id: TaskId) -> task::Result<()> {
        self.get_task(id)?.run(self)
    }

    #[inline]
    pub fn publish_object(&self, object: &[u8]) -> Result<Key> {
        self.store
            .put(object)
            .map_err(|error| Error::StorePutFailed { error })
    }

    #[inline]
    pub fn get_object(&self, key: &Key) -> Result<Vec<u8>> {
        self.store
            .get(key)
            .map_err(|error| Error::StoreGetFailed { error })
    }
}
