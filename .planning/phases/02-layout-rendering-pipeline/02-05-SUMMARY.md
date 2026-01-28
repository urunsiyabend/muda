---
phase: 02-layout-rendering-pipeline
plan: 05
subsystem: ui
tags: [div, text, elements, flexbox, glyphon, builder-api, primitives]

# Dependency graph
requires:
  - phase: 02-layout-rendering-pipeline
    plan: 02
    provides: Flexbox layout engine with LayoutContext, compute_flexbox, LayoutId system
  - phase: 02-layout-rendering-pipeline
    plan: 03
    provides: RectangleRenderer with RectInstance::from_style, instanced GPU rendering
  - phase: 02-layout-rendering-pipeline
    plan: 04
    provides: TextSystem with measure_text() and Buffer reuse pattern
provides:
  - Div element with builder API for flexbox layout and visual styling
  - TextElement with glyphon-based measurement and rendering
  - PaintCommand::StyledRect and PaintCommand::Text variants
  - paint_styled_rect() and paint_text() methods on PaintContext
  - measure_text() integration in LayoutContext with TextSystem reference
affects: [all future UI phases, component library, editor views]

# Tech tracking
tech-stack:
  added: []
  patterns: [Builder pattern for element configuration, Buffer reuse between measure and paint phases]

key-files:
  created:
    - ora/src/elements/div.rs
    - ora/src/elements/text.rs
    - ora/src/elements/mod.rs
  modified:
    - ora/src/element/trait_def.rs
    - ora/src/lib.rs

key-decisions:
  - "Div children stored internally and managed by element itself (not passed via AnyElement wrapper)"
  - "TextState stores Buffer from measure_text() in Option, takes it during paint to move into PaintCommand"
  - "PaintCommand::Text carries glyphon::Buffer directly for rendering system coordination"
  - "LayoutContext uses raw pointer to TextSystem for measure_text() integration (unsafe but necessary for borrow checker)"
  - "Fallback text measurement when TextSystem unavailable (0.6 * font_size width estimation)"

patterns-established:
  - "Builder pattern: Div::new().w(100).h(50).bg(color).child(element)"
  - "Three-phase lifecycle: request_layout stores state, prepaint prepares, paint consumes state"
  - "Intrinsic sizing: Text element measures itself and sets intrinsic size for layout engine"
  - "Child registration: Elements call child.request_layout() and cx.add_child() to build layout tree"

# Metrics
duration: 5min
completed: 2026-01-29
---

# Phase 2 Plan 05: Div & Text Primitives Summary

**Div and TextElement primitives with builder APIs for flexbox styling and glyphon text measurement/rendering**

## Performance

- **Duration:** 5 min
- **Started:** 2026-01-28T21:50:14Z
- **Completed:** 2026-01-28T21:54:49Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Div element implements complete flexbox builder API (layout, sizing, spacing, visual properties)
- TextElement implements text rendering with accurate glyphon measurement during layout
- PaintCommand enum extended with StyledRect and Text variants for GPU pipeline
- LayoutContext integrated with TextSystem for text measurement during layout phase
- Builder pattern enables fluent UI construction: `Div::new().flex_col().gap(8).child(...)`

## Task Commits

Each task was committed atomically:

1. **Task 1: Create Div element with style-based rendering** - `000b65a` (feat)
2. **Task 2: Create Text element with glyphon measurement** - `2fdb21d` (feat)

## Files Created/Modified

### Created
- `ora/src/elements/div.rs` - Div element with 30+ builder methods for style properties (224 lines)
- `ora/src/elements/text.rs` - TextElement with glyphon measurement integration (108 lines)
- `ora/src/elements/mod.rs` - Elements module exports (4 lines)

### Modified
- `ora/src/element/trait_def.rs` - Extended PaintCommand enum, added paint_styled_rect/paint_text methods, added measure_text to LayoutContext with TextSystem integration
- `ora/src/lib.rs` - Enabled rendering module, added elements module, re-exported Div and TextElement

## Decisions Made

1. **Div manages children internally** - Children stored in `Vec<AnyElement>` within Div struct. During request_layout, Div iterates children and registers them with LayoutContext. This matches the plan's intended API where `.child()` builder returns modified Div.

2. **TextState stores Buffer in Option** - The Buffer from measure_text() is stored in `Option<glyphon::Buffer>` in TextState. During paint, the Option is taken (leaving None) to move the Buffer into PaintCommand::Text. This ensures the same shaped Buffer is used for rendering.

3. **PaintCommand::Text carries Buffer directly** - Instead of using an index into a separate collection, PaintCommand::Text owns the glyphon::Buffer. This simplifies the rendering pipeline coordination - the rendering system can extract buffers from paint commands when processing them.

4. **LayoutContext uses raw pointer to TextSystem** - Added `text_system: Option<*mut crate::rendering::TextSystem>` field to LayoutContext. The measure_text() method dereferences this pointer in an unsafe block. This is necessary to work around the borrow checker - the TextSystem is owned by OraWindow and needs to be accessible during layout phase. The pointer is valid for the duration of the layout phase.

5. **Fallback text measurement** - When TextSystem is not available, measure_text() falls back to rough estimation (0.6 * font_size for character width). This provides a safe default for testing scenarios, though production should always have TextSystem available.

## Deviations from Plan

None - plan executed exactly as written. All builder methods, Element implementations, and PaintCommand extensions implemented as specified.

## Issues Encountered

None - implementation proceeded smoothly. The glyphon Buffer integration required careful attention to ownership (Option in TextState, moved to PaintCommand), but this was anticipated in the plan's "buffer reuse" requirement.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

**Ready for Plan 02-06 (Rendering Pipeline Integration):**
- Div emits PaintCommand::StyledRect with Style and bounds
- TextElement emits PaintCommand::Text with Buffer, color, and bounds
- PaintContext collects all paint commands for batch processing
- Layout engine computes bounds, elements use bounds for rendering

**Implementation notes for 02-06:**
- Rendering system should process PaintCommand::StyledRect by calling RectInstance::from_style
- Rendering system should process PaintCommand::Text by calling TextSystem::add_text_area
- Single prepare/render pass for all rectangles and text after paint phase

**No blockers** - all must-haves from plan met:
- Div renders styled rectangles using layout-computed bounds ✅
- Text measures itself during layout and renders at correct position ✅
- Builder APIs provide fluent style configuration ✅
- PaintCommand enum extended with StyledRect and Text ✅
- Elements compose (Div can contain Text children) ✅

---
*Phase: 02-layout-rendering-pipeline*
*Completed: 2026-01-29*
