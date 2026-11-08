# Continuous integration

GitHub Actions runs these sequential stages:

1. Formatting, Clippy, type checks, and dependency security checks.
2. Unit tests and integration test discovery.
3. Release builds.

Clippy, type checks, tests, and builds run on macOS, Windows, and Linux. Formatting and dependency
security checks run on Linux. Each stage requires the preceding stage to succeed.

The template has no integration test targets. CI reports their absence explicitly. Unit tests cover
the greeting layout, localization, graphics fallback, and native material state.

Run the same checks locally with `make check` and `make build`. Run `make fmt` to format Rust,
Markdown, JSON, and YAML files. Security checks use the complete lockfile and require network access
to refresh RustSec advisories.
