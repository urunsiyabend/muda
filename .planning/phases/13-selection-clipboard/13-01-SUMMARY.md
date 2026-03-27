# Phase 13 Plan 01: Click-to-Cursor Positioning Summary

**One-liner:** ClickAt/DragTo command pipeline from pixel coordinates through event loop to core_editor dispatcher with word/line selection and mid-character snap.

## What Was Done

### Task 1: Add ClickAt/DragTo command variants and adapter wiring
- Added `ClickAt { line, col, extend_selection, click_count }` and `DragTo { line, col }` variants to both ora (`EditorCommand`) and core_editor (`EditorCommand`) enums
- Implemented `handle_click_at` in CommandDispatcher: single click positions cursor, double-click selects word (VS Code-like char class boundaries), triple-click selects full line including trailing newline
- Implemented `handle_drag_to`: always extends selection from anchor to drag position
- Added `char_class` helper with Word/Punctuation/Whitespace classification for word boundary detection
- Wired ClickAt/DragTo through `to_core_command` in the adapter
- Added 7 unit tests covering: click positioning, double-click word select, shift-click extend, triple-click line select, end-of-line snap, below-last-line snap, drag selection

### Task 2: Wire mouse click dispatch in the event loop
- Added `sidebar_width_px()`, `gutter_width_chars()`, and `scroll_x()` to `BufferDataSource` trait
- Implemented `pixel_to_doc()` helper in event loop: converts pixel position to (line, col) with scroll offset handling and mid-character snap (past midpoint snaps right via `+0.5`)
- On left mouse button press in text area: dispatches `ClickAt` with shift state and click count
- On cursor move during active drag: dispatches `DragTo` for drag-to-select
- Added `text_area_drag` flag to track active drag state
- Syncs caret blink and scroll state after all mouse interactions

## Deviations from Plan

None - plan executed exactly as written.

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| Sidebar width hardcoded to 480.0px in adapter | Matches `SIDEBAR_DEFAULT_WIDTH` constant in ora sidebar view; avoids cross-crate coupling |
| `pixel_to_doc` returns `None` for clicks outside text area | Clean separation: sidebar, gutter, tab bar, status bar clicks go through existing hitbox system |
| `char_class` uses 3-way classification (Word/Punctuation/Whitespace) | Matches VS Code word boundary rules for double-click selection |
| DragTo always extends selection (no `extend_selection` parameter) | Drag inherently extends from the initial click anchor; simplifies API |

## Commits

| Hash | Message |
|------|---------|
| 58c14c8 | feat(13-01): add ClickAt/DragTo command variants and dispatcher handlers |
| 9852d15 | feat(13-01): wire mouse click-to-cursor positioning in event loop |

## Key Files

### Created
None (all modifications to existing files).

### Modified
- `ora/src/editor_adapter/types.rs` — ClickAt/DragTo variants in ora EditorCommand
- `ora/src/editor_adapter/mod.rs` — sidebar_width_px, gutter_width_chars, scroll_x in BufferDataSource
- `ora/src/platform/event_loop.rs` — pixel_to_doc helper, mouse click/drag dispatch, text_area_drag state
- `core_editor/src/commands/editor_command.rs` — ClickAt/DragTo variants in core EditorCommand
- `core_editor/src/commands/dispatcher.rs` — handle_click_at, handle_drag_to, find_word_boundaries, char_class
- `wgpu_client/src/adapter.rs` — ClickAt/DragTo routing, sidebar/gutter/scroll_x implementations

## Test Results

- core_editor: 123 passed (was 116, +7 new ClickAt/DragTo tests)
- ora: 163 passed
- Full workspace: 286 passed, 0 failed

## Duration

~8 minutes

## Next Phase Readiness

Plan 13-02 (Drag Selection + Shift-Click) can proceed. The ClickAt/DragTo pipeline is fully operational. The `text_area_drag` flag and `pixel_to_doc` conversion are ready for any refinements needed in subsequent plans.
