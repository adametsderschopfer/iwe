# Workspace

The [Deployment module](deployment.md) adds a right panel for server files and manual transfers.
Shared connections serve deployment profiles stored within each space.

IWE Code provides a native workspace for local directories, source previews, Git history, and CI
runs.

## Spaces, projects, and files

Use the plus beside **Spaces** and enter its name. Add existing project folders in the form, then
choose **Create space**. Select several folders in one directory dialog. IWE Code keeps each folder
once within the space. You can also create an empty space and add projects later. Canceling the form
discards its name, folder choices, and project order.

Use the settings button beside a space name to edit that space in the same form. Change its name,
add project folders, or remove projects, then choose **Save changes**. Cancel keeps the saved space
unchanged. Removing a project only detaches its folder. The application never deletes the folder.
The header shows **Space • Project** whenever a project is selected. Without a selected project, it
shows only the space name. Settings keeps its own heading.

The header shows up to two repository badges with their current branches. Projects with more
repositories show an arrow listing the remaining repository names and current branches. Click a
repository badge or list entry to open Git history and reveal that repository's current branch. This
selects its history filter without checking out a branch or changing project files. For a single
repository, the badge preserves the selected history filter. If Git history is already open, the
branches block shows a blue outline for 700 milliseconds. Clicking again restarts that interval.
Escape or clicking outside closes the repository list.

Right-click a file or directory in the project tree to open its context menu. Click **Copy** to copy
its absolute path. Hover **Copy** to choose an absolute path or a path relative to its repository
root. Nested repositories use their own root. Paths outside detected repositories use the project
root instead. Copying does not open files or change directory expansion.

Right-click a project and choose the red **Remove project from space** action to detach it
immediately. The action appears below **Pin project** or **Unpin project** and also works in the
pinned list. It removes only that space's project entry and saves the change. Project files remain
on disk. Other spaces retain their entries for the same directory. Removing the active project
selects another available project or returns to the empty workspace.

On Windows, the application header replaces the system title bar. Its right-hand buttons minimize,
maximize or restore, and close the window. Drag empty header space to move the window. Double-click
empty header space to maximize or restore it. Drag any edge or corner to resize a restored window.
Window controls remain available while a dialog is open.

Drag a space heading to change the space order. A line marks the drop position. Sidebar changes save
immediately. The open project and collapsed spaces keep their state. Dragging project rows still
changes only their order within that space.

Choose **Add project** to select existing folders or clone a remote repository. The space form
offers the same choice. Cloning accepts HTTPS and SSH repository URLs. The dialog derives the
repository folder name from the URL. Before a valid URL is entered, the destination field shows the
suggested parent folder. Entering a valid URL appends the repository folder name to that parent. You
can edit the complete destination or choose another parent folder before cloning.

The suggested parent is the most common parent folder among projects in the target space. The space
form uses its current folder choices. Empty or ambiguous choices use `IWE Code` inside the user's
home directory. Startup creates this fallback folder on first launch. With `IWE_DATA_DIR`, the
fallback folder stays inside that data directory for isolated runs.

Cloning runs in the background and adds the project only after success. Git uses existing credential
helpers and SSH configuration. Pipeline sign-in does not configure cloning access. Existing folders
are never overwritten. During cloning, the dialog prevents duplicate requests and cancellation.
Failures preserve the URL and destination for correction. A failed clone can leave its new folder on
disk. Choose another destination or inspect that folder before retrying. Canceling a space form
detaches its draft entries but preserves any repositories already cloned on disk.

Drag project rows to change their order. The form shows five complete rows and half the sixth row
when more projects exist. Scroll to reach the remaining projects. A line marks the drop position.
Holding a dragged row near the list edge scrolls the list. Dropping outside the list cancels the
move. **Save changes** applies the form's project order.

Drag project rows within their space in the sidebar to change the same order. Sidebar changes save
immediately and survive restarts. Reordering preserves the open project, file tabs, and loaded
history. Sidebar rows retain their existing icons. Dragging does not move projects between spaces.

The **Spaces** heading and its add button stay fixed above the scrolling project list.

Right-click a project and choose **Pin project**. Pinned projects appear before space groups in a
separate **Pinned** group. Each row identifies its space. The project retains its space membership
and disappears from that space's sidebar list. **Unpin project** returns it to the list. Pinning
survives restarts and does not reload the open project.

