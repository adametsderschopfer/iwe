# Continuous integration

GitHub Actions uses three workflows with English names.

- **Code Checks** runs application and website checks on main pushes. Application checks include
  formatting, lint, type checks, dependency security, script checks, and tests. Website checks
  include formatting, lint, type checks, dependency security, generation, and browser tests.
- **Release New Version** manually prepares a version, creates its tag, builds packages, and
  publishes the release. Its final jobs build and publish the website from the same prepared commit.

- **Build Website** manually generates the website from main and uploads a downloadable artifact. It
  does not publish GitHub Pages or create an application version.

Checks cover macOS, Windows, and Linux. Unit tests and release builds cover six architecture
targets. Integration discovery runs once per operating system. Formatting and dependency security
run on Linux.

The application has no root integration test targets. CI reports their absence explicitly. Adjacent
tests cover local Git repositories, persistence, previews, provider responses, translated layouts,
graphics, and native material state.

Run the same checks locally with `make check` and `make build`. Run `make fmt` to format Rust,
Markdown, JSON, and YAML files. Security checks use the complete lockfile and require network access
to refresh RustSec advisories.

GitHub Actions caches speed up repeated runs. Rust jobs restore the crate registry and compiled
dependencies. They disable incremental compilation because every run starts from cached artifacts.
The security job caches the cargo-audit binary. Website jobs cache npm packages and Playwright
browsers. Release builds set platform link flags before the Rust setup step. This keeps each cache
tied to one build configuration.

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

1. Collect reviewed English, Russian, and Spanish items in the unreleased release-note record.
2. Open **Release New Version** in GitHub Actions and select main.
3. Choose **Fixes — patch**, **New features — minor**, or **Major update — major**.
4. Run the workflow. It updates Cargo.toml, Cargo.lock, and the release-note ledger.
5. The workflow pushes the version commit and creates its tag.
6. Build jobs start immediately after tag creation in the same workflow.
7. Verify publication before recording published website metadata.

For version 1.0.7, patch produces 1.0.8, minor produces 1.1.0, and major produces 2.0.0. Empty
unreleased notes stop preparation. Existing version records also stop preparation. Version workflows
serialize their work. A concurrent main push causes a normal push conflict. Branch protections must
permit the workflow bot to push version commits. The workflow never force-pushes. Configure
repository permissions before using this workflow.

Code Checks runs only for main pushes. The version workflow uses needs dependencies for tagging,
builds, publication, and website deployment. No dispatch requests or additional personal access
tokens are required. Only version preparation, tagging, and release publication receive contents
write permission. Website deployment receives pages and id-token write permissions. The workflow
checks out the prepared commit explicitly because the initiating event references main. The built-in
token pushes the version commit without triggering another Code Checks run. Release does not wait
for or inspect earlier Code Checks results.

Retry failed release jobs after transient errors. A full rerun reuses its original version commit.
The run identifier in the commit message links reruns to that commit. If code needs correction, push
the correction and run a new version request with reviewed notes. Existing tags must point to the
prepared commit. The workflow never moves an existing tag.

Start releases through Release New Version on main. Choose patch, minor, or major. Tag pushes do not
start separate workflows. Retry failed jobs within the original run after transient build or
deployment errors. Website failures leave the published application release available. Release build
jobs validate tag syntax, release notes, and package version. They build the prepared commit.
Installer installation checks remain part of Windows packaging. All six builds and both Windows
installer jobs must succeed before publication.

The publication job signs one update manifest and publishes one GitHub Release with all eight
installation packages and six update archives. It verifies the required files and adds SHA256SUMS.
Configure `IWE_UPDATE_SIGNING_KEY` before publishing. See
[application updates](updates.md#release-signing) for key handling. The version workflow and
publication job can write to the repository. Build jobs have read-only access. Existing releases are
not overwritten. If uploading fails, inspect the remaining draft before retrying publication. Main
pushes cancel superseded branch checks. Manual version runs serialize their work.

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
