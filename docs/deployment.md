# Deployment

The Deployment system module transfers project files through SFTP, FTP, or FTPS. It provides manual
transfers and remote comparisons. It does not upload files automatically.

## Connections and profiles

Open **Settings → Deployment → Connections** to manage connections. Connections remain available
across all spaces. On macOS, the **Deployment** menu opens Connections or Profiles. Select a server
in the left list. Edit its aligned fields on the right. The plus button opens a name and protocol
dialog. The toolbar also supports deletion, duplication, and renaming. Protocol and authentication
controls use dropdown lists. **Save** applies the selected draft; **Revert changes** restores its
saved values. Selecting another item asks before discarding an edited draft. Switching settings
sections preserves each draft. Duplicating a password connection requires entering its password
again.

Open **Shared SSH keys** inside Connections to manage key files. Keys have no separate settings
navigation item. **Add key** opens a focused dialog with a name and file picker. Existing keys
support editing and deletion. The key selector also provides an add button. Saving through that
button selects the new key for the connection. The key manager preserves the connection's
authentication choice. Cancel returns without adding a key. SFTP supports passwords, private key
files, and the system SSH agent. The shared key list stores names and file paths. IWE Code does not
copy private key files. Entering a password replaces the saved password or key passphrase. Leaving
the field untouched preserves the existing secret. Passwords use Keychain, Windows Credential
Manager, or Linux Secret Service. Workspace metadata never stores passwords or private key contents.
Use **Test connection** before saving. The test status appears directly above its button. Progress
uses neutral text; success uses green, and failure uses red. The button stays unavailable while the
test runs. Cancellation and SSH fingerprint confirmation remain available beside the test controls.

An unknown SSH server requires fingerprint confirmation before authentication. A changed server key
blocks the connection. Verify the server independently before replacing its saved connection. FTPS
verifies certificates and server names. FTPS supports explicit TLS and implicit TLS. FTP uses
unencrypted connections. FTP and FTPS use passive data connections and binary transfers.

Open **Settings → Deployment → Profiles** for the selected space. The space context menu also opens
deployment settings. Each profile belongs to one space and targets one project. Several profiles can
target the same project. Select a profile in the left list. The right editor separates
**Connection**, **Mappings**, and **Excluded paths** into tabs. Connection selects the saved server,
root directory, and default profile behavior. The settings button beside the connection selector
opens shared connections. The folder browser selects the server root. Mappings pairs local and
remote directories. Excluded paths lists project-relative paths. Profile duplication preserves
mappings and exclusions without copying the default flag. Choose one profile as the project's
default. Selecting a profile in the right panel also updates that default.

Profile tabs fit their translated labels and share one baseline. The active tab has a blue
underline. Narrow editors allow horizontal scrolling without wrapping tab labels.

Mappings pair project-relative folders with server-root-relative folders. Empty paths select their
corresponding roots. For example, local `public` and remote `www` map `public/index.php` to `<server
root>/www/index.php`. Local mappings cannot overlap. Server mappings cannot overlap either.
Exclusions contain project-relative paths, including their descendants. The module always excludes
`.git` directories. It does not follow symbolic links inside mapped paths.

Deleting a connection requires removing its profiles first. Deleting a referenced key requires
changing its connections first. Removing a connection or key entry does not erase its saved secret
from the system credential store. Users can remove orphaned secrets through the operating system's
credential manager.

## Panel and file operations

The server icon on the right opens the Deployment panel. The panel shares the editor's height above
Git and terminal panels. Drag its left divider to resize it. Closing the panel preserves its loaded
tree and active operation. The module does not connect automatically on application startup.

Select a profile to browse its server root. Expand folders with their arrows or double-click their
rows. Double-click a remote file to open an editable text buffer. Refresh reloads the root directory
listing. Right-click local files or folders to upload them. Right-click remote files or folders to
download them through their mappings. Right-click a local or remote file to compare both copies.
Comparison immediately opens a separate tab. The right pane loads the local file while the left pane
waits for the server. The completed comparison displays the server snapshot on the left and the
local snapshot on the right. Identical files retain both comparison panes. Comparison tabs use a
server icon. Comparison never changes the local file. A server response updates its existing tab
without changing the selected tab. Closing the comparison prevents its response from reopening the
tab.

| Action                          | macOS default        | Windows and Linux default |
| ------------------------------- | -------------------- | ------------------------- |
| Compare active file with server | Command + Option + C | Ctrl + Alt + C            |
| Upload active file              | Command + Option + V | Ctrl + Alt + V            |

Deployment commands appear in **Settings → Keyboard shortcuts**. Physical keys also support
non-Latin keyboard layouts. Settings windows and modal dialogs do not execute deployment shortcuts.
Focused terminal input retains its keyboard events. The default deployment combinations do not run
from the terminal.

Upload reads the latest file from disk and replaces the remote file without another confirmation.
The bottom-right activity indicator and completion notification identify the selected profile.
Folder transfers preserve relative paths and create missing folders. Downloads require confirmation
before replacing existing local files. Each download uses a temporary file beside its destination.
The module replaces the destination only after successful transfer and a final local-change check.

Operations execute sequentially without opening the Deployment panel. The bottom-right status bar
shows the operation, profile, and transferred bytes. Click the activity indicator to expand details,
completed-file counts, and queued-operation counts. These details provide cancellation, explicit
retry, server-key confirmation, and overwrite confirmation. Errors expand the details automatically.
Closing the details preserves active transfers. Cancel stops subsequent transfer work. An active
network call can continue until its timeout. Files already uploaded can remain changed after
cancellation or failure. Uploads write directly to the destination; failed uploads can leave partial
remote files. Retry requires an explicit action. Switching projects cancels the previous project's
operations. Results from obsolete requests cannot replace the current project state.

## Limits and verification

Text previews and comparisons accept UTF-8 files up to 2 MiB per side. Binary files remain
transferable. Transfers use 64 KiB buffers. Recursive transfers allow 10,000 entries and 64
directory levels. The pending operation queue holds at most 32 requests. Transfer sessions are not
persisted across application restarts. The module does not delete remote files, synchronize
directories, edit remote files, or interpret SSH config and ProxyJump. FTP directory browsing
requires a supported LIST response. Server permissions and filesystem features can restrict
available operations. Local-change checks cannot eliminate every race with unrelated external
writers. SSH-agent availability depends on the operating system and its configured agent.

Automated tests use isolated temporary files and loopback servers. They exercise SFTP, FTP, explicit
FTPS, and implicit FTPS. Tests cover server trust, password authentication, private keys, encrypted
private keys, and an isolated Unix SSH agent. FTPS tests verify certificate rejection and
authenticated encrypted transfers. They never use production servers or saved user credentials.
Native credential prompts and Windows/Linux agent integration require separate platform checks.

Run `make check` and `make build` for the complete verification suite.

## Editing server files

Save sends the buffer to the profile that opened the tab, even if another profile becomes selected.
The toolbar names that destination. Saving requires an enabled deployment module and an idle
transfer queue. The server file must have a valid mapping and must not match exclusions or symbolic
links. Before writing, the transfer reads the current server content and compares it with the loaded
baseline. A conflict or transfer failure leaves the draft in the tab. Saves never retry
automatically. Remote writes use the existing streaming upload transport. An interrupted upload can
leave partial server content. Server protocols do not provide a universal atomic compare-and-swap;
concurrent writes during upload cannot be excluded. Explicit comparison tabs remain immutable
snapshots. Open a server file from the remote tree to edit it.
