# Development rules for iwe

Build a native Rust workspace with Iced.

## Essential requirements

- Maintain macOS, Windows, and Linux support.
- Organize shared code by layers and bundled capabilities under `src/modules/`. Follow
  [architecture rules](docs/rules/architecture.md).
- Preserve the graphics priorities in [platform rules](docs/rules/platforms.md).
- Use native Liquid Glass when available. Follow the gray fallback policy in
  [platform rules](docs/rules/platforms.md).
- Keep unit tests in adjacent `tests.rs` files. Reserve root `tests/` for integration tests.
- Add meaningful tests for behavior changes and regression tests for bug fixes.
- Run `make check` and `make build` after all edits and before completion, commits, or pushes.
- Fix failed checks. Do not weaken tests or skip checks.
- Investigate security advisories. Do not ignore advisories merely to pass checks.
- Verify interface changes with a local application run. Report unavailable platform checks
  explicitly.
- Use Make, Cargo, and dprint. Do not introduce Node.js or npm.
- Keep documentation current in the same change.
- Collect release-note items on commit requests. Prepare version notes before creating release tags.
- Keep technical details out of the interface. Follow
  [user-facing content rules](docs/rules/user-facing-content.md).
- Store every application text in i18n catalogs. Maintain English, Russian, and Spanish translations
  together.
- Write documentation and comments in English first. Use Russian as a secondary language when
  useful.
- Apply the five writing rules in [documentation rules](docs/rules/documentation.md) to prose.
- Do not add unsafe code.

## Read before working

The files below contain mandatory project rules. Open the required files explicitly before starting
relevant work. Do not treat links as loaded instructions. Read each applicable file once, then
reread it when its contents change. All rule files apply from the repository root. Read paths inside
them as repository-relative unless a link specifies otherwise.

Always read:

- [Development workflow](docs/rules/workflow.md)
- [Rust quality, test organization, and checks](docs/rules/quality.md)
- [Supported platforms and graphics priorities](docs/rules/platforms.md)
- [Documentation and writing](docs/rules/documentation.md)
- [User-facing content](docs/rules/user-facing-content.md)

Read additional rules for the task:

| Task                                                               | Required rules                                     |
| ------------------------------------------------------------------ | -------------------------------------------------- |
| Prepare a commit, version, release, or website release notes       | [Release notes](docs/rules/release-notes.md)       |
| Change code, modules, dependencies, or architecture                | [Architecture](docs/rules/architecture.md)         |
| Change Iced state, messages, views, styling, or animation          | [Iced and interface design](docs/rules/iced-ui.md) |
| Add or change application text or language behavior                | [Internationalization](docs/rules/i18n.md)         |
| Work on graph visualization, editing, audio, or AI                 | [Subsystem rules](docs/rules/subsystems.md)        |
| Change entities, storage, queries, search indexes, or file formats | [Entities and data access](docs/rules/data.md)     |

Read [the current architecture](docs/architecture.md) and affected feature documents before
implementation. Subsystem rules guide development within shared layers and module boundaries. Do not
create empty modules.

## Maintain the rules

Keep this file short. Store detailed requirements in `docs/rules/`. Update this reading guide when
adding, moving, or removing a rule file. Keep each detailed rule in one authoritative file. Keep
summaries consistent with that file.
