# Deployment

The Deployment system module transfers project files through SFTP, FTP, or FTPS. It provides manual
transfers and remote comparisons. It does not upload files automatically.

## Connections and profiles

Open **Deployment → Connections** to manage connections in a separate window. Connections remain
available across all spaces. The server panel and space context menu also open this window. Select a
server in the left list. Edit its aligned fields on the right. The plus button opens a name and
protocol dialog. The toolbar also supports deletion, duplication, and renaming. Protocol and
authentication controls use dropdown lists. **Save** applies the selected draft. Selecting another
item asks before discarding an edited draft. Switching deployment sections preserves each draft.
Duplicating a password connection requires entering its password again.

Open **Shared SSH keys** inside Connections to manage key files. Keys have no separate settings
navigation item. **Add key** opens a focused dialog with a name and file picker. Existing keys
support editing and deletion. The key selector also provides an add button. Saving through that
button selects the new key for the connection. The key manager preserves the connection's
authentication choice. Cancel returns without adding a key. SFTP supports passwords, private key
files, and the system SSH agent. Windows builds use bundled OpenSSL for Ed25519 host and user keys.
The shared key list stores names and file paths. IWE Code does not copy private key files. Entering
a password replaces the saved password or key passphrase. Leaving the field untouched preserves the
existing secret. Passwords use Keychain, Windows Credential Manager, or Linux Secret Service.
Workspace metadata never stores passwords or private key contents. Use **Test connection** before
saving. The test status appears directly above its button. Progress uses neutral text; success uses
green, and failure uses red. The button stays unavailable while the test runs. Cancellation and SSH
fingerprint confirmation remain available beside the test controls.

An unknown SSH server requires fingerprint confirmation before authentication. A changed server key
blocks the connection. Verify the server independently before replacing its saved connection. FTPS
verifies certificates and server names. FTPS supports explicit TLS and implicit TLS. FTP uses
unencrypted connections. FTP and FTPS use passive data connections and binary transfers.

Open **Deployment → Profiles** for the selected space. The space context menu also opens deployment
settings. Each profile belongs to one space and targets one project. Several profiles can target the
same project. Select a profile in the left list. The right editor separates **Connection**,
**Mappings**, and **Excluded paths** into tabs. Connection selects the saved server, root directory,
and default profile behavior. The settings button beside the connection selector opens shared
connections. A separate folder dialog selects the server root. Cancel leaves the root unchanged and
ignores late directory responses. Mappings pairs local and remote directories. Excluded paths lists
project-relative paths. Profile duplication preserves mappings and exclusions without copying the
default flag. Confirming duplication saves a separate profile immediately. An unsaved source also
remains available in the list. Choose one profile as the project's default in its Connection tab.
Selecting another profile in the server panel changes only the browsed server. Local upload and
comparison commands continue using the default profile. Without a default, these commands open a
profile chooser. Choosing a target applies only to that operation.

Profile tabs fit their translated labels and share one baseline. The active tab has a blue
underline. Narrow editors allow horizontal scrolling without wrapping tab labels.

Mappings display the absolute local folder and prefill the project directory. Local folders must
remain inside that project. Storage keeps local paths relative to the project. Server paths remain
relative to the configured server root, including paths entered with a leading slash. Empty paths
select their corresponding roots. Folder buttons open local and server directory pickers. The server
mapping picker starts at the profile root and stores the selected directory relatively. Cancelling
either picker preserves the mapping. For example, local `public` and remote `www` map
`public/index.php` to `<server root>/www/index.php`. With root `/home/bitrix/ext_www`, remote
`/shop` resolves to `/home/bitrix/ext_www/shop`. Local mappings cannot overlap. Server mappings
cannot overlap either. Exclusions contain project-relative paths, including their descendants. The
module always excludes `.git` directories. It does not follow symbolic links inside mapped paths.

Deleting a connection requires removing its profiles first. Deleting a referenced key requires
changing its connections first. Removing a connection or key entry does not erase its saved secret
from the system credential store. Users can remove orphaned secrets through the operating system's
credential manager.

## Panel and file operations

The server icon on the right opens the Deployment panel. The panel shares the editor's height above
Git and terminal panels. Drag its left divider to resize it. Closing the panel preserves its loaded
tree and active operation. The module does not connect automatically on application startup.

Select a profile to browse its server root. Expand folders with their arrows or double-click their
rows. Double-click a remote file to open an editable text buffer. Refresh invalidates cached folders
and reloads the root directory. Collapse all closes expanded folders without discarding cached
entries. Right-click local files or folders to upload them. Right-click remote files or folders to
download them through their mappings. The remote menu also offers upload, open local file, open
server file, and copy absolute path. Mapped actions stay unavailable outside valid mappings. Remote
menu actions use the browsed profile, independently of the default profile. Right-click a local or
remote file to compare both copies. Comparison immediately opens a separate tab. The right pane
loads the local file while the left pane waits for the server. The completed comparison displays the
server snapshot on the left and the local snapshot on the right. Identical files retain both
comparison panes. Comparison tabs use a server icon. Comparison never changes the local file. A
server response updates its existing tab without changing the selected tab. Closing the comparison
prevents its response from reopening the tab.

