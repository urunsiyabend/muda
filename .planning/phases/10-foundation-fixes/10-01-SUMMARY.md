---
phase: 10-foundation-fixes
plan: 01
subsystem: ui
tags: [winit, wgpu, event-loop, gpu-rendering, caret, controlflow]

# Dependency graph
requires: []
provides:
  - Event-driven redraw with ControlFlow state machine in about_to_wait
  - next_blink_instant field on OraApp for timer-driven caret blink wakeup
  - Zero unconditional request_redraw calls inside RedrawRequested handlers
affects:
  - 10-foundation-fixes (subsequent plans building on stable frame cadence)
  - All plans requiring observable GPU/CPU idle behavior

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ControlFlow state machine: Poll for dirty/animated, WaitUntil for caret blink, Wait for true idle"
    - "Blink timer on OraApp: next_blink_instant reset on each notify_caret_activity() call"

key-files:
  created: []
  modified:
    - ora/src/platform/event_loop.rs

key-decisions:
  - "Remove exactly the two unconditional request_redraw() calls inside RedrawRequested (lines 549, 567) — all others kept"
  - "Store next_blink_instant on OraApp struct, not computed from global statics each frame"
  - "ControlFlow::Poll only when has_dirty_entities() || has_active_transitions() — never unconditionally"
  - "Blink instant passes => request redraw immediately + schedule next blink at Instant::now() + BLINK_RATE"

patterns-established:
  - "about_to_wait owns the ControlFlow decision — window_event only requests redraw for immediate reaction"
  - "Caret blink driven by WaitUntil timer, not continuous render loop"

# Metrics
duration: 2min
completed: 2026-03-26
---

# Phase 10 Plan 01: Event-Driven Redraw Summary

**winit ControlFlow state machine eliminates idle GPU waste — caret blinks via WaitUntil timer, GPU drops to near-zero between blinks**

## Performance

- **Duration:** ~2 min
- **Started:** 2026-03-25T23:20:26Z
- **Completed:** 2026-03-25T23:22:41Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- Removed two unconditional `request_redraw()` calls from inside `RedrawRequested` handlers that caused infinite render loop
- Added `next_blink_instant: Option<Instant>` field to `OraApp`, initialized and reset on each keystroke via `notify_caret_activity()`
- Rewrote `about_to_wait` with full ControlFlow state machine: `Poll` during active animations/dirty entities, `WaitUntil` for caret blink timer, `Wait` when truly idle
- All 151 existing tests pass without modification

## Task Commits

Each task was committed atomically:

1. **Task 1: Remove unconditional redraws and add next_blink_instant to OraApp** - `d055aa0` (feat)
2. **Task 2: Implement ControlFlow state machine in about_to_wait** - `78117fd` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `ora/src/platform/event_loop.rs` — Added imports (Instant, ControlFlow, BLINK_RATE, ACTIVITY_TIMEOUT), next_blink_instant field, removed two unconditional redraws, updated both notify_caret_activity() call sites, rewrote about_to_wait

## Decisions Made

- Kept `next_blink_instant` on `OraApp` rather than computing from global statics each frame — simpler and avoids coupling to caret.rs internals
- `about_to_wait` handles the blink-passed case directly (request redraw + schedule next) rather than deferring to the next event
- The dirty-entity check at the end of `window_event` (lines 598-603) was kept — it ensures immediate redraw scheduling within the same event batch; `about_to_wait` handles the inter-event idle period

## Deviations from Plan

None — plan executed exactly as written.

## Issues Encountered

None — the two unconditional redraws were exactly where research predicted (lines 549, 567). `has_active_transitions()` was confirmed public on `AppContext`. `BLINK_RATE` and `ACTIVITY_TIMEOUT` were confirmed re-exported from `crate::elements`.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Frame cadence is now observable — idle GPU usage drops to near-zero between caret blinks
- Caret blink timer fires every 500ms via WaitUntil, toggling visibility via BLINK_EPOCH global in caret.rs
- Hover transitions and scroll still trigger redraws correctly via existing event handler calls
- Ready for Phase 10 Plan 02 (selection rendering fix) — frame behavior no longer masks rendering bugs

---
*Phase: 10-foundation-fixes*
*Completed: 2026-03-26*
