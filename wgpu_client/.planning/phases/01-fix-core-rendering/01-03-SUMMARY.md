---
phase: 01-fix-core-rendering
plan: 03
subsystem: rendering
tags: [debug, overlay, visualization, conditional-compilation]

dependency_graph:
  requires: ["01-01", "01-02"]
  provides: ["debug-overlays", "bounds-visualization"]
  affects: ["developer-experience", "debugging-workflow"]

tech_stack:
  added: []
  patterns:
    - "Conditional compilation with #[cfg(debug_assertions)]"
    - "Zero-overhead debug tooling via compile-time stripping"
    - "Overlay rendering on top of UI content"

key_files:
  created:
    - src/renderer/debug.rs
  modified:
    - src/renderer/mod.rs
    - src/app.rs

decisions:
  - decision: "F12 key for debug overlay toggle"
    context: "Need discoverable, non-conflicting key binding"
    rationale: "F12 is commonly used for developer tools across applications"
  - decision: "Off by default, manual toggle required"
    context: "Debug overlay can be visually noisy"
    rationale: "Developer enables it only when needed for debugging"
  - decision: "DebugRect struct instead of reusing crate::components::Rect"
    context: "Plan specified using Rect from components"
    rationale: "Rect requires different fields (uses Color directly, not through options)"

metrics:
  duration: "8 min"
  completed: "2026-01-28"
---

# Phase 01 Plan 03: Debug Overlays Summary

Debug overlay system with component bounds visualization, toggled via F12, stripped in release builds.

## What Was Built

### Debug Module (src/renderer/debug.rs)
- Color constants for each component type (sidebar=red, tab_bar=green, text_area=blue, gutter=yellow, status_bar=magenta, panel=cyan)
- `DebugRect` struct for overlay rectangle data
- `build_bounds_border()` function that creates 4 border rectangles for any component bounds
- `DebugOverlayConfig` for controlling what overlays to show
- `DebugOverlay` state struct with toggle capability and logging

### Renderer Integration (src/renderer/mod.rs)
- Conditional module declaration: `#[cfg(debug_assertions)] mod debug;`
- `debug_overlay` field on `GpuRenderer` struct (only in debug builds)
- `toggle_debug_overlay()` method (no-op in release builds)
- `build_debug_overlay_rects()` helper that builds StyledRect instances from layout bounds
- New render pass "debug_overlay_pass" rendered after UI text pass (on top of everything)

### Keyboard Shortcut (src/app.rs)
- F12 key toggles debug overlay visibility
- Works from any mode (editor, sidebar, etc.)
- Log message: "Debug overlay ENABLED" / "Debug overlay DISABLED"

## Technical Details

### Conditional Compilation
The entire debug system uses `#[cfg(debug_assertions)]` to ensure:
- Zero runtime overhead in release builds
- No debug code in release binary
- Complete stripping of debug module, fields, and methods

### Overlay Colors (Semi-Transparent)
| Component   | Color                      | RGBA Values       |
|-------------|----------------------------|-------------------|
| Sidebar     | Red                        | (255, 100, 100, 128) |
| Tab Bar     | Green                      | (100, 255, 100, 128) |
| Text Area   | Blue                       | (100, 100, 255, 128) |
| Gutter      | Yellow                     | (255, 255, 100, 128) |
| Status Bar  | Magenta                    | (255, 100, 255, 128) |
| Panel       | Cyan                       | (100, 255, 255, 128) |

### Render Order
Debug overlay renders last (PHASE 3), after:
1. All component backgrounds
2. Text area and gutter text
3. UI text pass

This ensures overlay borders are visible on top of all content.

## Commits

| Hash | Message |
|------|---------|
| 27b5046 | feat(01-03): add debug overlay module |
| 84c78c2 | feat(01-03): integrate debug overlays into renderer |
| 5a9f3c1 | feat(01-03): add F12 keyboard shortcut for debug overlay toggle |

## Deviations from Plan

### Deviation 1: DebugRect instead of crate::components::Rect
**Found during:** Task 1
**Issue:** Plan specified using `crate::components::Rect` but that struct has different fields (expects x, y, width, height, color in constructor)
**Fix:** Created local `DebugRect` struct with the same fields for simplicity
**Rule applied:** Rule 3 - Blocking (couldn't use the specified type directly)

## Files Changed

### Created
- `src/renderer/debug.rs` - Debug overlay module (101 lines)

### Modified
- `src/renderer/mod.rs` - Added debug module, overlay state, toggle method, render pass (+106 lines)
- `src/app.rs` - Added F12 keyboard handler (+12 lines)

## Verification Results

- Debug build: Compiles successfully
- Release build: Compiles successfully (no debug code included)
- Keyboard shortcut: F12 handler in place
- Log messages: Present in toggle() method

## Next Phase Readiness

Phase 1 plan 03 is complete. All debug overlay functionality is in place:
- Press F12 to see colored borders around all UI components
- Useful for verifying layout calculations from Plans 01 and 02
- Zero overhead in release builds

Next: Plan 04 (if any) or Phase 2.
