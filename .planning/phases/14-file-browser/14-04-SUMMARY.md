---
phase: 14-file-browser
plan: 04
subsystem: ui
tags: [rust, filesystem-watcher, tab-bar, external-changes, hitbox, scroll-area, layout-cache, theme]

# Dependency graph
requires:
  - phase: 14-file-browser/14-03
    provides: notify-debouncer-full watcher, poll_watcher_events infrastructure, mark_tree_dirty()
  - phase: 14-file-browser/14-01
    provides: TabPresentation, FileOpDataSource trait, buffer registry, workspace state
provides:
  - External file deletion detected via watcher — tab shows deleted indicator (muted color)
  - External file modification: clean buffers auto-reload from disk, dirty buffers show ExternalModificationPrompt dialog
  - Workspace switch (Open Folder) closes all tabs cleanly
  - deleted_paths HashSet and pending_reload_prompts Vec on CoreEditorAdapter
  - TabPresentation.is_deleted field drives deleted tab rendering in tab_bar
  - ExternalModificationPrompt dialog variant (Reload / Keep)
  - Scissor rect bounds clamping prevents GPU out-of-bounds draws
  - Scroll area hitbox offset corrects click targeting after scroll
  - Scroll area hitbox clipped to viewport bounds — off-screen areas don't steal events
  - needs_layout=true on scroll events ensures element count changes are handled correctly
  - Layout cache guard: force full layout when element tree outgrows cached outputs
  - Slate palette restored; SelectionInactive token added; BgSecondary/BgElevated usage corrected
  - Phase 14 complete: all 6 success criteria verified by human
affects:
  - 15-context-menu (hitbox/scroll-area rendering patterns established here)
  - 13.2-rendering-performance (known scroll perf issue: visible_lines count changes trigger full layout each scroll frame)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "deleted_paths HashSet on adapter: lightweight O(1) lookup for deleted file indicator, no core_editor domain changes needed"
    - "pending_reload_prompts Vec<PathBuf>: queue for external modification prompts, dequeued one at a time"
    - "Scissor rect clamping: y + h <= surface_height guard in trait_def prepaint prevents GPU validation errors"
    - "Scroll area hitbox offset: subtract scroll_offset from hit region y before registering hitbox"
    - "Scroll area viewport clip: intersect hitbox with viewport rect to prevent off-screen hitboxes capturing events"
    - "Layout cache guard: if element_id >= cached_outputs.len(), force needs_layout=true to extend cache"

key-files:
  created: []
  modified:
    - ora/src/element/trait_def.rs
    - ora/src/elements/scroll_area.rs
    - ora/src/platform/event_loop.rs
    - ora/src/editor_adapter/types.rs
    - ora/src/editor_adapter/mod.rs
    - wgpu_client/src/adapter.rs
    - ora/src/views/tab_bar.rs
    - ora/src/views/dialog.rs
    - ora/src/theme/color.rs
    - ora/src/theme/mod.rs
    - ora/src/views/gutter.rs
    - ora/src/views/status_bar.rs
    - core_editor/src/domain/document.rs
    - core_editor/src/view_model/mod.rs

key-decisions:
  - "deleted_paths HashSet on adapter (not core_editor) — avoids domain changes, lookup is O(1) per tab build"
  - "pending_reload_prompts Vec dequeues one prompt at a time — prevents dialog stacking when multiple files change simultaneously"
  - "Tab deleted indicator: muted color styling (not strikethrough) — fits existing tab rendering without text layout changes"
  - "needs_layout=true on all scroll events: proven necessary by debug logs showing LayoutId count mismatch between frames when visible_lines count changes"
  - "Force full layout when element_id >= cached_outputs.len(): guards against layout cache under-allocation after element tree growth"
  - "Scissor rect: clamp y+h to surface_height before submitting to GPU — prevents validation layer errors on partial-viewport elements"
  - "Scroll area hitbox offset: subtract scroll_offset.y from registered hitbox origin so click coordinates map correctly after scrolling"

patterns-established:
  - "Adapter-side deleted/modified tracking: HashSet<PathBuf> and Vec<PathBuf> on adapter, checked during build_render_model"
  - "Hitbox clipping pattern: intersect element hitbox with scroll viewport rect before registration"
  - "Layout cache growth guard: compare element_id to cached_outputs.len() before reuse"

# Metrics
duration: ~60min (including orchestrator fix passes during verification)
completed: 2026-03-27
---

# Phase 14 Plan 04: File Browser Completion Summary

