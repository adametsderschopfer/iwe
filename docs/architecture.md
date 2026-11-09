# Architecture

The application provides named spaces, projects, source previews, Git history, pipelines, and
settings.

| Module                                                | Responsibility                                                                           |
| ----------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| `main.rs`                                             | Select the language and start the graphics launcher                                      |
| `app/`                                                | Configure the window, own interface state, and process native material results           |
| `app/notifications.rs`                                | Bound operation notifications, coalesce duplicates, and manage dismissal timers          |
| `ui/notifications.rs`                                 | Overlay localized result cards without resizing the workspace                            |
| `domain/`                                             | Model spaces, projects, preferences, accounts, remotes, commits, and graph relationships |
| `ui/screens/`                                         | Compose workspace, Git, pipeline, and settings screens                                   |
| `ui/code.rs`, `ui/code/`                              | Prepare highlighted source lines and preserve their text layout                          |
| `ui/icons.rs`, `ui/file_icons.rs`                     | Draw interface icons and compact file-type symbols                                       |
| `ui/graph.rs`                                         | Draw commit ancestry                                                                     |
| `ui/theme/`                                           | Define interface colors, syntax colors, and the native material fallback                 |
| `ui/tooltip.rs`                                       | Apply shared spacing and styling to hover hints                                          |
| `i18n/`                                               | Format embedded Fluent translations and recover from missing messages                    |
| `infrastructure/locale.rs`                            | Read `IWE_LANG` and the system locale                                                    |
| `infrastructure/workspace.rs`                         | Save workspace metadata and read bounded local previews                                  |
| `infrastructure/project_index.rs`                     | Cache directory metadata and prioritize background folder reads                          |
| `infrastructure/repository.rs`                        | Read Git history, branches, changed paths, and historical blobs                          |
| `infrastructure/repository/discovery.rs`              | Discover containing and direct child Git repositories                                    |
| `infrastructure/repository/project.rs`                | Merge bounded history streams from discovered repositories                               |
| `domain/repository/project.rs`, `app/repositories.rs` | Preserve repository identities across history and source navigation                      |
| `domain/git.rs`, `app/git_actions/`                   | Define Git operations and coordinate dialogs, pending actions, and results               |
| `infrastructure/git/`                                 | Validate references and execute explicit Git operations                                  |
| `ui/context_menu/`                                    | Detect right-clicks and anchor menus inside the window                                   |
| `ui/resize/`                                          | Resize the file panel while preserving child widget state                                |
| `ui/changed_tree/`                                    | Display compact changed-file trees with counts and status colors                         |
| `infrastructure/pipelines.rs`                         | Read GitHub Actions and GitLab pipeline APIs                                             |
| `domain/jobs.rs`                                      | Model stages, jobs, and provider-supported actions                                       |
| `app/jobs.rs`                                         | Coordinate job selection, cached details, refresh, and pending actions                   |
| `app/jobs/overview.rs`                                | Cache visible pipeline summaries and coordinate stage or job popups                      |
| `infrastructure/pipelines/jobs.rs`                    | Load paginated jobs and submit explicit provider actions                                 |
| `infrastructure/auth/`                                | Implement GitHub device authorization and GitLab PKCE callbacks                          |
| `infrastructure/auth.rs`                              | Validate accounts, refresh tokens, and serialize credential operations                   |
| `infrastructure/credentials.rs`                       | Store provider credentials in the operating system vault and share session reads         |
| `infrastructure/http.rs`                              | Enforce HTTPS, response bounds, request deadlines, and redirect restrictions             |
| `infrastructure/process.rs`                           | Bound asynchronous subprocess time and output                                            |
| `infrastructure/graphics/`                            | Start graphics backends in platform priority order                                       |
| `infrastructure/window_effect/`                       | Place native Liquid Glass below the renderer on supported macOS versions                 |

