---
phase: 13
plan: 03
subsystem: clipboard
tags: [clipboard, copy, cut, paste, auto-indent, full-line]
depends_on:
  requires: [13-01]
  provides: ["Full-line copy/cut/paste with auto-indent"]
  affects: [13-04]
tech-stack:
  added: []
  patterns: ["clipboard_is_line_copy flag for line-vs-selection clipboard tracking", "auto-indent on multi-line paste"]
key-files:
  created: []
  modified: ["core_editor/src/commands/dispatcher.rs"]
decisions:
  - id: "CLIP-01"
    decision: "clipboard_is_line_copy bool field on CommandDispatcher tracks line-copy state"
    reason: "arboard has no metadata API; internal flag is simplest reliable approach"
  - id: "CLIP-02"
    decision: "Line paste inserts at line_start of current line (above), not at caret offset"
    reason: "Matches VS Code full-line paste behavior"
  - id: "CLIP-03"
    decision: "Auto-indent uses first non-empty paste line as indent reference"
    reason: "Handles code blocks that start with blank lines correctly"
  - id: "CLIP-04"
    decision: "Cut last line also removes preceding newline to avoid trailing blank line"
    reason: "VS Code behavior: cutting last line collapses to previous line end"
  - id: "CLIP-05"
    decision: "Paste tests use set_clipboard_for_test helper to avoid system clipboard race conditions"
    reason: "Parallel test execution causes heap corruption with multiple arboard::Clipboard instances"
metrics:
  duration: "~7m"
  completed: "2026-03-26"
---

# Phase 13 Plan 03: Full-Line Copy/Cut/Paste with Auto-Indent Summary

VS Code-style full-line clipboard operations with clipboard_is_line_copy tracking and auto-indent on multi-line paste.

## Tasks Completed

| Task | Name | Commit | Files Modified |
|------|------|--------|----------------|
| 1 | Full-line copy/cut and line-aware paste with auto-indent | 8e7a7b6 | core_editor/src/commands/dispatcher.rs |

## What Was Built

### clipboard_is_line_copy Flag
Added `clipboard_is_line_copy: bool` field to `CommandDispatcher` that tracks whether the last clipboard write was a full-line copy (no selection) or a selection copy.

### Full-Line Copy (Ctrl+C, no selection)
When no selection is active, `handle_copy` copies the entire current line including a trailing newline and sets `clipboard_is_line_copy = true`.

### Full-Line Cut (Ctrl+X, no selection)
When no selection is active, `handle_cut` copies the entire current line to clipboard and removes it from the document. Special handling for the last line: also removes the preceding newline to prevent a trailing blank line.

### Line-Aware Paste (Ctrl+V)
- **Line paste (is_line_copy && no selection):** Inserts the clipboard text above the current line at line_start offset.
- **Selection paste:** Deletes the selection first, then inserts.
- **Normal paste:** Inserts at caret position with auto-indent.

### Auto-Indent on Paste
`auto_indent_paste` adjusts multi-line paste indentation:
- Single-line paste: no adjustment.
- Multi-line paste: finds the first non-empty line's indentation in the paste text, compares it to the current line's indentation, and re-indents all paste lines accordingly.

## Tests Added

10 new tests (114 total in core_editor):
- `test_copy_no_selection_copies_full_line` -- Verifies clipboard_is_line_copy flag and document unchanged
- `test_copy_with_selection_copies_selection_only` -- Verifies flag is false with selection
- `test_cut_no_selection_removes_entire_line` -- Verifies middle line removal
- `test_cut_no_selection_last_line` -- Verifies last line removal with preceding newline cleanup
- `test_cut_with_selection_cuts_selection` -- Verifies existing selection cut behavior
- `test_paste_line_copy_inserts_above_current_line` -- Verifies line paste above
- `test_paste_with_selection_replaces_selection` -- Verifies selection replacement
- `test_auto_indent_multi_line_paste` -- Verifies multi-line indent adjustment
- `test_auto_indent_single_line_paste_no_change` -- Verifies single-line pass-through
- `test_clipboard_is_line_copy_flag_tracks_correctly` -- Verifies flag state transitions

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] System clipboard race condition in parallel tests**
- **Found during:** Task 1 test execution
- **Issue:** Multiple `arboard::Clipboard` instances created in parallel test threads cause STATUS_HEAP_CORRUPTION on Windows
- **Fix:** Added `set_clipboard_for_test` helper method (cfg(test) only) that sets clipboard text directly; paste tests use this instead of copy-then-paste roundtrip through system clipboard
- **Files modified:** core_editor/src/commands/dispatcher.rs
- **Commit:** 8e7a7b6

## Decisions Made

| ID | Decision | Rationale |
|----|----------|-----------|
| CLIP-01 | clipboard_is_line_copy bool field on CommandDispatcher | arboard has no metadata API; internal flag is simplest |
| CLIP-02 | Line paste inserts at line_start (above current line) | Matches VS Code behavior |
| CLIP-03 | Auto-indent uses first non-empty paste line as reference | Handles code blocks starting with blank lines |
| CLIP-04 | Cut last line removes preceding newline too | Prevents trailing blank line, matches VS Code |
| CLIP-05 | Test helper bypasses system clipboard | Avoids heap corruption from parallel arboard access |

## Next Phase Readiness

No blockers. The clipboard_is_line_copy flag and auto-indent infrastructure are ready for use by future plans. Plan 13-04 (if it exists) can build on these clipboard semantics.
