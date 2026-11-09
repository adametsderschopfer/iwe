//! Format embedded translations without global mutable state or runtime file access.

use fluent_bundle::{FluentBundle, FluentResource};
use unic_langid::{LanguageIdentifier, langid};

pub(crate) use fluent_bundle::FluentArgs;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Language {
    English,
    Russian,
    Spanish,
}

impl Language {
    pub(crate) fn select(explicit: Option<&str>, system: Option<&str>) -> Self {
        let locale = explicit
            .filter(|value| !value.trim().is_empty())
            .or(system)
            .unwrap_or("en");
        let primary = locale.trim().split(['-', '_', '.', '@']).next();

        match primary.map(str::to_ascii_lowercase).as_deref() {
            Some("ru") => Self::Russian,
            Some("es") => Self::Spanish,
            _ => Self::English,
        }
    }

    fn identifier(self) -> LanguageIdentifier {
        match self {
            Self::English => langid!("en"),
            Self::Russian => langid!("ru"),
            Self::Spanish => langid!("es"),
        }
    }

    fn source(self) -> &'static str {
        match self {
            Self::English => include_str!("../../locales/en.ftl"),
            Self::Russian => include_str!("../../locales/ru.ftl"),
            Self::Spanish => include_str!("../../locales/es.ftl"),
        }
    }
}

macro_rules! message_keys {
    ($($variant:ident => $id:literal),+ $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub(crate) enum Key { $($variant),+ }

        impl Key {
            #[cfg(test)]
            const ALL: &'static [Self] = &[$(Self::$variant),+];

            fn id(self) -> &'static str {
                match self { $(Self::$variant => $id),+ }
            }
        }
    };
}

