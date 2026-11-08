# Rust quality and verification

Follow these rules when [AGENTS.md](../../AGENTS.md) directs you here.

## Required tests

- Add meaningful tests for new functionality and behavior changes.
- Add regression tests for bug fixes.
- Test observable behavior, boundaries, and errors.
- Do not copy implementation logic into tests or limit tests to constant comparisons.
- Keep unit tests in adjacent `tests.rs` files and integration tests under root `tests/`.
- Use `iced_test` for interface tests without a display or GPU.
- Test window settings, native material completion, and opaque fallback behavior when changing
  transparency.
- Preserve assertions and coverage when moving tests.
- Do not disable or weaken tests to pass checks.
- Validate all three i18n catalogs and translated interface layouts when changing application text.

## Required checks

Run these commands before completion, commits, and pushes:

```sh
make check
make build
```

`make check` runs these checks:

1. `cargo fmt --all -- --check`
2. `dprint check`
3. `cargo clippy --locked --all-targets --all-features -- -D warnings`
4. `cargo check --locked --all-targets --all-features`
5. `cargo test --locked --all-targets --all-features`
6. `cargo audit --file Cargo.lock`

Install cargo-audit 0.22.2 with `cargo install cargo-audit --version 0.22.2 --locked`. Run `make
security` to check dependencies separately. This check requires network access to refresh RustSec
advisory data. Investigate vulnerability failures. Do not ignore advisories merely to pass CI.

CI runs checks, tests, and release builds in sequential stages. Keep formatter, linter, type
checker, and dependency security in separate jobs within the checks stage. Require all checks to
succeed before starting test jobs. Keep unit and integration tests in separate jobs within the tests
stage. Require test jobs to succeed before starting release builds. Keep the macOS, Windows, and
Linux matrix for lint, type checks, tests, and builds. Formatting and dependency security use
dedicated Linux jobs. Dependency security checks inspect the complete lockfile without platform
filters. Report missing integration test targets explicitly. Do not present missing tests as passed
tests. Use `needs` for stage dependencies. Do not use `always()` or `continue-on-error` to bypass
failed prerequisites. Add future jobs to the appropriate stage and update downstream dependencies
when required. See [the CI guide](../ci.md) for the implemented job graph and commands.

Run static checks after all edits. Fix failures before committing or pushing. Use `make fmt` to
apply formatting. Run formatting checks afterward. Use `make run` or `cargo run --locked` to start
the application. Do not introduce Node.js or npm for application development. Keep shared Make
commands independent of Unix shell utilities. Keep equivalent Cargo and dprint commands available on
every supported platform. Commit dependency changes with `Cargo.lock`. Pin formatter plugins in
`dprint.json`. Check CI results on every supported platform. Report unavailable checks explicitly.

macOS is the primary development platform. Preserve existing Windows and Linux compatibility. Cargo
forbids unsafe code in this crate.

## Rust Code Quality

Follow idiomatic Rust practices.

- Prefer stable Rust.
- Follow Rust API Guidelines.
- Use descriptive names.
- Prefer small, focused modules and functions.
- Use `Result` and meaningful error types.
- Avoid `unwrap()` and `expect()` in recoverable production paths.
- Avoid unnecessary cloning and allocations.
- Prefer borrowing when appropriate.
- Avoid excessive `Arc<Mutex<T>>` and shared mutable state.
- Use async only when beneficial.
- Do not add unsafe code. Cargo forbids it in this crate.
- Prefer established, actively maintained libraries.
- Minimize unnecessary dependencies.
- Pin dependencies reproducibly and commit `Cargo.lock`.

Do not optimize prematurely. Measure performance before introducing complex optimizations.

## Testing & Verification

Write meaningful automated tests.

Prioritize:

- Domain logic.
- Graph algorithms.
- Document editing operations.
- Data persistence.
- AI response transformations.
- Error handling and recovery.

Use unit tests for isolated logic and integration tests for interactions between components.

Keep unit tests beside the code they verify. Reserve root `tests/` for integration tests.

- Use a module directory with adjacent `mod.rs` and `tests.rs` files when unit tests are needed.
- Declare `#[cfg(test)] mod tests;` in the owning module. Use standard Rust module discovery.
- Do not use `#[path]` to connect unit tests to a distant test directory.
- Keep test functions and inline test module bodies out of production source files.
- Preserve private access through child test modules. Do not expose implementation details solely
  for testing.
- Place integration test entry points in `tests/*.rs`. Test public behavior through the
  application's available interfaces.
- Store shared integration helpers in `tests/common/mod.rs` when multiple tests need them.
- Create root `tests/` when the first integration test exists. Do not add placeholder files.
- Preserve test coverage and assertions when moving tests.

Check significant changes with:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Run checks supported by the current project configuration.

Never claim a test or check passed unless it was actually executed successfully.

Report failed or unavailable checks.
