---
phase: 11-buffer-registry-multi-tab
plan: 01
subsystem: editor-domain
tags: [workspace, buffer-registry, tab-order, mru, deduplication, rust]

# Dependency graph
requires:
  - phase: 10-foundation-fixes
    provides: "Workspace struct, Document/View lifecycle methods, App shell"
provides:
  - "path_to_doc HashMap<PathBuf, DocumentId> — Buffer Registry prevents duplicate Document creation"
  - "tab_order Vec<ViewId> — insertion-order tab strip for deterministic UI rendering"
  - "mru_stack Vec<ViewId> — activation history for close-tab focus fallback"
  - "open_document returns (DocumentId, bool) — callers can detect and reuse existing docs"
  - "get_open_views_info iterates tab_order for stable tab bar rendering"
affects:
  - 11-02 (tab close commands use mru_stack)
  - 11-03 (tab rendering reads tab_order)
  - plans that read workspace.tab_order() or workspace.mru_stack()

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Buffer Registry: canonical-path-keyed HashMap prevents duplicate Documents"
    - "MRU stack: Vec with retain+insert(0) for O(n) promotion to front"
    - "Tab order: Vec insertion after active position ensures intuitive placement"

key-files:
  created: []
  modified:
    - core_editor/src/domain/workspace.rs
    - core_editor/src/app.rs

key-decisions:
  - "open_document signature changed to return (DocumentId, bool) — bool is was_existing flag for caller deduplication logic"
  - "path_to_doc cleanup uses retain(|_, &mut id| id != doc_id) — avoids canonical path reconstruction on close"
  - "App::open_file (constructor) always has was_existing=false — dedup logic only in open_file_from_sidebar"
  - "get_open_views_info iterates tab_order not views HashMap — satisfies insertion-order tab strip requirement"

patterns-established:
  - "MRU promotion: self.mru_stack.retain(|&id| id != view_id); self.mru_stack.insert(0, view_id)"
  - "Tab insert: find active position in tab_order, insert at pos+1, fallback to push end"

# Metrics
duration: 15min
completed: 2026-03-26
---

# Phase 11 Plan 01: Buffer Registry, Tab Order, MRU Stack Summary

**Workspace gains path_to_doc deduplication HashMap, tab_order insertion-order Vec, and mru_stack activation-history Vec — the three data structures enabling all Phase 11 tab management**

## Performance

- **Duration:** ~15 min
- **Started:** 2026-03-26T08:41:00Z
- **Completed:** 2026-03-26T08:56:08Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- Buffer Registry (`path_to_doc`) canonicalizes file paths and returns existing DocumentId on re-open, preventing duplicate Document instances
- `tab_order` Vec maintains visual tab strip insertion order — new views insert after current active tab
- `mru_stack` Vec tracks activation history so closing the active tab falls back to the previously active one
- `open_document` signature changed to `(DocumentId, bool)` — all callers updated
- `get_open_views_info` now iterates `tab_order` instead of `views` HashMap for deterministic tab bar rendering
- 4 new targeted tests added (deduplication, tab order, MRU switch, close activates MRU)

## Task Commits

1. **Task 1: Add Buffer Registry, tab_order, and mru_stack fields to Workspace** - `abac982` (feat)
2. **Task 2: Update App callers of open_document for new signature** - `d827aa4` (feat)

## Files Created/Modified

- `core_editor/src/domain/workspace.rs` - Added path_to_doc/tab_order/mru_stack fields; updated open_document, create_view, close_view, set_active_view, close_document_force; added tab_order()/mru_stack() accessors; 4 new tests
- `core_editor/src/app.rs` - Updated open_file, open_file_from_sidebar for (DocumentId, bool); updated get_open_views_info to iterate tab_order

## Decisions Made

- `open_document` returns `(DocumentId, bool)` tuple — the bool (`was_existing`) lets callers decide whether to create a new view or switch to existing. This is a clean API since callers have different behavior (open_file always creates, open_file_from_sidebar deduplicates).
- `path_to_doc` cleanup uses `retain(|_, &mut id| id != doc_id)` — avoids trying to reconstruct the canonical path from the Document's stored path (which is the original non-canonical path), preventing potential mismatch bugs.
- `App::open_file` (static constructor) always starts with a fresh Workspace so `was_existing` is always false there — deduplication logic is only meaningful in `open_file_from_sidebar`.
- Fixed `get_open_views_info` to iterate `tab_order` (deviation from plan text but required to satisfy the must-have truth "Tabs render in insertion order").

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Updated get_open_views_info to iterate tab_order**

- **Found during:** Task 2 (App callers update)
- **Issue:** Plan's must-have truth stated "Tabs render in insertion order, not random HashMap order" but `get_open_views_info` iterated `self.workspace.views()` (HashMap, random order). This method drives the tab bar render model.
- **Fix:** Changed iteration to `self.workspace.tab_order().iter()` with a view lookup per ID.
- **Files modified:** core_editor/src/app.rs
- **Verification:** `test_new_app_render_model_for_gui` still passes; tab bar is now deterministic
- **Committed in:** d827aa4 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (missing critical)
**Impact on plan:** Required to satisfy the plan's explicit must-have truth. No scope creep.

## Issues Encountered

None — all changes went through cleanly on the first build attempt.

## Next Phase Readiness

- Plan 02 (close-tab command) can use `mru_stack` for focus fallback — the data is now maintained
- Plan 03 (tab strip rendering) can read `tab_order()` directly — the order is correct
- All 108 core_editor tests pass; full workspace builds cleanly
- No blockers

---
*Phase: 11-buffer-registry-multi-tab*
*Completed: 2026-03-26*