message_keys! {
    AppTitle => "app-title",
    Updates => "updates",
    UpdatesDescription => "updates-description",
    CheckUpdates => "check-updates",
    CheckingUpdates => "checking-updates",
    UpToDate => "up-to-date",
    UpdateAvailable => "update-available",
    DownloadUpdate => "download-update",
    DownloadingUpdate => "downloading-update",
    InstallUpdate => "install-update",
    InstallingUpdate => "installing-update",
    UpdateInvalid => "update-invalid",
    UpdateInstallFailed => "update-install-failed",
    UpdateUnsupported => "update-unsupported",
    UpdateCheckFailed => "update-check-failed",

    Spaces => "spaces",
    PinnedProjects => "pinned-projects",
    PinProject => "pin-project",
    UnpinProject => "unpin-project",
    GitFetchAll => "git-fetch-all",
    GitNewBranch => "git-new-branch",
    GitNewBranchFrom => "git-new-branch-from",
    GitCheckout => "git-checkout",
    GitUpdate => "git-update",
    GitUpdateSelected => "git-update-selected",
    GitPush => "git-push",
    GitRename => "git-rename",
    GitDelete => "git-delete",
    GitBranchName => "git-branch-name",
    GitRemote => "git-remote",
    GitSwitchAfterCreate => "git-switch-after-create",
    GitDeleteDescription => "git-delete-description",
    GitDeleteRemoteDescription => "git-delete-remote-description",
    GitWorking => "git-working",
    GitOperationFailed => "git-operation-failed",
    GitInvalidName => "git-invalid-name",
    GitMissingUpstream => "git-missing-upstream",
    GitDirty => "git-dirty",
    GitCurrentBranch => "git-current-branch",
    GitFetchDone => "git-fetch-done",
    GitCheckoutDone => "git-checkout-done",
    GitCreateDone => "git-create-done",
    GitUpdateDone => "git-update-done",
    GitPushDone => "git-push-done",
    GitRenameDone => "git-rename-done",
    GitDeleteDone => "git-delete-done",
    JobRunRequested => "job-run-requested",
    JobRetryRequested => "job-retry-requested",
    JobStopRequested => "job-stop-requested",
    PipelineStopRequested => "pipeline-stop-requested",
    SignedIn => "signed-in",
    SignedOut => "signed-out",
    Workspace => "workspace",
    ProjectFiles => "project-files",
    WorkingChanges => "working-changes",
    RefreshWorkingChanges => "refresh-working-changes",
    NoWorkingChanges => "no-working-changes",
    FileCount => "file-count",
    Projects => "projects",
    NewSpace => "new-space",
    SpaceName => "space-name",
    SpaceNamePlaceholder => "space-name-placeholder",
    SpaceDescription => "space-description",
    SpaceProjectsHint => "space-projects-hint",
    AddProject => "add-project",
    AddProjectFolder => "add-project-folder",
    RemoveProject => "remove-project",
    ReorderProjectsHint => "reorder-projects-hint",
    CreateSpace => "create-space",
    Cancel => "cancel",
    NoSpaces => "no-spaces",
    ChooseProject => "choose-project",
    ChooseProjectDescription => "choose-project-description",
    ChoosingDirectory => "choosing-directory",
    NoProjects => "no-projects",
    WelcomeDescription => "welcome-description",
    ChooseFile => "choose-file",
    ChooseFileDescription => "choose-file-description",
    Code => "code",
    CopyPath => "copy-path",
    CloseTab => "close-tab",
    CollapseUnchanged => "collapse-unchanged",
    ExpandUnchanged => "expand-unchanged",
    UnchangedLines => "unchanged-lines",
    DiffSideBySide => "diff-side-by-side",
    DiffUnified => "diff-unified",
    DiffBefore => "diff-before",
    DiffAfter => "diff-after",
    KeyboardShortcuts => "keyboard-shortcuts",
    CloseActiveTabDescription => "close-active-tab-description",
    ReopenClosedTab => "reopen-closed-tab",
    ReopenClosedTabDescription => "reopen-closed-tab-description",
    EditSpace => "edit-space",
    SaveChanges => "save-changes",
    FileUntracked => "file-untracked",
    FileAdded => "file-added",
    FileModified => "file-modified",
    FileDeleted => "file-deleted",
    FileRenamed => "file-renamed",
    FileConflict => "file-conflict",
    ComparisonFailed => "comparison-failed",
    LocalWorkspace => "local-workspace",
    Git => "git",
    Log => "log",
    Branches => "branches",
    LocalBranches => "local-branches",
    AllRepositories => "all-repositories",
    RemoteBranches => "remote-branches",
    Tags => "tags",
    SearchCommits => "search-commits",
    Commit => "commit",
    Author => "author",
    Date => "date",
    ChangedFiles => "changed-files",
    NoCommits => "no-commits",
    NoResults => "no-results",
    LoadingHistory => "loading-history",
    LoadOlderCommits => "load-older-commits",
    HistoryLoadFailed => "history-load-failed",
    DetachedHead => "detached-head",
    Pipelines => "pipelines",
    ChooseProvider => "choose-provider",
    NoPipelines => "no-pipelines",
    Jobs => "jobs",
    BackToPipelines => "back-to-pipelines",
    NoJobs => "no-jobs",
    PipelineStageTitle => "pipeline-stage-title",
    PipelineJobTitle => "pipeline-job-title",
    NoStageJobs => "no-stage-jobs",
    ViewPipelineJobs => "view-pipeline-jobs",
    RunJob => "run-job",
    RetryJob => "retry-job",
    RetryJobWithDependents => "retry-job-with-dependents",
    CancelJob => "cancel-job",
    CancelPipeline => "cancel-pipeline",
    JobActionPending => "job-action-pending",
    JobActionAccess => "job-action-access",
    TooManyJobs => "too-many-jobs",
    Canceling => "canceling",
    AccessRequired => "access-required",
    SignInDescription => "sign-in-description",
    SignIn => "sign-in",
    Disconnect => "disconnect",
    Connecting => "connecting",
    BrowserWaiting => "browser-waiting",
    DeviceCodeDescription => "device-code-description",
    CopyAuthCode => "copy-auth-code",
    OpenBrowser => "open-browser",
    TokenDescription => "token-description",
    GitHubTokenDescription => "github-token-description",
    TokenLabel => "token-label",
    TokenPlaceholder => "token-placeholder",
    CreateToken => "create-token",
    CancelSignIn => "cancel-sign-in",
    BrowserReturn => "browser-return",
    CredentialsUnavailable => "credentials-unavailable",
    AuthNotConfigured => "auth-not-configured",
    AuthExpired => "auth-expired",
    AuthDenied => "auth-denied",
    AuthPortBusy => "auth-port-busy",
    InvalidToken => "invalid-token",
    PermissionDenied => "permission-denied",
    RateLimited => "rate-limited",
    RetryDescription => "retry-description",
    Passed => "passed",
    Failed => "failed",
    Running => "running",
    Pending => "pending",
    Cancelled => "cancelled",
    Skipped => "skipped",
    Manual => "manual",
    UnknownStatus => "unknown-status",
    Settings => "settings",
    SettingsDescription => "settings-description",
    General => "general",
    Language => "language",
    LanguageDescription => "language-description",
    SystemLanguage => "system-language",
    Appearance => "appearance",
    AppearanceDescription => "appearance-description",
    Dark => "dark",
    Light => "light",
    FontSize => "font-size",
    FontSizeDescription => "font-size-description",
    LineNumbers => "line-numbers",
    LineNumbersDescription => "line-numbers-description",
    Panels => "panels",
    GitDescription => "git-description",
    PipelinesDescription => "pipelines-description",
    About => "about",
    AboutDescription => "about-description",
    SettingsSaved => "settings-saved",
    BackToWorkspace => "back-to-workspace",
    Refresh => "refresh",
    HidePanel => "hide-panel",
    Dismiss => "dismiss",
    Loading => "loading",
    Retry => "retry",
    GitMissing => "git-missing",
    NotRepository => "not-repository",
    ReadFailed => "read-failed",
    TooLarge => "too-large",
    Binary => "binary",
    Unavailable => "unavailable",
    StorageLoadFailed => "storage-load-failed",
    StorageSaveFailed => "storage-save-failed",
    OpenFailed => "open-failed",
    Greeting => "greeting",
    MessageUnavailable => "message-unavailable",
    GraphicsUnavailable => "graphics-unavailable",
    StartFailed => "start-failed",
    GraphicsTrying => "graphics-trying",
    UnsupportedOs => "unsupported-os",
    GraphicsInitializationFailed => "graphics-initialization-failed",
    ApplicationFailed => "application-failed",
}

