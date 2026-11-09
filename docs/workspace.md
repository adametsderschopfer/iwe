# Workspace

Iwe provides a native workspace for local directories, source previews, Git history, and CI runs.

## Spaces, projects, and files

Use the plus beside **Spaces** and enter its name. Add existing project folders in the form, then
choose **Create space**. Select several folders in one directory dialog. Iwe keeps each folder once
within the space. You can also create an empty space and add projects later. Canceling the form
discards its name, folder choices, and project order.

Use the settings button beside a space name to edit that space in the same form. Change its name,
add project folders, or remove projects, then choose **Save changes**. Cancel keeps the saved space
unchanged. Removing a project only detaches its folder. The application never deletes the folder.
The header shows the selected space. The files view shows **Space • Project**.

Drag project rows to change their order. The form shows five complete rows and half the sixth row
when more projects exist. Scroll to reach the remaining projects. A line marks the drop position.
Holding a dragged row near the list edge scrolls the list. Dropping outside the list cancels the
move. **Save changes** applies the form's project order.

Drag project rows within their space in the sidebar to change the same order. Sidebar changes save
immediately and survive restarts. Reordering preserves the open project, file tabs, and loaded
history. Sidebar rows retain their existing icons. Dragging does not move projects between spaces.

Right-click a project and choose **Pin project**. Pinned projects appear above **Spaces** in a
separate **Pinned** group. Each row identifies its space. The project retains its space membership
and disappears from that space's sidebar list. **Unpin project** returns it to the list. Pinning
survives restarts and does not reload the open project.

Click a space heading or its arrow to collapse or expand its projects. Spaces collapse
independently. Collapsing a space keeps its selected project open. The edit button remains
available.

Space settings remain at the sidebar's right edge. Project rows contain no separate file button.

Spaces group projects and have no filesystem path. Their sidebar headings do not open a project.
Each project represents one directory. Its folder name becomes the project name. Use **Add project**
beneath a space to attach another folder. Selecting a project opens its files and Git history.
Selecting the current project again preserves its files, history filter, and scroll positions.
Adding a folder again selects its existing entry within that space.

One click selects a project and immediately displays its files beside the projects sidebar. The file
panel sits between its vertical toolbar and the source preview. The files view lists project
contents directly, without a collapsible project row. Nested folders keep their expansion arrows.
The sidebar always retains its projects. Use the file toolbar to select **Project files** directly.
The panel heading aligns with the first toolbar button. Panel content and status messages share the
heading's left inset. The scroll area fills the file panel. Scrollbars stay at its edges, regardless
of filename length.

A thin divider separates the workspace heading from its content. Another divider separates the file
panel from the source preview. A full-height divider separates the spaces sidebar from the
workspace. All dividers follow the current appearance.

The vertical toolbar switches between **Project files** and **Uncommitted files**. The latter
includes staged, unstaged, and untracked files. Drag these icons to change their order. Iwe saves
the order and opens the first panel after reordering, restarting, or selecting another project.
**Project files** comes first by default. Missing deleted files open their previous committed
contents.

Selecting a project starts a background directory index. Cached folders appear immediately while
their listings refresh. Recently visited projects also retain their expanded folders in memory.
Opening a folder prioritizes its refresh before speculative reads. Refresh keeps existing listings
visible. A loading label appears only when an uncached request exceeds 350 milliseconds.

The index stores names, paths, and entry types in the system cache directory. It survives
application restarts. `IWE_DATA_DIR` places this cache under `cache/trees` inside the selected data
directory. Unreadable or damaged cache files are ignored. Failed folder reads keep cached entries
and offer a retry. Refresh or reopen a folder to pick up filesystem changes.

Background traversal excludes `.git` metadata, symbolic links, and common generated directories.
Examples include `node_modules`, `target`, `vendor`, `build`, `dist`, and virtual environments.
Generated directories remain available when opened explicitly. Background indexing stops at 4,096
directories, 100,000 entries, or 24 directory levels. Individual listings retain the 5,000-entry
limit. Cache files are bounded to 32 MiB. Four previous project trees remain available in memory.

