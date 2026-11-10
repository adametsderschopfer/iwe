//! Model local workspaces and repository data without operating-system access.
pub(crate) mod editor;

pub(crate) mod auth;
pub(crate) mod changes;
pub(crate) mod deployment;
pub(crate) mod git;
pub(crate) mod jobs;
pub(crate) mod repository;
pub(crate) mod shortcuts;
pub(crate) mod update;
pub(crate) mod workspace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Failure {
    MissingTool,
    NotRepository,
    Read,
    TooLarge,
    Binary,
    Access,
    Unavailable,
    Storage,
    Credentials,
    AuthNotConfigured,
    AuthCancelled,
    AuthExpired,
    AuthDenied,
    AuthPortBusy,
    InvalidToken,
    PermissionDenied,
    RateLimited,
}
