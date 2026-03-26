# Phase 12: File Operations - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Users can open, save, and create files using OS-native dialogs. Disk I/O never blocks the UI thread. Covers Ctrl+O open, Ctrl+S save, Ctrl+Shift+S Save As, Ctrl+N new buffer, and error handling for non-UTF-8 files. External file watching is out of scope (Phase 14).

</domain>

<decisions>
## Implementation Decisions

### Save feedback & flow
- Silent save feedback — dirty indicator disappears from tab, no other notification
- Save failures (permission denied, disk full) shown as toast notifications (non-blocking)
- Close dirty buffer: three-button dialog — Save / Don't Save / Cancel
- Ctrl+S on untitled buffer opens Save As dialog (no temp file auto-save)

### Untitled buffer behavior
- Naming scheme: "Untitled" for first, "Untitled (2)", "Untitled (3)" for subsequent
- Closing an empty untitled buffer (no changes) closes silently with no prompt
- Ctrl+N opens new untitled buffer and switches to it immediately (becomes active tab)
- Launch behavior unchanged in this phase — keep current startup behavior

### File dialog details
- Open dialog: no file type filters, show all files
- Dialogs remember last opened directory, persisted across sessions
- Ctrl+O supports multi-select — each selected file opens in its own tab
- Save As on untitled buffer: empty filename field (user types from scratch)

### Error presentation
- Non-UTF-8 files: toast error "Cannot open: file is not valid UTF-8", don't create a tab
- File deleted externally while open: keep buffer, mark tab title as "(deleted)", user can Save As
- External modification detection: not in this phase (Phase 14 scope)
- Error severity levels: warning toast (yellow) for recoverable issues, error toast (red) for failures

### Claude's Discretion
- Async I/O implementation approach (threading, channels, etc.)
- Large file loading strategy (streaming vs full read)
- Toast auto-dismiss timing and animation
- Dialog library choice for OS-native file picker
- How to persist last-used directory (config file, registry, etc.)

</decisions>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches.

</specifics>

<deferred>
## Deferred Ideas

- External file modification detection and reload prompt — Phase 14 (File Browser / file watcher)
- Hex editor for binary/non-UTF-8 files — add to backlog

</deferred>

---

*Phase: 12-file-operations*
*Context gathered: 2026-03-26*
