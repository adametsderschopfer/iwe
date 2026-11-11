# Continuous integration

GitHub Actions uses four named workflows.

- **Проверка кода** runs formatting, lint, type checks, security, and script checks before tests.
  Main pushes and pull requests start it. Manual version preparation also calls it for a pinned
  commit.
- **Выпуск новой версии** prepares a version manually, checks its commit, then creates its tag.
- **Сборка и публикация релиза** builds, packages, signs update metadata, and publishes tag
  releases.
- **Публикация сайта** checks and publishes the tagged website after successful release publication.
  Pull requests run website checks. Manual runs support website recovery. Main pushes do not deploy
  the website.

Checks cover macOS, Windows, and Linux. Unit tests and release builds cover six architecture
targets. Integration discovery runs once per operating system. Formatting and dependency security
run on Linux.

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

1. Collect reviewed English, Russian, and Spanish items in the unreleased release-note record.
2. Open **Выпуск новой версии** in GitHub Actions and select main.
3. Choose **Исправления — patch**, **Новые возможности — minor**, or **Крупное обновление — major**.
4. Run the workflow. It updates Cargo.toml, Cargo.lock, and the release-note ledger.
5. The workflow pushes a version commit and checks that exact commit through the reusable CI
   workflow.
6. Successful checks permit tag creation and explicitly start the release workflow.
7. Verify publication before recording published website metadata.

For version 1.0.7, patch produces 1.0.8, minor produces 1.1.0, and major produces 2.0.0. Empty
unreleased notes stop preparation. Existing version records also stop preparation. Version workflows
serialize their work. A concurrent main push causes a normal push conflict. Branch protections must
permit the workflow bot to push version commits. The workflow never force-pushes. Configure
repository permissions before using this workflow.

The built-in token cannot trigger push workflows through its own commits or tags. The version
workflow calls CI directly and dispatches the release workflow explicitly. The release workflow
calls the website workflow after publishing all packages. Website checkout uses the same tag as the
application release. A website failure leaves the published application release available. Retry the
failed website jobs. It requires contents write permission for version commits and actions write
permission for dispatch. No additional personal access token is required.

Failed checks leave the version commit on main without creating its tag. Retry failed jobs after
transient errors. A full rerun reuses its original version commit. The run identifier in the commit
message links reruns to that commit. If code needs correction, push the correction and run a new
version request with reviewed notes. Existing tags must point to the checked commit. The workflow
never moves an existing tag.

Release accepts stable vMAJOR.MINOR.PATCH tags and manual runs against such tags. Manual runs
support recovery after publication failures. A manually pushed tag also starts release directly, so
its author must select a checked commit. The release workflow validates tag syntax, release notes,
and package version before publication. It does not repeat lint, type checks, dependency audits, or
unit tests. Installer installation checks remain part of Windows packaging. All six builds and both
Windows installer jobs must succeed before publication.

The final job signs one update manifest and publishes one GitHub Release with all eight installation
packages and six update archives. It verifies the required files and adds SHA256SUMS. Configure
`IWE_UPDATE_SIGNING_KEY` before publishing. See [application updates](updates.md#release-signing)
for key handling. The version workflow and publication job can write to the repository. Build jobs
have read-only access. Existing releases are not overwritten. If uploading fails, inspect the
remaining draft before retrying publication. New pushes leave tag workflows running and cancel
superseded branch workflows.

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
