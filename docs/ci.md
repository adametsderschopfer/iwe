# Continuous integration

GitHub Actions runs these sequential stages:

1. Formatting, Clippy, type checks, and dependency security checks.
2. Unit tests and integration test discovery.
3. Release builds for six platform and architecture combinations.
4. GitHub Release publication for version tags.

Clippy, type checks, tests, and builds run on macOS, Windows, and Linux. Unit tests and release
builds use the same six-target matrix. Integration discovery runs once per operating system.
Formatting and dependency security checks run on Linux. Each stage requires the preceding stage to
succeed.

The application has no root integration test targets. CI reports their absence explicitly. Adjacent
tests cover local Git repositories, persistence, previews, provider responses, translated layouts,
graphics, and native material state.

Run the same checks locally with `make check` and `make build`. Run `make fmt` to format Rust,
Markdown, JSON, and YAML files. Security checks use the complete lockfile and require network access
to refresh RustSec advisories.

## Release packages

| Platform | Architecture         | Rust target              | Runner         | Package                |
| -------- | -------------------- | ------------------------ | -------------- | ---------------------- |
| macOS    | Apple Silicon, ARM64 | aarch64-apple-darwin     | macos-15       | DMG containing Iwe.app |
| macOS    | Intel, 64-bit        | x86_64-apple-darwin      | macos-15-intel | DMG containing Iwe.app |
| Windows  | x64, 64-bit          | x86_64-pc-windows-msvc   | windows-2022   | ZIP containing iwe.exe |
| Windows  | x86, 32-bit          | i686-pc-windows-msvc     | windows-2022   | ZIP containing iwe.exe |
| Linux    | x64, 64-bit          | x86_64-unknown-linux-gnu | ubuntu-22.04   | tar.gz containing iwe  |
| Linux    | x86, 32-bit          | i686-unknown-linux-gnu   | ubuntu-22.04   | tar.gz containing iwe  |

Windows x86 builds use the MSVC target on a 64-bit runner. Windows release binaries link the C
runtime statically. Linux x86 builds install multilib compilers and i386 development libraries.
Tests execute the selected architecture's binary, including both 32-bit targets.

macOS packages target macOS 11 or newer. Each DMG includes an Applications shortcut. Linux packages
target Ubuntu 22.04 or compatible systems with glibc 2.35 or newer. Linux users need a graphical
session, a supported graphics driver, and Secret Service for saved credentials. 32-bit Linux
binaries require matching 32-bit runtime libraries and graphics drivers. Windows packages require
Windows 10 or newer and a supported graphics driver. Git remains an external dependency on every
platform.

Every package uses the Iwe display name and generated application icon. macOS bundles include an
ICNS resource. Windows executables embed an ICO resource during compilation. Linux archives include
`iwe.desktop` and `iwe.png`. Install the executable on PATH, the desktop entry under
`~/.local/share/applications`, and the icon under `~/.local/share/icons`.

macOS bundles use ad-hoc signatures. Apple notarization and Windows certificate signing are not
configured. Operating systems can require user confirmation when opening downloaded packages.

## Publish a version

1. Update the package version in Cargo.toml and refresh Cargo.lock.
2. Commit the release changes after local checks pass.
3. Create a matching version tag, such as 0.0.1 or v0.0.1.
4. Push that tag to GitHub.

The push starts CI for the tagged commit. Every check, test job, and target build must pass before
publication. Tag versions must match Cargo.toml. Mismatches fail before packaging and publication.
Tags containing prerelease versions, such as v0.2.0-rc.1, create prereleases. Branch pushes and pull
requests build all targets without publishing releases. Manual workflow runs on tags create packages
without publishing them.

The final job downloads all six packages and verifies that each package exists. It adds SHA256SUMS
and creates a release with generated release notes. Only that job receives contents: write
permission. Build jobs retain read-only repository access. Existing releases are not overwritten. If
uploading fails, inspect the remaining draft before retrying publication. Tag workflows are not
canceled by newer pushes. Branch workflows still cancel superseded runs.

Set repository variables IWE_GITHUB_CLIENT_ID and IWE_GITLAB_CLIENT_ID to embed public OAuth Client
IDs in release builds. These values are public identifiers, not secrets. Missing values leave
browser sign-in unconfigured. See [authentication setup](authentication.md) for provider
registration.

Local workflow validation uses actionlint and ShellCheck. Packaging checks do not publish releases
or create remote tags. Actual Windows and Linux builds require their GitHub runners or equivalent
local environments.