Old project groups become named spaces. Their directories become individual projects. Names,
selected folders, preferences, and provider choices are preserved. Before migration, Iwe saves the
original configuration as `workspace.v1.json` beside `workspace.json`. Existing backups remain
unchanged. Opening an unassigned folder from the command line prefills the new space form.

The directory index contains no source contents. Iwe reads file contents only when a file is opened.
Source contents remain in memory. Settings storage contains project metadata and preferences. No
source code is uploaded.

## Source preview

The viewer displays UTF-8 text up to 512 KiB. Binary and larger files display an explanation. The
preview cannot edit files. Only visible lines and nearby lines become widgets. The visible range
follows the panel height. Stable line identities preserve text layout while scrolling. Long lines
stay unwrapped and use horizontal scrolling.

Files first display plain text. Syntax colors and Git changes follow on background workers. A thin
progress bar remains above the code until preparation finishes. Scrolling remains available during
preparation. Unchanged row blocks reuse their layout during scrolling. Closing a loading tab rejects
its pending results. Restoring that tab resumes preparation.

Scrollbar handles stay at least 28 pixels long in normal panels, even with very large content. Very
small panels use a shorter handle that fits their rail. Dragging reaches both ends without changing
the scroll range.

Opened files appear in a compact tab strip with file icons and close buttons. Selecting an existing
file reuses its tab and preserves horizontal and vertical scroll positions. Commit versions use
separate tabs with revision identifiers. Hovering a tab shows its complete path and revision.

The tab strip scrolls horizontally when needed and reveals the selected tab. Closing the active tab
selects its neighbor. Command+W closes the application on macOS, including from settings and
dialogs. Ctrl+W closes the active tab on Windows and Linux. Middle-clicking a tab closes that tab.
Command+Shift+T restores the last closed tab on macOS; Ctrl+Shift+T restores it on Windows and
Linux. Repeated presses restore earlier closures, including commit versions, without duplicating
open tabs. The latest 20 closed tabs retain their documents, scroll positions, and folded blocks in
memory. Working files refresh after restoration. Settings lists both shortcuts. Refresh keeps tabs
and closure history for the current project. Changing projects clears both. Closing the application
clears this memory.

The bottom bar shows the active file's full path. Clicking the path copies it to the clipboard. When
no file is open, the bar shows the project path. Revision identifiers remain in commit tabs.

Syntax highlighting uses Syntect, bundled `two-face` grammars, and the Gerry Darcula colors from the
reference editor. Keywords appear purple, functions gold, strings green, and comments gray. Syntax
grammars determine colors without language-server analysis. The extension or filename selects the
grammar. Unknown formats remain readable as plain text. Supported grammars include Rust, Go,
JavaScript, TypeScript, TSX, Vue, Svelte, Python, Java, Kotlin, Swift, and C-family languages. They
also include SQL, JSON, YAML, TOML, Dockerfile, Makefile, Terraform, Bash, and Markdown.

The project tree uses 24-pixel rows, 14-pixel text, 16-pixel icons, and 20-pixel nesting steps,
following the supplied JetBrains reference. Each row centers its name and icon. Folders show
separate expansion arrows and folder icons. Recognized Python packages show a package marker.
Selecting a folder highlights its row without closing the source preview. Long filenames remain
complete and scroll horizontally. File-type icons also appear beside changed files in Git.

Git decorations color modified and renamed filenames cyan. Added and untracked filenames appear
green. Deleted files and conflicts appear red. Status letters and folder dots are absent. Commit
files and uncommitted files share the same tree style. Their folders compact consecutive single
directories and show file counts. File tooltips describe each status. Ignored files keep their
normal appearance. Status and the open working file refresh in the background every three seconds.

Working-file comparisons use the current branch's `HEAD`, including both staged and unstaged
changes. Two-column comparisons use green for additions, blue for replacements, and red for
deletions. Removed lines retain their original line numbers. Opening a file outside Git shows a
normal source preview. Failed comparisons keep readable source and show a retry explanation.