Composition connects the interface, localization, and infrastructure. Interface modules never import
infrastructure. Translation logic performs no operating-system access. Add application and domain
layers when concrete features require them. Current application orchestration remains in `app/`.

Filesystem work and highlighting run on blocking workers. Subprocesses use Tokio tasks with bounded
output and timeouts. Generation identifiers prevent stale results from replacing a newer selection.
Views do not access the operating system. Source contents are never indexed or persisted. Saved
metadata uses Serde and atomic temporary-file replacement. Native directory selection uses `rfd`.
Syntect highlights source with bundled `two-face` grammars and the Gerry Darcula palette. `open`
delegates approved links to the system browser.

The shared process launcher resolves Git for repository reads and explicit Git operations. On
Windows, it prefers `git.exe` from `PATH`, then checks standard Git for Windows installation
directories. Custom installations still require `PATH` configuration. The launcher does not install
Git or change the environment.

Notifications keep typed message keys and optional user context in application memory. They never
persist with workspace settings. Successful action responses enqueue notifications after request and
project checks. Error messages use existing localized mappings. Timers run only while unpaused
successful notifications remain visible. Hovering pauses all notification timers, including new
arrivals. The notification stack captures clicks inside its cards and preserves workspace clicks
outside them. Modal dialogs keep their existing error messages.

Git status uses NUL-delimited records and preserves renamed paths. Application state aggregates file
status into folder decorations. Background polling refreshes status, affected directories, and the
open working file every three seconds. Separate request identifiers reject stale project and preview
results. Matching content fingerprints avoid repeated highlighting and preserve scroll position. Git
commands disable optional index writes. Source comparison reads bounded blobs and keeps all source
contents in memory. The `similar` crate compares source lines with a 300-millisecond algorithm
budget on blocking workers. The viewer retains unchanged lines and marks inserted, replaced, and
removed lines without losing syntax colors.

Comparisons retain aligned pairs that reference the unified document's highlighted lines. Each pair
keeps both original line numbers. Change blocks provide connector geometry without copying source
text. Word comparison uses a shared 150-millisecond deadline and skips lines over 8 KiB. Blocking
workers prepare these structures. The original single-column view remains available through a saved
preference. The default view places the previous and current versions in separate columns.

A direct index maps unified rows to their comparison pairs. Unified rendering reads both original
line numbers and word highlights through this index. Both comparison views share the highlighted
span builder. Unified rows distinguish previous replacement blocks from their new lines. Compact
wave separators use the existing folding ranges, expansion messages, and fixed row heights.

The comparison view virtualizes both columns with the existing source-row cache. Scroll messages
synchronize their vertical offsets while retaining independent horizontal offsets. Each tab keeps
separate positions for the two comparison views. Shared folding maps unchanged ranges into both
projections. Expanding either column expands the same source range in both columns and the unified
view. Connector drawing visits only blocks intersecting the viewport.

Document preparation merges adjacent changes into status ranges for the scrollbar overview. Folding
projects these ranges into visible rows. Two-column overviews reuse the projected change blocks.
Cached canvas geometry keeps marks independent of scrolling and refreshes after content, folding,
font size, or viewport changes. A passive layer draws marks beside the scrollbar handle and forwards
pointer input to the existing scrollable widget.

Each selected project uses one background index worker. Opened folders take priority over bounded
breadth-first traversal. Cached directory metadata appears before refresh results. Atomic disk
snapshots survive restarts, while a small memory cache preserves recent projects and expanded paths.
Worker cancellation and separate index generations isolate project switches. Failed reads preserve
cached listings. Delayed loading messages apply only to uncached requests. Cache writes never change
project files or store their contents.

Source previews build rows for the current viewport and a surrounding buffer of stable blocks. Keyed
rows preserve shaped text when the visible range moves. Unchanged blocks reuse their layout.
Content, folding, language, font size, and available bounds invalidate that layout. Fixed line
heights keep the scroll extent stable. The source-row widget uses Iced's advanced API to retain
overlapping rows during scrolling and resizing. It also forwards pointer events to collapsed-range
buttons.

