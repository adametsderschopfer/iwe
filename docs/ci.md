# Continuous integration

GitHub Actions runs these sequential stages:

1. Formatting, Clippy, type checks, and dependency security checks.
2. Unit tests and integration test discovery.
3. Release builds for six platform and architecture combinations.
4. Windows installer packaging and installation checks for version tags.
5. GitHub Release publication for version tags.

Clippy, type checks, tests, and builds run on macOS, Windows, and Linux. Unit tests and release
builds use the same six-target matrix for ordinary branches and shared release tags. Platform
release tags scope lint, type checks, tests, and builds to the selected operating system and both
architectures. Integration discovery runs once per selected operating system. Formatting and
dependency security checks run on Linux. Each stage requires the preceding stage to succeed.

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
session, a supported graphics driver, and Secret Service for saved credentials. 32-bit Linux
binaries require matching 32-bit runtime libraries and graphics drivers. Windows packages require
Windows 10 or newer and a supported graphics driver. Git remains an external dependency on every
platform.

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
4. Create platform tags, such as `macos-v0.1.12`, `windows-v0.1.12`, and `linux-v0.1.12`.
5. Push the required platform tags to GitHub. Each tag publishes its own release independently.
6. Verify publication, then update the website's publication metadata through the authorized commit
   workflow.

Planning validates the release-note ledger. Version tags require a matching prepared or released
version with authored items. Missing or incomplete notes stop release jobs before builds begin. The
publication script renders the English GitHub release body from the website ledger and links to all
three translations.

The push starts CI for the tagged commit. Each platform tag requires that platform's checks, tests,
and two architecture builds before publication. Formatting and dependency security remain mandatory.
Windows additionally requires both installer packaging jobs. macOS and Linux do not wait for Windows
packaging. Failed or cancelled prerequisites prevent publication. Tag versions must match
Cargo.toml. Only stable version tags qualify. Branch pushes and pull requests build all targets
without publishing releases. Manual workflow runs on tags create packages without publishing them.

All three platform tags use the same application version. A platform can publish later without
blocking the others. Reuse the same commit for all three tags when releasing identical source.
Shared tags such as `v0.1.12` remain supported for migration releases covering all six targets.

The final job downloads the selected platform's installation packages and two update archives.
Shared tags still require eight installation packages and six update archives. The job verifies each
required package, signs the update manifest, adds SHA256SUMS, and creates a separate release.
Platform releases include their platform in the title and do not replace GitHub's shared Latest
release. Configure `IWE_UPDATE_SIGNING_KEY` before publishing. See
[application updates](updates.md#release-signing) for key handling. Only that job receives contents:
write permission. Build jobs retain read-only repository access. Existing releases are not
overwritten. If uploading fails, inspect the remaining draft before retrying publication. Tag
workflows are not canceled by newer pushes. Branch workflows still cancel superseded runs.

Set repository variables IWE_GITHUB_CLIENT_ID and IWE_GITLAB_CLIENT_ID to embed public OAuth Client
IDs in release builds. These values are public identifiers, not secrets. Missing values leave
browser sign-in unconfigured. See [authentication setup](authentication.md) for provider
registration.

Local workflow validation uses actionlint and ShellCheck. Packaging checks do not publish releases
or create remote tags. Actual Windows and Linux builds require their GitHub runners or equivalent
local environments.

## Naming compatibility

Public downloads use the `iwe-code-` prefix. Executables retain `iwe` or `iwe.exe`. The macOS bundle
identifier and existing workspace paths remain unchanged. Update archives retain the `iwe-` prefix
and their previous payload roots, including `IWE.app`. Existing updater versions require those
names. The GitHub repository redirect preserves their previous update URLs.