Comparisons open in two columns by default. The previous version appears on the left; the current
version appears on the right. Each column shows its own line numbers and syntax colors. Colored
connectors join changed blocks. Changed words receive stronger highlights. Empty rows align
insertions and deletions with the other version. Vertical scrolling moves both columns together.
Each column scrolls horizontally on its own.

Vertical scrollbars show changes throughout the file, including changes outside the visible area.
Green marks identify additions, blue marks identify replacements, and red marks identify deletions.
Both comparison views include these marks. Folding or expanding unchanged ranges updates their
positions. The marks leave the scrollbar handle visible and preserve wheel scrolling and dragging.

Use the view button beside the file tabs to switch between two columns and the unified single
column. Iwe saves this choice for all files and across restarts. Both views retain their scroll
positions. Switching views keeps the document and expanded unchanged ranges. Commit comparisons use
the same views. Files without a comparison keep their normal preview and hide the view button.

The unified view shows previous and current line numbers in separate gutter columns. Missing numbers
identify lines present in only one version. Replaced previous lines use gray-blue backgrounds; their
replacements use green backgrounds. Deletions without replacements remain red. Word highlights
retain syntax colors. The line-number setting controls both gutter columns. Unchanged ranges appear
as clickable wave separators. Hover over a separator to see its line range.

Source rows meet the tab strip and left edge without an outer inset. Line numbers and change markers
use their own gutter. Space below the final line keeps the horizontal scrollbar clear.

Use the button beside the file tabs to collapse unchanged lines. Changed lines keep their original
line numbers and colors. Click a collapsed range to expand that range. The button restores all lines
and the previous scroll position. This mode applies to all files, including commit versions. Iwe
saves the mode across restarts and applies it to new and restored tabs. Each tab retains its
individually expanded ranges while its contents remain unchanged. Files without unchanged ranges or
valid comparisons do not show this button.

## Git

Drag either vertical separator to resize the branches, history, and changed-files columns. Column
widths remain available during the current application session. Commit titles and branch names keep
their truncation marker on one line. Branch names fit the available width, including folder names.
Hover over a branch to see its full name.

History opens without a selected commit. Selecting a commit opens its changed-files column. Close
that column with its cross to clear the selection. Open file tabs remain available. Selecting
another branch also closes the changed-files column.

Install Git and make it available on `PATH`. The Git panel displays local branches, remote branches,
commit ancestry, and files changed by a selected commit. Selecting a branch filters history without
checking it out. Search matches commit messages, authors, hashes, references, and dates. The search
field sits above the commits column. Its position and width follow the column when separators move.

The branch toolbar offers **Fetch all remotes**, **New branch**, **Update selected**, and **Push**.
Right-click a branch for **Checkout**, **New branch from…**, **Update**, **Push**, **Rename**, and
**Delete**. Each operation targets that branch. Unavailable actions remain disabled. Creation,
renaming, pushing, and deletion use dialogs. See [Git actions](git-actions.md) for operation
behavior.

Operation results appear in the lower-right corner without resizing workspace panels. Git
operations, pipeline actions, sign-in, and sign-out report their results. Background refreshes do
not announce successful reads. Errors opening links or saving workspace data also appear here.

The application shows up to three notifications. Repeated identical results replace their previous
notification. Successful results disappear after five seconds. Hovering pauses their timers. Errors
remain until dismissed or replaced by newer notifications. Each notification has a close button.
Pipeline confirmations describe accepted requests, not completed jobs.

Opening or refreshing a project selects the checked-out branch and shows its history. Detached
checkouts show history from HEAD.

Local branches, remote branches, and tags have separate groups. Click a group heading to collapse or
expand it. Each heading shows a small expansion arrow. Groups collapse independently and preserve
the selected reference and nested folder states. Selecting a lightweight or annotated tag shows its
commit history without checking it out.