Click a space heading or its arrow to collapse or expand its projects. Spaces collapse
independently. Collapsing a space keeps its selected project open. The edit button remains
available.

Right-click a space heading to open **Edit space** and **Delete space**. The edit form also places
**Delete space** at the left of its bottom action row. Deleting a space removes its saved project
entries, including pinned entries, while preserving all project files on disk. Other spaces retain
their project entries. Deleting the active space selects another available project or opens the
empty workspace. Deleting another space preserves the active project and the remaining spaces'
collapsed states.

Empty space lists align their message with the **Spaces** heading.

Space settings remain at the sidebar's right edge. Project rows contain no separate file button. The
space list reserves a separate scrollbar lane, with a gap before space controls and project rows.

Projects with uncommitted files show a small green change icon at the right of their sidebar row.
Pinned projects use the same indicator. The **Uncommitted files** section uses the same
circle-and-line icon. Changes include staged, unstaged, untracked, and deleted files within the
project's discovered repositories. The selected project's existing three-second status refresh
updates its indicator. Other projects refresh at startup and every ten seconds, with at most two
simultaneous status reads. Repeated project directories share one result. Failed reads preserve the
last known indicator.

After copying a commit hash, its hover hint turns green and shows **Copied to clipboard** for two
seconds. The hint retains the complete hash. Repeated copying restarts that interval.

Hover hints use a consistent eight-pixel gap from their controls. Cursor-following hints keep the
same gap above the pointer. Hints remain inside the window near its edges.

Spaces group projects and have no filesystem path. Their sidebar headings do not open a project.
Each project represents one directory. Its folder name becomes the project name. Use **Add project**
beneath a space to attach another folder or clone a repository. Selecting a project opens its files
and Git history. Selecting the current project again preserves its files, history filter, and scroll
positions. Adding a folder again selects its existing entry within that space.

One click selects a project and immediately displays its files beside the projects sidebar. The file
panel sits between its vertical toolbar and the source preview. The files view lists project
contents directly, without a collapsible project row. Nested folders keep their expansion arrows.
The sidebar always retains its projects. Use the file toolbar to select **Project files** directly.
The empty preview hides its **Project files** button while that panel is already visible. The button
remains available when the panel is hidden or **Uncommitted files** is selected. The panel heading
aligns with the first toolbar button. Panel content and status messages share the heading's left
inset. The scroll area fills the file panel. Scrollbars stay at its edges, regardless of filename
length.

Click the active file toolbar icon again to hide its panel. The source preview fills the available
width while the toolbar remains visible. Click either icon to reopen its panel. Open files, source
scroll positions, and folder expansion remain unchanged.

Drag the divider beside the file panel to change its width. IWE Code saves the width when dragging
ends. Both file panels share this width across projects and restarts. Narrow windows limit the
visible panel width to preserve space for code. The panel heading and file tabs share the same
bottom edge.

A thin divider separates the workspace heading from its content. Another divider separates the file
panel from the source preview. A full-height divider separates the spaces sidebar from the
workspace. All dividers follow the current appearance.

The vertical toolbar switches between **Project files** and **Uncommitted files**. The latter
includes staged, unstaged, and untracked files. Drag these icons to change their order. IWE Code
saves the order and opens the first panel after reordering, restarting, or selecting another
project. **Project files** comes first by default. Missing deleted files open their previous
committed contents.

IWE Code checks the opened directory and its immediate subdirectories for Git repositories. It does
not scan deeper directories or follow child directory links. Linked worktrees are supported. Opening
a folder inside a repository still uses that containing repository.

For a parent folder containing `backend` and `front`, each repository has its own colored group in
**Uncommitted files**. The project tree marks repository directories with the same colors. File
comparisons use their owning repository, including deleted files and identical relative filenames.
An unavailable repository does not hide other repositories. Refresh the project after repairing it.

In **Uncommitted files**, collapsing a folder also collapses every nested folder. Reopening it
leaves nested folders collapsed. Collapsing a repository group applies the same rule to its folders.
Other folders and repository groups retain their expanded state.

Selecting a project starts a background directory index. Cached folders appear immediately while
their listings refresh. Recently visited projects also retain their expanded folders in memory.
Collapsing a folder also collapses all its nested folders. Reopening it leaves nested folders
collapsed. Other folders retain their expanded state. Opening a folder prioritizes its refresh
before speculative reads. Refresh keeps existing listings visible. A loading label appears only when
an uncached request exceeds 350 milliseconds.

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
selected folders, preferences, and provider choices are preserved. Before migration, IWE Code saves
the original configuration as `workspace.v1.json` beside `workspace.json`. Existing backups remain
unchanged. Opening an unassigned folder from the command line prefills the new space form.

