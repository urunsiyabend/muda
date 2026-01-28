---
phase: 01-fix-core-rendering
plan: 01
subsystem: ui
tags: [wgpu, glyphon, text-rendering, gpu, batch-rendering]

# Dependency graph
requires: []
provides:
  - Single-batch UI text rendering via collect_all_ui_texts()
  - Bounds-validated text rendering preventing GPU crashes
  - Increased text capacity (512 blocks) for larger UIs
affects: [02-fix-layout, 03-improve-ux, ui-components]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Single prepare() per TextRenderer per frame"
    - "Collect-then-render pattern for batched GPU operations"
    - "Bounds clamping for GPU-safe rendering"

key-files:
  created: []
  modified:
    - src/renderer/mod.rs
    - src/components/ui_text_renderer.rs

key-decisions:
  - "Single text render pass at end of frame rather than per-component"
  - "512 text block capacity based on UI component inventory"
  - "Bounds clamping to screen dimensions for GPU safety"

patterns-established:
  - "collect_all_ui_texts(): Gather all UI text before GPU operations"
  - "Validate bounds against screen dimensions before rendering"

# Metrics
duration: 8min
completed: 2026-01-28
---

# Phase 01 Plan 01: Fix Disappearing Text Summary

**Consolidated glyphon text rendering into single prepare() call per frame, fixing sidebar/tab/status text disappearance**

## Performance

- **Duration:** 8 min
- **Started:** 2026-01-28T11:28:41Z
- **Completed:** 2026-01-28T11:36:51Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Fixed root cause of disappearing text: multiple prepare() calls overwriting glyph buffer
- Added collect_all_ui_texts() method to batch all UI component text
- Single prepare() call now handles sidebar, tabs, status line, panel, and command palette
- Increased text block capacity from 256 to 512 for larger UI batches
- Added bounds validation to prevent GPU crashes from out-of-bounds rendering
- Added capacity warning when text count approaches limit

## Task Commits

Each task was committed atomically:

1. **Task 1: Refactor renderer to collect all texts before rendering** - `93e544d` (feat)
2. **Task 2: Update UITextRenderer for larger batches** - `171ee46` (feat)

## Files Created/Modified
- `src/renderer/mod.rs` - Added collect_all_ui_texts(), single prepare() call, reorganized render phases
- `src/components/ui_text_renderer.rs` - Increased capacity, added bounds validation and warnings

## Decisions Made
- Render all UI text in single pass at end of frame (after all backgrounds)
- Use 512 text block capacity based on component inventory (~50 sidebar + ~20 tabs + ~5 status + ~20 panel + ~15 palette + margin)
- Clamp text bounds to screen dimensions to prevent GPU validation errors

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None - the fix directly addressed the identified root cause from RESEARCH.md.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Text rendering now stable and visible across all UI components
- Ready for layout/clipping fixes in subsequent plans
- No blockers identified

---
*Phase: 01-fix-core-rendering*
*Completed: 2026-01-28*