The local group starts expanded. Remote branches, tags, and nested branch folders start collapsed
when a project opens or refreshes. Their arrows expand each group independently.

The checked-out branch appears first, including branches inside collapsed folders. Then `main` and
`master` precede other local branches. Remote folders also place `main` and `master` first. Other
branches retain alphabetical order. Selecting a history filter does not change this order.

History loads 250 commits at a time. Scrolling near the bottom loads the next page until the oldest
local commit. Pages share a fixed revision snapshot. Refresh includes new commits and changed
references. A failed page keeps loaded history and offers a retry. Search loads remaining pages in
the background. Clearing search stops further background loading. Only visible rows and nearby rows
become widgets.

The graph uses actual parent hashes and preserves branch lanes across pages. Commit file changes
compare against the first parent. Root commits compare against an empty tree. Commit previews
highlight changes against that same parent. Deleted files show their previous contents in red. The
file tab identifies the selected commit. Other files open their selected commit revision. History
and source previews remain read-only. Only explicit Git actions change branches, files, or remotes.
Iwe does not create commits.

Commit titles follow the active graph lanes in each row. Wider sections elsewhere do not add empty
space before titles. Author and date columns remain aligned. Dates include hours and minutes in the
format `YYYY-MM-DD HH:MM`, using the author's recorded time zone. The selected commit's details show
the same timestamp.

## Pipelines

The pipeline button appears when the repository has a network remote. Iwe prefers `origin`, then the
first configured remote. Local filesystem remotes do not enable pipelines.

Known GitHub and GitLab hostnames select their provider automatically. For other hostnames, choose
GitHub or GitLab once. Iwe remembers this choice for that host. SSH transport ports do not become
API ports.

Iwe reads GitHub and GitLab APIs directly. Provider CLIs are not required. Public projects can load
without authentication when the provider allows anonymous access. The application requests pipeline
data when Git or pipelines is enabled. The panel lists the latest 30 runs. Each row includes a
compact chain of status icons. The external-link icon opens the provider page.

GitLab shows one icon per stage, in the provider's order. GitHub shows one icon per job, including
jobs with identical names. GitHub icons retain the provider's order and do not describe job
dependencies. Long chains scroll horizontally.

Click an icon to open its stage or job. The popup shows job names, statuses, and available actions.
Click a job name to open its provider page. Choose **All jobs** to open the complete job list. Click
outside the popup or press Escape to close it. Long job lists scroll within the popup.

Status symbols distinguish success, failure, running, waiting, manual, cancelling, cancelled,
skipped, and unknown states. Colors follow the current appearance. GitLab supplies each stage's
status, including its treatment of permitted job failures. Missing stage statuses use an aggregate
that keeps failed, unfinished, cancelled, and unknown jobs visible.

Visible rows and one following row load their summaries. At most two summary requests run
simultaneously. Completed summaries remain cached until the pipeline status changes or the user
refreshes them. Active visible summaries refresh every ten seconds. Failed summary reads preserve
cached icons and offer an explicit retry. Switching projects or signing out clears these summaries.

Click a run's title to open its GitHub or GitLab page. Each row shows its branch and abbreviated
commit identifier. Hover over the identifier to see the full value. Missing identifiers remain
hidden.

GitLab stages retain the provider's order. Click a stage heading to collapse its jobs. Each job
shows its name, status, and available actions. Manual jobs offer **Run**. Retryable jobs offer **Run
again**, and cancellable jobs offer **Stop**. These actions use the provider's existing job
configuration. Additional variables and inputs require the provider page.

GitHub Actions has no stage groups, so its jobs appear under one **Jobs** heading. **Rerun with
dependent jobs** repeats the selected job and its dependent jobs. GitHub supports cancellation of
the entire workflow run, not an individual job. The separate **Stop pipeline** button explicitly
cancels the entire run on either provider.

