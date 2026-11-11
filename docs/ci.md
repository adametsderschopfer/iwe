# Continuous integration

GitHub Actions runs these sequential stages:

1. Formatting, Clippy, type checks, and dependency security checks.
2. Unit tests and integration test discovery.
3. Release builds for six platform and architecture combinations.
4. Windows installer packaging and installation checks for version tags.
5. GitHub Release publication for version tags.

Clippy, type checks, tests, and builds run on macOS, Windows, and Linux. Unit tests and release
builds use the same six-target matrix. Pushes to `main` run checks and tests. They skip the release
build, so a simultaneous version tag push builds the release once. Integration discovery runs once
per operating system. Formatting and dependency security checks run on Linux. Each stage requires
the preceding stage to succeed.

The application has no root integration test targets. CI reports their absence explicitly. Adjacent
tests cover local Git repositories, persistence, previews, provider responses, translated layouts,
graphics, and native material state.

Run the same checks locally with `make check` and `make build`. Run `make fmt` to format Rust,
Markdown, JSON, and YAML files. Security checks use the complete lockfile and require network access
to refresh RustSec advisories.

## Release packages

| Platform | Architecture         | Rust target              | Runner         | Package                     |
| -------- | -------------------- | ------------------------ | -------------- | --------------------------- |
| macOS    | Apple Silicon, ARM64 | aarch64-apple-darwin     | macos-15       | DMG containing IWE Code.app |
| macOS    | Intel, 64-bit        | x86_64-apple-darwin      | macos-15-intel | DMG containing IWE Code.app |
| Windows  | x64, 64-bit          | x86_64-pc-windows-msvc   | windows-2022   | Setup EXE and portable ZIP  |
| Windows  | x86, 32-bit          | i686-pc-windows-msvc     | windows-2022   | Setup EXE and portable ZIP  |
| Linux    | x64, 64-bit          | x86_64-unknown-linux-gnu | ubuntu-22.04   | tar.gz containing iwe       |
| Linux    | x86, 32-bit          | i686-unknown-linux-gnu   | ubuntu-22.04   | tar.gz containing iwe       |

Windows x86 builds use the MSVC target on a 64-bit runner. Windows release binaries link the C
runtime statically. Linux x86 builds install multilib compilers and i386 development libraries.
Tests execute the selected architecture's binary, including both 32-bit targets. Linked-worktree
fixtures pass relative targets to Git. This avoids Windows verbatim-path prefixes during worktree
creation.

macOS packages target macOS 11 or newer. Each DMG includes an Applications shortcut. Linux packages
target Ubuntu 22.04 or compatible systems with glibc 2.35 or newer. Linux users need a graphical
session and a supported graphics driver. 32-bit Linux binaries require matching 32-bit runtime
libraries and graphics drivers. Windows packages require Windows 10 or newer and a supported
graphics driver. Git remains an external dependency on every platform.

Every package uses the IWE Code display name and generated application icon. macOS bundles include
an ICNS resource. Windows executables embed an ICO resource during compilation. Linux archives
include `iwe.desktop` and `iwe.png`. Install the executable on PATH, the desktop entry under
`~/.local/share/applications`, and the icon under `~/.local/share/icons`.

macOS bundles use ad-hoc signatures. Apple notarization is not configured. Windows installers use
Inno Setup 7.1.1 with modern light and dark pages. They support English, Russian, and Spanish.
Installation uses the current user's Programs directory without administrator access. The installer
creates a Start menu shortcut and offers a desktop shortcut. Uninstalling preserves workspace data.
CI checks silent installation, registration, and removal on both Windows architectures.

Windows packages remain unsigned by default. Optional Microsoft Artifact Signing covers the
application, installer, and uninstaller. See [Windows signing setup](windows-signing.md). Operating
systems can require confirmation when opening downloaded packages.

## Publish a version

1. Update the package version in Cargo.toml and refresh Cargo.lock.
2. Prepare its authored release notes following [release-note rules](rules/release-notes.md).
3. Commit the release changes and notes after the authorized checks pass.
4. Create and push one stable tag in the form `vMAJOR.MINOR.PATCH`.
5. Verify publication, then update the website's publication metadata through the authorized commit
   workflow.

Planning validates the release-note ledger. Version tags require a matching prepared or released
version with authored items. Missing or incomplete notes stop release jobs before builds begin. The
publication script renders the English GitHub release body and links to all three translations.

The tag push starts CI for the tagged commit. CI requires checks, tests, and builds for all six
targets. Both Windows installer jobs must also succeed. The tag version must match Cargo.toml.
Failed or cancelled prerequisites prevent publication. Pull requests, pushes to other branches, and
manual runs build all targets. Manual runs on release tags also create packages. Only tag pushes
publish releases.

The final job signs one update manifest and publishes one GitHub Release with all eight installation
packages and six update archives. It verifies the required files and adds SHA256SUMS. Configure
`IWE_UPDATE_SIGNING_KEY` before publishing. See [application updates](updates.md#release-signing)
for key handling. Only the publication job can write to the repository. Build jobs have read-only
access. Existing releases are not overwritten. If uploading fails, inspect the remaining draft
before retrying publication. New pushes leave tag workflows running and cancel superseded branch
workflows.

Set repository variables IWE_GITHUB_CLIENT_ID and IWE_GITLAB_CLIENT_ID to embed public OAuth Client
IDs in release builds. These values are public identifiers, not secrets. Missing values leave
browser sign-in unconfigured. See [authentication setup](authentication.md) for provider
registration.

Linux builds and website browser tests share an APT setup script. If setup fails, the script
replaces the configured Azure Ubuntu mirror with the official Ubuntu archive and retries. It
preserves the Ubuntu release, security repositories, and signature checks.

Local workflow validation uses actionlint and ShellCheck. Packaging checks do not publish releases
or create remote tags. Actual Windows and Linux builds require their GitHub runners or equivalent
local environments.

## Naming compatibility

Public downloads use the `iwe-code-` prefix. Executables retain `iwe` or `iwe.exe`. The macOS bundle
identifier and existing workspace paths remain unchanged. Update archives retain the `iwe-` prefix
and their previous payload roots, including `IWE.app`. Existing updater versions require those
names. The GitHub repository redirect preserves their previous update URLs.
