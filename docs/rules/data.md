# Entities and data access

Follow these rules when [AGENTS.md](../../AGENTS.md) directs you here. Read
[workspace behavior](../workspace.md) for current persistence and repository limits.

Iwe reads project files and Git history without modifying repositories. It stores workspace metadata
and derived directory caches separately. Store credentials in the operating-system credential store.
Never store credentials in workspace metadata.

Apply operation and verification rules to current storage changes. Apply vault-specific rules only
when adding an application-managed Markdown vault. Existing project directories are not managed
vaults. Do not convert project files or workspace metadata to Markdown.

## Replaceable data access

Keep existing orchestration in `app` until concrete scenarios require a separate `application`
layer. Apply application-layer contracts when introducing that layer.

- Define operation contracts in `application`, beside their consumers.
- Select and inject concrete adapters in `main` or `app` only.
- Keep business validation and transformations in `domain` and `application`.
- Keep paths, serialization, filesystem access, and future HTTP conversion in infrastructure.
- Never expose file handles, YAML values, HTTP DTOs, or transport status codes to views.
- Keep application and domain code independent of Iced, serializers, filesystem APIs, and
  translation catalogs.
- Add operation-specific methods with their first scenario. Avoid generic CRUD repositories and
  unused remote implementations.
- Return typed entities, bounded results, and application-owned error categories.
- Preserve identifiers, conditional writes, ordering, and operation semantics when replacing
  adapters.
- Express related writes as one repository operation. Future backends must provide equivalent server
  operations.
- Treat offline synchronization separately. Define conflict and authority rules before combining
  sources.
- Test application scenarios with injected repositories and adapters with isolated temporary
  storage.

Separate startup preparation contracts from interactive repository operations. Serialize related
interactive writes through an ordered background worker. See
[the current architecture](../architecture.md) for existing storage and background workers.

## Managed Markdown files and identity

- Use Markdown as the sole persistent source for records and graph data.
- Keep record bodies below YAML front matter. Keep graph membership and positions in separate
  Markdown documents.
- Assign stable, opaque IDs before writing. Never derive identity from titles, paths, or
  coordinates.
- Preserve IDs during renaming, moving, importing, and exporting.
- Preserve user text, Unicode, whitespace, and line endings unless an explicit operation changes
  them.
- Preserve unknown metadata values. Avoid rewriting unchanged documents.
- Validate metadata, unique IDs, positive revisions, timestamps, and finite coordinates before
  committing.
- Store UTC milliseconds. Preserve creation times and prevent update times from moving backward.
- Allow empty drafts. Reject ambiguous nonempty titles during application renames.
- Preserve record content when changing membership or positions.
- Treat search results, layout caches, and UI state as derived data.
- Persist automatic component separation through repository operations, never through rendering
  callbacks.
- Preserve component geometry, record content, and membership when correcting overlapping group
  regions.
- Increment graph revisions only for changed layouts. Settled workspace loads must not rewrite
  files.
- Commit relationship changes and resulting layout corrections together.
- Return every corrected position after a node drop so displayed and stored coordinates agree.
- Introduce attachments, deletion, and trash through explicit operations with documented
  preservation rules.

## Managed Markdown relationships

- Derive wikilink targets from current bodies and unique titles.
- Preserve unresolved text. Never create placeholder records implicitly.
- Preserve unchanged link IDs and creation times. Keep explicit kinds separate from derived
  wikilinks.
- Retain source-to-target order. Never merge reverse edges into one undirected relationship.
- Allow self-links unless a domain requirement explicitly forbids them.
- Commit incoming reference rewrites with application renames. Increment each changed source
  revision once.
- Keep links global and positions specific to each graph.
- Do not promise incoming rewrites for external title edits.

## Reads, writes, and conflicts

Apply save, revision, locking, and journal requirements when implementing mutable managed documents.
Keep current source previews read-only.

- Keep disk access outside UI callbacks and rendering. Keep active graph data in memory during
  interactions.
- Bound sizes, counts, nesting, and graph results. Reject oversized writes before changing stored
  content.
- Compare expected revisions and opaque content tokens for record edits and graph mutations.
- Detect external edits without requiring external revision updates.
- Preserve drafts on failures. Never silently overwrite newer content.
- Debounce automatic saves and serialize submitted snapshots. Do not block editing while a snapshot
  commits.
- Apply completion metadata to the saved baseline. Never replace newer draft text with an older save
  result.
- Preserve caret position and undo history during ordinary saves.
- Flush the latest draft before navigation or normal window closure. Keep failed drafts available
  for explicit recovery.
- Stop automatic retries after failures. Resume through an explicit retry instead of repeatedly
  issuing conflicting writes.

- Serialize application writers through the vault lock and ordered worker. Bound lock waits.
- Use atomic file replacement and a recoverable journal for related changes.
- Report success only after the complete operation commits.
- Recover interrupted operations before reading workspace data.
- Validate all rollback targets before restoration. Preserve unexpected edits and the unresolved
  journal.
- Reject path traversal and symbolic links within managed directories.
- Never replace invalid or unsupported documents with empty data.
- Document external-editor race windows. Advisory locking does not isolate arbitrary external
  writers.
- Measure opening, searching, saving, and memory usage before introducing caches or indexes.

## Format changes, backups, and verification

Apply vault-format and whole-vault backup rules when a managed vault exists.

- Version the vault format explicitly. Reject unsupported versions without changing user documents.
- Preserve data during upgrades. Verify conversions before switching the active store.
- Close Iwe before whole-vault backups, restores, or external merges.
- Include hidden graph metadata and pending journals in backups.
- Keep GitHub synchronization separate. Define merge, conflict, and deletion rules before
  implementing synchronization.
- Keep technical diagnostics outside the interface. Exclude record bodies and private search terms
  from logs.
- Test reopening, conflicts, metadata, independent positions, recovery, limits, and invalid-format
  preservation.
- Keep unit tests in adjacent `tests.rs` files and integration tests in root `tests/`.
- Never run automated tests against the user's data directory.
- Update feature documentation and architectural decisions with storage changes.
- Run `make check` and `make build` after all edits.