Visible jobs refresh every ten seconds. Cached jobs stay visible during refresh and after errors.
Actions require authentication and the corresponding project permissions. A pending action disables
further action buttons until its response arrives. Failed actions show an explanation and are never
resubmitted automatically. Refresh or sign in before trying again. Switching projects, changing
remotes, and signing out clear job details. Stale responses cannot replace another selection.

Job loading follows pagination, up to 1,000 jobs and 100 stages. Oversized lists offer the provider
page instead of incomplete results. Job loading has a 45-second total deadline. Each HTTP request
retains its 25-second deadline. Local fixtures verify reads and actions without starting real jobs.

History marks commits with active runs from that list. Full commit identifiers connect runs to
commits. Green markers indicate running jobs, while amber markers indicate waiting jobs. Loaded
pipeline data refreshes every 30 seconds while Git or pipelines remains visible. Refresh preserves
existing markers until results arrive. Failed requests clear the markers and stop automatic polling.
Use the pipeline panel to retry or sign in.

Project refresh keeps known Git metadata, pipeline availability, account details, and loaded runs
visible while replacement data loads. The pipeline panel retains its size during refresh. A
confirmed remote change or removed repository invalidates cached pipeline data. Switching projects
clears this cache so another project's data cannot appear.

An access error offers **Sign in**. GitHub opens the browser and displays a confirmation code.
GitLab.com opens browser authorization and returns through a local callback. The panel refreshes
automatically after successful authentication. Self-hosted GitLab requests a personal access token
with the api scope for job actions. The dialog always identifies the destination server. Iwe
validates the account before saving credentials in the system credential store. The panel header
shows the account and offers **Sign out**. Sign-out removes local credentials for that provider and
server. It does not revoke the provider's authorization grant.

Permission errors can persist when the account cannot access the repository. Network failures offer
a retry. API operations time out after 25 seconds. Expired OAuth credentials refresh automatically
when the provider supplies a refresh token. Canceling the dialog stops authorization and discards
its result. Switching directories also cancels pending authorization. See
[authentication setup](authentication.md) for Client IDs, scopes, and storage details.

## Layout and settings

The gear at the bottom left opens settings. Tabs separate General, Code, Panels, Keyboard shortcuts,
and About. General selects English, Russian, Spanish, or the system language. Language changes apply
immediately. General also switches between **Dark** and **Light** appearance. Dark remains the
default. Appearance changes apply immediately to panels, dialogs, source code, and both diff views.
The choice survives restarts. Switching themes preserves open files and scroll positions. Code
adjusts the source font size and line numbers. Opening Git closes pipelines. Opening pipelines
closes Git. Both panels can remain hidden. Drag the horizontal divider to resize the visible panel.

Preferences and projects are saved automatically in the system configuration directory. Writes
replace the previous configuration atomically. A damaged configuration is preserved without
automatic overwriting. Set `IWE_DATA_DIR` to use a separate configuration directory for development.
`IWE_LANG` overrides system language when the saved preference is **System**.

Keyboard shortcuts:

| Shortcut                 | Action                          |
| ------------------------ | ------------------------------- |
| Command/Ctrl + W         | Close the active file tab       |
| Command/Ctrl + Shift + T | Reopen the last closed file tab |

## Verification limits

Local tests cover persistence, previews, Git reads, provider detection, API responses, asynchronous
selection, and translated layouts. Tests use temporary repositories and recorded API shapes. They
never authenticate real accounts or contact providers. Authentication tests use local HTTP fixtures
and an in-memory credential store. They cover token validation, refresh rotation, callback state,
cancellation, and redirected requests. Live provider authorization and native credential-store
integration require configured accounts. Windows/Linux window behavior requires platform
verification.

RustSec reports upstream maintenance advisories and `RUSTSEC-2026-0253` for Iced's transitive `lru`
dependency. The `lru` issue requires a key with a panicking destructor during `pop()` and subsequent
unwinding recovery. The inspected `cryoglyph` cache uses copyable glyph keys and `pop_lru()`. This
limits the identified exposure but does not remove the upstream advisory. The compatible Iced
release still requires the older dependency. No advisory exclusions are configured.
