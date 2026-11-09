//! Describe explicit repository operations independently from their execution.

#[derive(Debug, Clone)]
pub(crate) enum Operation {
    Fetch,
    Checkout(String),
    Create {
        source: String,
        name: String,
        checkout: bool,
    },
    Update(String),
    Push {
        source: String,
        remote: String,
        name: String,
    },
    Rename {
        source: String,
        name: String,
    },
    Delete(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Error {
    Failed,
    InvalidName,
    MissingUpstream,
    Dirty,
    CurrentBranch,
}
