# Bundled modules

Status: accepted; activation policy updated for the four bundled modules.

## Context

IWE Code needs optional capabilities that users can enable independently. The terminal provides the
first concrete module. System modules must remain installed with the application.

## Decision

Keep a pure typed catalog under `src/modules/catalog/`. Use stable identifiers for bundled modules.
Keep each bundled module under `src/modules/<name>/`. Let the module own its runtime, interface,
input handling, styling, translations, and external adapters.

Preserve shared architectural layers and one application crate. Keep panel visibility and workspace
coordination in `app/modules/`. Keep module settings in `ui/modules.rs`. Modules expose state, typed
events, subscriptions, and views. Modules must not import application state.

Keep terminal execution behind the module's session boundary. Use `portable-pty` for cross-platform
shell sessions and `vt100` for terminal screen parsing. Keep the runtime independent of the rendered
terminal cells.

System modules remain installed, but each module can be disabled. All modules are enabled by
default. Persist disabled identifiers in workspace preferences. Preserve unknown identifiers and all
module configuration. Disabling a module stops subscriptions and invalidates pending results.
Terminal sessions end; deployment transfers receive cancellation. An available module does not start
a shell until the user opens its panel. Keep module translations in `modules/<name>/locales/`. The
shared localizer combines resources without overriding duplicate identifiers.

## Alternatives

A terminal-only panel would avoid a catalog. It would not provide shared module activation or
system-module settings.

Distributing each module across shared layers would preserve one global grouping convention. It
would make a module's implementation harder to locate and review together.

Dynamically loaded native libraries would introduce a binary compatibility contract and process
failure risks. A sandboxed external module runtime would require permissions and a separate
interface protocol. Neither approach has a current installation scenario.

## Consequences

New bundled modules require compilation and explicit registration. Each module keeps its
implementation and unit tests together. Host integration tests remain beside the host integration
code.

An eventual store requires another decision. It must define packaging, trust, permissions, updates,
and compatibility before external modules can execute.