Opening a file first publishes plain text, then syntax colors, then Git comparison lines. Blocking
workers prepare each document. Comparison reuses the current file's highlighted lines. An overlaid
progress bar leaves the code viewport size unchanged. Shared immutable documents keep background
preparation separate from visible content. Stage and generation guards reject results from closed
tabs or replaced projects.

File tabs own separate source documents, loading states, and scroll offsets. Selecting tabs reuses
documents without copying their contents. Stable request identifiers route background results to
their original tabs. Closed tabs reject pending results. Project generations isolate refreshed and
replaced projects. Refresh preserves current tabs and restarts their pending reads. Inactive working
files refresh when selected. Commit tabs retain their original revision and comparison source.

Each tab caches its collapsed-line projection. Only projection changes scan the document; scrolling
builds rows from the cached projection. Expanding a range preserves all original source lines.
Restoring the complete file restores its previous scroll position. Changed document fingerprints
rebuild the projection and reset its expanded ranges. One saved preference controls unchanged-code
folding across all tabs. Loaded, restored, and newly prepared documents receive that preference.
Expanded ranges and scroll positions remain specific to each tab.

The local `iced_widget` patch keeps scrollbar handles usable with very large content. Handles have a
28-pixel minimum, limited to half the available rail in very small panels. Offset calculations use
the remaining handle travel, preserving accurate dragging at both ends. The patch applies to all
scrollable panels and preserves native wheel and touch behavior. See
[the patch notes](../vendor/iced_widget/README.md) for its source version and update policy.

Project trees use 24-pixel rows, 14-pixel text, 16-pixel icons, and 20-pixel nesting increments.
Full filenames remain available through horizontal scrolling. Folder selection remains independent
from the opened preview. Recent project caches preserve the selected entry.

Spaces contain named projects and have no directory of their own. Each project owns one directory
path. The application keeps space drafts separate from saved state until confirmation. Folder
pickers return multiple directories. Request identifiers prevent canceled drafts from receiving
stale selections. Shared modal composition blocks interaction with the underlying view. Legacy
directory groups migrate to spaces without losing folders or preferences. Storage preserves the
original configuration before migration.

The shared `ui/reorder` widget handles project dragging, drop indicators, and edge scrolling. It
defers row clicks until release and preserves trailing action buttons. Space forms reorder their
drafts. Sidebar drops update domain order and save immediately. Both paths preserve the selected
directory and existing project metadata. Project metadata includes a persistent pinned flag. Sidebar
rendering moves pinned rows above the spaces without changing membership or selection. The shared
reorder widget also handles file-panel icons. Their saved order determines the panel selected at
startup and after project changes.

Git history uses bounded pages with frozen revision hashes and continuing graph lanes. Scrolling
loads older pages and preserves the current position. Search continues loading until history ends or
the query is cleared. Request identifiers reject pages from previous directories or branch
selections. The interface builds only visible commit rows and a small surrounding buffer. Local
branches, remote branches, and tags share validated reference reads. Nested pane grids resize the
Git columns. The changed-files column exists only while a commit is selected. Closing the column
invalidates pending file-list requests. Commit titles use measured single-line truncation with
cached text layouts.

Repository discovery inspects the opened directory and its immediate child directories. Child
directory links are excluded. Worktree metadata files remain supported. Each working change belongs
to the deepest discovered root. Parent repository status cannot duplicate a child's changes.

Project history merges repository streams in bounded pages. The merge compares commit timestamps
while preserving each repository's topology. It refills an exhausted stream before selecting the
next row. Every cursor freezes revision hashes and retains unread rows. Branch keys, commit keys,
and graph lanes include repository paths. Clones with identical hashes retain separate identities.
Partial repository failures remain visible without replacing successful repository data.

Multiple repositories initially show all references in one history. Selecting a branch filters
history to its repository. The all-repositories control restores the merged history. Pipeline
accounts follow the selected repository; the first discovered repository is the default.