The directory index contains no source contents. IWE Code reads file contents only when a file is
opened. Source contents remain in memory. Settings storage contains project metadata and
preferences. No source code is uploaded.

## Source preview

The code area uses a separate neutral background from surrounding panels. Dark appearance uses a
darker code background. Light appearance uses a white code background. Plain previews and both
comparison views share this background, including their line-number gutters.

The viewer displays UTF-8 text. Files larger than 512 KiB open in parts instead of being rejected.
Use **Previous part**, **Next part**, and **Refresh** above the text to navigate or reload a part.
Each part contains at most 512 KiB and 4,096 newline characters. Long lines can continue into the
next part with the same line number. UTF-8 characters and CRLF pairs remain intact at boundaries.
Binary content displays an explanation when encountered.

Large working files and historical Git versions use the same part controls. Deleted working files
can show their last committed contents in parts. Part navigation retains offsets rather than source
contents. Working files reopen at the first part when their size or modification time changes. Large
files refresh only through their part controls, not through automatic working-file polling. Git
comparisons remain unavailable for files opened in parts. Syntax highlighting restarts at each part,
so multiline constructs can lose their preceding context. Lines longer than 16 KiB use plain text to
bound syntax work.

The preview cannot edit files. Only visible lines and nearby lines become widgets. The visible range
follows the panel height. Stable line identities preserve text layout while scrolling. Long lines
stay unwrapped and use horizontal scrolling.

The viewer prepares new rows in small groups while scrolling. Source text uses Menlo on macOS and
Consolas on Windows. Linux uses the renderer's monospace family. Text selection uses the same font
as the displayed source.

