---
phase: 05-element-library
plan: 02
subsystem: ui
tags: [stack, z-layering, absolute-positioning, flexbox, containers]

# Dependency graph
requires:
  - phase: 02-layout-rendering-pipeline
    provides: Flexbox layout system with Position::Absolute support
  - phase: 01-foundation-view-system
    provides: Element trait with three-phase lifecycle
provides:
  - Stack container for z-axis layering with paint-order stacking
  - AbsoluteWrapper pattern for positioning children at same origin
  - stack() constructor for fluent API
affects: [05-element-library, 06-builder-api, user-facing-ui]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "AbsoluteWrapper pattern for internal absolute positioning"
    - "Stack leverages Position::Absolute from existing layout system"

key-files:
  created:
    - ora/src/elements/stack.rs
  modified:
    - ora/src/elements/mod.rs
    - ora/src/lib.rs

key-decisions:
  - "Stack wraps each child in AbsoluteWrapper with position:absolute, top:0, left:0"
  - "Paint order determines z-index (first child = bottom, last child = top)"
  - "Leverages existing Position::Absolute support in flexbox layout system"

patterns-established:
  - "Internal wrapper elements for layout behavior modification"
  - "Absolute positioning at (0,0) for overlapping children"

# Metrics
duration: 5min
completed: 2026-01-29
---

# Phase 05 Plan 02: Stack Container Summary

**Stack container with z-axis layering via absolute positioning at same origin, paint-order stacking for overlapping UI elements**

## Performance

- **Duration:** 5 min
- **Started:** 2026-01-29T20:02:00Z
- **Completed:** 2026-01-29T20:06:58Z
- **Tasks:** 3
- **Files modified:** 3

## Accomplishments
- Stack container positions all children at the same origin using Position::Absolute
- Children paint in order (first = bottom, last = top) creating z-layering effect
- Leverages existing layout system without modifications
- Fluent API with stack(), child(), w(), h(), bg() methods

## Task Commits

Each task was committed atomically:

1. **Task 1: Create Stack element with paint-order z-layering** - `b6e1cde` (feat)
2. **Task 2: Export Stack from elements module and lib.rs** - `0042a91` (feat)
3. **Task 3: Verify Stack works with layout system** - `ed1d518` (docs)

## Files Created/Modified
- `ora/src/elements/stack.rs` - Stack container with AbsoluteWrapper for overlapping children
- `ora/src/elements/mod.rs` - Export stack module and constructor
- `ora/src/lib.rs` - Re-export Stack and stack() from ora crate root

## Decisions Made

**1. AbsoluteWrapper internal element pattern**
- Each child wrapped in element with `position: absolute`, `top: 0`, `left: 0`
- Enables overlapping without modifying children's original styles
- Clean separation: Stack handles positioning, children handle their own rendering

**2. Leverage existing Position::Absolute support**
- Flexbox layout system already implements absolute positioning (flexbox.rs:layout_absolute_child)
- Stack doesn't need custom layout logic, just wraps children appropriately
- All children positioned at same offset create overlapping effect

**3. Paint order determines z-index**
- First child painted first (bottom layer), last child painted last (top layer)
- Natural ordering without explicit z-index values
- Matches CSS stacking context behavior

## Deviations from Plan

None - plan executed exactly as written.

The plan anticipated needing to investigate layout system integration. Investigation confirmed that Position::Absolute was already fully supported, enabling clean implementation without modifications to the layout system.

## Issues Encountered

None.

The existing Position::Absolute support in the flexbox layout system (implemented in Phase 02) worked perfectly for Stack's needs. The `layout_absolute_child` function handles positioning at specified offsets, which Stack leverages by setting all children to offset (0, 0).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Stack container ready for use in element library and builder API phases. Enables:
- Modal dialogs with backdrop overlays
- Tooltips positioned over content
- Layered UI components (background/foreground separation)
- Badge/notification overlays on buttons

No blockers for subsequent plans in Phase 5.

---
*Phase: 05-element-library*
*Completed: 2026-01-29*
