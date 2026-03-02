---
phase: 08-advanced-ui-widgets
plan: 03
subsystem: ui
tags: [rust, gpui, element, checkbox, toggle, boolean-control, theme-tokens, colortoken]

# Dependency graph
requires:
  - phase: 05-element-library
    provides: Element trait pattern (request_layout/prepaint/paint three-phase lifecycle)
  - phase: 06-design-system
    provides: ColorToken system (Accent, Border, BgSecondary, BgElevated, FgPrimary, FgMuted)
provides:
  - Checkbox boolean control widget with checked/unchecked/disabled/hover/focus states
  - Toggle pill-shaped switch with on/off states and knob position animation
  - CheckboxSize enum (Sm/Md/Lg) for sizing consistency
  - Local constructor functions: checkbox() and toggle()
affects:
  - 08-advanced-ui-widgets (other plans may use checkbox/toggle in demos)
  - 08-08 (cleanup: unify CheckboxSize with WidgetSize from 08-01)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Color extraction before mutable paint calls: extract all theme.color() calls into local Color values before calling cx.paint_styled_rect() to avoid borrow checker conflicts"
    - "Always-paint pattern for child TextElements: call paint() even when invisible (transparent color) to consume the glyphon Buffer and avoid resource leaks"
    - "Disabled state via alpha: multiply all Color.a by 0.5 for consistent 50% dimming"
    - "Knob positioning math: knob_x computed from bounds + inset for off, bounds + width - knob_size - inset for on"

key-files:
  created:
    - ora/src/elements/checkbox.rs
  modified:
    - ora/src/elements/mod.rs
    - ora/src/lib.rs

key-decisions:
  - "CheckboxSize local enum (not WidgetSize) for wave isolation — Plan 08-08 will unify"
  - "CheckboxSize::Sm=14px, Md=18px, Lg=22px — smaller than button heights, checkbox-specific box dimensions"
  - "Toggle fixed size TOGGLE_WIDTH_MD=36px x TOGGLE_HEIGHT_MD=18px — pill track, no label"
  - "Checkmark always painted (transparent when unchecked) to consume glyphon Buffer"
  - "Focus ring painted at -2px inset (expanded 2px beyond box) with 2px border in Accent"

patterns-established:
  - "Color extraction pattern: all cx.theme().color() calls must complete before first cx.paint_*() call"
  - "Disabled dimming: alpha *= 0.5 applied to all colors when disabled"

# Metrics
duration: 7min
completed: 2026-03-02
---

# Phase 8 Plan 03: Boolean Controls Summary

**Checkbox (square box + label) and Toggle (pill switch + knob) boolean controls with checked/unchecked/disabled/hover states using ColorToken theme system**

## Performance

- **Duration:** ~7 min
- **Started:** 2026-03-02T08:28:32Z
- **Completed:** 2026-03-02T08:35:00Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- Checkbox element with flex row layout (box + label), checked (Accent fill) and unchecked (1px Border ring) states, hover (BgElevated), focus ring (2px Accent), disabled (50% alpha)
- Toggle element with fixed-size pill track (BgSecondary/Accent) and circular white knob positioned left (off) or right (on), hover state brightening
- Both widgets respect disabled state via 50% alpha multiplier on all colors
- 3 unit tests: test_checkbox_default, test_checkbox_checked, test_toggle_default — all passing
- 65 total ora tests passing (no regressions)

## Task Commits

Each task was committed atomically:

1. **Task 1: Checkbox element with module registration** - `40c1490` (feat)
2. **Task 2: Toggle element and exports update** - `40c1490` (included in same commit — checkbox.rs contained both)

Note: checkbox.rs was first created by the 08-02 agent as a deviation (Rule 1 bug fix stub). This plan's execution replaced that stub with the full implementation, committed as part of Task 1.

## Files Created/Modified

- `ora/src/elements/checkbox.rs` - Checkbox and Toggle element implementations with CheckboxSize enum, constants, tests
- `ora/src/elements/mod.rs` - Added `pub mod checkbox` and exports for checkbox, Checkbox, CheckboxSize, toggle, Toggle
- `ora/src/lib.rs` - Added re-exports: checkbox, Checkbox, CheckboxSize, toggle, Toggle

## Decisions Made

- **CheckboxSize local enum** — Plan 08-01 (WidgetSize) may run concurrently. Using local CheckboxSize avoids wave-1 dependency conflicts. Plan 08-08 will unify these.
- **Checkmark always painted** — glyphon::Buffer from request_layout must be consumed in paint(). Setting color to transparent when unchecked is cleaner than conditional paint paths.
- **Color extraction pattern** — `cx.theme()` returns `&Theme` tied to `cx` lifetime, blocking mutable `cx.paint_*()` calls. Must extract all `Color` values (Copy type) into locals before any paint operations.
- **Toggle has no label** — Toggle is just the switch pill; users compose it with a TextElement for labeling, matching typical UI library patterns.

## Deviations from Plan

### Context

The checkbox.rs file was partially created by the 08-02 plan agent as a deviation (Rule 1) to handle a forward reference. This plan executed as authored, writing the full implementation.

None — plan executed exactly as written. The borrow checker issue (holding `&Theme` across mutable paint calls) was automatically resolved by following the color-extraction pattern established in button.rs.

## Issues Encountered

- **Borrow checker: `let theme = cx.theme()` across mutable borrows** — Storing `&Theme` reference in a `let theme` binding prevented subsequent `cx.paint_styled_rect()` calls. Resolved by extracting all `Color` values from theme into local `Color` variables (which implement `Copy`) before the first paint call. This is a known pattern in the codebase.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Checkbox and Toggle are exported from `ora` crate and ready for use in demos and views
- Plan 08-08 cleanup: replace `CheckboxSize` with `WidgetSize` from `button.rs` once wave 1 completes
- No blockers for remaining Phase 8 plans

---
*Phase: 08-advanced-ui-widgets*
*Completed: 2026-03-02*
