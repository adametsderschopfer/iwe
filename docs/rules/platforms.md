# Supported platforms

Follow these rules when [AGENTS.md](../../AGENTS.md) directs you here.

## Required platform behavior

- Maintain macOS, Windows, and Linux support.
- Prioritize Metal, Vulkan, then OpenGL on macOS.
- Prioritize Vulkan, DirectX 12, then OpenGL on Windows.
- Prioritize Vulkan, then OpenGL on Linux.
- Retry only after graphics initialization fails.
- Use cross-platform APIs. Isolate platform-specific code with `cfg`.
- Keep the three-platform CI matrix.
- Check interface changes with a local application run.
- Report visual checks separately from compilation checks.
- Use native Liquid Glass for the window on supported macOS versions.
- Show an opaque neutral gray background when Liquid Glass is unavailable or initialization fails.
- Keep the Iced background transparent only after native material initialization succeeds.
- Keep native material below the renderer. Never blur application content with the background
  material.
- Preserve native window controls and system window corners.
- Do not simulate window glass with inset panels, gradients, outlines, shadows, or custom corner
  radii.

## Cross-Platform Development

Develop for macOS first without unnecessarily coupling the application to macOS.

- Prefer cross-platform APIs.
- Isolate platform-specific implementations.
- Avoid hardcoded paths and platform assumptions.
- Handle filesystem locations using appropriate platform APIs.
- Consider platform-specific keyboard shortcuts.
- Verify Windows and Linux compatibility as development progresses.
- Keep native integrations isolated behind clear boundaries.
