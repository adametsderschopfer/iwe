# Application icon

The icon was generated with the built-in image generation tool. The original transparent PNG remains
in `icon.png`. macOS ImageIO and `iconutil` produced the platform formats without changing the
artwork.

- `icon.icns`: macOS application bundle.
- `icon.ico`: Windows executable resource.
- `window-icon.png`: embedded Windows and X11 window icon.
- `iwe.desktop`: Linux desktop entry. Install `iwe.png` under the user's icon directory.

The `icon.iconset` directory contains the source resolutions for `iconutil`.

## Generation prompt

```text
Use case: logo-brand. Asset type: a single production application icon for Iwe, a native desktop workspace for projects, files, Git history and CI jobs. Create an original, highly legible geometric mark suggesting neatly joined workspace panels and a subtle abstract W. Premium restrained developer-tool aesthetic, front-facing flat composition with very subtle depth, no perspective mockup. Palette belongs to a charcoal desktop interface with muted ice-blue accent. A dark graphite rounded-square macOS-style tile, crisp pale-blue and off-white mark centered inside, generous balanced inner margin, very slight soft edge lighting. The tile occupies about 84 percent of a square 1024 by 1024 canvas; the area outside the rounded tile must be truly transparent. The mark must stay recognizable at 16px and 32px, with broad clean shapes, no tiny details. No words, no letters outside the abstract symbol, no watermark, no other icons, no grid of variants, no background scene, no green ChatGPT knot, no JetBrains branding.
```
