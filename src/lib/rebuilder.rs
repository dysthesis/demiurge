/// Whether or not the cached result is stale for a given task.
pub enum Colour {
    /// Stale, needs to be rebuilt.
    Red,
    /// Can used cached value.
    Green,
    /// Has not been checked.
    Unknown,
}
