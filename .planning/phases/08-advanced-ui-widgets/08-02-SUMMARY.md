---
phase: 08-advanced-ui-widgets
plan: 02
subsystem: ui
tags: [rust, gpui, ora, elements, tab, list-item, tree-item, navigation-widgets, theme]

# Dependency graph
requires:
  - phase: 07-editor-chrome-text-editing
    provides: TextElement, three-phase element lifecycle, ColorToken system
  - phase: 08-advanced-ui-widgets (plan 01)
    provides: WidgetSize enum, Button with size tiers, checkbox/toggle elements

provides:
  - Tab element with active/inactive/dirty states and close button
  - ListItem element with selection, hover, disabled, and optional secondary text
  - TreeItem element with depth indentation, expand/collapse chevron, indent guides

affects:
  - 08-advanced-ui-widgets (plan 04 CommandPalette uses ListItem)
  - 08-advanced-ui-widgets (plan 05 FileTree uses TreeItem)
  - 08-advanced-ui-widgets (plan 06 PanelManager uses Tab)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Color extraction before mutable borrows: call cx.theme().color() for all needed colors before any cx.paint_* calls"
    - "Separate hitboxes for composite clickable areas (tab body + close button, tree row + chevron)"
    - "Indent guide rendering: loop 0..depth, paint 1px vertical Border-color rects"

key-files:
  created:
    - ora/src/elements/tab.rs
    - ora/src/elements/list_item.rs
    - ora/src/elements/tree_item.rs
  modified:
    - ora/src/elements/mod.rs
    - ora/src/lib.rs
    - ora/src/elements/checkbox.rs

key-decisions:
  - "TAB_HEIGHT=36px matches Phase 7 TAB_BAR_HEIGHT constant — tabs keep their established height"
  - "ListItem uses low-alpha Accent (0.15) for selected state — distinguishable without harsh contrast"
  - "TreeItem chevron uses 'v'/'>' ASCII — no icon system dependency"
  - "Indent guides painted as 1px rects (not borders) — simpler and GPU-efficient"
  - "Color extraction pattern: all cx.theme().color() calls before any mutable cx.paint_* calls"

patterns-established:
  - "Color extraction before mutable borrows: extract all theme colors into local variables before any cx.paint_styled_rect / element.paint() calls to avoid E0502 borrow conflicts"
  - "Separate hitboxes for sub-regions: register small hitbox (opaque:false) for close/chevron areas within the larger row hitbox (opaque:true)"

# Metrics
duration: 4m 24s
completed: 2026-03-02
---

# Phase 8 Plan 02: Navigation Widgets Summary

**Three navigation widgets (Tab, ListItem, TreeItem) with theme-aware styling, three-phase Element lifecycle, and separate hitboxes for sub-regions like close buttons and expand chevrons**

## Performance

- **Duration:** 4m 24s
- **Started:** 2026-03-02T08:27:50Z
- **Completed:** 2026-03-02T08:32:14Z
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments

- Tab element: active/inactive/dirty states, Accent bottom border for active, optional close button with separate hitbox
- ListItem element: selected (low-alpha Accent bg), hover (BgElevated), disabled (50% alpha), secondary text right-aligned in FgMuted
- TreeItem element: depth indentation via left padding, "v"/">" chevron, per-level indent guides as 1px Border-color rects, icon color support
- Fixed pre-existing borrow error in checkbox.rs (Plan 08-01 artifact) — same `let theme = cx.theme()` pattern issue

## Task Commits

Each task was committed atomically:

1. **Task 1: Tab widget element** - `0f5bea1` (feat)
2. **Task 2: ListItem widget element** - `447d097` (feat) — includes checkbox.rs bug fix
3. **Task 3: TreeItem widget element** - `774a5b2` (feat)

**Plan metadata:** (docs commit — see below)

## Files Created/Modified

- `ora/src/elements/tab.rs` - Tab with active/inactive/dirty states and close button
- `ora/src/elements/list_item.rs` - Selectable list row with selection/hover/disabled/secondary text
- `ora/src/elements/tree_item.rs` - Tree node with depth indent, chevron, indent guides
- `ora/src/elements/mod.rs` - Added tab, list_item, tree_item module declarations and re-exports
- `ora/src/lib.rs` - Added tab, Tab, list_item, ListItem, tree_item, TreeItem to public re-exports
- `ora/src/elements/checkbox.rs` - Fixed borrow error (E0502) in Checkbox::paint and Toggle::paint

## Decisions Made

- TAB_HEIGHT=36px matches Phase 7 TAB_BAR_HEIGHT — consistency with established layout constants
- Selected state uses low-alpha Accent (0.15 alpha) for ListItem and TreeItem — distinguishable without harsh contrast
- TreeItem chevron uses ASCII "v"/">" characters — avoids dependency on icon system not yet built
- Indent guides as 1px `paint_styled_rect` calls in a loop — simpler than border-based approach, GPU-efficient
- Color extraction pattern established: all `cx.theme().color()` calls must precede any `cx.paint_*` or child `element.paint()` calls

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed Rust borrow error E0502 in checkbox.rs (from Plan 08-01)**

- **Found during:** Task 2 (ListItem compilation triggered checkbox.rs to be compiled together)
- **Issue:** `let theme = cx.theme()` created an immutable borrow of `cx`, then `cx.paint_styled_rect()` needed a mutable borrow. Same pattern already fixed in tab.rs during Task 1.
- **Fix:** Extracted all `theme.color(ColorToken::*)` calls into local `Color` variables before any `cx.paint_styled_rect()` or `child_el.paint()` calls in both `Checkbox::paint` and `Toggle::paint`
- **Files modified:** `ora/src/elements/checkbox.rs`
- **Verification:** `cargo test -p ora --lib` went from compile error to 63 passing tests
- **Committed in:** `447d097` (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (Rule 1 - Bug)
**Impact on plan:** Fix was necessary to compile. Checkbox behavior unchanged — purely a borrow-checker structural fix.

## Issues Encountered

The borrow pattern `let theme = cx.theme(); ... cx.paint_styled_rect(...)` triggers E0502 because `cx.theme()` returns `&Theme` via raw pointer dereference, but the borrow checker models it as an immutable borrow of `cx`. Solution: inline `cx.theme().color(token)` calls or extract colors into local variables before any mutable borrow of `cx`.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Tab, ListItem, TreeItem elements are ready to use in views
- Plan 08-04 (CommandPalette) can use `list_item()` for results
- Plan 08-05 (FileTree view) can use `tree_item()` for nodes
- Plan 08-06 (PanelManager) can use `tab()` for panel tabs
- All 65 ora unit tests pass, `cargo build -p ora` succeeds

---
*Phase: 08-advanced-ui-widgets*
*Completed: 2026-03-02*
