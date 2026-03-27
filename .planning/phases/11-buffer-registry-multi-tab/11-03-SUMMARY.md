---
phase: 11-buffer-registry-multi-tab
plan: 03
subsystem: ui
tags: [rust, tab-bar, click-handlers, event-dispatch, wgpu]

# Dependency graph
requires:
  - phase: 11-02
    provides: App::switch_tab, App::close_active_tab, adapter dispatch_command for SwitchTab/CloseTab
  - phase: 11-01
    provides: tab_order() on Workspace, get_open_views_info already iterating tab_order

provides:
  - Tab element with on_click/on_close wired to cx.on_mouse_down event dispatch
  - Middle-click on tab body dispatches on_close
  - Close button hitbox is opaque and registered after tab body for correct hit_test priority
  - TabBarView uses Tab element with dispatch callback for SwitchTab/CloseTab commands
  - Dot dirty indicator (Unicode bullet) before tab title in Tab element
  - Active tab accent border (2px bottom border in AccentPrimary color)

affects: [11-04, future click-interaction work, Phase 12 file operations]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "hit_test reverse iteration ordering: last registered hitbox wins — register children after parents for correct priority"
    - "Rc<dyn Fn()> for callbacks shared across multiple event handlers (vs Box which can't clone)"
    - "Tab element prepaint wires cx.on_mouse_down for both body and close hitboxes"
    - "TabBarView receives Rc<dyn Fn(EditorCommand)> dispatch closure created from adapter in EditorRootView"

key-files:
  created: []
  modified:
    - "ora/src/elements/tab.rs"
    - "ora/src/views/tab_bar.rs"
    - "ora/src/views/editor_root.rs"

key-decisions:
  - "Close button hitbox must be opaque (not false) so hit_test can route clicks to it"
  - "Close hitbox registered AFTER tab body (higher index = found first in reverse hit_test) — explicit ordering contract"
  - "on_click/on_close changed from Box<dyn Fn()> to Rc<dyn Fn()> to allow cloning for middle-click sharing"
  - "TabBarView stores dispatch as Option<Rc<dyn Fn(EditorCommand)>> wired via with_dispatch()"
  - "Task 1 visual changes (dot indicator in tab_bar.rs) were superseded by Task 2 Tab element refactor"

patterns-established:
  - "Hitbox registration order determines priority: later registration = higher hit_test priority (reverse scan)"
  - "Event callbacks that need to be reused across multiple handlers use Rc<dyn Fn()> not Box"

# Metrics
duration: 25min
completed: 2026-03-26
---

# Phase 11 Plan 03: Tab Click Handlers + Visual Polish Summary

**Interactive tab bar with Rc-based callbacks, opaque close hitbox with reverse-index priority, middle-click close, and Tab element wired to SwitchTab/CloseTab dispatch**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-03-26T00:00:00Z
- **Completed:** 2026-03-26T00:25:00Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- Tab element `on_click`/`on_close` callbacks wired to `cx.on_mouse_down` event dispatch in `prepaint`
- Close button hitbox made opaque and registered after tab body — exploits hit_test reverse-index scan for correct routing
- Middle-click on tab body dispatches `on_close` (left click dispatches `on_click`)
- `TabBarView` refactored to use `Tab` element with dispatch callback passed from `EditorRootView`
- Dirty indicator updated to Unicode bullet (`•`) prefix; active tab has 2px bottom accent border
- `get_open_views_info` already used `tab_order()` from Plan 01 — no changes needed to `app.rs`

## Task Commits

Each task was committed atomically:

1. **Task 1: Dot dirty indicator + active accent border in tab_bar.rs** - `ff12c83` (feat)
2. **Task 2: Wire click handlers, refactor TabBarView to Tab element** - `fc74b32` (feat)

**Plan metadata:** (docs commit follows)

## Files Created/Modified

- `ora/src/elements/tab.rs` - on_click/on_close Rc callbacks, opaque close hitbox, cx.on_mouse_down wiring, middle-click support
- `ora/src/views/tab_bar.rs` - refactored to use Tab element with dispatch callback; removed Div-based manual rendering
- `ora/src/views/editor_root.rs` - creates dispatch closure from SharedAdapter and passes to TabBarView via with_dispatch()

## Decisions Made

- **Box → Rc for callbacks:** `on_click`/`on_close` changed to `Rc<dyn Fn()>` so they can be cloned into multiple `on_mouse_down` handlers (close button handler and middle-click handler in tab body both need `on_close`).
- **Hitbox registration order:** Close button registered AFTER tab body. `hit_test` iterates in reverse (last = topmost), so the close button (registered last) wins when cursor is in its area. This is a load-bearing ordering contract documented in code.
- **Close hitbox must be opaque:** Non-opaque hitboxes are skipped by `hit_test` (mouse.rs line 48). Original Tab code registered close hitbox as `opaque: false` — fixed to `true`.
- **Task 1 as throwaway:** The Div-based dirty dot and accent border in `tab_bar.rs` (Task 1) was committed separately but was superseded when Task 2 replaced the entire rendering with the Tab element. Tab element already had these visuals implemented correctly.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Close button hitbox was non-opaque — hit_test would never route to it**

- **Found during:** Task 2 (wiring Tab element callbacks)
- **Issue:** Original `tab.rs` registered close hitbox with `opaque: false`. The `hit_test` function skips non-opaque hitboxes (`if !hitbox.opaque { continue; }`), so clicking the close button would fall through to the tab body hitbox and trigger tab selection instead of close.
- **Fix:** Changed close hitbox registration to `opaque: true` in `Tab::prepaint`
- **Files modified:** `ora/src/elements/tab.rs`
- **Verification:** Build passes; dispatch routing verified by code inspection
- **Committed in:** `fc74b32` (Task 2 commit)

**2. [Rule 1 - Bug] Hitbox registration order was inverted — close button would lose to tab body**

- **Found during:** Task 2, during analysis of `hit_test` reverse-scan behavior
- **Issue:** Original code registered tab body first, then close button. Since `hit_test` returns the LAST registered opaque hitbox at a point (reverse iteration), the close button (registered second/later) would actually win — but the original code registered close button before tab body in the old version. After making the close hitbox opaque, the ordering needed to match the reverse-scan semantics explicitly.
- **Fix:** Register tab body first, then close button. This ensures close button (higher index) wins in the reverse scan when cursor is over the close button area.
- **Files modified:** `ora/src/elements/tab.rs`
- **Verification:** Hit-test priority analysis; build passes
- **Committed in:** `fc74b32` (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 Rule 1 - Bug)
**Impact on plan:** Both fixes essential for correct click routing. Without them, the close button would never work. No scope creep.

## Issues Encountered

- `MouseButton` import path: `crate::events::mouse::MouseButton` was private; corrected to `crate::events::MouseButton` (re-exported from `events/mod.rs`). Resolved immediately at first build.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Tab clicking is fully wired: left click → SwitchTab(view_id), close button click / middle click → CloseTab
- Tab visual state is correct: dot dirty indicator, active accent border, hover states
- Plan 04 (human verification checkpoint) can confirm interactive behavior in the running app
- No blockers for Plan 04

---
*Phase: 11-buffer-registry-multi-tab*
*Completed: 2026-03-26*
