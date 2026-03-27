---
phase: 10-foundation-fixes
plan: "03"
subsystem: ui
tags: [rust, wgpu, selection, rendering, text-area, theme, stack-layout]

# Dependency graph
requires:
  - phase: 10-02
    provides: EditorDataSource trait split (BufferDataSource + CommandDispatcher + WindowDataSource)

provides:
  - LinePresentation.selection_ranges field populated from core_editor span data
  - render_selection_bg_layer() renders selection highlights from selection_ranges
  - render_current_line_bg_layer() promotes current-line bg to own Stack layer
  - Stack z-order: base_bg -> current_line -> selection -> text -> caret
  - Brighter selection color (#264F80) replacing dim blue_900 (#0D3870)
  - Text span filter removed: all spans render, selected text no longer disappears

affects:
  - Phase 13 (syntax-in-selection): full syntax preservation in selected regions

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Layered Stack rendering: each visual concern (bg, current-line, selection, text, caret) is its own Stack child"
    - "Selection data flows via selection_ranges (column offsets) separate from text styling"
    - "Adapter extracts selection ranges from TextStyle::Selection spans at conversion boundary"

key-files:
  created: []
  modified:
    - ora/src/editor_adapter/types.rs
    - wgpu_client/src/adapter.rs
    - ora/src/views/text_area.rs
    - ora/src/theme/mod.rs

key-decisions:
  - "selection_ranges carries (start_col, end_col) character offsets; extracted in adapter, not in text_area"
  - "Removed TextStyle::Selection filter from render_line() — all spans render as text"
  - "Current-line bg promoted from per-line-div property to dedicated Stack layer to avoid blocking selection"
  - "Selection color updated to Color::rgb(0.15, 0.31, 0.50) inline in match arm (no new palette entry)"

patterns-established:
  - "Stack layer ordering for editor text area: base_bg, current_line_bg, selection_bg, text, caret"
  - "Background highlights belong in their own Stack layers, never on text container divs"

# Metrics
duration: 3min
completed: 2026-03-26
---

# Phase 10 Plan 03: Selection Rendering Fix Summary

**Selection backgrounds now render as a dedicated Stack layer using column-offset ranges extracted from core_editor spans, with text span filtering removed so selected text is visible and Zed-level contrast color applied.**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-03-25T23:30:09Z
- **Completed:** 2026-03-25T23:33:06Z
- **Tasks:** 2/2
- **Files modified:** 4

## Accomplishments

- `LinePresentation` gains `selection_ranges: Vec<(usize, usize)>` field; adapter populates it by scanning TextStyle::Selection spans at conversion time
- Removed `.filter(|span| span.style != TextStyle::Selection)` from `render_line()` — selected text now renders instead of disappearing entirely
- New `render_selection_bg_layer()` uses `selection_ranges` (spacer + rect per range) instead of scanning spans for style
- New `render_current_line_bg_layer()` renders a flex-col of per-line rows; current line gets `CurrentLineBg`, others transparent — avoids opaque bg blocking selection layer below
- Stack z-order restructured: `base_bg -> current_line -> selection -> text (transparent) -> caret`
- Dark theme selection color updated from `blue_900` (#0D3870) to `Color::rgb(0.15, 0.31, 0.50)` (~#264F80)

## Task Commits

Each task was committed atomically:

1. **Task 1: Add selection_ranges to LinePresentation and populate from core_editor** - `e6a59c0` (feat)
2. **Task 2: Render selection backgrounds and fix text layer filtering** - `61ca273` (feat)

**Plan metadata:** (this summary commit)

## Files Created/Modified

- `ora/src/editor_adapter/types.rs` - Added `selection_ranges` field, `with_selection()` builder; updated constructors
- `wgpu_client/src/adapter.rs` - `convert_line_presentation()` extracts selection ranges from Selection-styled spans
- `ora/src/views/text_area.rs` - New bg layers, removed span filter, transparent text layer, 5-layer Stack
- `ora/src/theme/mod.rs` - Dark Selection token updated to brighter #264F80

## Decisions Made

- **selection_ranges at adapter boundary:** Extraction happens in `convert_line_presentation()` in the adapter, not in text_area. This keeps ora free of knowledge about how core_editor encodes selection state.
- **Inline color value:** Dark Selection color set as `Color::rgb(0.15, 0.31, 0.50)` directly in the match arm rather than adding a new palette function. Simple and sufficient for Phase 10.
- **Per-line row approach for bg layers:** `render_current_line_bg_layer()` and `render_selection_bg_layer()` each produce a flex-col of line-height rows. Transparent rows for non-highlighted lines ensure base_bg shows through without gaps.
- **Advisor insight applied:** Current-line bg was originally applied per-line-div (would have blocked selection layer). Promoted to its own Stack layer below selection per advisor recommendation.

## Deviations from Plan

### Auto-fixed Issues

None — plan executed exactly as written, plus one advisor-recommended structural improvement applied before writing:

The plan's wording about "removing `bg(BgPrimary)` from text_layer" implied keeping current-line bg on individual line divs. The advisor correctly identified this would block the selection layer. Applied the full promotion of current-line bg to a dedicated Stack layer, which is architecturally cleaner and correct.

This was an anticipatory improvement, not a deviation from an executed step.

---

**Total deviations:** 0

## Issues Encountered

None. Both tasks compiled and all 160 tests passed on first attempt.

## Next Phase Readiness

- Selection rendering is structurally correct; visual verification requires running the editor with `cargo run -p wgpu_client` and using Shift+arrow keys
- Phase 13 can extend this by having core_editor provide both selection state and syntax style per span, enabling full syntax-highlighting preservation within selected text
- The `selection_ranges` field and the layered Stack pattern are the foundation for that work

---
*Phase: 10-foundation-fixes*
*Completed: 2026-03-26*