Press Command+L on macOS or Ctrl+G on Windows and Linux to open **Go to Line**. These shortcuts
match [JetBrains defaults](https://blog.jetbrains.com/idea/2022/10/top-underrated-shortcuts/). Enter
a positive line number and press Enter. The viewer reveals hidden lines, scrolls to the target, and
selects its text. The highlight covers the full row width, including its line number. Clicking
source text or pressing Escape clears the row highlight. Invalid or missing numbers keep the dialog
open with an explanation. Escape or **Cancel** closes the dialog. Settings lists the shortcut.

Side-by-side comparisons use the selected source column, defaulting to the current version when it
exists. Deleted files default to their previous version. Unified comparisons prefer current source
line numbers. Large files load preceding parts in the background until the target is found. Only one
part remains in memory during scanning. Line searches have a 45-second deadline. Cancelled searches
cannot replace another file or a newer search result.

Drag across source text to select characters or multiple lines. Copy with Command+C on macOS or
Ctrl+C on Windows and Linux. Click the code before using Command+A or Ctrl+A to select its full
text. Escape clears the selection. Selection works in ordinary previews, unified comparisons, and
both columns of side-by-side comparisons. Each comparison column selects and copies only its own
source. Copied text excludes line numbers and change markers, preserves source tabs, and uses LF
line endings. Empty lines remain empty. Selection spans folded lines between its endpoints,
including their hidden text. For paged files, selection covers only the current part. Changing the
source clears its selection. Settings lists the copy and select-all shortcuts.

Files first display plain text. Syntax colors and Git changes follow on background workers. A thin
progress bar remains above the code until preparation finishes. Scrolling remains available during
preparation. Unchanged row blocks reuse their layout during scrolling. Closing a loading tab rejects
its pending results. Restoring that tab resumes preparation.

Scrollbar handles stay at least 28 pixels long in normal panels, even with very large content. Very
small panels use a shorter handle that fits their rail. Dragging reaches both ends without changing
the scroll range.

Opened files appear in a compact tab strip with file icons and close buttons. Selecting an existing
file reuses its tab and preserves horizontal and vertical scroll positions. Commit versions use
separate tabs with revision identifiers. Hovering a tab shows its complete path and revision. Tab
widths grow with their labels and reserve space for the close button. File names use at most 35
characters, including the trailing ellipsis for longer names. Commit tabs retain a separate
seven-character revision identifier after the file name.

The tab strip scrolls horizontally when needed and reveals the selected tab. Closing the active tab
selects its neighbor. When tabs exceed the available width, an arrow appears before the source-view
controls. The arrow opens a scrollable list of all open files, including their commit identifiers
when applicable. Choosing a file activates and reveals its existing tab. Escape or clicking outside
closes the list. Command+W closes the active tab on macOS. Ctrl+W closes the active tab on Windows
and Linux. These shortcuts keep the application open, including when no tabs remain. Settings and
modal dialogs block tab shortcuts. Middle-clicking a tab closes that tab. Command+Shift+T restores
the last closed tab on macOS; Ctrl+Shift+T restores it on Windows and Linux. Repeated presses
restore earlier closures, including commit versions, without duplicating open tabs. The latest 20
closed tabs retain their paths and scroll positions. Their prepared documents share a 16 MiB memory
budget. Older documents leave memory when that budget fills. Reopening them reads the original file
or Git revision on a background worker. Large files retain their current part. Unchanged documents
restore expanded blocks. Changed files rebuild their blocks, and changed large files restart at the
first part. Working files refresh after restoration. Settings lists both shortcuts. Refresh keeps
tabs and closure history for the current project. Changing projects clears both. Closing the
application clears this memory.

The bottom bar shows the active file's full path. Clicking the path copies it to the clipboard. When
no file is open, the bar shows the project path. Revision identifiers remain in commit tabs.

Syntax highlighting uses Syntect, bundled `two-face` grammars, and the Gerry Darcula colors from the
reference editor. Keywords appear purple, functions gold, strings green, and comments gray. Syntax
grammars determine colors without language-server analysis. The extension or filename selects the
grammar. Unknown formats remain readable as plain text. Supported grammars include Rust, Go,
JavaScript, TypeScript, TSX, Vue, Svelte, Python, Java, Kotlin, Swift, and C-family languages. They
also include SQL, JSON, YAML, TOML, Dockerfile, Makefile, Terraform, Bash, and Markdown.

C++ and C# files have dedicated icons in project trees, changed-file lists, and source tabs. C++
highlighting covers `.cpp`, `.cc`, `.cxx`, `.c++`, `.C`, `.h`, `.hpp`, `.hh`, `.hxx`, and `.h++`. It
also covers `.ipp`, `.tpp`, `.inl`, `.ixx`, `.cppm`, and `.cxxm`. Headers with `.h` use the C++
grammar, which also handles C declarations. C# highlighting covers `.cs` and `.csx`. These
extensions also work in uppercase, with `.C` specifically selecting C++ instead of C.

The project tree uses 24-pixel rows, 14-pixel text, 16-pixel icons, and 20-pixel nesting steps,
following the supplied JetBrains reference. Each row centers its name and icon. Folders show
separate expansion arrows and folder icons. Recognized Python packages show a package marker.
Selecting a folder highlights its row without closing the source preview. Long filenames remain
complete and scroll horizontally. File-type icons also appear beside changed files in Git. The
active working file stays highlighted in project files and uncommitted files when selecting a
folder. File highlights span the available row width and follow panel resizing. Commit-file lists
highlight the active file only when its repository and revision match the selected commit.

Git decorations color modified and renamed filenames cyan. Added and untracked filenames appear
green. Deleted files and conflicts appear red. Status letters and folder dots are absent. Commit
files and uncommitted files share the same tree style. Their folders compact consecutive single
directories and show file counts. File tooltips describe each status. Ignored files keep their
normal appearance. Status and the open working file refresh in the background every three seconds.

The uncommitted files panel has a toolbar below its heading separator. Its refresh button manually
reloads working-file status and the open working file. Existing rows remain visible during refresh.
Manual refresh also retries repositories after automatic status polling stops.

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
column. IWE Code saves this choice for all files and across restarts. Both views retain their scroll
positions. Switching views keeps the document and expanded unchanged ranges. Commit comparisons use
the same views. Files without a comparison keep their normal preview and hide the view button.

The unified view shows previous and current line numbers in separate gutter columns. Missing numbers
identify lines present in only one version. Replaced previous lines use gray-blue backgrounds; their
replacements use green backgrounds. Deletions without replacements remain red. Word highlights
retain syntax colors. The line-number setting controls both gutter columns. Unchanged ranges appear
as clickable wave separators. Hover over a separator to see its line range.

Source rows meet the tab strip and left edge without an outer inset. Line numbers and change markers
stay fixed at the left during horizontal scrolling. This applies to plain source and both comparison
views. Vertical scrolling moves numbers with their corresponding source rows. Space below the final
line keeps the horizontal scrollbar clear.

Use the button beside the file tabs to collapse unchanged lines. Changed lines keep their original
line numbers and colors. Click a collapsed range to expand that range. The button restores all lines
and the previous scroll position. This mode applies to all files, including commit versions. IWE
Code saves the mode across restarts and applies it to new and restored tabs. Each tab retains its
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

Hold Command on macOS or Ctrl on Windows and Linux to select several branches or branch folders.
Folders select their nested branches. The counter below the branch list shows the selected branch
count. Right-click the selection for bulk push, delete, and pull. Each action opens a review dialog.
An ordinary branch click or another workspace click clears selection. See
[Git actions](git-actions.md#multiple-branches) for eligibility, confirmation, and retry behavior.

Operation results appear in the lower-right corner without resizing workspace panels. Git
operations, pipeline actions, sign-in, and sign-out report their results. Background refreshes do
not announce successful reads. Errors opening links or saving workspace data also appear here.

The application shows up to three notifications. Repeated identical results replace their previous
notification. After five seconds, each notification slides right out of the window over 200
milliseconds. The notifications above then move down over 200 milliseconds. Closing a notification
uses the same animation. A shrinking bar shows their remaining time. Hovering pauses only that
notification and its bar. Moving away resumes its remaining time. Other notifications continue
counting down. Each notification also has a close button. Pipeline confirmations describe accepted
requests, not completed jobs.

For one repository, opening or refreshing a project selects the checked-out branch and shows its
history. Detached checkouts show history from HEAD.

Repository labels in history fit their text and sit close to the graph. Long labels truncate within
160 pixels. Hovering a label shows the complete repository path.

For multiple repositories, history initially includes references from every repository. Colored
labels identify each commit's repository. Branches appear under separate repository headings.
Selecting a branch shows that repository's branch history. **All repositories** restores the
combined history. Search also matches repository names. Pipeline data follows the selected
repository.

Local branches, remote branches, and tags have separate groups. Click a group heading to collapse or
expand it. Each heading shows a small expansion arrow. Groups collapse independently and preserve
the selected reference and nested folder states. Selecting a lightweight or annotated tag shows its
commit history without checking it out.

The vertical toolbar to the left of branches contains refresh, fetch, new branch, update, and push.
A vertical separator divides the toolbar from the branches. Commit search remains above the history.
The toolbar scrolls independently when panel height is limited. The toolbar has no hide-panel
button.

The local group starts expanded. Remote branches, tags, and nested branch folders start collapsed
when a project opens or refreshes. Their arrows expand each group independently.

The checked-out branch appears first, including branches inside collapsed folders. At each tree
level, branches appear before folders containing branches. Within the branch rows, `main` and
`master` keep their priority, followed by other branches alphabetically. Folders retain alphabetical
order. Remote groups use the same branch-before-folder ordering. Selecting a history filter does not
change this order.

History loads 250 commits at a time. Scrolling near the bottom loads the next page until the oldest
local commit. IWE Code caches the initial history page and repository metadata for eight recently
opened projects in memory. Returning to a cached project immediately restores its branches, initial
history, selected branch, and expanded groups. The application refreshes that snapshot in the
background without replacing it with an empty loading view. Temporary refresh failures retain the
snapshot and show an error notification. If Git confirms that the project is no longer a repository,
the application discards its snapshot. The cache does not survive application restarts. Additional
history pages retain the existing paging behavior. Pages share a fixed revision snapshot. Refresh
includes new commits and changed references. A failed page keeps loaded history and offers a retry.
Search loads remaining pages in the background. Clearing search stops further background loading.
Only visible rows and nearby rows become widgets.

The combined history compares commit timestamps while preserving each repository's ancestry order.
Identical hashes in different repositories remain separate entries. The graph uses actual parent
hashes and preserves separate repository lanes across pages. Commit file changes compare against the
first parent. Root commits compare against an empty tree. Commit previews highlight changes against
that same parent. Deleted files show their previous contents in red. The file tab identifies the
selected commit. Other files open their selected commit revision. History and source previews remain
read-only for Git revisions and comparisons. Local and server text files support explicit editing
and saving. IWE Code does not create commits.

Commit titles follow the active graph lanes in each row. Wider sections elsewhere do not add empty
space before titles. Author and date columns remain aligned. Dates include hours and minutes in the
format `YYYY-MM-DD HH:MM`, using the author's recorded time zone. The selected commit's details show
the same timestamp.

Each history row shows a gray seven-character commit identifier after its title. Click the
identifier to copy the full commit identifier without changing the selected commit. Hover over the
identifier to see the full value and the copy action.

## Pipelines

The pipeline button appears when the repository has a network remote. IWE Code prefers `origin`,
then the first configured remote. Local filesystem remotes do not enable pipelines.

Known GitHub and GitLab hostnames select their provider automatically. For other hostnames, choose
GitHub or GitLab once. IWE Code remembers this choice for that host. SSH transport ports do not
become API ports.

IWE Code reads GitHub and GitLab APIs directly. Provider CLIs are not required. Public projects can
load without authentication when the provider allows anonymous access. The application requests
pipeline data when Git or pipelines is enabled. The panel lists the latest 30 runs. Each row
includes a compact chain of status icons. The external-link icon opens the provider page.

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
existing markers until results arrive. Temporary read failures preserve loaded runs and markers, but
stop automatic polling. Access and credential failures clear loaded runs and the session cache. Use
the pipeline panel to retry or sign in.

Project refresh keeps known Git metadata, pipeline availability, account details, and loaded runs
visible while replacement data loads. The pipeline panel retains its size during refresh. A
confirmed remote change or removed repository clears the displayed pipeline data.

IWE Code keeps pipeline lists for eight recently used repositories in memory. Each entry includes up
to 30 runs. Cache keys include the repository directory, remote host, remote project, and provider.
Returning to a repository restores its list while a background request loads current data. Switching
projects never displays another repository's runs. Signing in or signing out clears the session
cache. The cache stores no credentials and does not survive application restarts. Stage summaries
and job details still reload after project changes.

An access error offers **Sign in**. GitHub opens the browser and displays a confirmation code.
GitLab.com opens browser authorization and returns through a local callback. The panel refreshes
automatically after successful authentication. Self-hosted GitLab requests a personal access token
with the api scope for job actions. The dialog always identifies the destination server. IWE Code
validates the account before saving credentials in the system credential store. The panel header
shows the account and offers **Sign out**. Sign-out removes local credentials for that provider and
server. It does not revoke the provider's authorization grant.

Permission errors can persist when the account cannot access the repository. Network failures offer
a retry. API operations time out after 25 seconds. Expired OAuth credentials refresh automatically
when the provider supplies a refresh token. Canceling the dialog stops authorization and discards
its result. Switching directories also cancels pending authorization. See
[authentication setup](authentication.md) for Client IDs, scopes, and storage details.

## Layout and settings

The gear at the bottom left opens settings in a separate window. Reopening settings focuses the
existing window. Closing settings keeps the workspace open. Settings changes apply to both windows
immediately. Command+, on macOS or Ctrl+, on Windows and Linux also opens settings. A persistent
left navigation column lists General, Code, Panels, Keyboard shortcuts, and About. The selected
section appears on the right with its own heading and scrollable controls. Section headings and
controls share one left edge. Compact rows place descriptions left and controls right. Subtle
separators divide rows. Module settings use the same compact list layout. On macOS, an opaque
application header covers the titlebar area while preserving native window controls. The header
supports dragging and double-clicking to maximize or restore the settings window. General selects
English, Russian, Spanish, or the system language. Language changes apply immediately. General also
switches between **Dark** and **Light** appearance. Dark remains the default. Appearance changes
apply immediately to panels, dialogs, source code, and both diff views. The choice survives
restarts. Switching themes preserves open files and scroll positions. Code adjusts the source font
size and line numbers. Opening Git closes pipelines. Opening pipelines closes Git. Both panels can
remain hidden. Drag the horizontal divider to resize the visible panel.

On macOS, **Spaces**, **Projects**, **Settings**, and **Deployment** appear beside the **IWE Code**
application menu. **Deployment** offers separate windows for connections and servers. The SFTP key
selector opens the shared key manager. **Spaces** contains only **New space**. **Settings** opens
the settings window through its menu item. **Projects** groups projects by their space. An empty
**Projects** menu shows a disabled **No projects yet** item. Choosing a project brings the workspace
window forward. The menu follows language and workspace changes. Modal dialogs disable workspace
navigation in the menu. Windows and Linux retain their existing sidebar controls.

Escape dismisses dialogs and context menus, including forms with focused text fields. Escape closes
the focused settings or deployment window. If a nested dialog is open, Escape dismisses that dialog
first. While recording a shortcut, Escape cancels recording.

Preferences and projects are saved automatically in the system configuration directory. Writes
replace the previous configuration atomically. A damaged configuration is preserved without
automatic overwriting. Set `IWE_DATA_DIR` to use a separate configuration directory for development.
`IWE_LANG` overrides system language when the saved preference is **System**.

Keyboard shortcuts use two groups in settings: **Code** and **Tabs**. Click a shortcut button, then
press the new combination. Escape or **Cancel** keeps the previous shortcut. Use Ctrl, Command,
Super, or Alt with a letter, digit, or F1–F12. Shift can supplement these modifiers. IWE Code
rejects combinations assigned to another listed action or reserved application commands. A rejected
combination keeps recording open and preserves the previous shortcut. Valid changes apply
immediately and persist with workspace preferences. Shortcuts follow physical key positions across
keyboard layouts. Text fields and the terminal retain their own keyboard controls. The
settings-opening shortcut remains fixed. macOS also reserves its hide and quit shortcuts. The table
lists the defaults.

| Group | macOS               | Windows and Linux | Action                          |
| ----- | ------------------- | ----------------- | ------------------------------- |
| Code  | Command + L         | Ctrl + G          | Go to a source line             |
| Code  | Command + C         | Ctrl + C          | Copy selected source text       |
| Code  | Command + A         | Ctrl + A          | Select all source text          |
| Tabs  | Command + W         | Ctrl + W          | Close the active file tab       |
| Tabs  | Command + Shift + T | Ctrl + Shift + T  | Reopen the last closed file tab |

## Modules

Open **Settings → Modules** to enable or disable Git, Pipelines, Terminal, and Deployment. All
modules are enabled by default. Disabling preserves their saved settings. Modules cannot be removed.
Open **Terminal** in the sidebar to run commands in the selected project's folder. See
[modules and terminal sessions](modules.md) for controls, session behavior, and limits.

## Application updates

IWE Code checks for new releases at startup and hourly. Returning to the window can also trigger a
check after a 15-minute cooldown. Failed checks retry after 15 minutes. A blue sidebar button offers
available updates. Download the update, then choose **Install and restart**. Settings also offers a
manual check under **About**. IWE Code verifies signed packages before installation. See
[application updates](updates.md) for supported installations and recovery behavior.

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

## Text editing

Local text files and server files open in an editable buffer when their size does not exceed 2 MB.
The editor saves after an 800 ms typing pause. Server buffers save to their original deployment
profile. Saving local files does not upload them. Command+S on macOS, or Ctrl+S on Windows and
Linux, saves immediately or retries a failed save. Shortcuts remain configurable and follow physical
keys across keyboard layouts. Undo uses Command+Z or Ctrl+Z. Add Shift for redo.

The editor supports selection, cut, copy, paste, multiline syntax highlighting, and line navigation.
Tab inserts four spaces. Line numbers follow the editor's layout and scrolling, including cursor
navigation. The line-number preference controls their visibility. The separate save and
saved-version toolbar is removed. The tab marks unsaved changes and shows save errors in its
tooltip. Errors also appear in notifications. Failed saves stop automatic retries, including after
further edits.

Project files open without Git comparison preparation. The uncommitted-files section opens separate
comparison tabs for the same paths. Comparison controls appear in comparison tabs, including
explicit server comparisons. Ordinary file tabs have no comparison controls. Git revisions,
comparison snapshots, and paged files retain the source viewer.

Closing edited tabs, switching projects, disabling modules, installing updates, and closing the
application flush pending edits first. Failed saves keep the buffer open for retry, discard, or
cancel. Edits made during saving are scheduled again after completion. Local saves prepare a
temporary file beside the destination and preserve file permissions. The editor checks disk content
before replacement and refuses conflicting external edits. Symbolic link files must be opened
through their original path before saving. Buffers and bounded undo history remain in memory; crash
recovery is not provided.

Project clicks remain active after scrolling the sidebar. Small pointer movements do not start
reordering. Only space headers start space reordering. The path-copy highlight fits the displayed
path in the footer.

The file panel, file tabs, and deployment panel have borders against the code area. File hints
follow the cursor. File lists scroll horizontally when a visible row exceeds their width. The status
bar aligns background operations to the right.

The editor keeps its vertical scrollbar handle visible in large files. Drag the handle or click its
track to scroll without moving the text cursor.