**External file change handling (auto-reload + deletion indicator + dirty prompt), workspace switch tab cleanup, plus 6 rendering bug fixes uncovered during human verification — completing Phase 14 File Browser**

## Performance

- **Duration:** ~60 min (task execution + verification fix cycles)
- **Started:** 2026-03-27T00:00:00Z
- **Completed:** 2026-03-27T00:00:00Z
- **Tasks:** 1 (+ checkpoint)
- **Files modified:** 14

## Accomplishments

- `TabPresentation.is_deleted` field propagated from `deleted_paths` HashSet on adapter to tab bar rendering
- `ExternalModificationPrompt` dialog variant with Reload/Keep actions for externally modified dirty buffers
- Clean buffer auto-reload from disk on external modification (same UTF-8/BOM strip pattern as Phase 12 file open)
- `handle_folder_opened` closes all open tabs before switching workspace
- Six rendering bugs fixed during human verification: scissor rect clamping, scroll hitbox offset, scroll hitbox viewport clip, needs_layout on scroll, layout cache growth guard, theme/palette restoration

## Task Commits

1. **Task 1: External file change handling + tab close on workspace switch** - `3ec7b9a` (feat)

Orchestrator fix commits (applied during verification):

2. **Scissor rect bounds + scroll area hitbox offset** - `a7c1cb3` (fix)
3. **needs_layout on scroll-area scroll events** - `3425a06` (fix)
4. **Clip scroll area hitboxes to viewport bounds** - `f50746c` (fix)
5. **Restore slate palette + theme adjustments + SelectionInactive** - `82e0859` (style)
6. **Force full layout when element tree outgrows cached outputs** - `d6732d8` (fix)
7. **needs_layout=true on all scroll, remove debug traces** - `15a3710` (fix)

**Plan metadata:** (forthcoming docs commit)

## Files Created/Modified

- `ora/src/element/trait_def.rs` — Scissor rect y+h clamping to surface_height; hitbox offset and clip helpers; next_layout_id guard for cache growth
- `ora/src/elements/scroll_area.rs` — Hitbox registration with scroll_offset subtraction and viewport rect intersection in prepaint
- `ora/src/platform/event_loop.rs` — needs_layout=true on all scroll events; watcher polling; layout cache growth guard
- `ora/src/editor_adapter/types.rs` — `is_deleted: bool` on `TabPresentation`; `ExternalModificationPrompt` dialog variant
- `ora/src/editor_adapter/mod.rs` — `handle_external_file_changes` and `handle_folder_opened` methods on `FileOpDataSource`
- `wgpu_client/src/adapter.rs` — `deleted_paths: HashSet<PathBuf>`, `pending_reload_prompts: Vec<PathBuf>`, full external change handling logic
- `ora/src/views/tab_bar.rs` — Deleted indicator rendering (muted color) when `is_deleted` is true
- `ora/src/views/dialog.rs` — `ExternalModificationPrompt` dialog (Reload / Keep buttons)
- `ora/src/theme/color.rs` — Slate palette restored
- `ora/src/theme/mod.rs` — `SelectionInactive` token added; `BgSecondary`/`BgElevated` usage corrected
- `ora/src/views/gutter.rs` — Uses `BgSecondary` + left border
- `ora/src/views/status_bar.rs` — Uses `BgElevated` + border
- `core_editor/src/domain/document.rs` — `reload_content` method for in-place buffer reload
- `core_editor/src/view_model/mod.rs` — `is_deleted` field on view model

## Decisions Made

- **deleted_paths on adapter (not core_editor domain):** Tracking deletion state in the adapter as a `HashSet<PathBuf>` avoids propagating filesystem concerns into the core editor domain. Lookup is O(1) per tab during `build_render_model`.
- **Prompt queue dequeues one at a time:** `pending_reload_prompts` yields the first entry as a dialog; subsequent entries wait. Prevents simultaneous dialogs when multiple files are modified externally at once.
- **Muted color for deleted tab indicator:** Simpler than strikethrough (no text layout changes needed), communicates deletion clearly, consistent with VS Code's approach.
- **needs_layout=true on ALL scroll events:** Debug logs showed LayoutId count mismatch between scroll frames because `visible_lines` count varies with scroll position. Skipping layout recomputation on scroll caused element tree / cache desync.
- **Scissor y+h clamping:** GPU validation layer rejects scissor rects that extend beyond surface bounds. Clamp `y + h <= surface_height` in prepaint before submitting draw commands.
- **Scroll hitbox offset + clip:** After scroll, the logical element position and the physical hitbox diverge. Subtract `scroll_offset` from the hitbox y-origin and intersect with the viewport rect so only on-screen portions can receive pointer events.