pub(crate) struct I18n {
    selected: Option<FluentBundle<FluentResource>>,
    english: Option<FluentBundle<FluentResource>>,
}

impl I18n {
    pub(crate) fn new(language: Language) -> Self {
        Self {
            selected: bundle(language, language.source()),
            english: bundle(Language::English, Language::English.source()),
        }
    }

    pub(crate) fn text(&self, key: Key) -> String {
        self.format(key, &FluentArgs::new())
    }

    pub(crate) fn format(&self, key: Key, args: &FluentArgs<'_>) -> String {
        self.resolve(key, args)
            .or_else(|| self.resolve(Key::MessageUnavailable, &FluentArgs::new()))
            .unwrap_or_default()
    }

    fn resolve(&self, key: Key, args: &FluentArgs<'_>) -> Option<String> {
        self.selected
            .as_ref()
            .and_then(|bundle| format(bundle, key, args))
            .or_else(|| {
                self.english
                    .as_ref()
                    .and_then(|bundle| format(bundle, key, args))
            })
    }
}

fn bundle(language: Language, source: &str) -> Option<FluentBundle<FluentResource>> {
    let resource = FluentResource::try_new(source.to_owned()).ok()?;
    let mut bundle = FluentBundle::new(vec![language.identifier()]);
    bundle.add_resource(resource).ok()?;
    Some(bundle)
}

fn format(
    bundle: &FluentBundle<FluentResource>,
    key: Key,
    args: &FluentArgs<'_>,
) -> Option<String> {
    let pattern = bundle.get_message(key.id())?.value()?;
    let mut errors = Vec::new();
    let text = bundle.format_pattern(pattern, Some(args), &mut errors);
    (errors.is_empty() && !text.trim().is_empty()).then(|| text.into_owned())
}

#[cfg(test)]
mod tests;
