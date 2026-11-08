# Architecture

The application contains one greeting screen and no persistent state.

| Module                          | Responsibility                                                                 |
| ------------------------------- | ------------------------------------------------------------------------------ |
| `main.rs`                       | Select the language and start the graphics launcher                            |
| `app/`                          | Configure the window, own interface state, and process native material results |
| `ui/screens/hello_world.rs`     | Center the translated greeting                                                 |
| `ui/theme/`                     | Define greeting size and the native material fallback                          |
| `i18n/`                         | Format embedded Fluent translations and recover from missing messages          |
| `infrastructure/locale.rs`      | Read `IWE_LANG` and the system locale                                          |
| `infrastructure/graphics/`      | Start graphics backends in platform priority order                             |
| `infrastructure/window_effect/` | Place native Liquid Glass below the renderer on supported macOS versions       |

Composition connects the interface, localization, and infrastructure. Interface modules never import
infrastructure. Translation logic performs no operating-system access. Add application and domain
layers when concrete features require them.

## Window and graphics

The window opens at 800 × 600 pixels. Its minimum size is 400 × 280 pixels. It preserves native
window controls and closes through the standard close button. The background stays opaque gray until
native Liquid Glass initialization succeeds. Unsupported systems keep the opaque fallback.

The launcher tries these backends in order:

| Platform | Backend priority           |
| -------- | -------------------------- |
| macOS    | Metal, Vulkan, OpenGL      |
| Windows  | Vulkan, DirectX 12, OpenGL |
| Linux    | Vulkan, OpenGL             |

Each attempt uses a separate process. Only graphics initialization failures trigger another attempt.
The macOS fallback paths require external Vulkan/MoltenVK or ANGLE runtimes. The application does
not bundle those runtimes. Normal macOS startup uses Metal.

## Localization

The executable embeds English, Russian, and Spanish catalogs from `locales/`. `IWE_LANG` overrides
the system locale. Unsupported languages select English. Missing selected messages fall back to
English, then to localized fallback text. The interface never displays translation keys or technical
diagnostics.

Unit tests verify catalog completeness, language selection, translated layouts, native material
state, and graphics fallback behavior. Headless tests cannot verify native material appearance.
Check appearance with a local application run.