| Action                          | macOS default       | Windows and Linux default |
| ------------------------------- | ------------------- | ------------------------- |
| Compare active file with server | Command + Shift + C | Ctrl + Shift + C          |
| Upload active file              | Command + Shift + A | Ctrl + Shift + A          |

Deployment commands appear in **Settings → Keyboard shortcuts**. Physical keys also support
non-Latin keyboard layouts. Settings windows and modal dialogs do not execute deployment shortcuts.
Focused terminal input retains its keyboard events. The default deployment combinations do not run
from the terminal.

Upload and comparison first finish autosaving affected local buffers. A save error prevents the
operation and preserves the draft. Folder uploads flush local buffers inside that folder only.
Upload then reads the latest file from disk and replaces the remote file without another
confirmation. The bottom-right activity indicator and completion notification identify the selected
profile. Folder transfers preserve relative paths and create missing folders. Downloads require
confirmation before replacing existing local files. Each download uses a temporary file beside its
destination. The module replaces the destination only after successful transfer and a final
local-change check.

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
directories, or interpret SSH config and ProxyJump. FTP directory browsing requires a supported LIST
response. Server permissions and filesystem features can restrict available operations. Local-change
checks cannot eliminate every race with unrelated external writers. SSH-agent availability depends
on the operating system and its configured agent.

Automated tests use isolated temporary files and loopback servers. They exercise SFTP, FTP, explicit
FTPS, and implicit FTPS. Tests cover server trust, password authentication, private keys, encrypted
private keys, and an isolated Unix SSH agent. FTPS tests verify certificate rejection and
authenticated encrypted transfers. They never use production servers or saved user credentials.
Native credential prompts and Windows/Linux agent integration require separate platform checks.

Run `make check` and `make build` for the complete verification suite.

## Editing server files

Autosave sends edits after an 800 ms typing pause to the profile that opened the tab. Changing the
selected profile does not change that destination. The tab title names the destination. Saving
requires an enabled deployment module. Autosave waits for the transfer queue before submitting a
snapshot. Command+S or Ctrl+S saves immediately or retries a failed save. The server file must have
a valid mapping and must not match exclusions or symbolic links. Before writing, the transfer reads
the current server content and compares it with the loaded baseline. A conflict or transfer failure
leaves the draft in the tab and stops automatic retries. Further typing preserves the error until an
explicit retry. Local-file autosave never uploads files. Remote writes use the existing streaming
upload transport. An interrupted upload can leave partial server content. Server protocols do not
provide a universal atomic compare-and-swap; concurrent writes during upload cannot be excluded.
Explicit comparison tabs remain immutable snapshots. Open a server file from the remote tree to edit
it.

## JetBrains workflow reference

The existing feature follows the relevant PhpStorm workflows. This comparison covers implemented
file operations, connection forms, profile selection, mappings, and remote browsing.

| Workflow                 | IWE Code behavior                                                                               |
| ------------------------ | ----------------------------------------------------------------------------------------------- |
| Connections and profiles | Shared connections, separate project profiles, left list, right form, three profile tabs        |
| Server selection         | Browsing does not change the default transfer target; missing defaults prompt for a target      |
| Mappings                 | Absolute local display, relative server paths, leading slash compatibility, folder pickers      |
| Connection test          | Authentication works without mappings; results belong to the tested connection                  |
| Remote menu              | Mapped upload, download, local comparison, open local/server file, copy path                    |
| Refresh                  | Reloads the server root and invalidates cached descendant listings                              |
| File commands            | Save pending local edits before upload or comparison; show progress in the bottom activity area |

Intentional differences preserve the requested product behavior. Deployment uses a separate window.
Local and mapped server text files autosave. Comparison tabs remain immutable snapshots. The chosen
shortcuts remain Command+Shift+A and Command+Shift+C on macOS. Overlapping mappings and
symbolic-link transfers remain prohibited. Directory synchronization, server deletion, WebDAV,
rsync, SSH config, and ProxyJump remain outside this feature.

Official references:

- [Deployment settings](https://www.jetbrains.com/help/phpstorm/settings-deployment.html)
- [Connection settings](https://www.jetbrains.com/help/phpstorm/deployment-connection-tab.html)
- [Mappings](https://www.jetbrains.com/help/phpstorm/deployment-mappings-tab.html)
- [Remote Host window](https://www.jetbrains.com/help/phpstorm/remote-host-tool-window.html)
- [Comparison workflow](https://www.jetbrains.com/help/phpstorm/comparing-deployed-files-and-folders-with-their-local-versions.html)
- [File transfers](https://www.jetbrains.com/help/phpstorm/uploading-and-downloading-files.html)
