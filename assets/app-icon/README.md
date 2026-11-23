# PhotoCraft app icon

The owner's 2026-11-23 raster design: a **dark slate rounded square carrying a red letter
pair**, echoing the classic desktop-image-editor tile (the active lettering red is sampled at
`#c32e47`, which the activation gate's typed code also draws in).

## Palette

| Colour | Hex (sampled) | Used for |
|---|---|---|
| Slate tile | `#3d4450`–`#2a2f38` | the rounded-square field |
| Letter red | `#c32e47` | the lettering |

## Geometry

A 512 px tile with transparent corners (the rounding lives in the pixels, not a mask). The
macOS render scales it onto Apple's 1024 grid; the Windows and Linux renders use the whole
tile, as the other Crafting Apps do.

## Provenance

The owner's own 2026-11-23 raster design (`himaps-photoshop.png`, 512 px). No vector master:
the scalable slots carry the raster in an SVG wrapper. License: see `LICENSE.txt`.

## Files

- `hicolor/<size>/apps/ai.storyteller.photocraft.png` (16–512): the Linux icon theme, and the
  runtime window icon (`apps/photocraft/src/app_icon.rs`) and the activation gate's mark
  (`crates/ui-egui/src/activation_ui.rs`) read the 256 px and 128 px renders.
- `photocraft.ico`: Windows icon, 16–256 px, embedded in the `.exe` by `apps/photocraft/build.rs`.
- `photocraft-1024.png`: 1024 px render on the macOS grid.
- `photocraft.icns`: macOS bundle icon (`CFBundleIconFile`) — **still the previous design**;
  no macOS release is cut from this fork and this machine has no `iconutil` to repack it.
- `photocraft.svg`, `photocraft-small.svg`, `hicolor/scalable/...svg`: SVG wrappers embedding
  the 512 px raster (the design has no vector master).

The app also sets the window icon and Wayland app ID at runtime (`apps/photocraft/src/app_icon.rs`,
`main.rs`). On Windows the taskbar shows the icon of the Start Menu shortcut that launches the
exe, so the MSI's `<Icon Id>` keeps the `.exe` extension (ICE50); `app_icon.rs` documents the
details and tests the `.ico` sizes and the WiX icon references.

## Regenerate

From a new 512 px raster master: downscale the hicolor PNGs (LANCZOS), pack `photocraft.ico`
with the 16–256 frames, scale the 1024 macOS render, and write the SVG wrappers that embed the
master (this was done with Pillow, 2026-11-23). `packaging/icons.sh` renders from SVG masters
and needs `resvg`/`iconutil`; with the raster master it no longer applies.
