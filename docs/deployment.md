# Deployment

The Deployment system module transfers project files through SFTP, FTP, or FTPS. It provides manual
transfers and remote comparisons. It does not upload files automatically.

## Connections and profiles

Open **Settings → Deployment → Connections** to manage connections. **Add connection** opens a
separate dialog. Connections are available across all spaces. On macOS, the **Deployment** menu
opens Connections, SSH keys, or Profiles. Open **Settings → Deployment → Shared SSH keys** to manage
key files. **Add key** opens a separate dialog with a name and file picker. You can also add a key
from a connection dialog without losing its fields. Saving that key selects it for the connection.
Cancel returns without adding the key. Enter the server, port, user, and authentication method. Use
**Test connection** before saving. SFTP supports passwords, private key files, and the system SSH
agent. The shared key list stores names and file paths. IWE Code does not copy private key files.
Entering a password replaces the saved password or key passphrase. Leaving the field untouched
preserves the existing secret. Passwords use Keychain, Windows Credential Manager, or Linux Secret
Service. Workspace metadata never stores passwords or private key contents.

An unknown SSH server requires fingerprint confirmation before authentication. A changed server key
blocks the connection. Verify the server independently before replacing its saved connection. FTPS
verifies certificates and server names. FTPS supports explicit TLS and implicit TLS. FTP uses
unencrypted connections. FTP and FTPS use passive data connections and binary transfers.

Open **Settings → Deployment → Profiles** for the selected space. The space context menu also opens
deployment settings. Each profile belongs to one space and targets one project. Several profiles can
target the same project. Select the connection, server root, and folder mappings. The folder browser
can select the server root. Choose one profile as the project's default. Selecting a profile in the
right panel also updates that default.

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
rows. Double-click a remote file to open a read-only snapshot. Refresh reloads the root directory
listing. Right-click local files or folders to upload them. Right-click remote files or folders to
download them through their mappings. Right-click a local or remote file to compare both copies.
Comparison never changes the local file. The comparison displays the server snapshot on the left and
the local snapshot on the right.

| Action                          | macOS default        | Windows and Linux default |
| ------------------------------- | -------------------- | ------------------------- |
| Compare active file with server | Command + Option + C | Ctrl + Alt + C            |
| Upload active file              | Command + Option + V | Ctrl + Alt + V            |

Deployment commands appear in **Settings → Keyboard shortcuts**. Physical keys also support
non-Latin keyboard layouts. Settings windows and modal dialogs do not execute deployment shortcuts.
Focused terminal input retains its keyboard events. The default deployment combinations do not run
from the terminal.

Upload reads the latest file from disk and replaces the remote file without another confirmation.
The panel and completion notification identify the selected profile. Folder transfers preserve
relative paths and create missing folders. Downloads require confirmation before replacing existing
local files. Each download uses a temporary file beside its destination. The module replaces the
destination only after successful transfer and a final local-change check.

Operations execute sequentially. The panel reports transferred bytes and completed files. Cancel
stops subsequent transfer work. An active network call can continue until its timeout. Files already
uploaded can remain changed after cancellation or failure. Uploads write directly to the
destination; failed uploads can leave partial remote files. Retry requires an explicit action.
Switching projects cancels the previous project's operations. Results from obsolete requests cannot
replace the current project state.

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
