# Application updates

IWE Code checks GitHub Releases at startup and every six hours. Checks never use repository accounts
or tokens. It selects stable releases for the current operating system, with shared releases as a
compatibility fallback. Only versions newer than the running version qualify. Failed checks stop until
another scheduled or explicit check. Open **Settings → About** to check manually.

A download row with a dark blue icon appears near the sidebar settings button when an update becomes
available. Choose **Download update** to download the package for the current operating system and
processor. Choose **Install and restart** after the download finishes. IWE Code saves workspace
settings before exiting its native event loop. A failed save keeps the window open and stops
installation. Installation never requests administrator access. The installation directory must be
writable by the current user.

The first installation of version 0.1.10 still requires a downloaded release package. That version
can receive future updates. Older versions do not acquire an updater automatically. Development
binaries cannot install updates. macOS development runs outside an application bundle also cannot
install updates.

## Package verification

The fixed update source is
[the IWE Code release repository](https://github.com/adametsderschopfer/iwe-code/releases). The
updater accepts HTTPS redirects only to GitHub API, release, and asset hosts. It never forwards
provider credentials. Metadata reads have size limits and a 45-second combined deadline. Downloads
have a five-minute deadline and a 512 MiB maximum.

Each platform release includes `updates.json`, its Ed25519 signature `updates.sig`, and two update
archives for that operating system. Shared migration releases contain six archives. The embedded
public key verifies the manifest before version or package data becomes
trusted. The signed manifest identifies each package's version, platform, byte size, and SHA-256
digest. The updater verifies package bytes before extraction and repeats verification immediately
before installation. Archives may contain only regular files and directories beneath the expected
application root. Extraction rejects links, path traversal, duplicate paths, excess entries, and
oversized expanded content.

## Installation and recovery

Staging directories live beside the installed application, keeping replacements on one filesystem. A
copied installer confirms that it accepted the handoff before the application exits. Failed helper
startup keeps the application open and reports an installation error. The installer normalizes
Windows paths before validation. It waits for the application's file lock after shutdown.
Replacement retries temporary file locks for ten seconds while the graphics launcher exits. The
installer replaces the complete macOS application bundle, including its signature and icon. Windows
and Linux updates replace the standalone executable. Workspace data and credentials remain separate.

The installer moves the current application into its staging directory before moving the new
application into place. A failed replacement restores the previous application. A failed process
launch also restores it and relaunches the previous application. Successful process creation does
not prove that the new application will remain running. The restarted application removes its
staging directory. A power failure can leave a staging directory with the previous application. No
background cleanup deletes unrelated application directories.

## Release signing

Platform tags use `macos-vVERSION`, `windows-vVERSION`, or `linux-vVERSION`. Their numeric version
must match Cargo.toml. The updater reads at most three pages of 100 GitHub releases and selects the
highest eligible version. Equal versions prefer a platform release over a shared release. Drafts,
prereleases, unrelated tags, and other operating systems do not qualify. List responses are bounded
to 4 MiB per page and share the existing 45-second metadata deadline.

Older updater versions only understand the shared Latest release and six-archive manifests.
Publish one shared migration release containing this updater before relying on platform-only tags.
Platform releases do not replace that shared Latest release. Older installations can receive the
migration release, then discover subsequent platform releases. Existing published releases remain unchanged.

The publication job signs the manifest only after its selected platform packages pass their CI
prerequisites. Shared release tags still require every platform. Only the publication step receives
the repository secret `IWE_UPDATE_SIGNING_KEY`.
That secret contains an Ed25519 private key in PEM format. Keep its backup outside the repository.
`assets/update-public-key.bin` contains the matching 32-byte public key. The signing script rejects
a private key that does not match the embedded public key. Missing or invalid signing configuration
prevents publication. Unsigned updates are never accepted.

The signing script requires Python 3.11 or newer and OpenSSL with Ed25519 support. Run
`.github/scripts/sign-updates.py` with `RELEASE_TAG` and `IWE_UPDATE_SIGNING_KEY` after generating
the selected platform's two archives, or all six archives for a shared tag. Never print the private
key or place it in command arguments.

## Verification

Local tests verify version selection, signatures, digest mismatches, redirect restrictions, metadata
bounds, extraction limits, and replacement recovery. Interface tests cover translated update actions
and duplicate requests. They never replace the running application or contact GitHub. Platform CI
compiles and tests all six targets. A live update between signed releases still requires installed
application testing.