## Deviations from Plan

### Auto-fixed Issues (during orchestrator verification passes)

**1. [Rule 1 - Bug] Scissor rect extends beyond surface height**
- **Found during:** Human verification (rendering artifacts on scrolled sidebar)
- **Issue:** Scissor rect y + h exceeded surface_height, causing GPU validation errors and visual corruption at viewport edges
- **Fix:** Clamp scissor rect bounds in `trait_def.rs` prepaint: `h = min(h, surface_height - y)`
- **Files modified:** `ora/src/element/trait_def.rs`
- **Committed in:** `a7c1cb3`

**2. [Rule 1 - Bug] Scroll area hitbox origin not adjusted for scroll offset**
- **Found during:** Human verification (click targeting wrong rows after scrolling sidebar)
- **Issue:** Hitboxes were registered at the pre-scroll element position; clicks landed on wrong elements after scrolling
- **Fix:** Subtract `scroll_offset` from hitbox y when registering in `scroll_area.rs` prepaint
- **Files modified:** `ora/src/elements/scroll_area.rs`
- **Committed in:** `a7c1cb3`

**3. [Rule 1 - Bug] Off-viewport hitboxes capturing pointer events**
- **Found during:** Human verification (clicks near sidebar edge triggering wrong file open)
- **Issue:** Scroll area elements below the viewport still had hitboxes registered, stealing pointer events from other UI regions
- **Fix:** Clip hitbox to viewport bounds (intersect with scroll viewport rect) in `scroll_area.rs` prepaint
- **Files modified:** `ora/src/elements/scroll_area.rs`
- **Committed in:** `f50746c`

**4. [Rule 1 - Bug] Layout cache desync on scroll — LayoutId count mismatch**
- **Found during:** Human verification (sidebar element count changing erratically during scroll)
- **Issue:** `visible_lines` changes frame-to-frame during scroll, so element tree size varies. Without `needs_layout=true`, the layout cache served stale outputs for the wrong element count.
- **Fix:** Set `needs_layout=true` on all scroll events in `event_loop.rs`; add layout cache growth guard in `next_layout_id`
- **Files modified:** `ora/src/platform/event_loop.rs`, `ora/src/element/trait_def.rs`
- **Committed in:** `3425a06`, `d6732d8`, `15a3710`

**5. [Rule 1 - Bug] Theme palette regression (slate colors, BgSecondary/BgElevated swap)**
- **Found during:** Human verification (gutter and status bar colors incorrect)
- **Issue:** Slate palette missing from `color.rs`; `BgSecondary` and `BgElevated` tokens used in wrong surfaces; `SelectionInactive` token missing
- **Fix:** Restored slate palette in `color.rs`; added `SelectionInactive` to `theme/mod.rs`; corrected token usage in gutter and status bar views
- **Files modified:** `ora/src/theme/color.rs`, `ora/src/theme/mod.rs`, `ora/src/views/gutter.rs`, `ora/src/views/status_bar.rs`
- **Committed in:** `82e0859`

---

**Total deviations:** 5 auto-fixed (4 bugs, 1 style/regression)
**Impact on plan:** All fixes necessary for correct pointer event handling and visual correctness. No scope creep — all fixes address rendering infrastructure issues exposed by the complete file browser feature set exercising scroll areas for the first time under real conditions.

## Issues Encountered

**Known performance issue (not fixed in this plan):** Scroll triggers full layout every frame because `visible_lines` count changes between scroll frames (viewport virtualization calculates a different visible slice each frame). This is documented as a Phase 13.2 (Rendering Performance — View-level Dirty Checking) concern. The fix (frame-stable `visible_lines` or view-level dirty checking) requires deeper changes to the rendering pipeline.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 14 File Browser complete — all 6 roadmap success criteria verified:
  - SC1: Sidebar shows real files and folders (no mock data) ✓
  - SC2: Click file opens in editor via Buffer Registry ✓
  - SC3: Expand/collapse persists across renders ✓
  - SC4: Open Folder sets new workspace root ✓
  - SC5: External changes reflected without restart ✓
  - SC6: .git/ excluded via ignored_patterns ✓
- Scroll area hitbox/scissor patterns established and ready for Phase 15 (context menu uses same scroll infrastructure)
- Known scroll performance issue documented for Phase 13.2

---
*Phase: 14-file-browser*
*Completed: 2026-03-27*
