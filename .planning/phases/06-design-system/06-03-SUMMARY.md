---
phase: 06-design-system
plan: 03
subsystem: ui
tags: [theme, context, design-system, event-system]

# Dependency graph
requires:
  - phase: 06-01
    provides: Theme struct with color token system
  - phase: 06-02
    provides: Spacing and typography tokens
provides:
  - AppContext stores Theme with dark mode default
  - ThemeChanged global event for runtime theme switching
  - theme() accessor on all context types (AppContext, ViewContext, WindowContext, PaintContext)
  - set_theme() for runtime theme changes with automatic event propagation
affects: [06-04, 06-05, element-styling, theming]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Theme stored in AppContext for global access"
    - "ThemeChanged global event for reactive theme updates"
    - "Raw pointer in PaintContext for borrow-checker-safe theme access"
    - "Context delegation pattern (ViewContext/WindowContext → AppContext)"

key-files:
  created: []
  modified:
    - ora/src/context.rs
    - ora/src/element/trait_def.rs
    - ora/src/platform/event_loop.rs
    - ora/src/lib.rs

key-decisions:
  - "Theme defaults to dark mode (Theme::dark()) in AppContext::new()"
  - "Raw pointer for app_context in PaintContext to avoid aliasing during paint phase"
  - "ThemeChanged is a simple marker event (zero-size struct)"

patterns-established:
  - "Context theme access pattern: all contexts delegate to AppContext.theme()"
  - "Global event emission pattern: set_theme() calls emit_global(ThemeChanged)"

# Metrics
duration: 7m
completed: 2026-01-30
---

# Phase 6 Plan 3: Theme Context Integration Summary

**Theme integrated into AppContext with dark mode default, runtime switching via set_theme(), and ThemeChanged global events**

## Performance

- **Duration:** 7 min
- **Started:** 2026-01-30T13:21:10Z
- **Completed:** 2026-01-30T13:28:04Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- Theme storage in AppContext with Theme::dark() as default
- set_theme() enables runtime theme switching with automatic ThemeChanged event emission
- theme() accessor available on all context types used by elements
- PaintContext.theme() provides elements access to theme during paint phase

## Task Commits

Each task was committed atomically:

1. **Task 1: Add Theme to AppContext with set_theme() and ThemeChanged event** - `76564db` (feat)
2. **Task 2: Add theme() accessor to PaintContext** - `c96b485` (feat)

## Files Created/Modified
- `ora/src/context.rs` - Added Theme field to AppContext, ThemeChanged event, theme()/set_theme() methods
- `ora/src/element/trait_def.rs` - Added app_context pointer and theme() accessor to PaintContext
- `ora/src/platform/event_loop.rs` - Updated PaintContext::new() call to pass app_context
- `ora/src/lib.rs` - Re-exported ThemeChanged event

## Decisions Made

**1. Theme defaults to dark mode**
- AppContext::new() initializes with Theme::dark()
- Matches prior decision from 06-01 that ThemeMode::Dark is the default variant
- Ensures consistent dark mode experience out of the box

**2. Raw pointer for app_context in PaintContext**
- Used `*const AppContext` instead of `&AppContext` in PaintContext
- Necessary to avoid borrow checker aliasing (immutable app_context + mutable entity_storage)
- Follows same pattern as OraWindow::render() raw pointer usage
- Safe because app_context is not mutated during paint phase

**3. ThemeChanged as simple marker event**
- Zero-size struct with Clone, Copy, Debug
- No payload needed - subscribers query theme() directly
- Enables reactive UI updates when theme changes

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

**Borrow checker aliasing in PaintContext**
- **Issue:** Cannot borrow app_context immutably and entity_storage mutably from same struct
- **Solution:** Used raw pointer pattern (`*const AppContext`) already established in codebase (OraWindow::render)
- **Verification:** All tests pass, cargo check succeeds with only existing warnings

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Theme context integration complete. Ready for:
- Element styling to use cx.theme().color(ColorToken::BgPrimary)
- Runtime theme switching demonstrations
- Global theme change subscriptions via subscribe_global<ThemeChanged>()

All foundation pieces for theme-aware UI components are now in place.

---
*Phase: 06-design-system*
*Completed: 2026-01-30*
