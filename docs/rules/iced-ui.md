# Iced and interface design

Follow these rules when [AGENTS.md](../../AGENTS.md) directs you here.

## Iced Architecture

Follow **The Elm Architecture** consistently.

Use the following concepts:

- **State:** Represents the current state of the UI.
- **Message:** Describes user actions and application events.
- **Update:** Processes messages and updates state.
- **View:** Declaratively describes the interface.
- **Task:** Handles asynchronous operations.
- **Subscription:** Handles ongoing event streams and subscriptions.

Rules:

1. Keep views declarative and free from business logic.
2. Do not perform blocking I/O inside UI update or rendering paths.
3. Use typed messages for state transitions.
4. Keep visual state near screens and components. Coordinate application-wide messages in `app`.
5. Delegate reusable operations to `application` when that layer exists. Keep data invariants and
   algorithms in `domain`.
6. Use `Task` for asynchronous operations and `Subscription` for ongoing event streams.
7. Do not store persistent domain data exclusively in UI state.
8. Keep UI-specific state separate from domain data.
9. Avoid unnecessary global application state.
10. Avoid rebuilding or recalculating expensive data when localized updates are sufficient.
11. Obtain every display text from i18n. Follow [the translation rules](i18n.md).
12. Keep technical details out of widgets. Follow
    [user-facing content rules](user-facing-content.md).

Use the Iced API idiomatically rather than forcing patterns from React, Redux, or other UI
frameworks into the project.

## UI, Design System & Animations

Build a consistent, maintainable design system.

### Application palette

Preserve Iwe's neutral dark appearance. Follow
[the window and graphics guide](../architecture.md#window-and-graphics) for current behavior.

| Role       | Color     | Intended use                           |
| ---------- | --------- | -------------------------------------- |
| Background | `#212121` | Workspace, headers, and settings       |
| Sidebar    | `#181818` | Navigation surfaces                    |
| Elevated   | `#2f2f2f` | Raised badges and controls             |
| Text       | `#ececec` | Primary readable text                  |
| Secondary  | `#b4b4b4` | Supporting text and secondary elements |
| Accent     | `#8bb4eb` | Blue interface accents                 |

Centralize colors in `ui/theme`. Do not duplicate hexadecimal values across screens. Do not derive
entity meaning from color alone. Keep foregrounds readable on each surface. Preserve native Liquid
Glass and the platform's opaque gray fallback. Native window material and opaque content surfaces
have separate responsibilities. Keep workspace surfaces opaque above the native material.

Centralize:

- Colors and semantic color tokens.
- Typography.
- Spacing.
- Border radii.
- Shadows and effects.
- Animation durations and easing.
- Reusable components.
- Light and dark themes.

Avoid hardcoded styling values scattered across features.

Prefer reusable, composable UI components.

Keep graph style presets in `ui/theme` when presets exist. Pass the selected style from workspace
state to the renderer. Add future presets through the same typed boundary. Keep style parameters
separate from stored relationships and coordinates.

For animations:

- Separate animation state from persistent domain state.
- Use time-based animation logic rather than frame-count assumptions.
- Avoid unnecessary redraws.
- Support smooth transitions, hover effects, and animated state changes.
- Profile performance-sensitive effects.
- Provide reduced-motion behavior where practical.

Use Iced's native rendering capabilities, custom widgets, and Canvas where appropriate.

Introduce custom GPU rendering only when necessary.
