---
phase: 13-selection-clipboard
plan: 02
subsystem: selection
tags: [mouse-drag, word-boundary, snap-mode, scroll-while-drag]
depends_on: [13-01]
provides: [drag-selection, word-snap-drag, line-snap-drag, edge-scroll]
affects: [13-04]
tech-stack:
  added: []
  patterns: [snap-mode-dispatch, word-boundary-detection]
key-files:
  created: []
  modified:
    - ora/src/platform/event_loop.rs
    - core_editor/src/domain/text_buffer.rs
    - core_editor/src/commands/dispatcher.rs
    - core_editor/src/commands/editor_command.rs
    - core_editor/src/view/editor_view.rs
    - ora/src/editor_adapter/types.rs
    - wgpu_client/src/adapter.rs
decisions:
  - id: snap-mode-u32
    description: "snap_mode as u32 (0=char, 1=word, 2=line) threaded through DragTo command"
  - id: word-boundary-in-textbuffer
    description: "word_boundary_at on TextBuffer, replacing inline find_word_boundaries in dispatcher"
metrics:
  duration: 8m
  completed: 2026-03-27
---

# Phase 13 Plan 02: Drag Selection + Snap Modes Summary

**One-liner:** Mouse drag selection with 3px threshold, word/line snap modes, and scroll-while-drag at viewport edges.

## What Was Done

### Task 1: Mouse Capture Drag and DragTo Dispatch in Event Loop
- Added `drag_snap_mode`, `drag_start_pos`, `drag_threshold_met` fields to `OraApp`
- Set snap mode from `click_count` on mouse press (1=char, 2=word, 3=line)
- Implemented 3px drag threshold: DragTo commands only dispatch after cursor moves at least 3 pixels from press point, preventing accidental selections on click
- Scroll-while-drag: when dragging within 20px of text area top/bottom edges, dispatch Scroll(-1)/Scroll(1) before DragTo, enabling document scrolling during selection
- Clear all drag state on mouse release

### Task 2: Word Boundary Helper and Snap-Mode Drag in Dispatcher
- Added `TextBuffer::word_boundary_at(offset)` with VS Code-like character classification (alphanumeric+underscore vs punctuation vs whitespace)
- Added `snap_mode: u32` field to `DragTo` command variant across all three layers (ora types, core_editor, wgpu_client adapter)
- Updated `handle_drag_to` for three modes:
  - **Char (0):** extend selection to exact offset (existing behavior)
  - **Word (1):** snap to word boundary; extends toward word_end when dragging forward from anchor, word_start when backward
  - **Line (2):** snap to line boundary; extends to start of next line when forward, start of current line when backward
- Refactored `find_word_boundaries` to delegate to `TextBuffer::word_boundary_at`, removing duplicate `CharClass` enum from dispatcher
- Added `EditorView::selection_anchor()` method for snap-mode direction detection
- 10 new tests total: 6 for `word_boundary_at` (word chars, whitespace, punctuation, underscore, empty buffer, past-end clamp) + 4 for drag modes (char drag, word-snap forward, word-snap backward, line-snap)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added selection_anchor() to EditorView**
- **Found during:** Task 2
- **Issue:** Snap-mode drag needs to know the selection anchor to determine drag direction, but EditorView had no accessor for it
- **Fix:** Added `selection_anchor() -> Option<TextOffset>` that returns the primary selection's anchor
- **Files modified:** `core_editor/src/view/editor_view.rs`

**2. [Rule 3 - Blocking] Removed dead CharClass/char_class code**
- **Found during:** Task 2
- **Issue:** After delegating to `TextBuffer::word_boundary_at`, the `CharClass` enum and `char_class` function in dispatcher.rs became dead code
- **Fix:** Removed and replaced with a comment noting the relocation
- **Files modified:** `core_editor/src/commands/dispatcher.rs`

## Verification

- `cargo build` succeeds with no errors
- `cargo test -p core_editor -- word_boundary` passes 6/6
- `cargo test -p core_editor -- drag_to` passes 4/4
- All pre-existing tests continue to pass

## Commits

| Hash | Description |
|------|-------------|
| 11df1ea | feat(13-02): mouse drag state tracking with threshold and edge scrolling |
| d6dac93 | feat(13-02): word boundary helpers and snap-mode drag selection |
