# iwe

A minimal native desktop application built with Rust and Iced.

The application opens one window with a centered Hello World greeting. It has no storage or sample
data. English, Russian, and Spanish translations are embedded in the executable.

## Requirements

- Rust 1.90.0, installed through rustup.
- dprint 0.64.3 for documentation and configuration formatting.
- cargo-audit 0.22.2 for dependency security checks.
- Make for the convenience commands below.

On macOS, install Xcode Command Line Tools. On Windows, install Visual Studio Build Tools with the
C++ workload. On Linux, install `pkg-config`, `libx11-dev`, `libxkbcommon-dev`, `libwayland-dev`,
and `libudev-dev`.

Install the dependency checker:

```sh
cargo install cargo-audit --version 0.22.2 --locked
```

## Run

```sh
make run
```

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
