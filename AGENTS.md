# Development rules for iwe

Build a native Rust application with Iced. Maintain macOS, Windows, and Linux support.

- Read [the architecture](docs/architecture.md) before changing source dependencies.
- Keep window composition in `src/app`, screens in `src/ui/screens`, and styles in `src/ui/theme`.
- Keep operating-system access in `src/infrastructure` and translation logic in `src/i18n`.
- Add application and domain layers only when behavior requires them. Do not create empty modules.
- Keep views free of blocking operations and infrastructure dependencies.
- Store application text in English, Russian, and Spanish Fluent catalogs. Update all catalogs
  together.
- Keep technical diagnostics outside the interface.
- Preserve native window controls, Liquid Glass, and the opaque gray fallback.
- Preserve the platform graphics priorities documented in [the architecture](docs/architecture.md).
- Keep unit tests in adjacent `tests.rs` files. Add root `tests/` only for integration tests.
- Test changed behavior, catalog completeness, translated layouts, and failure paths when relevant.
- Run `make check` and `make build` after all edits, before completion, commits, or pushes.
- Investigate failed checks. Do not weaken tests or ignore security advisories to pass checks.
- Verify interface changes with a local application run. Report unavailable platform checks
  explicitly.
- Use Make, Cargo, and dprint. Do not introduce Node.js or npm for application development.
- Do not add unsafe code.
- Update affected documentation with the implementation.
- Write documentation and comments primarily in English.
- Use active voice, one idea per sentence, consistent terms, and plain language.
- Limit prose sentences to 20 words.

See [the CI guide](docs/ci.md) for verification stages and supported platforms.
