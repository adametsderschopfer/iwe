# Modules

IWE Code provides bundled modules under **Settings → Modules**. Each system module displays a
**System** badge. System modules are always enabled. You cannot disable or remove a system module.
Legacy disabled-module preferences do not affect system modules.

The terminal is the first module. Git and pipelines keep their existing panel settings. The current
application has no module store, external module installation, or external module loader.

## Deployment

The Deployment system module provides shared SFTP, FTP, and FTPS connections. Profiles belong to
spaces and target individual projects. Its resizable right panel browses remote files independently
of the terminal and Git panels. See [deployment](deployment.md) for mappings, credentials, transfer
behavior, keyboard shortcuts, and verification limits.

## Terminal

Select a project, then choose **Terminal** in the sidebar. Module cards show the name, system badge,
and enabled status. They have no activation switch. The first opening creates a terminal tab in the
project's directory. Later openings restore existing tabs. Each tab owns an independent interactive
shell. Commands inherit your account's permissions and environment. Commands can modify files,
launch programs, and access the network.

Click the terminal area before typing. The terminal supports Unicode text, colors, cursor movement,
command history, Tab completion, and common function keys. Ctrl+C interrupts the current command.
The shell determines which commands and programs are available.

| Control                             | Action                                                 |
| ----------------------------------- | ------------------------------------------------------ |
| Plus beside the tabs                | Create and select another terminal tab                 |
| Tab label                           | Select that tab without changing its session or output |
| Cross on a tab                      | Close that tab and end its session                     |
| Cross opposite the Terminal heading | Hide the panel while keeping all tabs and sessions     |
| Mouse wheel                         | Read earlier terminal output                           |
| Mouse drag                          | Select visible terminal text                           |

Closing the last tab hides the panel. Opening the panel again creates a new tab when no tabs remain.
Tabs scroll horizontally when they exceed the available width. New tabs scroll into view. An ended
shell retains its tab and output until you close the tab.

Each tab groups its label and close control within one shared surface. The plus follows the last tab
inside the same horizontal scrolling area.

On macOS, Command+C copies selected text and Command+V pastes text. With no selection, copying uses
the visible screen. Windows and Linux use Ctrl+Shift+C and Ctrl+Shift+V. Ctrl+C remains available to
the shell on every platform. Clicking the terminal clears source-text and branch selections.

Dragging the horizontal divider resizes the terminal. The terminal also follows window resizing. The
terminal uses the saved source font size and current appearance. Output does not change when the
appearance changes.

Selecting a different project stops all sessions and closes the panel. Reopening the panel creates a
tab for that project. Refreshing the same project keeps its existing tabs and sessions. Sessions and
output do not survive an application restart. Programs that deliberately detach from the shell may
continue independently.

## Resource and platform limits

Each tab retains up to 2,000 scrollback lines. Screen dimensions stay within 200 rows and 400
columns. Input requests have a 64 KiB limit. Paste requests reserve 12 bytes for bracketed-paste
markers. Input and output queues each hold up to 64 requests. Output requests contain at most 4 KiB.
The application consumes at most 32 output requests per message.

Large paste requests are rejected. The terminal removes NUL and escape characters from pasted text.
Shells that enable bracketed paste receive the corresponding boundary markers. The terminal ignores
program requests to read or replace the system clipboard. It does not interpret output as
application commands or open links automatically.

macOS and Linux use a pseudo-terminal and the operating system's default shell. Windows uses ConPTY
and the system's default command processor. Windows requires a ConPTY-capable Windows version. No
shell is downloaded or installed.

The module keeps terminal tabs for the selected project. It has no terminal search, clickable links,
program-controlled mouse reporting, or input-method composition interface. Tests verify process
input, output, resizing, cleanup, preferences, translated settings, and session guards. Native
checks on one platform do not verify the other platforms.

## Source organization

Each bundled module has its own directory under `src/modules/`. The terminal keeps its
implementation and unit tests under `src/modules/terminal/`:

```text
modules/
├── mod.rs
├── catalog/
└── terminal/
    ├── mod.rs
    ├── style.rs
    ├── locales/
    │   ├── en.ftl
    │   ├── ru.ftl
    │   └── es.ftl
    ├── state/
    │   ├── mod.rs
    │   └── tests.rs
    ├── session/
    │   ├── mod.rs
    │   └── tests.rs
    └── view/
        ├── mod.rs
        ├── input.rs
        └── tests.rs
```

The module exposes its state, events, subscriptions, and view through `terminal/mod.rs`. The module
does not import host application state. The session adapter does not import presentation code.
Shared application code connects module events to workspace actions. Host integration tests remain
under `src/app/modules/`. Shared settings remain under `src/ui/modules.rs`.

Terminal translations live in the module's own `locales/` directory. Shared application translations
remain under root `locales/`. The shared localizer embeds and combines both resource sets. Duplicate
message identifiers reject the combined catalog. Typed built-in message keys remain registered in
`src/i18n/mod.rs`. Catalog tests validate each source and the combined messages.
