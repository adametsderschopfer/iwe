# Architecture

Follow these rules when [AGENTS.md](../../AGENTS.md) directs you here.

## Technology Stack

- **Language:** Rust (stable, latest compatible edition).
- **UI Framework:** Iced.
- **Rendering:** Iced's native rendering stack, with GPU acceleration where appropriate.
- **Primary Platform:** macOS.
- **Additional Platforms:** Windows and Linux.
- **Architecture:** Layered architecture in one Rust crate, with The Elm Architecture for the
  interface.
- **Code Organization:** Layers with explicit responsibilities and dependency boundaries.

Do not introduce Electron, Tauri, HTML, CSS, JavaScript UI frameworks, Qt, or custom C++ code.

Prefer Rust-native solutions and cross-platform libraries.

Use the official Iced documentation and examples compatible with the project's pinned Iced version.
Never assume that APIs from older examples are still valid.

## Architecture Principles

Follow these principles:

1. **Separation of Concerns** — UI, application logic, domain logic, and infrastructure must have
   clearly defined responsibilities.
2. **Single Responsibility Principle** — modules and components should have focused
   responsibilities.
3. **Dependency Inversion** — domain logic must remain independent of UI frameworks, databases, and
   external services.
4. **High Cohesion, Low Coupling** — related functionality belongs together; unrelated modules
   should not depend on each other's implementation details.
5. **Composition over Inheritance** — use Rust composition and traits where they provide real value.
6. **Explicit Dependencies** — avoid global mutable state, hidden dependencies, and unnecessary
   singletons.
7. **KISS, DRY, YAGNI** — prefer simple solutions, avoid meaningful duplication, and do not
   introduce abstractions for hypothetical requirements.
8. **Type Safety** — use Rust's type system to make invalid states difficult or impossible to
   represent.
9. **Testability** — design business logic so that it can be tested independently from the UI.

Avoid overengineering.

Do not introduce architectural complexity without a concrete reason.

## Project Structure

Organize source code by responsibility at the top level. Use the following layers as implementation
needs arise:

| Layer             | Responsibility                                                                             |
| ----------------- | ------------------------------------------------------------------------------------------ |
| `app/`            | Compose dependencies, configure windows, own application UI state, and route messages      |
| `ui/screens/`     | Describe screens and screen-specific presentation behavior                                 |
| `ui/components/`  | Provide reusable interface components when needed                                          |
| `ui/theme/`       | Define presentation styles and design values                                               |
| `application/`    | Coordinate user operations when reusable application scenarios require a separate layer    |
| `domain/`         | Define workspace entities, repository data, graph algorithms, and business rules           |
| `infrastructure/` | Implement storage, operating-system access, graphics startup, audio, and external services |
| `i18n/`           | Resolve supported languages and format translated messages                                 |

Keep `main.rs` small. Use `main` and `app` to connect concrete implementations. Keep translation
catalogs under root `locales/`. Keep unit tests in adjacent `tests.rs` files. Reserve root `tests/`
for integration tests. Follow [test organization rules](quality.md#testing--verification) for module
declarations and private access.

The current application uses `app`, `domain`, `ui`, `infrastructure`, and `i18n`. `app` coordinates
existing operations. `domain` owns workspace entities and repository data. Add `application` when
concrete scenarios require it. Keep its contracts and operations independent of presentation. Add
`ui/components` when multiple screens need reusable components. Do not create empty directories,
placeholder types, or one file for every trivial method.

Do not restore top-level `features` or miscellaneous `shared` directories. Group related modules
inside their responsible layer when that improves navigation. Keep feature guides under `docs/`.
Documentation topics do not define source architecture.

## Dependency Rules

Apply application-layer rules when concrete scenarios require that layer. Read
[the current architecture](../architecture.md) for existing composition boundaries.

- `domain` must not depend on `app`, `ui`, `application`, `infrastructure`, or `i18n`.
- `application` may use `domain` and contracts required by its scenarios.
- Keep `application` independent of Iced, presentation state, localization, and concrete external
  services.
- Define external-operation contracts near the application scenarios that need them.
- Route replaceable data operations through application-owned contracts. Inject storage and API
  adapters at composition boundaries.
- Follow [data access rules](data.md#replaceable-data-access) for repository semantics, errors, and
  replacement tests.
- `infrastructure` may implement those contracts and use domain types.
- Infrastructure modules must not import `app` or `ui`.
- Startup diagnostics may use `i18n`. Keep those messages outside the interface.
- `ui` may use application results, read-only domain data, and `i18n`.
- UI modules must not import concrete infrastructure or the application composition layer.
- Pass presentation messages or callbacks explicitly when the application needs screen events.
- `i18n` must not import application layers or perform operating-system or file access.
- `main` and `app` may connect the layers through explicit dependencies.
- Keep public APIs small. Avoid circular dependencies.
- Introduce additional Cargo crates only when concrete dependency boundaries justify them.

These rules describe source dependencies. Runtime calls can reach infrastructure through injected
contracts. Do not introduce interfaces for types that have no external boundary or substitution
need.

## Logic Placement

Keep visual interaction state near its screen or component. Examples include selected tabs, open
dialogs, focus, and temporary input values. Keep application-wide UI state and message routing in
`app`.

Keep current operation coordination in `app`. Introduce `application` when reusable scenarios
require it, then place those scenarios there. Put data invariants and algorithms in `domain`. Put
actual disk, database, microphone, and network access in `infrastructure`.

For example, an Iced message can request a save operation. The application update function starts a
task that calls the save scenario. The scenario applies domain rules and uses an injected storage
implementation. A completion message updates UI state. The interface chooses a localized,
nontechnical result message. Never block the interface while waiting for an external operation.

## Domain & Business Logic

Keep domain logic independent of Iced.

Domain modules must not depend on:

- UI widgets or rendering code.
- Iced application state.
- Concrete database implementations.
- Filesystem APIs.
- External AI SDK implementations.

Use application services or use cases to coordinate business operations.

Use traits at real infrastructure boundaries where testing or multiple implementations require them.

Do not create repository interfaces, service layers, or dependency injection abstractions
automatically for every type.

## Core Engineering Philosophy

**Build for long-term maintainability without sacrificing development speed.**

The application should be:

- Modular.
- Performant.
- Testable.
- Cross-platform.
- Extensible.
- Visually polished.
- Easy to understand.
- Easy to refactor.

Favor clean architecture over clever abstractions.

Favor measurable performance over premature optimization.

Favor simplicity over unnecessary complexity.

**Every architectural decision should make the application easier to develop, understand, test, or
maintain.**