Git actions use typed operations and validated full references. Dialog drafts remain available after
failures. One pending operation blocks duplicate submissions. Request identifiers and repository
roots reject stale results. Successful actions refresh repository data, source tabs, and directory
listings. Git subprocesses have a 180-second deadline and bounded output. Selected-branch updates
permit only fast-forward changes. Checkout requires clean files. Deletion requires confirmation and
does not force local branch removal. Git actions use Git's configured authentication separately from
pipeline API accounts. See [Git actions](git-actions.md).

Pipeline responses include full commit identifiers. History matches active runs against those
identifiers. Visible Git or pipeline panels refresh loaded data every 30 seconds. Request guards
prevent overlapping loads, and generation identifiers reject stale responses. Failed loads stop
automatic polling.

`app/pipeline_cache.rs` retains successful pipeline lists for eight recently used repositories in
memory. Keys combine the local repository root with the complete remote identity. Restored lists
remain visible during background refresh. Temporary read failures retain these lists and stop
polling. Access failures and account changes clear the session cache. The cache excludes credentials
and does not persist to disk. Generation and request guards also apply to cache writes.

Pipeline rows project GitLab stages or individual GitHub jobs into status icons. GitLab supplies
stage statuses. Domain aggregation covers missing statuses and keeps unsuccessful or unfinished jobs
visible. Connectors indicate display order, not inferred dependencies. Popup targets use stage names
or job identifiers and reuse existing capability checks and actions.

The list requests summaries for its visible rows and one following row, with two concurrent reads.
Request and project identifiers reject obsolete responses. Completed summaries reuse their cache
until their run status changes or an explicit refresh occurs. Active visible summaries refresh every
ten seconds. Failed reads retain cached content and stop automatic retries for that run. Opening
complete details reuses cached jobs and starts a fresh read. Project changes, remote changes, and
sign-out clear both caches. Missing popup targets close after refresh.

GitLab job reads use GraphQL to preserve stage order and obtain current action capabilities. GitHub
job reads use REST pagination for the latest attempt. GitHub has no stage entity. Job requests stop
at 1,000 jobs, 100 stages, or 45 seconds. Incomplete responses never become successful partial
lists. Visible job details refresh every ten seconds without clearing cached rows. Errors stop
automatic job polling. Selection and project identifiers reject stale reads and action responses.
Pending actions block duplicate submissions. Provider writes use authenticated POST requests to
endpoints derived from the repository remote. Returned browser URLs never determine action
destinations. Actions share credential serialization and are never retried automatically after a
response failure.

Authentication uses direct HTTPS requests through Reqwest and Rustls. The keyring crate selects
Keychain, Windows Credential Manager, or Linux Secret Service. Linux builds vendor the D-Bus
library, but still require a running Secret Service. Credential operations run on blocking workers.
The session store shares successful reads, missing accounts, and access failures across service
clones. Explicit retries clear only cached failures for the selected account. Successful writes and
deletions update the session cache. Failed mutations block cached credential use until an explicit
retry. Session credentials use the existing zeroizing secret type and never enter workspace files.
Token refresh, storage, and sign-out share a mutex. Cancellation waits for active vault writes
before releasing that mutex. Authorization results carry request identifiers and cannot replace
newer dialog state. Secrets have redacted debug output and zeroize their owned buffers when dropped.
Dependencies may retain internal copies during requests or rendering. Workspace metadata never
contains credentials. See [authentication setup](authentication.md) for provider registration and
protocol details.

## Window and graphics

The display name is IWE. Embedded PNG pixels supply Windows and X11 window icons. Windows builds
embed an ICO resource through `winresource`. macOS bundles include the ICNS icon. Linux packages
include the PNG icon and desktop entry. The repository keeps the generated source artwork in
`assets/`. The `png` crate decodes the embedded window icon without filesystem access.

