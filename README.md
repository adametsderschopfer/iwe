# Iwe

<img src="assets/icon.png" alt="Iwe icon" width="128" height="128">

A native workspace for projects, source code, Git history, and pipelines. Built with Rust and Iced.

Create named spaces and add local project folders. Browse files with syntax highlighting and inspect
branches, commits, and historical file versions. Connect GitHub Actions or GitLab pipelines through
browser sign-in. Self-hosted GitLab supports personal access tokens. Open settings with the gear at
the bottom left. Preferences persist between launches. English, Russian, and Spanish translations
are embedded in the executable.

Iwe caches project trees locally and refreshes folder listings in the background. It reads source
contents when files open. Repository files remain unchanged, and source contents stay local. See
[workspace behavior](docs/workspace.md) for limits, authentication, and keyboard shortcuts.

Version tags publish application packages through GitHub Actions after checks and all six builds
pass. Packages cover macOS Apple Silicon and Intel, Windows x86 and x64, and Linux x86 and x64. See
[release packages and publishing](docs/ci.md#release-packages) for installation requirements and the
tag workflow.

## Requirements

- Rust 1.90.0, installed through rustup.
- dprint 0.64.3 for documentation and configuration formatting.
- cargo-audit 0.22.2 for dependency security checks.
- Make for the convenience commands below.
- Git for repository history.
- A browser for GitHub and GitLab.com sign-in.
- An unlocked system credential store for saved accounts.

On macOS, install Xcode Command Line Tools. On Windows, install Visual Studio Build Tools with the
C++ workload. On Linux, install `pkg-config`, `libx11-dev`, `libxkbcommon-dev`, `libwayland-dev`,
and `libudev-dev`.

Linux accounts require a running Secret Service, such as GNOME Keyring or KWallet with Secret
Service support. Iwe does not require GitHub CLI or GitLab CLI. Browser sign-in requires registered
provider applications. See [authentication setup](docs/authentication.md) for public Client IDs and
registration instructions.

Install the dependency checker:

```sh
cargo install cargo-audit --version 0.22.2 --locked
```

## Run

```sh
make run
```

You can also open a directory directly:

```sh
cargo run --locked -- /path/to/project
```

Existing project folders open in their saved space. New folders appear in the space creation form.

The application uses the system language. Set `IWE_LANG` to override it:

```sh
IWE_LANG=en make run
IWE_LANG=ru make run
IWE_LANG=es make run
```

In PowerShell, set `$env:IWE_LANG = "en"` before running `cargo run --locked`. Unsupported languages
fall back to English.

## Build and verify

```sh
make fmt
make check
make build
make release
```

`make check` runs formatting, Clippy, type checks, tests, and dependency security checks. `make
build` creates `target/debug/iwe`. `make release` creates `target/release/iwe`. Windows executable
names include `.exe`.

Without Make, use the Cargo and dprint commands defined in [Makefile](Makefile). See
[the architecture](docs/architecture.md) and [the CI guide](docs/ci.md) for implementation details.

Read [the development rules](AGENTS.md) before changing the project. That guide lists mandatory
rules and additional reading for each task.
