# Subsystem rules

Follow these rules when [AGENTS.md](../../AGENTS.md) directs you here.

Apply each section when working on that subsystem. These rules do not claim that the subsystem
already exists.

## Graph Visualization

Treat graph visualization as an independent subsystem.

Separate:

1. Graph data model.
2. Graph algorithms.
3. Layout and physics simulation.
4. Interaction state.
5. Rendering.
6. Animation.

Graph algorithms must not depend on Iced.

Requirements:

- Support incremental graph updates.
- Avoid unnecessary full layout recalculations.
- Keep rendering independent from graph persistence.
- Support pan, zoom, selection, and drag operations.
- Optimize rendering for large graphs.
- Use caching, spatial indexing, and viewport culling when justified by profiling.
- Keep expensive calculations off the UI thread.
- Write tests for graph algorithms and data transformations.

## Text Editor

Separate the document model from editor interaction and rendering.

Keep independent:

- Document content.
- Cursor and selection state.
- Editing commands.
- Undo/redo history.
- Markdown parsing and formatting.
- Persistence.

Avoid coupling document storage to specific Iced widgets.

Preserve document integrity during asynchronous saves.

Design editor components with keyboard navigation, clipboard operations, Unicode text handling, and
future extensibility in mind.

Do not implement a complex custom editor engine unless existing solutions are insufficient.

## Audio & AI

Separate the following concerns:

- Microphone capture.
- Audio recording state.
- Encoding and file storage.
- Transcription.
- AI processing.
- Knowledge extraction.

Never block the UI thread during recording, encoding, transcription, or AI processing.

Keep external AI providers behind clear interfaces when provider substitution or testing requires
it.

Do not couple the domain model to any specific AI vendor.

Handle errors, cancellation, microphone permissions, and resource cleanup explicitly.