The window opens at 1440 × 920 pixels. Its minimum size is 1080 × 720 pixels. It preserves native
window controls and closes through the standard close button.

On macOS, content extends into the native title bar. The sidebar reserves 32 pixels above its
heading for native window controls. The main header remains 56 pixels high, including in settings.
Drag either header's empty area to move the window. Double-click its empty area to maximize or
restore the window. Windows and Linux keep their native title bars.

On macOS, renderer layers anchor their contents at the top left instead of stretching the previous
frame during resizing. AppKit requests redraws during live resizing. This policy preserves display
scaling and applies without Liquid Glass. The native glass and window controls remain separate.
Foundation and QuartzCore bindings provide safe access to these native layer settings.

The default dark theme uses an opaque charcoal background (`#212121`) and navigation surfaces
(`#181818`). The optional light theme uses `#fafafa` and `#f0f0f0`. Headers, settings, and panel
gaps share the workspace background. Desktop colors never show through these surfaces.

Settings persist the selected appearance. Semantic colors in `ui/theme` resolve against Iced's
current theme. Syntax preparation stores paired colors. Switching themes preserves documents and
does not repeat highlighting. Source row and overview caches invalidate their colors when appearance
changes.

The left sidebar contains navigation and projects. A separate file panel sits beside the source
preview, with a vertical toolbar on its left. Flat content panes use subtle separators instead of
nested frames. Buttons, inputs, scrollbars, tooltips, and authentication dialogs share neutral
styles. macOS uses Helvetica Neue. Windows uses Segoe UI. Linux retains the bundled sans-serif
fallback. Source previews keep their monospace font. Empty states use concise messages in all three
catalogs.

Selecting a project immediately opens its file panel. The file toolbar selects the project-files
mode directly. Project rows have no separate file button. Project selection preserves the sidebar.
Clicking an active file toolbar icon hides its panel. The resize widget keeps preview widgets in
stable child positions while panel visibility changes. Dragging updates the width in memory and
saves it on release. The view limits panel width to preserve source space without changing the saved
preference. File panel headings and file tabs share a common bottom edge.

Git and pipelines share one optional panel position. The workspace guide lists platform-specific
keyboard shortcuts. Middle-click closes the targeted tab.

Native Liquid Glass remains beneath the renderer on supported macOS versions. The renderer keeps an
opaque neutral fallback until native initialization succeeds. The fallback follows the selected
theme. Other platforms retain the same opaque workspace palette.

The launcher tries these backends in order:

| Platform | Backend priority           |
| -------- | -------------------------- |
| macOS    | Metal, Vulkan, OpenGL      |
| Windows  | Vulkan, DirectX 12, OpenGL |
| Linux    | Vulkan, OpenGL             |

Each attempt uses a separate process. Only graphics initialization failures trigger another attempt.
The macOS fallback paths require external Vulkan/MoltenVK or ANGLE runtimes. The application does
not bundle those runtimes. Normal macOS startup uses Metal.

## Application updates

`app/updates` coordinates release checks, downloads, workspace persistence, and restart requests.
`domain/update` carries verified release metadata without operating-system access.
`infrastructure/updates` verifies signed manifests, stages packages, and installs updates after
shutdown. `ui/updates` presents localized actions in the sidebar and About settings. The startup
entry point handles installer and cleanup processes before starting the graphics launcher. See
[application updates](updates.md) for trust boundaries, package limits, and recovery behavior.

## Localization

The executable embeds English, Russian, and Spanish catalogs from `locales/`. `IWE_LANG` overrides
the system locale. Unsupported languages select English. Missing selected messages fall back to
English, then to localized fallback text. The interface never displays translation keys or technical
diagnostics.

Unit tests verify catalog completeness, translated layouts, persistence, repository reads, previews,
and asynchronous selection. They also verify native material state and graphics fallback behavior.
Headless tests cannot verify native material appearance. Check appearance with a local application
run. See [workspace behavior](workspace.md) for feature limits and provider authentication.
